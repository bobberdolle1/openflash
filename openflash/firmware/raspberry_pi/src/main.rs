//! OpenFlash agent for the Raspberry Pi.
//!
//! Unlike the microcontroller firmware this is a Linux daemon that runs on the
//! Pi itself and drives the flash chip over the Pi's own SPI controller. Hosts
//! reach it over a Unix socket or TCP with `openflash --unix` / `--tcp`.
//!
//! It speaks the shared protocol from `openflash-protocol`, the same framing and
//! the same opcodes as everything else. It previously declared its own opcode
//! table in which `Ping` was `0x00` rather than `0x01`, sent unframed replies
//! with no length or checksum, and answered `0xFF` to every command except ping,
//! version and device-info — the GPIO and SPI modules were never wired to the
//! command handler at all.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;

use log::{error, info, warn};

use openflash_protocol::frame::{Frame, FrameError, MAX_FRAME};
use openflash_protocol::{
    Command, FlashInterface, Platform, Status, VersionInfo, PROTOCOL_VERSION,
};

mod gpio_nand;
mod gpio_spi;

use gpio_spi::{GpioSpi, SpiConfig};

/// Agent version, reported in the `GetVersion` reply.
const VERSION: (u8, u8, u8) = (3, 1, 0);

/// Default Unix socket path.
const SOCKET_PATH: &str = "/tmp/openflash.sock";

/// Largest read this agent will serve in one frame.
const MAX_READ: usize = openflash_protocol::MAX_PAYLOAD;

fn main() {
    env_logger::init();

    info!(
        "OpenFlash Raspberry Pi agent v{}.{}.{}, protocol v{PROTOCOL_VERSION}",
        VERSION.0, VERSION.1, VERSION.2
    );

    match detect_pi_model() {
        Some(model) => info!("Detected {model}"),
        // Not fatal: the agent is useful on any board with a compatible spidev,
        // and refusing to start on an unrecognised revision would be unhelpful.
        None => warn!("Could not identify the board from /proc/cpuinfo; continuing anyway"),
    }

    let mut flash = GpioSpi::new(SpiConfig::default());
    if let Err(error) = flash.init() {
        error!("Cannot open the SPI device: {error}");
        error!("Enable SPI with `raspi-config` and check that /dev/spidev0.0 exists");
        std::process::exit(1);
    }

    let listen_tcp = std::env::var("OPENFLASH_TCP").ok();
    if let Some(address) = listen_tcp {
        serve_tcp(&address, &mut flash);
    } else {
        serve_unix(SOCKET_PATH, &mut flash);
    }
}

fn serve_unix(path: &str, flash: &mut GpioSpi) {
    if Path::new(path).exists() {
        // A leftover socket from a previous run would make bind fail.
        if let Err(error) = std::fs::remove_file(path) {
            error!("Cannot remove the stale socket {path}: {error}");
            std::process::exit(1);
        }
    }

    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(error) => {
            error!("Cannot bind {path}: {error}");
            std::process::exit(1);
        }
    };
    info!("Listening on {path}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("Client connected");
                serve_client(stream, flash);
                info!("Client disconnected");
            }
            Err(error) => warn!("Connection failed: {error}"),
        }
    }
}

fn serve_tcp(address: &str, flash: &mut GpioSpi) {
    let listener = match TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(error) => {
            error!("Cannot bind {address}: {error}");
            std::process::exit(1);
        }
    };
    info!("Listening on {address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".into());
                info!("Client connected from {peer}");
                serve_client(stream, flash);
                info!("Client {peer} disconnected");
            }
            Err(error) => warn!("Connection failed: {error}"),
        }
    }
}

