//! The protocol side of an agent: frames in, frames out.
//!
//! Shared by every single-board-computer agent. A board supplies a [`SpiBus`]
//! and a [`Platform`]; everything about the wire format, request validation and
//! error mapping lives here, so the three agents cannot drift apart the way the
//! firmware opcode tables did.

use std::io::{Read, Write};

use log::warn;

use openflash_protocol::frame::{Frame, FrameError, MAX_FRAME};
use openflash_protocol::{
    Command, FlashInterface, Platform, Status, VersionInfo, PROTOCOL_VERSION,
};

use crate::bus::SpiBus;
use crate::nor::{NorError, SpiNor};

/// Largest read served in one frame.
pub const MAX_READ: usize = openflash_protocol::MAX_PAYLOAD;

/// An agent: a flash chip on a bus, plus the identity it reports.
pub struct Agent<B: SpiBus> {
    flash: SpiNor<B>,
    platform: Platform,
    version: (u8, u8, u8),
}

impl<B: SpiBus> Agent<B> {
    /// Build an agent for `platform` around `flash`.
    pub fn new(flash: SpiNor<B>, platform: Platform, version: (u8, u8, u8)) -> Self {
        Self {
            flash,
            platform,
            version,
        }
    }

    /// What this agent reports for `GetVersion`.
    ///
    /// Only SPI NOR is advertised, and nothing at all when the bus could not be
    /// opened: a host uses this to decide what to offer, so over-claiming turns
    /// into operations that fail one at a time.
    pub fn version_info(&self) -> VersionInfo {
        let interfaces = if self.flash.is_available() {
            VersionInfo::bitmap_of(&[FlashInterface::SpiNor])
        } else {
            0
        };

        VersionInfo {
            protocol: PROTOCOL_VERSION,
            firmware: self.version,
            platform: Some(self.platform),
            interfaces,
        }
    }

    /// Turn one request frame into the bytes of a response frame.
    pub fn handle_frame(&mut self, frame: &Frame<'_>) -> Vec<u8> {
        let Some(command) = frame.decoded_command() else {
            return encode(Frame::response_raw(
                frame.command,
                Status::UnsupportedCommand,
                &[],
            ));
        };

        let (status, payload) = self.dispatch(command, frame.payload);
        encode(Frame::response(command, status, &payload))
    }

    /// Handle a decoded command.
    pub fn dispatch(&mut self, command: Command, payload: &[u8]) -> (Status, Vec<u8>) {
        match command {
            Command::Ping => (Status::Ok, Vec::new()),
            Command::GetVersion => (Status::Ok, self.version_info().to_bytes().to_vec()),
            Command::GetCapabilities => (Status::Ok, vec![self.version_info().interfaces]),
            // Accepted and ignored: the bus speed is set when the agent opens the
            // device, and a host asking for a different one is not an error.
            Command::BusConfig => (Status::Ok, Vec::new()),
            Command::Reset | Command::SpiNorReset => match self.flash.write_disable() {
                Ok(()) => (Status::Ok, Vec::new()),
                Err(error) => failure(error),
            },
            Command::SetInterface => {
                match payload.first().copied().and_then(FlashInterface::from_u8) {
                    Some(FlashInterface::SpiNor) => (Status::Ok, Vec::new()),
                    Some(_) => (Status::UnsupportedOnInterface, Vec::new()),
                    None => (Status::InvalidArgument, Vec::new()),
                }
            }

            Command::SpiNorReadJedecId => match self.flash.read_jedec_id() {
                Ok(id) => (Status::Ok, id.to_vec()),
                Err(error) => failure(error),
            },
            Command::SpiNorReadStatus1 => match self.flash.read_status() {
                Ok(status) => (Status::Ok, vec![status]),
                Err(error) => failure(error),
            },
            Command::SpiNorWriteEnable => match self.flash.write_enable() {
                Ok(()) => (Status::Ok, Vec::new()),
                Err(error) => failure(error),
            },
            Command::SpiNorWriteDisable => match self.flash.write_disable() {
                Ok(()) => (Status::Ok, Vec::new()),
                Err(error) => failure(error),
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
                match self.flash.read(address, &mut data) {
                    Ok(()) => (Status::Ok, data),
                    Err(error) => failure(error),
                }
            }

            // `[address: u32 LE][data…]`
            Command::SpiNorPageProgram => {
                if payload.len() < 5 {
                    return (Status::InvalidArgument, Vec::new());
                }
                let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                match self.flash.page_program(address, &payload[4..]) {
                    Ok(()) => (Status::Ok, Vec::new()),
                    Err(error) => failure(error),
                }
            }

            // `[address: u32 LE]`
            Command::SpiNorSectorErase => self.erase(payload, 4096),
            Command::SpiNorBlockErase32K => self.erase(payload, 32 * 1024),
            Command::SpiNorBlockErase64K => self.erase(payload, 64 * 1024),
            Command::SpiNorChipErase => match self.flash.chip_erase() {
                Ok(()) => (Status::Ok, Vec::new()),
                Err(error) => failure(error),
            },

            // Everything else names an interface these agents do not drive.
            other if other.interface().is_some() => (Status::UnsupportedOnInterface, Vec::new()),
            _ => (Status::UnsupportedCommand, Vec::new()),
        }
    }

