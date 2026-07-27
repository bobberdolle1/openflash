//! Framing for protocol revision 2.
//!
//! Revision 1 put a bare `[command][args…]` into a 64-byte USB packet with no
//! length, no checksum and no delimiter. A single dropped or inserted byte
//! desynchronised the stream permanently and silently — which for a tool whose
//! whole job is byte-exact flash images is the worst possible failure mode.
//!
//! A revision 2 frame is:
//!
//! ```text
//! offset  size  field
//!      0     2  magic 'O' 'F'
//!      2     1  protocol version
//!      3     1  command
//!      4     1  status        (0x00 in a request)
//!      5     1  flags         (reserved, must be 0)
//!      6     2  payload_len   (u16, little endian)
//!      8     2  header CRC-16/CCITT-FALSE over offsets 0..8
//!     10   len  payload
//! 10+len     2  payload CRC-16/CCITT-FALSE
//! ```
//!
//! The header carries its own CRC so a receiver can trust `payload_len` before
//! it reserves a buffer for it — a corrupted length can never make a device
//! wait for 64 KiB that will never arrive, nor make a host allocate on garbage.
//! The magic lets a receiver that *has* lost sync scan forward and recover; see
//! [`Frame::find`].

use crate::command::{Command, Status, PROTOCOL_VERSION};
use crate::crc;

/// Frame delimiter: ASCII "OF".
pub const MAGIC: [u8; 2] = [0x4F, 0x46];

/// Size of the fixed frame header, including its CRC.
pub const HEADER_LEN: usize = 10;

/// Size of a CRC field.
pub const CRC_LEN: usize = 2;

/// Largest payload a frame may carry.
///
/// Sized to hold the biggest single transfer any supported interface needs — a
/// 4 KiB SPI NOR sector, or a 4096+256-byte NAND page with its OOB area — so
/// bare-metal firmware can statically allocate one frame buffer.
pub const MAX_PAYLOAD: usize = 8192;

/// Total wire size of a frame carrying `payload_len` bytes.
pub const fn frame_len(payload_len: usize) -> usize {
    HEADER_LEN + payload_len + CRC_LEN
}

/// Largest possible frame, for sizing static buffers.
pub const MAX_FRAME: usize = frame_len(MAX_PAYLOAD);

/// Why a frame could not be encoded or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// More bytes are needed. `needed` is the total frame size when known, or
    /// the minimum to make progress when the header has not arrived yet.
    Incomplete {
        /// Total number of bytes required before decoding can be retried.
        needed: usize,
    },
    /// The buffer does not start with [`MAGIC`].
    BadMagic,
    /// The peer speaks a protocol revision this build does not implement.
    UnsupportedVersion {
        /// Version byte received from the peer.
        found: u8,
        /// Version byte this build speaks.
        expected: u8,
    },
    /// The header CRC did not match: the header is corrupt.
    HeaderCrcMismatch,
    /// The payload CRC did not match: the payload is corrupt.
    PayloadCrcMismatch,
    /// The frame declares a payload larger than [`MAX_PAYLOAD`].
    PayloadTooLarge {
        /// Declared payload length.
        len: usize,
    },
    /// The destination buffer cannot hold the encoded frame.
    BufferTooSmall {
        /// Bytes required.
        needed: usize,
        /// Bytes available.
        available: usize,
    },
    /// The reserved flags byte was not zero.
    ReservedFlagsSet {
        /// Flags byte received.
        flags: u8,
    },
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Incomplete { needed } => write!(f, "incomplete frame, need {needed} bytes"),
            Self::BadMagic => write!(f, "frame does not start with the OF magic"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "peer speaks protocol v{found}, this build speaks v{expected}"
            ),
            Self::HeaderCrcMismatch => write!(f, "frame header CRC mismatch (corrupt link)"),
            Self::PayloadCrcMismatch => write!(f, "frame payload CRC mismatch (corrupt link)"),
            Self::PayloadTooLarge { len } => {
                write!(
                    f,
                    "payload of {len} bytes exceeds the {MAX_PAYLOAD}-byte limit"
                )
            }
            Self::BufferTooSmall { needed, available } => {
                write!(f, "buffer holds {available} bytes, frame needs {needed}")
            }
            Self::ReservedFlagsSet { flags } => {
                write!(f, "reserved flags byte is 0x{flags:02X}, must be 0x00")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FrameError {}

/// A decoded or to-be-encoded frame.
///
/// `command` and `status` are kept as raw bytes so that a frame naming an
/// opcode this build does not know still decodes — the receiver needs to be
/// able to answer "unsupported command" rather than drop the frame and hang.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Protocol revision the sender used.
    pub version: u8,
    /// Raw command byte.
    pub command: u8,
    /// Raw status byte; `0x00` in requests.
    pub status: u8,
    /// Frame payload.
    pub payload: &'a [u8],
}