/// Read frames from one client until it goes away.
fn serve_client<S: Read + Write>(mut stream: S, flash: &mut GpioSpi) {
    let mut buffer: Vec<u8> = Vec::with_capacity(MAX_FRAME);
    let mut chunk = [0u8; 4096];

    loop {
        // Serve every complete frame already buffered before reading more: a
        // client may pipeline requests, and one read may carry several.
        loop {
            match Frame::decode(&buffer) {
                Ok(frame) => {
                    let consumed = frame.encoded_len();
                    let response = handle_frame(&frame, flash);
                    buffer.drain(..consumed);
                    if stream.write_all(&response).is_err() {
                        return;
                    }
                    let _ = stream.flush();
                }
                Err(FrameError::Incomplete { .. }) => break,
                // Corrupt input: resynchronise on the next magic rather than
                // dropping the connection, and say so.
                Err(error) => {
                    warn!("Discarding a malformed frame: {error}");
                    match Frame::find(&buffer[1..]) {
                        Ok((skipped, _)) => {
                            buffer.drain(..1 + skipped);
                        }
                        Err(_) => {
                            buffer.clear();
                            break;
                        }
                    }
                }
            }
        }

        match stream.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(error) => {
                warn!("Read error: {error}");
                return;
            }
        }

        if buffer.len() > MAX_FRAME * 2 {
            warn!("Client sent more than two frames' worth of garbage; dropping it");
            return;
        }
    }
}

fn version_info() -> VersionInfo {
    VersionInfo {
        protocol: PROTOCOL_VERSION,
        firmware: VERSION,
        platform: Some(Platform::RaspberryPi),
        // Only SPI NOR is wired up. Claiming more would make a host offer
        // operations this agent cannot perform.
        interfaces: VersionInfo::bitmap_of(&[FlashInterface::SpiNor]),
    }
}

/// Turn one request frame into the bytes of a response frame.
fn handle_frame(frame: &Frame<'_>, flash: &mut GpioSpi) -> Vec<u8> {
    let Some(command) = frame.decoded_command() else {
        return encode(Frame::response_raw(
            frame.command,
            Status::UnsupportedCommand,
            &[],
        ));
    };

    let (status, payload) = dispatch(command, frame.payload, flash);
    encode(Frame::response(command, status, &payload))
}

fn encode(frame: Frame<'_>) -> Vec<u8> {
    frame.encode_to_vec().unwrap_or_else(|error| {
        // Only possible if a handler built an oversized payload, which is a bug
        // here rather than something the client did.
        error!("Cannot encode a response: {error}");
        Frame::response_raw(0x00, Status::Error, &[])
            .encode_to_vec()
            .expect("an empty payload always encodes")
    })
}

fn dispatch(command: Command, payload: &[u8], flash: &mut GpioSpi) -> (Status, Vec<u8>) {
    match command {
        Command::Ping => (Status::Ok, Vec::new()),
        Command::GetVersion => (Status::Ok, version_info().to_bytes().to_vec()),
        Command::GetCapabilities => {
            // An agent whose SPI device could not be opened advertises nothing,
            // so a host reports "no interfaces" instead of failing every
            // operation one at a time.
            let interfaces = if flash.is_initialized() {
                version_info().interfaces
            } else {
                0
            };
            (Status::Ok, vec![interfaces])
        }
        Command::BusConfig => (Status::Ok, Vec::new()),
        Command::Reset | Command::SpiNorReset => match flash.write_disable() {
            Ok(()) => (Status::Ok, Vec::new()),
            Err(error) => spi_failure(error),
        },
        Command::SetInterface => match payload.first().copied().and_then(FlashInterface::from_u8) {
            Some(FlashInterface::SpiNor) => (Status::Ok, Vec::new()),
            Some(_) => (Status::UnsupportedOnInterface, Vec::new()),
            None => (Status::InvalidArgument, Vec::new()),
        },

        Command::SpiNorReadJedecId => match flash.read_jedec_id() {
            Ok(id) => (Status::Ok, id.to_vec()),
            Err(error) => spi_failure(error),
        },
        Command::SpiNorReadStatus1 => match flash.read_status() {
            Ok(status) => (Status::Ok, vec![status]),
            Err(error) => spi_failure(error),
        },
        Command::SpiNorWriteEnable => match flash.write_enable() {
            Ok(()) => (Status::Ok, Vec::new()),
            Err(error) => spi_failure(error),
        },
        Command::SpiNorWriteDisable => match flash.write_disable() {
            Ok(()) => (Status::Ok, Vec::new()),
            Err(error) => spi_failure(error),
        },

        // `[address: u32 LE][length: u16 LE]`
        Command::SpiNorRead | Command::SpiNorFastRead => {
            let Some((address, length)) = parse_address_and_length(payload) else {
                return (Status::InvalidArgument, Vec::new());
            };
            if length == 0 || length > MAX_READ {
                return (Status::InvalidArgument, Vec::new());
            }
            let mut data = vec![0u8; length];
            match flash.read(address, &mut data) {
                Ok(()) => (Status::Ok, data),
                Err(error) => spi_failure(error),
            }
        }

        // `[address: u32 LE][data…]`
        Command::SpiNorPageProgram => {
            if payload.len() < 5 {
                return (Status::InvalidArgument, Vec::new());
            }
            let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            match flash.page_program(address, &payload[4..]) {
                Ok(()) => (Status::Ok, Vec::new()),
                Err(error) => spi_failure(error),
            }
        }

        // `[address: u32 LE]`
        Command::SpiNorSectorErase => erase(flash, payload, 4096),
        Command::SpiNorBlockErase32K => erase(flash, payload, 32 * 1024),
        Command::SpiNorBlockErase64K => erase(flash, payload, 64 * 1024),
        Command::SpiNorChipErase => match flash.chip_erase() {
            Ok(()) => (Status::Ok, Vec::new()),
            Err(error) => spi_failure(error),
        },

        // Everything else names an interface this agent does not drive.
        other if other.interface().is_some() => (Status::UnsupportedOnInterface, Vec::new()),
        _ => (Status::UnsupportedCommand, Vec::new()),
    }
}

