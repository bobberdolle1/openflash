//! Talking to a device.
//!
//! Everything above this module — chip identification, dumping, programming —
//! is expressed in terms of the [`Transport`] trait, so the same code drives a
//! USB device, an SBC agent over TCP or a Unix socket, or the in-process
//! [`crate::emulator`] used by the test suite.
//!
//! A transport moves *frames*, not bytes. [`Transport::transact`] sends a
//! request frame and returns the payload of the matching response, and it is
//! responsible for the parts that are easy to get wrong:
//!
//! - it reads until a complete frame has arrived rather than assuming one
//!   packet is one frame,
//! - it verifies both CRCs and reports corruption instead of returning damaged
//!   data as if it were flash contents,
//! - it checks that the response echoes the command it asked for, so a stale
//!   reply can never be mistaken for the answer to the current request,
//! - it turns a non-`Ok` status into an error rather than a successful read of
//!   whatever happened to be in the buffer.

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use openflash_protocol::frame::{Frame, FrameError, HEADER_LEN, MAX_FRAME};
use openflash_protocol::{Command, Status};

#[cfg(feature = "usb")]
pub mod usb;

#[cfg(feature = "usb")]
pub use usb::{list_devices, UsbDeviceInfo, UsbTransport};

/// Default time to wait for a device to answer.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Time to wait for operations that erase, which are slow on real chips.
///
/// A full-chip erase on a large SPI NOR part can take well over a minute.
pub const ERASE_TIMEOUT: Duration = Duration::from_secs(120);

/// Something went wrong talking to a device.
#[derive(Debug)]
pub enum TransportError {
    /// Underlying I/O failure.
    Io(io::Error),
    /// The link produced bytes that are not a valid frame.
    Frame(FrameError),
    /// The device answered with a failure status.
    Device {
        /// Command that failed.
        command: Command,
        /// Status the device reported.
        status: Status,
    },
    /// The device answered with a status byte this build does not know.
    UnknownStatus {
        /// Command that was sent.
        command: Command,
        /// Raw status byte.
        status: u8,
    },
    /// The response echoed a different command than the one that was sent.
    ///
    /// This means the link is out of step — a previous reply was never
    /// consumed, most likely. Continuing would attribute one command's data to
    /// another, so it is always an error.
    CommandMismatch {
        /// Command that was sent.
        sent: Command,
        /// Command byte the device echoed.
        received: u8,
    },
    /// The device did not answer within the timeout.
    Timeout {
        /// Command that was sent.
        command: Command,
        /// How long the host waited.
        waited: Duration,
    },
    /// The device returned a payload of an unusable size.
    UnexpectedPayload {
        /// Command that was sent.
        command: Command,
        /// Bytes expected, when a fixed size was required.
        expected: usize,
        /// Bytes received.
        received: usize,
    },
    /// The link dropped bytes that had to be skipped to regain frame sync.
    ///
    /// Reported rather than silently tolerated: a link that loses bytes is a
    /// link whose dumps cannot be trusted.
    Desynchronised {
        /// Number of bytes discarded before a valid frame was found.
        skipped: usize,
    },
    /// This transport cannot do what was asked of it.
    Unsupported(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Frame(e) => write!(f, "malformed frame: {e}"),
            Self::Device { command, status } => {
                write!(f, "device rejected {command:?}: {}", status.as_str())
            }
            Self::UnknownStatus { command, status } => write!(
                f,
                "device answered {command:?} with unknown status 0x{status:02X}"
            ),
            Self::CommandMismatch { sent, received } => write!(
                f,
                "sent {sent:?} (0x{:02X}) but the device answered 0x{received:02X}; \
                 the link is out of step",
                *sent as u8
            ),
            Self::Timeout { command, waited } => {
                write!(f, "no answer to {command:?} within {waited:?}")
            }
            Self::UnexpectedPayload {
                command,
                expected,
                received,
            } => write!(
                f,
                "{command:?} returned {received} bytes, expected {expected}"
            ),
            Self::Desynchronised { skipped } => write!(
                f,
                "link lost sync: discarded {skipped} bytes before a valid frame"
            ),
            Self::Unsupported(what) => write!(f, "transport does not support {what}"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Frame(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for TransportError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<FrameError> for TransportError {
    fn from(e: FrameError) -> Self {
        Self::Frame(e)
    }
}

/// Result of a transport operation.
pub type TransportResult<T> = Result<T, TransportError>;

/// How a device is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportKind {
    /// A USB device.
    Usb {
        /// Bus address or serial number, as reported by the OS.
        address: String,
    },
    /// An SBC agent over TCP.
    Tcp {
        /// `host:port`.
        endpoint: String,
    },
    /// An SBC agent over a Unix domain socket.
    Unix {
        /// Socket path.
        path: String,
    },
    /// The in-process emulator.
    Emulator {
        /// Description of the emulated chip.
        chip: String,
    },
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb { address } => write!(f, "usb:{address}"),
            Self::Tcp { endpoint } => write!(f, "tcp:{endpoint}"),
            Self::Unix { path } => write!(f, "unix:{path}"),
            Self::Emulator { chip } => write!(f, "emulator:{chip}"),
        }
    }
}

/// A link to a device that exchanges protocol frames.
pub trait Transport {
    /// How this device is reached, for display and logging.
    fn kind(&self) -> TransportKind;