impl<'a> Frame<'a> {
    /// Build a request frame.
    pub fn request(command: Command, payload: &'a [u8]) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            command: command as u8,
            status: Status::Ok as u8,
            payload,
        }
    }

    /// Build a response frame.
    pub fn response(command: Command, status: Status, payload: &'a [u8]) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            command: command as u8,
            status: status as u8,
            payload,
        }
    }

    /// Build a response frame echoing a raw command byte, for replying to a
    /// command this firmware does not implement.
    pub fn response_raw(command: u8, status: Status, payload: &'a [u8]) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            command,
            status: status as u8,
            payload,
        }
    }

    /// The decoded command, or `None` if this build does not know the opcode.
    pub fn decoded_command(&self) -> Option<Command> {
        Command::from_u8(self.command)
    }

    /// The decoded status, or `None` if this build does not know the code.
    pub fn decoded_status(&self) -> Option<Status> {
        Status::from_u8(self.status)
    }

    /// Total wire size of this frame.
    pub fn encoded_len(&self) -> usize {
        frame_len(self.payload.len())
    }

    /// Write this frame into `out`, returning the number of bytes written.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, FrameError> {
        let len = self.payload.len();
        if len > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLarge { len });
        }
        let total = frame_len(len);
        if out.len() < total {
            return Err(FrameError::BufferTooSmall {
                needed: total,
                available: out.len(),
            });
        }

        out[0..2].copy_from_slice(&MAGIC);
        out[2] = self.version;
        out[3] = self.command;
        out[4] = self.status;
        out[5] = 0; // reserved flags
        out[6..8].copy_from_slice(&(len as u16).to_le_bytes());
        let header_crc = crc::checksum(&out[0..8]);
        out[8..10].copy_from_slice(&header_crc.to_le_bytes());

        out[HEADER_LEN..HEADER_LEN + len].copy_from_slice(self.payload);
        let payload_crc = crc::checksum(self.payload);
        out[HEADER_LEN + len..total].copy_from_slice(&payload_crc.to_le_bytes());

        Ok(total)
    }

    /// Encode into a freshly allocated vector.
    #[cfg(feature = "std")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, FrameError> {
        let mut out = vec![0u8; frame_len(self.payload.len())];
        let written = self.encode_into(&mut out)?;
        debug_assert_eq!(written, out.len());
        Ok(out)
    }

    /// Decode a frame that starts at the beginning of `buf`.
    ///
    /// On success the frame borrows its payload from `buf`. The caller learns
    /// how many bytes were consumed from [`Frame::encoded_len`].
    pub fn decode(buf: &'a [u8]) -> Result<Self, FrameError> {
        if buf.len() < HEADER_LEN {
            return Err(FrameError::Incomplete { needed: HEADER_LEN });
        }
        if buf[0..2] != MAGIC {
            return Err(FrameError::BadMagic);
        }

        let stored_header_crc = u16::from_le_bytes([buf[8], buf[9]]);
        if crc::checksum(&buf[0..8]) != stored_header_crc {
            return Err(FrameError::HeaderCrcMismatch);
        }

        // Only now that the header is known intact may its fields be trusted.
        let version = buf[2];
        if version != PROTOCOL_VERSION {
            return Err(FrameError::UnsupportedVersion {
                found: version,
                expected: PROTOCOL_VERSION,
            });
        }
        let flags = buf[5];
        if flags != 0 {
            return Err(FrameError::ReservedFlagsSet { flags });
        }
        let len = u16::from_le_bytes([buf[6], buf[7]]) as usize;
        if len > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLarge { len });
        }

        let total = frame_len(len);
        if buf.len() < total {
            return Err(FrameError::Incomplete { needed: total });
        }

        let payload = &buf[HEADER_LEN..HEADER_LEN + len];
        let stored_payload_crc =
            u16::from_le_bytes([buf[HEADER_LEN + len], buf[HEADER_LEN + len + 1]]);
        if crc::checksum(payload) != stored_payload_crc {
            return Err(FrameError::PayloadCrcMismatch);
        }

        Ok(Self {
            version,
            command: buf[3],
            status: buf[4],
            payload,
        })
    }

    /// Find the next frame in `buf`, skipping leading bytes that cannot start
    /// one.
    ///
    /// This is how a receiver recovers after a corrupt or partially consumed
    /// transfer: it scans forward for [`MAGIC`] and a header whose CRC checks
    /// out. Returns the number of bytes skipped along with the frame, so the
    /// caller can report that the link dropped data instead of silently
    /// papering over it.
    pub fn find(buf: &'a [u8]) -> Result<(usize, Self), FrameError> {
        let mut offset = 0;
        loop {
            if buf.len() - offset < HEADER_LEN {
                // Keep whatever might be the start of a magic sequence.
                return Err(FrameError::Incomplete { needed: HEADER_LEN });
            }
            match Self::decode(&buf[offset..]) {
                Ok(frame) => return Ok((offset, frame)),
                // Not a frame boundary, or a corrupt header: step one byte and
                // keep looking.
                Err(FrameError::BadMagic) | Err(FrameError::HeaderCrcMismatch) => offset += 1,
                // A valid header whose payload has not fully arrived: the caller
                // must read more, not resynchronise.
                Err(FrameError::Incomplete { needed }) => {
                    return Err(FrameError::Incomplete {
                        needed: offset + needed,
                    })
                }
                Err(other) => return Err(other),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(cmd: Command, payload: &[u8]) -> Vec<u8> {
        Frame::request(cmd, payload).encode_to_vec().unwrap()
    }

    #[test]
    fn request_round_trips() {
        let payload = [0xDE, 0xAD, 0xBE, 0xEF];
        let bytes = encoded(Command::NandReadPage, &payload);
        assert_eq!(bytes.len(), frame_len(payload.len()));

        let frame = Frame::decode(&bytes).unwrap();
        assert_eq!(frame.decoded_command(), Some(Command::NandReadPage));
        assert_eq!(frame.decoded_status(), Some(Status::Ok));
        assert_eq!(frame.payload, &payload);
        assert_eq!(frame.version, PROTOCOL_VERSION);
    }

    #[test]
    fn response_round_trips_with_its_status() {
        let frame = Frame::response(Command::SpiNorRead, Status::EccFailure, &[1, 2, 3]);
        let bytes = frame.encode_to_vec().unwrap();
        let decoded = Frame::decode(&bytes).unwrap();
        assert_eq!(decoded.decoded_status(), Some(Status::EccFailure));
        assert_eq!(decoded.decoded_command(), Some(Command::SpiNorRead));
        assert_eq!(decoded.payload, &[1, 2, 3]);
    }

    #[test]
    fn empty_payload_round_trips() {
        let bytes = encoded(Command::Ping, &[]);
        assert_eq!(bytes.len(), HEADER_LEN + CRC_LEN);
        let frame = Frame::decode(&bytes).unwrap();
        assert!(frame.payload.is_empty());
        assert_eq!(frame.decoded_command(), Some(Command::Ping));
    }

    #[test]
    fn maximum_payload_round_trips() {
        let payload = vec![0xA5; MAX_PAYLOAD];
        let bytes = encoded(Command::SpiNorRead, &payload);
        assert_eq!(bytes.len(), MAX_FRAME);
        assert_eq!(Frame::decode(&bytes).unwrap().payload, &payload[..]);
    }

    #[test]
    fn oversized_payload_is_refused_rather_than_truncated() {
        let payload = vec![0u8; MAX_PAYLOAD + 1];
        let frame = Frame::request(Command::SpiNorRead, &payload);
        assert_eq!(
            frame.encode_to_vec(),
            Err(FrameError::PayloadTooLarge {
                len: MAX_PAYLOAD + 1
            })
        );
    }

    #[test]
    fn encoding_into_a_short_buffer_reports_what_it_needed() {
        let frame = Frame::request(Command::Ping, &[1, 2, 3]);
        let mut buf = [0u8; 4];
        assert_eq!(
            frame.encode_into(&mut buf),
            Err(FrameError::BufferTooSmall {
                needed: frame_len(3),
                available: 4
            })
        );
    }

    /// The point of the whole exercise: a corrupted byte must be reported, not
    /// handed to the caller as flash contents.
    #[test]
    fn every_single_bit_flip_is_detected() {
        let bytes = encoded(Command::NandReadPage, &[0x11, 0x22, 0x33, 0x44, 0x55]);

        for index in 0..bytes.len() {
            for bit in 0..8 {
                let mut corrupted = bytes.clone();
                corrupted[index] ^= 1 << bit;

                match Frame::decode(&corrupted) {
                    Err(_) => {}
                    Ok(frame) => panic!(
                        "bit {bit} of byte {index} was flipped but the frame still \
                         decoded as {:?} with payload {:?}",
                        frame.decoded_command(),
                        frame.payload
                    ),
                }
            }
        }
    }

    #[test]
    fn truncation_is_reported_as_incomplete_with_the_full_size() {
        let payload = [7u8; 32];
        let bytes = encoded(Command::SpiNorRead, &payload);
        let total = bytes.len();

        for cut in 0..total {
            match Frame::decode(&bytes[..cut]) {
                Err(FrameError::Incomplete { needed }) => {
                    let expected = if cut < HEADER_LEN { HEADER_LEN } else { total };
                    assert_eq!(needed, expected, "wrong hint after truncating to {cut}");
                }
                other => panic!("truncating to {cut} bytes gave {other:?}"),
            }
        }
        assert!(Frame::decode(&bytes).is_ok());
    }

    #[test]
    fn a_corrupt_length_cannot_make_the_receiver_wait_for_garbage() {
        let mut bytes = encoded(Command::SpiNorRead, &[0u8; 16]);
        // Claim a huge payload without fixing the header CRC.
        bytes[6] = 0xFF;
        bytes[7] = 0xFF;
        assert_eq!(Frame::decode(&bytes), Err(FrameError::HeaderCrcMismatch));
    }

    #[test]
    fn a_length_over_the_limit_is_refused_even_with_a_valid_header_crc() {
        let mut bytes = encoded(Command::SpiNorRead, &[0u8; 16]);
        let bogus_len = (MAX_PAYLOAD + 1) as u16;
        bytes[6..8].copy_from_slice(&bogus_len.to_le_bytes());
        // Recompute the header CRC so the length itself is what gets rejected.
        let fixed = crc::checksum(&bytes[0..8]);
        bytes[8..10].copy_from_slice(&fixed.to_le_bytes());
        assert_eq!(
            Frame::decode(&bytes),
            Err(FrameError::PayloadTooLarge {
                len: MAX_PAYLOAD + 1
            })
        );
    }

    #[test]
    fn a_different_protocol_version_is_named_in_the_error() {
        let mut bytes = encoded(Command::Ping, &[]);
        bytes[2] = 99;
        let fixed = crc::checksum(&bytes[0..8]);
        bytes[8..10].copy_from_slice(&fixed.to_le_bytes());
        assert_eq!(
            Frame::decode(&bytes),
            Err(FrameError::UnsupportedVersion {
                found: 99,
                expected: PROTOCOL_VERSION
            })
        );
    }

    #[test]
    fn reserved_flags_must_be_zero() {
        let mut bytes = encoded(Command::Ping, &[]);
        bytes[5] = 0x01;
        let fixed = crc::checksum(&bytes[0..8]);
        bytes[8..10].copy_from_slice(&fixed.to_le_bytes());
        assert_eq!(
            Frame::decode(&bytes),
            Err(FrameError::ReservedFlagsSet { flags: 0x01 })
        );
    }

    #[test]
    fn an_unknown_opcode_still_decodes_so_it_can_be_answered() {
        // 0xB5 is in the reserved host-side range: no Command maps to it, but a
        // device must still be able to reply "unsupported" rather than hang.
        let frame = Frame::response_raw(0xB5, Status::UnsupportedCommand, &[]);
        let bytes = frame.encode_to_vec().unwrap();
        let decoded = Frame::decode(&bytes).unwrap();
        assert_eq!(decoded.command, 0xB5);
        assert_eq!(decoded.decoded_command(), None);
        assert_eq!(decoded.decoded_status(), Some(Status::UnsupportedCommand));
    }

    #[test]
    fn find_recovers_after_leading_garbage() {
        let mut stream = vec![0x00, 0xFF, 0x4F, 0x11, 0xAB]; // includes a false 'O'
        let skipped_bytes = stream.len();
        stream.extend_from_slice(&encoded(Command::NandReadId, &[0xEC, 0xF1]));

        let (skipped, frame) = Frame::find(&stream).unwrap();
        assert_eq!(skipped, skipped_bytes);
        assert_eq!(frame.decoded_command(), Some(Command::NandReadId));
        assert_eq!(frame.payload, &[0xEC, 0xF1]);
    }

    #[test]
    fn find_skips_a_frame_with_a_corrupt_header_and_returns_the_next_one() {
        let mut broken = encoded(Command::Ping, &[]);
        broken[3] ^= 0xFF; // breaks the header CRC
        let mut stream = broken;
        stream.extend_from_slice(&encoded(Command::GetVersion, &[1, 2]));

        let (_, frame) = Frame::find(&stream).unwrap();
        assert_eq!(frame.decoded_command(), Some(Command::GetVersion));
        assert_eq!(frame.payload, &[1, 2]);
    }

    #[test]
    fn find_on_a_valid_but_unfinished_frame_asks_for_more_bytes() {
        let bytes = encoded(Command::SpiNorRead, &[0u8; 64]);
        let partial = &bytes[..HEADER_LEN + 10];
        assert_eq!(
            Frame::find(partial),
            Err(FrameError::Incomplete {
                needed: frame_len(64)
            })
        );
    }

    #[test]
    fn find_accounts_for_skipped_bytes_when_asking_for_more() {
        let garbage = [0xAAu8; 5];
        let mut stream = garbage.to_vec();
        let bytes = encoded(Command::SpiNorRead, &[0u8; 64]);
        stream.extend_from_slice(&bytes[..HEADER_LEN + 4]);

        assert_eq!(
            Frame::find(&stream),
            Err(FrameError::Incomplete {
                needed: garbage.len() + frame_len(64)
            })
        );
    }

    #[test]
    fn back_to_back_frames_decode_one_after_another() {
        let first = encoded(Command::Ping, &[]);
        let second = encoded(Command::NandReadId, &[1, 2, 3, 4, 5]);
        let mut stream = first.clone();
        stream.extend_from_slice(&second);

        let a = Frame::decode(&stream).unwrap();
        assert_eq!(a.decoded_command(), Some(Command::Ping));
        let b = Frame::decode(&stream[a.encoded_len()..]).unwrap();
        assert_eq!(b.decoded_command(), Some(Command::NandReadId));
        assert_eq!(b.payload, &[1, 2, 3, 4, 5]);
    }

    proptest::proptest! {
        /// Any payload survives a round trip byte-for-byte.
        #[test]
        fn arbitrary_payloads_round_trip(
            payload in proptest::collection::vec(proptest::num::u8::ANY, 0..600usize)
        ) {
            let bytes = encoded(Command::SpiNorRead, &payload);
            let frame = Frame::decode(&bytes).unwrap();
            proptest::prop_assert_eq!(frame.payload, &payload[..]);
        }

        /// A payload that happens to contain the magic sequence must not confuse
        /// the decoder — length-prefixed framing, not delimiter scanning.
        #[test]
        fn payloads_containing_the_magic_round_trip(
            prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..40usize),
            suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..40usize),
        ) {
            let mut payload = prefix.clone();
            payload.extend_from_slice(&MAGIC);
            payload.extend_from_slice(&suffix);

            let bytes = encoded(Command::SpiNorRead, &payload);
            let frame = Frame::decode(&bytes).unwrap();
            proptest::prop_assert_eq!(frame.payload, &payload[..]);
        }

        /// Garbage must never decode into a frame. It may be rejected or come
        /// up short, but it must not produce a payload.
        #[test]
        fn random_bytes_do_not_decode_as_a_frame(
            noise in proptest::collection::vec(proptest::num::u8::ANY, 0..64usize)
        ) {
            if let Ok(frame) = Frame::decode(&noise) {
                // The only way random bytes legitimately decode is if they
                // happen to form a valid frame, which requires both CRCs to
                // match. Re-encoding must then reproduce them exactly.
                let re_encoded = frame.encode_to_vec().unwrap();
                proptest::prop_assert_eq!(&noise[..re_encoded.len()], &re_encoded[..]);
            }
        }
    }
}