fn erase(flash: &mut GpioSpi, payload: &[u8], granularity: usize) -> (Status, Vec<u8>) {
    if payload.len() < 4 {
        return (Status::InvalidArgument, Vec::new());
    }
    let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    match flash.erase(address, granularity) {
        Ok(()) => (Status::Ok, Vec::new()),
        Err(error) => spi_failure(error),
    }
}

fn parse_address_and_length(payload: &[u8]) -> Option<(u32, usize)> {
    if payload.len() < 6 {
        return None;
    }
    let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
    Some((address, length))
}

/// Map an SPI failure onto a protocol status, keeping the distinction between
/// "the request was wrong" and "the chip did not answer".
fn spi_failure(error: gpio_spi::SpiError) -> (Status, Vec<u8>) {
    use gpio_spi::SpiError;

    warn!("SPI operation failed: {error}");
    let status = match error {
        SpiError::PageOverrun { .. } | SpiError::Unaligned { .. } => Status::InvalidArgument,
        SpiError::Timeout(_) => Status::Timeout,
        SpiError::NotInitialized | SpiError::Spi(_) => Status::Error,
    };
    (status, Vec::new())
}

/// Identify the board from `/proc/cpuinfo`.
fn detect_pi_model() -> Option<&'static str> {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    model_from_cpuinfo(&cpuinfo)
}