    /// Send a request frame and return the raw bytes of the response frame.
    ///
    /// Implementations must return one complete frame's bytes. They must not
    /// interpret the status byte; [`Transport::transact`] does that.
    fn exchange(&mut self, request: &[u8], timeout: Duration) -> TransportResult<Vec<u8>>;

    /// Send `command` with `payload` and return the response payload.
    ///
    /// This is the method callers should use. It rejects a corrupt frame, a
    /// mismatched command echo and a failure status, so a successful return
    /// really does mean the device did what was asked.
    fn transact(
        &mut self,
        command: Command,
        payload: &[u8],
        timeout: Duration,
    ) -> TransportResult<Vec<u8>> {
        let request = Frame::request(command, payload).encode_to_vec()?;
        let response_bytes = self.exchange(&request, timeout)?;

        let (skipped, frame) = Frame::find(&response_bytes)?;
        if skipped != 0 {
            return Err(TransportError::Desynchronised { skipped });
        }

        if frame.command != command as u8 {
            return Err(TransportError::CommandMismatch {
                sent: command,
                received: frame.command,
            });
        }

        match frame.decoded_status() {
            Some(Status::Ok) => Ok(frame.payload.to_vec()),
            Some(status) => Err(TransportError::Device { command, status }),
            None => Err(TransportError::UnknownStatus {
                command,
                status: frame.status,
            }),
        }
    }

    /// Like [`Transport::transact`], but require an exact payload size.
    fn transact_exact(
        &mut self,
        command: Command,
        payload: &[u8],
        expected: usize,
        timeout: Duration,
    ) -> TransportResult<Vec<u8>> {
        let response = self.transact(command, payload, timeout)?;
        if response.len() != expected {
            return Err(TransportError::UnexpectedPayload {
                command,
                expected,
                received: response.len(),
            });
        }
        Ok(response)
    }
}

/// Read exactly one frame from a blocking byte stream.
///
/// Reads in chunks and re-attempts decoding after each one, because a stream
/// gives no guarantee that a frame arrives in a single read: USB bulk transfers
/// are capped at the endpoint's packet size and TCP may split anywhere.
fn read_one_frame<R: Read>(reader: &mut R) -> TransportResult<Vec<u8>> {
    let mut buffer = Vec::with_capacity(HEADER_LEN + 64);
    let mut chunk = [0u8; 512];

    loop {
        match Frame::decode(&buffer) {
            Ok(frame) => {
                buffer.truncate(frame.encoded_len());
                return Ok(buffer);
            }
            Err(FrameError::Incomplete { .. }) => {}
            Err(other) => return Err(other.into()),
        }

        let read = reader.read(&mut chunk)?;
        if read == 0 {
            return Err(TransportError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "device closed the connection after {} bytes, mid-frame",
                    buffer.len()
                ),
            )));
        }
        buffer.extend_from_slice(&chunk[..read]);

        if buffer.len() > MAX_FRAME {
            return Err(TransportError::Frame(FrameError::PayloadTooLarge {
                len: buffer.len(),
            }));
        }
    }
}

/// A device reached over TCP, as exposed by the SBC agents.
pub struct TcpTransport {
    stream: TcpStream,
    endpoint: String,
}

impl TcpTransport {
    /// Connect to an agent.
    pub fn connect(endpoint: &str, timeout: Duration) -> TransportResult<Self> {
        let address = endpoint
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::other(format!("cannot resolve {endpoint}")))?;
        let stream = TcpStream::connect_timeout(&address, timeout)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            endpoint: endpoint.to_string(),
        })
    }
}