    fn erase(&mut self, payload: &[u8], granularity: usize) -> (Status, Vec<u8>) {
        if payload.len() < 4 {
            return (Status::InvalidArgument, Vec::new());
        }
        let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        match self.flash.erase(address, granularity) {
            Ok(()) => (Status::Ok, Vec::new()),
            Err(error) => failure(error),
        }
    }

    /// Serve one client until it disconnects.
    ///
    /// Buffers rather than assuming one read is one frame: a client may pipeline
    /// requests, and a stream splits wherever it likes.
    pub fn serve_client<S: Read + Write>(&mut self, mut stream: S) {
        let mut buffer: Vec<u8> = Vec::with_capacity(MAX_FRAME);
        let mut chunk = [0u8; 4096];

        loop {
            // Answer everything already buffered before reading more.
            loop {
                match Frame::decode(&buffer) {
                    Ok(frame) => {
                        let consumed = frame.encoded_len();
                        let response = self.handle_frame(&frame);
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
                warn!("Client sent more than two frames' worth of unusable data; dropping it");
                return;
            }
        }
    }
}

fn encode(frame: Frame<'_>) -> Vec<u8> {
    frame.encode_to_vec().unwrap_or_else(|error| {
        // Only reachable if a handler built an oversized payload, which is a bug
        // here rather than something the client did.
        warn!("Cannot encode a response: {error}");
        Frame::response_raw(0x00, Status::Error, &[])
            .encode_to_vec()
            .expect("an empty payload always encodes")
    })
}

/// Map a SPI NOR failure onto a protocol status.
///
/// Keeps the distinction between "the request was wrong" and "the chip did not
/// answer": a host retries the second and fixes the first.
fn failure(error: NorError) -> (Status, Vec<u8>) {
    warn!("SPI NOR operation failed: {error}");
    let status = match error {
        NorError::PageOverrun { .. }
        | NorError::Unaligned { .. }
        | NorError::UnsupportedGranularity { .. } => Status::InvalidArgument,
        NorError::Timeout { .. } => Status::Timeout,
        NorError::Bus(_) => Status::Error,
    };
    (status, Vec::new())
}

fn parse_address_and_length(payload: &[u8]) -> Option<(u32, usize)> {
    if payload.len() < 6 {
        return None;
    }
    let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
    Some((address, length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::fake::FakeChip;
    use std::time::Duration;

    fn agent(size: usize) -> Agent<FakeChip> {
        let flash = SpiNor::new(FakeChip::new(size)).with_poll_interval(Duration::ZERO);
        Agent::new(flash, Platform::OrangePi, (3, 1, 0))
    }

    /// Round-trip a request through the framing, as a host would.
    fn request(agent: &mut Agent<FakeChip>, command: Command, payload: &[u8]) -> (Status, Vec<u8>) {
        let bytes = Frame::request(command, payload).encode_to_vec().unwrap();
        let frame = Frame::decode(&bytes).unwrap();
        let response = agent.handle_frame(&frame);

        let decoded = Frame::decode(&response).expect("the agent must emit a valid frame");
        assert_eq!(decoded.command, command as u8, "the echo must match");
        (
            decoded.decoded_status().expect("a known status"),
            decoded.payload.to_vec(),
        )
    }

    #[test]
    fn ping_is_answered() {
        let mut agent = agent(64 * 1024);
        let (status, payload) = request(&mut agent, Command::Ping, &[]);
        assert_eq!(status, Status::Ok);
        assert!(payload.is_empty());
    }

    #[test]
    fn the_version_reply_is_what_a_host_parses() {
        let mut agent = agent(64 * 1024);
        let (status, payload) = request(&mut agent, Command::GetVersion, &[]);
        assert_eq!(status, Status::Ok);

        let info = VersionInfo::from_bytes(&payload).expect("a six-byte version payload");
        assert_eq!(info.protocol, PROTOCOL_VERSION);
        assert_eq!(info.firmware, (3, 1, 0));
        assert_eq!(info.platform, Some(Platform::OrangePi));
        assert!(info.supports(FlashInterface::SpiNor));
    }

    #[test]
    fn only_spi_nor_is_advertised() {
        let agent = agent(64 * 1024);
        let info = agent.version_info();
        assert!(info.supports(FlashInterface::SpiNor));
        for other in [
            FlashInterface::ParallelNand,
            FlashInterface::SpiNand,
            FlashInterface::Emmc,
            FlashInterface::Ufs,
        ] {
            assert!(!info.supports(other), "{other:?} must not be advertised");
        }
    }

    #[test]
    fn an_agent_with_no_usable_bus_advertises_nothing() {
        let mut fake = FakeChip::new(64 * 1024);
        fake.available = false;
        let flash = SpiNor::new(fake).with_poll_interval(Duration::ZERO);
        let agent = Agent::new(flash, Platform::BananaPi, (3, 1, 0));

        assert_eq!(agent.version_info().interfaces, 0);
    }

    #[test]
    fn the_chip_id_is_reported() {
        let mut agent = agent(2 * 1024 * 1024);
        let (status, payload) = request(&mut agent, Command::SpiNorReadJedecId, &[]);
        assert_eq!(status, Status::Ok);
        // 2 MiB is 2^21, so the capacity byte is 0x15 — a W25Q16JV id.
        assert_eq!(payload, vec![0xEF, 0x40, 0x15]);
    }

    /// The whole point: a host writing through the protocol gets the same bytes
    /// back.
    #[test]
    fn a_program_then_read_round_trips_through_the_protocol() {
        let mut agent = agent(64 * 1024);
        let payload: Vec<u8> = (0..200u32).map(|i| (i * 7 % 251) as u8).collect();

        let mut program = 0u32.to_le_bytes().to_vec();
        program.extend_from_slice(&payload);
        let (status, _) = request(&mut agent, Command::SpiNorPageProgram, &program);
        assert_eq!(status, Status::Ok);

        let mut read = 0u32.to_le_bytes().to_vec();
        read.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        let (status, data) = request(&mut agent, Command::SpiNorRead, &read);
        assert_eq!(status, Status::Ok);
        assert_eq!(data, payload);
    }

    #[test]
    fn erase_blanks_the_sector() {
        let mut agent = agent(64 * 1024);

        let mut program = 0u32.to_le_bytes().to_vec();
        program.extend_from_slice(&[0x00; 16]);
        request(&mut agent, Command::SpiNorPageProgram, &program);

        let (status, _) = request(&mut agent, Command::SpiNorSectorErase, &0u32.to_le_bytes());
        assert_eq!(status, Status::Ok);

        let mut read = 0u32.to_le_bytes().to_vec();
        read.extend_from_slice(&16u16.to_le_bytes());
        let (_, data) = request(&mut agent, Command::SpiNorRead, &read);
        assert_eq!(data, vec![0xFF; 16]);
    }

    #[test]
    fn a_malformed_read_request_is_rejected() {
        let mut agent = agent(64 * 1024);
        // An address with no length.
        let (status, _) = request(&mut agent, Command::SpiNorRead, &[0, 0, 0, 0]);
        assert_eq!(status, Status::InvalidArgument);
    }

    #[test]
    fn a_zero_length_or_oversized_read_is_rejected() {
        let mut agent = agent(64 * 1024);

        for length in [0u16, u16::MAX] {
            let mut payload = 0u32.to_le_bytes().to_vec();
            payload.extend_from_slice(&length.to_le_bytes());
            let (status, _) = request(&mut agent, Command::SpiNorRead, &payload);
            assert_eq!(status, Status::InvalidArgument, "length {length}");
        }
    }

    #[test]
    fn an_unaligned_erase_is_rejected() {
        let mut agent = agent(64 * 1024);
        let (status, _) = request(
            &mut agent,
            Command::SpiNorSectorErase,
            &100u32.to_le_bytes(),
        );
        assert_eq!(status, Status::InvalidArgument);
    }

    #[test]
    fn a_program_crossing_a_page_boundary_is_rejected() {
        let mut agent = agent(64 * 1024);
        let mut payload = 252u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&[0u8; 10]);

        let (status, _) = request(&mut agent, Command::SpiNorPageProgram, &payload);
        assert_eq!(status, Status::InvalidArgument);
    }

    #[test]
    fn commands_for_other_interfaces_are_refused() {
        let mut agent = agent(64 * 1024);
        for command in [
            Command::NandReadPage,
            Command::SpiNandReadId,
            Command::EmmcReadBlock,
            Command::UfsRead10,
        ] {
            let (status, _) = request(&mut agent, command, &[0; 8]);
            assert_eq!(
                status,
                Status::UnsupportedOnInterface,
                "{command:?} should be refused"
            );
        }
    }

    #[test]
    fn an_unknown_opcode_is_echoed_back_as_unsupported() {
        let mut agent = agent(64 * 1024);
        // 0xB5 is in the reserved host-side range.
        let bytes = Frame::response_raw(0xB5, Status::Ok, &[])
            .encode_to_vec()
            .unwrap();
        let frame = Frame::decode(&bytes).unwrap();
        let response = agent.handle_frame(&frame);
        let decoded = Frame::decode(&response).unwrap();

        assert_eq!(decoded.command, 0xB5);
        assert_eq!(decoded.decoded_status(), Some(Status::UnsupportedCommand));
    }

    #[test]
    fn selecting_an_interface_the_agent_lacks_is_refused() {
        let mut agent = agent(64 * 1024);

        let (status, _) = request(
            &mut agent,
            Command::SetInterface,
            &[FlashInterface::SpiNor as u8],
        );
        assert_eq!(status, Status::Ok);

        let (status, _) = request(
            &mut agent,
            Command::SetInterface,
            &[FlashInterface::Emmc as u8],
        );
        assert_eq!(status, Status::UnsupportedOnInterface);

        let (status, _) = request(&mut agent, Command::SetInterface, &[0x7F]);
        assert_eq!(status, Status::InvalidArgument);
    }

    /// A well-formed request that the bus cannot serve must come back as an
    /// error, not as an empty success.
    #[test]
    fn a_request_on_a_dead_bus_reports_an_error() {
        let mut fake = FakeChip::new(64 * 1024);
        fake.available = false;
        let flash = SpiNor::new(fake).with_poll_interval(Duration::ZERO);
        let mut agent = Agent::new(flash, Platform::OrangePi, (3, 1, 0));

        let (status, _) = request(&mut agent, Command::SpiNorReadJedecId, &[]);
        assert_eq!(status, Status::Error);
    }

    /// A pipe that hands over the input in one go and records the output.
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

    #[test]
    fn two_pipelined_requests_are_both_answered_in_order() {
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
        agent(64 * 1024).serve_client(&mut pipe);

        let first = Frame::decode(&pipe.output).expect("a reply to the ping");
        assert_eq!(first.decoded_command(), Some(Command::Ping));

        let second =
            Frame::decode(&pipe.output[first.encoded_len()..]).expect("a reply to GetVersion");
        assert_eq!(second.decoded_command(), Some(Command::GetVersion));
    }

    #[test]
    fn garbage_before_a_frame_is_skipped_and_the_frame_still_answered() {
        let mut input = vec![0xAAu8; openflash_protocol::frame::HEADER_LEN + 4];
        input.extend_from_slice(&Frame::request(Command::Ping, &[]).encode_to_vec().unwrap());

        let mut pipe = Pipe {
            input,
            position: 0,
            output: Vec::new(),
        };
        agent(64 * 1024).serve_client(&mut pipe);

        let frame = Frame::decode(&pipe.output).expect("the real frame must still be answered");
        assert_eq!(frame.decoded_command(), Some(Command::Ping));
    }

    #[test]
    fn a_client_that_sends_nothing_is_dropped_without_a_reply() {
        let mut pipe = Pipe {
            input: Vec::new(),
            position: 0,
            output: Vec::new(),
        };
        agent(64 * 1024).serve_client(&mut pipe);
        assert!(pipe.output.is_empty());
    }
}