fn model_from_cpuinfo(cpuinfo: &str) -> Option<&'static str> {
    // Ordered longest-prefix first so BCM2710 does not match before BCM2711.
    const MODELS: &[(&str, &str)] = &[
        ("BCM2712", "Raspberry Pi 5"),
        ("BCM2711", "Raspberry Pi 4"),
        ("BCM2837", "Raspberry Pi 3B+"),
        ("BCM2710", "Raspberry Pi Zero 2W"),
        ("BCM2835", "Raspberry Pi Zero / 1"),
    ];

    MODELS
        .iter()
        .find(|(soc, _)| cpuinfo.contains(soc))
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The framing the host expects, exercised without any hardware: a request
    /// this agent cannot serve must still get a well-formed, correctly attributed
    /// response.
    fn respond(command: Command, payload: &[u8]) -> Frame<'static> {
        let mut flash = GpioSpi::new(SpiConfig::default());
        let request = Frame::request(command, payload).encode_to_vec().unwrap();
        let decoded = Frame::decode(&request).unwrap();
        let response = handle_frame(&decoded, &mut flash);

        // Leak so the returned frame can borrow it; only ever used in tests.
        let leaked: &'static [u8] = Box::leak(response.into_boxed_slice());
        Frame::decode(leaked).expect("the agent must always emit a valid frame")
    }

    #[test]
    fn ping_is_answered_without_touching_the_chip() {
        let frame = respond(Command::Ping, &[]);
        assert_eq!(frame.decoded_command(), Some(Command::Ping));
        assert_eq!(frame.decoded_status(), Some(Status::Ok));
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn the_version_reply_is_what_the_host_parses() {
        let frame = respond(Command::GetVersion, &[]);
        let info = VersionInfo::from_bytes(frame.payload).expect("a six-byte version payload");

        assert_eq!(info.protocol, PROTOCOL_VERSION);
        assert_eq!(info.firmware, VERSION);
        assert_eq!(info.platform, Some(Platform::RaspberryPi));
        assert!(info.supports(FlashInterface::SpiNor));
    }

    /// Only SPI NOR is wired up, so the agent must not claim the rest: a host
    /// that believed it could would offer operations that cannot work.
    #[test]
    fn only_the_interfaces_that_are_wired_up_are_advertised() {
        let info = version_info();
        assert!(info.supports(FlashInterface::SpiNor));
        assert!(!info.supports(FlashInterface::ParallelNand));
        assert!(!info.supports(FlashInterface::SpiNand));
        assert!(!info.supports(FlashInterface::Emmc));
        assert!(!info.supports(FlashInterface::Ufs));
    }

    #[test]
    fn commands_for_other_interfaces_are_refused() {
        for command in [
            Command::NandReadPage,
            Command::SpiNandReadId,
            Command::EmmcReadBlock,
            Command::UfsRead10,
        ] {
            let frame = respond(command, &[0; 8]);
            assert_eq!(
                frame.decoded_status(),
                Some(Status::UnsupportedOnInterface),
                "{command:?} should be refused"
            );
            assert_eq!(frame.command, command as u8, "the echo must match");
        }
    }

    #[test]
    fn an_unknown_opcode_is_echoed_back_as_unsupported() {
        let mut flash = GpioSpi::new(SpiConfig::default());
        // 0xB5 is in the reserved host-side range.
        let request = Frame::response_raw(0xB5, Status::Ok, &[])
            .encode_to_vec()
            .unwrap();
        let decoded = Frame::decode(&request).unwrap();
        let response = handle_frame(&decoded, &mut flash);
        let frame = Frame::decode(&response).unwrap();

        assert_eq!(frame.command, 0xB5);
        assert_eq!(frame.decoded_status(), Some(Status::UnsupportedCommand));
    }

    #[test]
    fn a_malformed_read_request_is_rejected_as_an_invalid_argument() {
        // Four bytes: an address with no length.
        let frame = respond(Command::SpiNorRead, &[0, 0, 0, 0]);
        assert_eq!(frame.decoded_status(), Some(Status::InvalidArgument));
    }

    #[test]
    fn a_zero_length_read_is_rejected() {
        let mut payload = 0u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&0u16.to_le_bytes());
        let frame = respond(Command::SpiNorRead, &payload);
        assert_eq!(frame.decoded_status(), Some(Status::InvalidArgument));
    }

    #[test]
    fn a_read_larger_than_a_frame_payload_is_rejected() {
        let mut payload = 0u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&u16::MAX.to_le_bytes());
        let frame = respond(Command::SpiNorRead, &payload);
        assert_eq!(frame.decoded_status(), Some(Status::InvalidArgument));
    }

    /// A well-formed request that the chip cannot serve — because there is no
    /// chip in CI — must come back as an error, not as an empty success.
    #[test]
    fn a_well_formed_request_without_hardware_reports_an_error() {
        let frame = respond(Command::SpiNorReadJedecId, &[]);
        assert_eq!(frame.decoded_status(), Some(Status::Error));
    }

    #[test]
    fn an_unaligned_erase_is_reported_as_an_invalid_argument() {
        let frame = respond(Command::SpiNorSectorErase, &100u32.to_le_bytes());
        assert_eq!(frame.decoded_status(), Some(Status::InvalidArgument));
    }

    #[test]
    fn parsing_an_address_and_length_needs_six_bytes() {
        assert_eq!(parse_address_and_length(&[1, 2, 3, 4, 5]), None);
        let mut payload = 0x1234u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&256u16.to_le_bytes());
        assert_eq!(parse_address_and_length(&payload), Some((0x1234, 256)));
    }

    #[test]
    fn board_detection_prefers_the_longer_soc_match() {
        assert_eq!(
            model_from_cpuinfo("Hardware\t: BCM2711\n"),
            Some("Raspberry Pi 4")
        );
        assert_eq!(
            model_from_cpuinfo("Hardware\t: BCM2710A1\n"),
            Some("Raspberry Pi Zero 2W")
        );
        assert_eq!(model_from_cpuinfo("Hardware\t: SomethingElse\n"), None);
    }

    #[test]
    fn a_client_can_pipeline_two_requests_in_one_write() {
        // Exercises the buffering in serve_client: two frames arriving together
        // must both be answered, in order.
        struct Pipe {
            input: Vec<u8>,
            position: usize,
            output: Vec<u8>,
        }

        impl Read for Pipe {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let remaining = self.input.len() - self.position;
                if remaining == 0 {
                    return Ok(0);
                }
                let take = remaining.min(buffer.len());
                buffer[..take].copy_from_slice(&self.input[self.position..self.position + take]);
                self.position += take;
                Ok(take)
            }
        }

        impl Write for Pipe {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.output.extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut input = Frame::request(Command::Ping, &[]).encode_to_vec().unwrap();
        input.extend_from_slice(
            &Frame::request(Command::GetVersion, &[])
                .encode_to_vec()
                .unwrap(),
        );

        let mut pipe = Pipe {
            input,
            position: 0,
            output: Vec::new(),
        };
        let mut flash = GpioSpi::new(SpiConfig::default());
        serve_client(&mut pipe, &mut flash);

        let first = Frame::decode(&pipe.output).expect("a reply to the ping");
        assert_eq!(first.decoded_command(), Some(Command::Ping));

        let second =
            Frame::decode(&pipe.output[first.encoded_len()..]).expect("a reply to GetVersion");
        assert_eq!(second.decoded_command(), Some(Command::GetVersion));
    }

    #[test]
    fn garbage_before_a_frame_is_skipped_and_the_frame_still_answered() {
        struct Pipe {
            input: Vec<u8>,
            position: usize,
            output: Vec<u8>,
        }

        impl Read for Pipe {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let remaining = self.input.len() - self.position;
                if remaining == 0 {
                    return Ok(0);
                }
                let take = remaining.min(buffer.len());
                buffer[..take].copy_from_slice(&self.input[self.position..self.position + take]);
                self.position += take;
                Ok(take)
            }
        }

        impl Write for Pipe {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.output.extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        // Enough leading noise to fill a header's worth, then a real frame.
        let mut input = vec![0xAA; openflash_protocol::frame::HEADER_LEN + 4];
        input.extend_from_slice(&Frame::request(Command::Ping, &[]).encode_to_vec().unwrap());

        let mut pipe = Pipe {
            input,
            position: 0,
            output: Vec::new(),
        };
        let mut flash = GpioSpi::new(SpiConfig::default());
        serve_client(&mut pipe, &mut flash);

        let frame = Frame::decode(&pipe.output).expect("the real frame must still be answered");
        assert_eq!(frame.decoded_command(), Some(Command::Ping));
    }

    #[test]
    fn capabilities_are_empty_when_the_spi_device_could_not_be_opened() {
        // No /dev/spidev in CI, so the agent must report no usable interface
        // rather than advertising SPI NOR and failing every request.
        let frame = respond(Command::GetCapabilities, &[]);
        assert_eq!(frame.decoded_status(), Some(Status::Ok));
        assert_eq!(frame.payload, &[0]);
    }
}