impl Transport for TcpTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Tcp {
            endpoint: self.endpoint.clone(),
        }
    }

    fn exchange(&mut self, request: &[u8], timeout: Duration) -> TransportResult<Vec<u8>> {
        self.stream.set_read_timeout(Some(timeout))?;
        self.stream.set_write_timeout(Some(timeout))?;
        self.stream.write_all(request)?;
        self.stream.flush()?;
        read_one_frame(&mut self.stream)
    }
}

/// A device reached over a Unix domain socket, as exposed by the SBC agents.
#[cfg(unix)]
pub struct UnixTransport {
    stream: std::os::unix::net::UnixStream,
    path: String,
}

#[cfg(unix)]
impl UnixTransport {
    /// Connect to an agent socket.
    pub fn connect(path: &str) -> TransportResult<Self> {
        let stream = std::os::unix::net::UnixStream::connect(path)?;
        Ok(Self {
            stream,
            path: path.to_string(),
        })
    }
}

#[cfg(unix)]
impl Transport for UnixTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Unix {
            path: self.path.clone(),
        }
    }

    fn exchange(&mut self, request: &[u8], timeout: Duration) -> TransportResult<Vec<u8>> {
        self.stream.set_read_timeout(Some(timeout))?;
        self.stream.set_write_timeout(Some(timeout))?;
        self.stream.write_all(request)?;
        self.stream.flush()?;
        read_one_frame(&mut self.stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openflash_protocol::frame::Frame;

    /// A transport whose responses the test dictates, used to prove that
    /// `transact` rejects everything it is supposed to reject.
    struct ScriptedTransport {
        responses: Vec<Vec<u8>>,
        requests: Vec<Vec<u8>>,
    }

    impl ScriptedTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses,
                requests: Vec::new(),
            }
        }
    }

    impl Transport for ScriptedTransport {
        fn kind(&self) -> TransportKind {
            TransportKind::Emulator {
                chip: "scripted".into(),
            }
        }

        fn exchange(&mut self, request: &[u8], _timeout: Duration) -> TransportResult<Vec<u8>> {
            self.requests.push(request.to_vec());
            Ok(self.responses.remove(0))
        }
    }

    fn ok_response(command: Command, payload: &[u8]) -> Vec<u8> {
        Frame::response(command, Status::Ok, payload)
            .encode_to_vec()
            .unwrap()
    }

    #[test]
    fn transact_returns_the_payload_of_a_successful_response() {
        let mut transport =
            ScriptedTransport::new(vec![ok_response(Command::NandReadId, &[0xEC, 0xF1, 0x00])]);
        let payload = transport
            .transact(Command::NandReadId, &[], DEFAULT_TIMEOUT)
            .unwrap();
        assert_eq!(payload, vec![0xEC, 0xF1, 0x00]);

        // The request that went out must be a well-formed frame for the command.
        let sent = Frame::decode(&transport.requests[0]).unwrap();
        assert_eq!(sent.decoded_command(), Some(Command::NandReadId));
        assert!(sent.payload.is_empty());
    }

    #[test]
    fn a_failure_status_becomes_an_error_not_an_empty_success() {
        let response = Frame::response(Command::SpiNorRead, Status::ChipNotFound, &[])
            .encode_to_vec()
            .unwrap();
        let mut transport = ScriptedTransport::new(vec![response]);

        match transport.transact(Command::SpiNorRead, &[], DEFAULT_TIMEOUT) {
            Err(TransportError::Device { command, status }) => {
                assert_eq!(command, Command::SpiNorRead);
                assert_eq!(status, Status::ChipNotFound);
            }
            other => panic!("expected a device error, got {other:?}"),
        }
    }

    /// The bug this guards against: a reply left over from an earlier command
    /// being handed back as the answer to the current one, so a page read
    /// returns some other page's data.
    #[test]
    fn a_response_echoing_another_command_is_rejected() {
        let mut transport = ScriptedTransport::new(vec![ok_response(Command::Ping, &[0xAA; 16])]);

        match transport.transact(Command::NandReadPage, &[], DEFAULT_TIMEOUT) {
            Err(TransportError::CommandMismatch { sent, received }) => {
                assert_eq!(sent, Command::NandReadPage);
                assert_eq!(received, Command::Ping as u8);
            }
            other => panic!("expected a command mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_corrupt_response_is_an_error_rather_than_damaged_data() {
        let mut response = ok_response(Command::SpiNorRead, &[1, 2, 3, 4]);
        let last = response.len() - 3;
        response[last] ^= 0x01; // flip a payload bit, leaving the CRC stale
        let mut transport = ScriptedTransport::new(vec![response]);

        match transport.transact(Command::SpiNorRead, &[], DEFAULT_TIMEOUT) {
            Err(TransportError::Frame(FrameError::PayloadCrcMismatch)) => {}
            other => panic!("expected a payload CRC error, got {other:?}"),
        }
    }

    #[test]
    fn leading_garbage_in_a_response_is_reported_as_lost_sync() {
        let mut response = vec![0xFF, 0xFF, 0x00];
        response.extend_from_slice(&ok_response(Command::Ping, &[]));
        let mut transport = ScriptedTransport::new(vec![response]);

        match transport.transact(Command::Ping, &[], DEFAULT_TIMEOUT) {
            Err(TransportError::Desynchronised { skipped }) => assert_eq!(skipped, 3),
            other => panic!("expected desynchronisation, got {other:?}"),
        }
    }

    #[test]
    fn transact_exact_rejects_a_short_payload() {
        let mut transport =
            ScriptedTransport::new(vec![ok_response(Command::NandReadId, &[0xEC, 0xF1])]);

        match transport.transact_exact(Command::NandReadId, &[], 5, DEFAULT_TIMEOUT) {
            Err(TransportError::UnexpectedPayload {
                expected, received, ..
            }) => {
                assert_eq!(expected, 5);
                assert_eq!(received, 2);
            }
            other => panic!("expected a payload size error, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_status_byte_is_reported_rather_than_treated_as_success() {
        let mut response = ok_response(Command::Ping, &[]);
        response[4] = 0x7F; // not a defined Status
        let fixed = openflash_protocol::crc::checksum(&response[0..8]);
        response[8..10].copy_from_slice(&fixed.to_le_bytes());
        let mut transport = ScriptedTransport::new(vec![response]);

        match transport.transact(Command::Ping, &[], DEFAULT_TIMEOUT) {
            Err(TransportError::UnknownStatus { status, .. }) => assert_eq!(status, 0x7F),
            other => panic!("expected an unknown status error, got {other:?}"),
        }
    }

    #[test]
    fn a_frame_split_across_reads_is_reassembled() {
        // Feeds the frame back a few bytes at a time, the way a USB endpoint or
        // a TCP segment boundary would.
        struct Trickle {
            data: Vec<u8>,
            position: usize,
            chunk: usize,
        }

        impl Read for Trickle {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let remaining = self.data.len() - self.position;
                if remaining == 0 {
                    return Ok(0);
                }
                let take = self.chunk.min(remaining).min(out.len());
                out[..take].copy_from_slice(&self.data[self.position..self.position + take]);
                self.position += take;
                Ok(take)
            }
        }

        let payload: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
        let frame = ok_response(Command::SpiNorRead, &payload);

        for chunk in [1usize, 3, 7, 64, 512] {
            let mut reader = Trickle {
                data: frame.clone(),
                position: 0,
                chunk,
            };
            let read = read_one_frame(&mut reader).unwrap();
            assert_eq!(read, frame, "failed to reassemble with {chunk}-byte reads");
            let decoded = Frame::decode(&read).unwrap();
            assert_eq!(decoded.payload, &payload[..]);
        }
    }

    #[test]
    fn read_one_frame_stops_at_the_end_of_a_frame_and_leaves_the_rest() {
        let first = ok_response(Command::Ping, &[]);
        let mut stream = first.clone();
        stream.extend_from_slice(&ok_response(Command::GetVersion, &[1, 2, 3]));

        let read = read_one_frame(&mut io::Cursor::new(stream)).unwrap();
        assert_eq!(read, first);
    }

    #[test]
    fn a_connection_closed_mid_frame_is_an_error() {
        let frame = ok_response(Command::SpiNorRead, &[0u8; 64]);
        let truncated = &frame[..20];

        match read_one_frame(&mut io::Cursor::new(truncated.to_vec())) {
            Err(TransportError::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
            other => panic!("expected an EOF error, got {other:?}"),
        }
    }

    #[test]
    fn transport_kinds_display_as_a_url_like_string() {
        assert_eq!(
            TransportKind::Usb {
                address: "1-2".into()
            }
            .to_string(),
            "usb:1-2"
        );
        assert_eq!(
            TransportKind::Tcp {
                endpoint: "10.0.0.5:9999".into()
            }
            .to_string(),
            "tcp:10.0.0.5:9999"
        );
    }
}
