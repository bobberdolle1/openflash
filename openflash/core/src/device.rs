//! Chip operations over a [`Transport`].
//!
//! This is the layer the CLI and the GUI drive, and it is where the operations
//! a flash programmer exists for actually happen: identify a chip, dump it,
//! erase it, program it, verify it. Every method here performs real I/O against
//! whatever is on the other end of the transport, and returns an error when it
//! cannot — there is no path through this module that reports success without
//! having done the work.
//!
//! The details that make the difference between a tool that works and one that
//! looks like it works:
//!
//! - reads are chunked to the frame payload limit and streamed to a sink, so
//!   dumping a 1 GiB chip does not need 1 GiB of memory,
//! - programming erases the affected sectors first, and preserves the parts of
//!   a partially covered sector instead of erasing a neighbour's data,
//! - programming respects the 256-byte page boundary a chip enforces, and sets
//!   the write-enable latch before every operation that needs it,
//! - pages that are entirely `0xFF` are skipped, because erased flash already
//!   reads that way,
//! - verification reads the chip back and reports the first differing offset.

use std::fmt;
use std::io::Write;
use std::time::{Duration, Instant};

use openflash_protocol::{Command, FlashInterface, VersionInfo, PROTOCOL_VERSION};

use crate::spi_nor::{get_spi_nor_chip_info, SpiNorChipInfo};
use crate::transport::{Transport, TransportError, TransportKind, DEFAULT_TIMEOUT, ERASE_TIMEOUT};

/// Largest number of bytes moved in one frame.
///
/// Bounded by the protocol's payload limit minus the address and length fields
/// a read or program request carries.
pub const MAX_CHUNK: usize = openflash_protocol::MAX_PAYLOAD - 8;

/// Bytes in a program page. A single program operation may not cross this.
pub const PAGE_SIZE: usize = 256;

/// Something went wrong performing a chip operation.
#[derive(Debug)]
pub enum DeviceError {
    /// The link to the device failed.
    Transport(TransportError),
    /// Writing the dump to its destination failed.
    Io(std::io::Error),
    /// The device speaks a protocol revision this build cannot talk to.
    ProtocolMismatch {
        /// Revision the device reported.
        device: u8,
        /// Revision this build speaks.
        host: u8,
    },
    /// The device does not implement the interface the operation needs.
    InterfaceUnsupported {
        /// Interface that was requested.
        interface: FlashInterface,
    },
    /// No chip answered, or it returned an all-ones/all-zeroes id.
    NoChipDetected {
        /// Raw id bytes that were read.
        id: Vec<u8>,
    },
    /// The chip answered with an id that is not in the database.
    UnknownChip {
        /// Raw JEDEC id.
        jedec_id: [u8; 3],
    },
    /// The requested range does not fit on the chip.
    RangeOutOfBounds {
        /// Requested start address.
        start: u64,
        /// Requested length.
        length: u64,
        /// Chip capacity.
        capacity: u64,
    },
    /// Verification found a difference between the chip and the expected data.
    VerificationFailed {
        /// Absolute address of the first differing byte.
        offset: u64,
        /// Byte that was expected there.
        expected: u8,
        /// Byte the chip returned.
        found: u8,
    },
    /// An erase left a region that does not read as erased.
    EraseFailed {
        /// Absolute address of the first byte that is not `0xFF`.
        offset: u64,
        /// Byte the chip returned.
        found: u8,
    },
    /// The operation would modify the chip but the session is read-only.
    ///
    /// Set by [`Device::set_read_only`], which the CLI turns on for dump and
    /// verify so a forensic read cannot alter the evidence.
    ReadOnlySession {
        /// Command that was blocked.
        command: Command,
    },
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::ProtocolMismatch { device, host } => write!(
                f,
                "device speaks protocol v{device} but this build speaks v{host}; \
                 update the firmware or the host tools so both match"
            ),
            Self::InterfaceUnsupported { interface } => write!(
                f,
                "device does not implement the {} interface",
                interface.as_str()
            ),
            Self::NoChipDetected { id } => write!(
                f,
                "no chip detected (id read back as {id:02X?}); check wiring, \
                 orientation and power"
            ),
            Self::UnknownChip { jedec_id } => write!(
                f,
                "chip id {:02X}:{:02X}:{:02X} is not in the database",
                jedec_id[0], jedec_id[1], jedec_id[2]
            ),
            Self::RangeOutOfBounds {
                start,
                length,
                capacity,
            } => write!(
                f,
                "range {start:#x}..{:#x} does not fit on a {capacity}-byte chip",
                start + length
            ),
            Self::VerificationFailed {
                offset,
                expected,
                found,
            } => write!(
                f,
                "verification failed at {offset:#x}: expected {expected:#04x}, \
                 chip returned {found:#04x}"
            ),
            Self::EraseFailed { offset, found } => write!(
                f,
                "erase did not take effect at {offset:#x}: chip returned {found:#04x}, \
                 expected 0xFF"
            ),
            Self::ReadOnlySession { command } => write!(
                f,
                "{command:?} would modify the chip, but this session is read-only"
            ),
        }
    }
}

impl std::error::Error for DeviceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for DeviceError {
    fn from(e: TransportError) -> Self {
        Self::Transport(e)
    }
}

impl From<std::io::Error> for DeviceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Result of a chip operation.
pub type DeviceResult<T> = Result<T, DeviceError>;

/// A chip that was identified by reading its id off the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedChip {
    /// Manufacturer name.
    pub manufacturer: String,
    /// Part number.
    pub model: String,
    /// Raw JEDEC id bytes.
    pub jedec_id: [u8; 3],
    /// Capacity in bytes.
    pub capacity: u64,
    /// Program page size.
    pub page_size: u32,
    /// Smallest erasable unit.
    pub sector_size: u32,
    /// Whether the part was matched exactly or by a capacity-based fallback.
    pub exact_match: bool,
}

impl DetectedChip {
    fn from_chip_info(info: &SpiNorChipInfo, exact_match: bool) -> Self {
        Self {
            manufacturer: info.manufacturer.clone(),
            model: info.model.clone(),
            jedec_id: info.jedec_id,
            capacity: u64::from(info.size_bytes),
            page_size: info.page_size,
            sector_size: info.sector_size,
            exact_match,
        }
    }
}

/// What a read actually transferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadReport {
    /// Bytes written to the sink.
    pub bytes_read: u64,
    /// Number of frames exchanged.
    pub chunks: u64,
    /// Wall-clock time the transfer took.
    pub duration: Duration,
}

impl ReadReport {
    /// Average throughput in bytes per second, or `None` for a transfer too
    /// short to measure.
    pub fn bytes_per_second(&self) -> Option<u64> {
        let seconds = self.duration.as_secs_f64();
        if seconds <= 0.0 {
            return None;
        }
        Some((self.bytes_read as f64 / seconds) as u64)
    }
}

/// How to program a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramOptions {
    /// Erase the affected sectors before programming.
    ///
    /// Programming can only clear bits, so writing over data that was not
    /// erased produces the bitwise AND of old and new. Leave this on unless the
    /// target is known to be erased already.
    pub erase_first: bool,
    /// Read the region back and compare it after programming.
    pub verify: bool,
    /// Skip pages that are entirely `0xFF`.
    ///
    /// Erased flash already reads as `0xFF`, so on a freshly erased region these
    /// writes are redundant. Turn it off to force every byte to be written.
    pub skip_blank_pages: bool,
}

impl Default for ProgramOptions {
    fn default() -> Self {
        Self {
            erase_first: true,
            verify: true,
            skip_blank_pages: true,
        }
    }
}

/// What a program operation actually did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProgramReport {
    /// Bytes handed to the chip.
    pub bytes_written: u64,
    /// Number of page program operations issued.
    pub pages_written: u64,
    /// Number of pages skipped because they were entirely `0xFF`.
    pub pages_skipped: u64,
    /// Number of sectors erased.
    pub sectors_erased: u64,
    /// Number of sectors that were read out and rewritten because the target
    /// range covered only part of them.
    pub sectors_preserved: u64,
    /// Whether the region was read back and compared.
    pub verified: bool,
}

/// Progress callback: `(bytes_done, bytes_total)`.
pub type ProgressFn<'a> = &'a mut dyn FnMut(u64, u64);

fn no_progress(_done: u64, _total: u64) {}

/// A connected device.
pub struct Device<T: Transport> {
    transport: T,
    version: VersionInfo,
    chip: Option<DetectedChip>,
    read_only: bool,
}

// Written by hand rather than derived: a transport is not required to be
// `Debug`, and the useful information is what the device reported about itself.
impl<T: Transport> fmt::Debug for Device<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("connection", &self.transport.kind().to_string())
            .field("protocol", &self.version.protocol)
            .field("chip", &self.chip.as_ref().map(|c| &c.model))
            .field("read_only", &self.read_only)
            .finish()
    }
}

impl<T: Transport> Device<T> {
    /// Handshake with a device and check that both sides speak the same
    /// protocol revision.
    ///
    /// Done up front, and deliberately not skippable: two builds that disagree
    /// about the wire format will otherwise appear to work and return wrong
    /// data.
    pub fn connect(mut transport: T) -> DeviceResult<Self> {
        transport.transact(Command::Ping, &[], DEFAULT_TIMEOUT)?;

        let payload = transport.transact(Command::GetVersion, &[], DEFAULT_TIMEOUT)?;
        let version = VersionInfo::from_bytes(&payload).ok_or(DeviceError::Transport(
            TransportError::UnexpectedPayload {
                command: Command::GetVersion,
                expected: openflash_protocol::VERSION_INFO_LEN,
                received: payload.len(),
            },
        ))?;

        if version.protocol != PROTOCOL_VERSION {
            return Err(DeviceError::ProtocolMismatch {
                device: version.protocol,
                host: PROTOCOL_VERSION,
            });
        }

        Ok(Self {
            transport,
            version,
            chip: None,
            read_only: false,
        })
    }

    /// What the device reported about itself.
    pub fn version(&self) -> &VersionInfo {
        &self.version
    }

    /// How this device is reached.
    pub fn kind(&self) -> TransportKind {
        self.transport.kind()
    }

    /// Refuse any operation that would modify the chip.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// Whether this session refuses modifications.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// The chip found by the last [`Device::identify`] call.
    pub fn chip(&self) -> Option<&DetectedChip> {
        self.chip.as_ref()
    }

    /// Borrow the underlying transport, for callers that need to issue a command
    /// this layer does not wrap.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    fn guard_write(&self, command: Command) -> DeviceResult<()> {
        if self.read_only && command.is_destructive() {
            return Err(DeviceError::ReadOnlySession { command });
        }
        Ok(())
    }

    fn require_interface(&self, interface: FlashInterface) -> DeviceResult<()> {
        if !self.version.supports(interface) {
            return Err(DeviceError::InterfaceUnsupported { interface });
        }
        Ok(())
    }

    /// Select the interface subsequent commands address.
    pub fn set_interface(&mut self, interface: FlashInterface) -> DeviceResult<()> {
        self.require_interface(interface)?;
        self.transport
            .transact(Command::SetInterface, &[interface as u8], DEFAULT_TIMEOUT)?;
        Ok(())
    }

    /// Read the chip id and look it up in the database.
    pub fn identify(&mut self) -> DeviceResult<DetectedChip> {
        self.require_interface(FlashInterface::SpiNor)?;

        let id =
            self.transport
                .transact_exact(Command::SpiNorReadJedecId, &[], 3, DEFAULT_TIMEOUT)?;
        let jedec_id = [id[0], id[1], id[2]];

        // A bus with nothing on it, or a chip held in reset, reads as all-ones
        // or all-zeroes. Reporting that as a chip is how a tool ends up
        // "identifying" hardware that is not connected.
        if jedec_id.iter().all(|&b| b == 0xFF) || jedec_id.iter().all(|&b| b == 0x00) {
            return Err(DeviceError::NoChipDetected { id });
        }

        let info = get_spi_nor_chip_info(&jedec_id).ok_or(DeviceError::UnknownChip { jedec_id })?;
        // The database falls back to a capacity-derived guess when the exact
        // part is unknown; say so rather than presenting a guess as a match.
        let exact_match = info.model.chars().any(|c| c.is_ascii_digit())
            && !info.model.to_ascii_lowercase().contains("generic");

        let chip = DetectedChip::from_chip_info(&info, exact_match);
        self.chip = Some(chip.clone());
        Ok(chip)
    }

    fn capacity(&mut self) -> DeviceResult<u64> {
        if let Some(chip) = &self.chip {
            return Ok(chip.capacity);
        }
        Ok(self.identify()?.capacity)
    }

    fn check_range(&mut self, start: u64, length: u64) -> DeviceResult<()> {
        let capacity = self.capacity()?;
        if start.saturating_add(length) > capacity {
            return Err(DeviceError::RangeOutOfBounds {
                start,
                length,
                capacity,
            });
        }
        Ok(())
    }

    /// Read `length` bytes from `start` into `sink`.
    ///
    /// Streams in chunks so memory use stays constant regardless of chip size.
    pub fn read_into<W: Write>(
        &mut self,
        start: u64,
        length: u64,
        sink: &mut W,
        progress: Option<ProgressFn<'_>>,
    ) -> DeviceResult<ReadReport> {
        self.require_interface(FlashInterface::SpiNor)?;
        self.check_range(start, length)?;

        let mut noop = no_progress;
        let progress: ProgressFn<'_> = match progress {
            Some(callback) => callback,
            None => &mut noop,
        };

        let started = Instant::now();
        let mut done = 0u64;
        let mut chunks = 0u64;

        progress(0, length);
        while done < length {
            let chunk = MAX_CHUNK.min((length - done) as usize);
            let address = (start + done) as u32;

            let mut request = address.to_le_bytes().to_vec();
            request.extend_from_slice(&(chunk as u16).to_le_bytes());

            let data = self.transport.transact_exact(
                Command::SpiNorRead,
                &request,
                chunk,
                DEFAULT_TIMEOUT,
            )?;

            sink.write_all(&data)?;
            done += chunk as u64;
            chunks += 1;
            progress(done, length);
        }
        sink.flush()?;

        Ok(ReadReport {
            bytes_read: done,
            chunks,
            duration: started.elapsed(),
        })
    }

    /// Read `length` bytes from `start` into memory.
    ///
    /// Prefer [`Device::read_into`] for whole-chip dumps.
    pub fn read_to_vec(&mut self, start: u64, length: u64) -> DeviceResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(length as usize);
        self.read_into(start, length, &mut buffer, None)?;
        Ok(buffer)
    }

    fn sector_size(&mut self) -> DeviceResult<u64> {
        if let Some(chip) = &self.chip {
            return Ok(chip.sector_size.max(1) as u64);
        }
        Ok(self.identify()?.sector_size.max(1) as u64)
    }

    fn write_enable(&mut self) -> DeviceResult<()> {
        self.transport
            .transact(Command::SpiNorWriteEnable, &[], DEFAULT_TIMEOUT)?;
        Ok(())
    }

    fn erase_one_sector(&mut self, address: u64) -> DeviceResult<()> {
        self.write_enable()?;
        self.transport.transact(
            Command::SpiNorSectorErase,
            &(address as u32).to_le_bytes(),
            ERASE_TIMEOUT,
        )?;
        Ok(())
    }

    fn program_one_page(&mut self, address: u64, data: &[u8]) -> DeviceResult<()> {
        debug_assert!(data.len() <= PAGE_SIZE);
        debug_assert!(
            address as usize % PAGE_SIZE + data.len() <= PAGE_SIZE,
            "a program at {address:#x} of {} bytes would cross a page boundary",
            data.len()
        );

        self.write_enable()?;
        let mut request = (address as u32).to_le_bytes().to_vec();
        request.extend_from_slice(data);
        self.transport
            .transact(Command::SpiNorPageProgram, &request, DEFAULT_TIMEOUT)?;
        Ok(())
    }

    /// Erase whole sectors covering `start..start + length`.
    ///
    /// Erase granularity is a property of the chip, so the range is rejected
    /// unless it is sector-aligned: erasing more than was asked for would
    /// destroy neighbouring data without the caller knowing. Use
    /// [`Device::program`], which preserves partially covered sectors, to write
    /// at an arbitrary offset.
    pub fn erase_range(&mut self, start: u64, length: u64) -> DeviceResult<u64> {
        self.guard_write(Command::SpiNorSectorErase)?;
        self.require_interface(FlashInterface::SpiNor)?;
        self.check_range(start, length)?;

        let sector = self.sector_size()?;
        if start % sector != 0 || length % sector != 0 {
            return Err(DeviceError::RangeOutOfBounds {
                start,
                length,
                capacity: self.capacity()?,
            });
        }

        let mut erased = 0u64;
        let mut address = start;
        while address < start + length {
            self.erase_one_sector(address)?;
            address += sector;
            erased += 1;
        }
        Ok(erased)
    }

    /// Confirm that `start..start + length` reads as erased.
    pub fn verify_erased(&mut self, start: u64, length: u64) -> DeviceResult<()> {
        let mut done = 0u64;
        while done < length {
            let chunk = MAX_CHUNK.min((length - done) as usize);
            let data = self.read_to_vec(start + done, chunk as u64)?;
            if let Some(index) = data.iter().position(|&b| b != 0xFF) {
                return Err(DeviceError::EraseFailed {
                    offset: start + done + index as u64,
                    found: data[index],
                });
            }
            done += chunk as u64;
        }
        Ok(())
    }

    /// Write `data` to `start`.
    ///
    /// Erases first by default. A sector the range covers only partially is read
    /// out, patched in memory and rewritten, so data next to the target range
    /// survives.
    pub fn program(
        &mut self,
        start: u64,
        data: &[u8],
        options: ProgramOptions,
        progress: Option<ProgressFn<'_>>,
    ) -> DeviceResult<ProgramReport> {
        self.guard_write(Command::SpiNorPageProgram)?;
        self.require_interface(FlashInterface::SpiNor)?;
        self.check_range(start, data.len() as u64)?;

        let mut noop = no_progress;
        let progress: ProgressFn<'_> = match progress {
            Some(callback) => callback,
            None => &mut noop,
        };

        let total = data.len() as u64;
        let mut report = ProgramReport::default();
        if data.is_empty() {
            return Ok(report);
        }

        let sector = self.sector_size()?;
        let end = start + total;
        let first_sector = start / sector;
        let last_sector = (end - 1) / sector;

        progress(0, total);

        for index in first_sector..=last_sector {
            let sector_start = index * sector;
            let overlap_start = start.max(sector_start);
            let overlap_end = end.min(sector_start + sector);
            let partial = overlap_start > sector_start || overlap_end < sector_start + sector;

            // The bytes to end up in this sector, and where they start.
            let (buffer, buffer_start) = if options.erase_first && partial {
                // Preserve what the caller did not ask to change.
                let mut existing = self.read_to_vec(sector_start, sector)?;
                let offset = (overlap_start - sector_start) as usize;
                let slice_from = (overlap_start - start) as usize;
                let slice_to = (overlap_end - start) as usize;
                existing[offset..offset + (slice_to - slice_from)]
                    .copy_from_slice(&data[slice_from..slice_to]);
                report.sectors_preserved += 1;
                (existing, sector_start)
            } else {
                let slice_from = (overlap_start - start) as usize;
                let slice_to = (overlap_end - start) as usize;
                (data[slice_from..slice_to].to_vec(), overlap_start)
            };

            if options.erase_first {
                self.erase_one_sector(sector_start)?;
                report.sectors_erased += 1;
            }

            // Program page by page, never crossing a page boundary.
            let mut written = 0usize;
            while written < buffer.len() {
                let address = buffer_start + written as u64;
                let page_room = PAGE_SIZE - (address as usize % PAGE_SIZE);
                let chunk = page_room.min(buffer.len() - written);
                let page = &buffer[written..written + chunk];

                if options.skip_blank_pages && page.iter().all(|&b| b == 0xFF) {
                    report.pages_skipped += 1;
                } else {
                    self.program_one_page(address, page)?;
                    report.pages_written += 1;
                    report.bytes_written += chunk as u64;
                }
                written += chunk;
            }

            progress(overlap_end - start, total);
        }

        if options.verify {
            self.verify(start, data, None)?;
            report.verified = true;
        }

        Ok(report)
    }

    /// Read `start..start + expected.len()` back and compare it to `expected`.
    pub fn verify(
        &mut self,
        start: u64,
        expected: &[u8],
        progress: Option<ProgressFn<'_>>,
    ) -> DeviceResult<()> {
        self.require_interface(FlashInterface::SpiNor)?;
        self.check_range(start, expected.len() as u64)?;

        let mut noop = no_progress;
        let progress: ProgressFn<'_> = match progress {
            Some(callback) => callback,
            None => &mut noop,
        };

        let total = expected.len() as u64;
        let mut done = 0u64;
        progress(0, total);

        while done < total {
            let chunk = MAX_CHUNK.min((total - done) as usize);
            let actual = self.read_to_vec(start + done, chunk as u64)?;
            let slice = &expected[done as usize..done as usize + chunk];

            if let Some(index) = actual.iter().zip(slice).position(|(a, b)| a != b) {
                return Err(DeviceError::VerificationFailed {
                    offset: start + done + index as u64,
                    expected: slice[index],
                    found: actual[index],
                });
            }
            done += chunk as u64;
            progress(done, total);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulator::EmulatedDevice;

    fn connected() -> Device<EmulatedDevice> {
        Device::connect(EmulatedDevice::w25q16jv()).expect("emulator handshake")
    }

    #[test]
    fn connecting_reads_the_device_version() {
        let device = connected();
        assert_eq!(device.version().protocol, PROTOCOL_VERSION);
        assert!(device.version().supports(FlashInterface::SpiNor));
    }

    #[test]
    fn identify_names_the_chip_from_its_jedec_id() {
        let mut device = connected();
        let chip = device.identify().unwrap();
        assert_eq!(chip.jedec_id, [0xEF, 0x40, 0x15]);
        assert_eq!(chip.manufacturer, "Winbond");
        assert_eq!(chip.model, "W25Q16JV");
        assert_eq!(chip.capacity, 2 * 1024 * 1024);
        assert_eq!(chip.sector_size, 4096);
    }

    /// A transport that answers the id query with a fixed value, for simulating
    /// a bus with nothing on it. The emulator cannot do this: it derives its id
    /// from its own array, so it can only report a chip that exists.
    struct FixedIdDevice([u8; 3]);

    impl Transport for FixedIdDevice {
        fn kind(&self) -> TransportKind {
            TransportKind::Emulator {
                chip: "fixed-id".into(),
            }
        }

        fn exchange(
            &mut self,
            request: &[u8],
            _timeout: Duration,
        ) -> crate::transport::TransportResult<Vec<u8>> {
            let frame = openflash_protocol::Frame::decode(request).unwrap();
            let command = frame.decoded_command().unwrap();
            let payload = match command {
                Command::SpiNorReadJedecId => self.0.to_vec(),
                Command::GetVersion => VersionInfo {
                    protocol: PROTOCOL_VERSION,
                    firmware: (3, 1, 0),
                    platform: Some(openflash_protocol::Platform::Rp2040),
                    interfaces: VersionInfo::bitmap_of(&[FlashInterface::SpiNor]),
                }
                .to_bytes()
                .to_vec(),
                _ => Vec::new(),
            };
            Ok(openflash_protocol::Frame::response(
                command,
                openflash_protocol::Status::Ok,
                &payload,
            )
            .encode_to_vec()
            .unwrap())
        }
    }

    /// A disconnected bus reads back as all-ones. Reporting that as a chip is
    /// exactly how a tool ends up "detecting" hardware that is not there.
    #[test]
    fn an_empty_bus_is_reported_as_no_chip() {
        let mut device = Device::connect(FixedIdDevice([0xFF, 0xFF, 0xFF])).unwrap();

        match device.identify() {
            Err(DeviceError::NoChipDetected { id }) => assert_eq!(id, vec![0xFF, 0xFF, 0xFF]),
            Err(other) => panic!("expected no-chip detection, got {other:?}"),
            Ok(chip) => panic!("an empty bus must not identify as {chip:?}"),
        }
    }

    #[test]
    fn a_grounded_bus_is_reported_as_no_chip() {
        let mut device = Device::connect(FixedIdDevice([0x00, 0x00, 0x00])).unwrap();
        assert!(matches!(
            device.identify(),
            Err(DeviceError::NoChipDetected { .. })
        ));
    }

    /// An id whose capacity byte decodes to no known size must be reported as
    /// unknown rather than guessed at.
    #[test]
    fn an_unrecognised_capacity_byte_is_reported_as_an_unknown_chip() {
        let mut device = Device::connect(FixedIdDevice([0xEF, 0x40, 0x7F])).unwrap();
        match device.identify() {
            Err(DeviceError::UnknownChip { jedec_id }) => {
                assert_eq!(jedec_id, [0xEF, 0x40, 0x7F])
            }
            Err(other) => panic!("expected an unknown chip, got {other:?}"),
            Ok(chip) => panic!("an unknown capacity byte must not identify as {chip:?}"),
        }
    }

    #[test]
    fn reading_a_whole_chip_streams_it_to_the_sink() {
        let size = 64 * 1024;
        let image: Vec<u8> = (0..size).map(|i| (i % 253) as u8).collect();
        let mut device = Device::connect(EmulatedDevice::with_image(
            "PRELOAD",
            [0xEF, 0x40, 0x15],
            image.clone(),
        ))
        .unwrap();

        let mut sink = Vec::new();
        let mut seen: Vec<(u64, u64)> = Vec::new();
        let report = device
            .read_into(
                0,
                size as u64,
                &mut sink,
                Some(&mut |done, total| seen.push((done, total))),
            )
            .unwrap();

        assert_eq!(sink, image, "the dump must match the chip byte for byte");
        assert_eq!(report.bytes_read, size as u64);
        assert!(report.chunks >= 1);
        // Progress must start at zero and finish at the total.
        assert_eq!(seen.first(), Some(&(0, size as u64)));
        assert_eq!(seen.last(), Some(&(size as u64, size as u64)));
    }

    #[test]
    fn reading_past_the_end_of_the_chip_is_refused_before_any_io() {
        let mut device = connected();
        let capacity = device.identify().unwrap().capacity;

        match device.read_to_vec(capacity - 4, 64) {
            Err(DeviceError::RangeOutOfBounds { capacity: c, .. }) => assert_eq!(c, capacity),
            other => panic!("expected an out-of-bounds error, got {other:?}"),
        }
    }

    /// The end-to-end property that matters: what was written comes back.
    #[test]
    fn program_then_read_returns_exactly_what_was_written() {
        let mut device = connected();
        let payload: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();

        let report = device
            .program(0, &payload, ProgramOptions::default(), None)
            .unwrap();
        assert!(report.verified);
        assert!(report.sectors_erased > 0, "must erase before programming");

        let read_back = device.read_to_vec(0, payload.len() as u64).unwrap();
        assert_eq!(read_back, payload);
    }

    #[test]
    fn programming_at_an_unaligned_offset_preserves_the_neighbours() {
        let mut device = connected();

        // Fill two sectors with a known pattern.
        let background: Vec<u8> = (0..8192u32).map(|i| (i % 97) as u8 | 0x80).collect();
        device
            .program(0, &background, ProgramOptions::default(), None)
            .unwrap();

        // Overwrite 16 bytes in the middle of the first sector.
        let patch = [0x11u8; 16];
        device
            .program(2000, &patch, ProgramOptions::default(), None)
            .unwrap();

        let mut expected = background.clone();
        expected[2000..2016].copy_from_slice(&patch);

        let actual = device.read_to_vec(0, expected.len() as u64).unwrap();
        assert_eq!(
            actual, expected,
            "programming part of a sector must not disturb the rest"
        );
    }

    #[test]
    fn programming_spanning_a_sector_boundary_works() {
        let mut device = connected();
        let start = 4096 - 100;
        let payload: Vec<u8> = (0..400u32).map(|i| (i % 250) as u8).collect();

        device
            .program(start, &payload, ProgramOptions::default(), None)
            .unwrap();
        assert_eq!(
            device.read_to_vec(start, payload.len() as u64).unwrap(),
            payload
        );
    }

    #[test]
    fn programming_spanning_a_page_boundary_works() {
        let mut device = connected();
        // A page is 256 bytes; start 10 bytes before a boundary.
        let start = 246;
        let payload: Vec<u8> = (0..600u32).map(|i| (i % 199) as u8).collect();

        device
            .program(start, &payload, ProgramOptions::default(), None)
            .unwrap();
        assert_eq!(
            device.read_to_vec(start, payload.len() as u64).unwrap(),
            payload
        );
    }

    /// Without an erase, programming can only clear bits. The device layer must
    /// not paper over that: with `erase_first` off over non-erased data, the
    /// verify step has to fail rather than report success.
    #[test]
    fn programming_without_erasing_over_written_data_fails_verification() {
        let mut device = connected();

        device
            .program(0, &[0x0F; 64], ProgramOptions::default(), None)
            .unwrap();

        let result = device.program(
            0,
            &[0xF0; 64],
            ProgramOptions {
                erase_first: false,
                verify: true,
                skip_blank_pages: true,
            },
            None,
        );

        match result {
            Err(DeviceError::VerificationFailed {
                offset,
                expected,
                found,
            }) => {
                assert_eq!(offset, 0);
                assert_eq!(expected, 0xF0);
                assert_eq!(found, 0x00, "0x0F programmed with 0xF0 gives 0x00");
            }
            other => panic!("expected verification to fail, got {other:?}"),
        }
    }

    #[test]
    fn blank_pages_are_skipped_but_still_read_back_as_blank() {
        let mut device = connected();
        let mut payload = vec![0xFFu8; 1024];
        payload[512] = 0x42;

        let report = device
            .program(0, &payload, ProgramOptions::default(), None)
            .unwrap();

        assert!(
            report.pages_skipped >= 3,
            "all-0xFF pages should be skipped, got {report:?}"
        );
        assert_eq!(report.pages_written, 1);
        assert_eq!(device.read_to_vec(0, 1024).unwrap(), payload);
    }

    #[test]
    fn skip_blank_pages_off_writes_every_page() {
        let mut device = connected();
        // 1024 bytes covers only part of a 4096-byte sector, so the sector is
        // read out and rewritten in full: 4096 / 256 = 16 pages.
        let report = device
            .program(
                0,
                &[0xFFu8; 1024],
                ProgramOptions {
                    erase_first: true,
                    verify: true,
                    skip_blank_pages: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(report.pages_skipped, 0);
        assert_eq!(report.pages_written, 16);
        assert_eq!(report.sectors_preserved, 1);

        // A payload that fills whole sectors needs no read-modify-write.
        let report = device
            .program(
                0,
                &[0xFFu8; 4096],
                ProgramOptions {
                    erase_first: true,
                    verify: true,
                    skip_blank_pages: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(report.pages_written, 16);
        assert_eq!(report.sectors_preserved, 0);
    }

    #[test]
    fn erase_range_clears_and_verifies() {
        let mut device = connected();
        device
            .program(0, &[0x00; 8192], ProgramOptions::default(), None)
            .unwrap();

        let sectors = device.erase_range(0, 8192).unwrap();
        assert_eq!(sectors, 2);
        device.verify_erased(0, 8192).unwrap();
    }

    #[test]
    fn erase_range_refuses_an_unaligned_range_rather_than_erasing_a_neighbour() {
        let mut device = connected();
        assert!(matches!(
            device.erase_range(100, 4096),
            Err(DeviceError::RangeOutOfBounds { .. })
        ));
        assert!(matches!(
            device.erase_range(0, 100),
            Err(DeviceError::RangeOutOfBounds { .. })
        ));
    }

    #[test]
    fn verify_reports_the_first_differing_byte() {
        let mut device = connected();
        let payload: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();
        device
            .program(0, &payload, ProgramOptions::default(), None)
            .unwrap();

        let mut wrong = payload.clone();
        wrong[700] ^= 0xFF;

        match device.verify(0, &wrong, None) {
            Err(DeviceError::VerificationFailed { offset, .. }) => assert_eq!(offset, 700),
            other => panic!("expected verification to fail, got {other:?}"),
        }
    }

    #[test]
    fn a_read_only_session_refuses_writes_and_erases_but_allows_reads() {
        let mut device = connected();
        device.identify().unwrap();
        device.set_read_only(true);

        match device.program(0, &[0x00; 16], ProgramOptions::default(), None) {
            Err(DeviceError::ReadOnlySession { command }) => {
                assert_eq!(command, Command::SpiNorPageProgram)
            }
            other => panic!("expected the write to be blocked, got {other:?}"),
        }
        assert!(matches!(
            device.erase_range(0, 4096),
            Err(DeviceError::ReadOnlySession { .. })
        ));

        // Reads must still work: that is the whole point of the mode.
        assert_eq!(device.read_to_vec(0, 16).unwrap().len(), 16);
    }

    #[test]
    fn a_write_protected_chip_surfaces_the_chip_s_refusal() {
        let mut chip = EmulatedDevice::w25q16jv();
        chip.set_write_protected(true);
        let mut device = Device::connect(chip).unwrap();

        match device.program(0, &[0x00; 16], ProgramOptions::default(), None) {
            Err(DeviceError::Transport(TransportError::Device { status, .. })) => {
                assert_eq!(status, openflash_protocol::Status::WriteProtected)
            }
            other => panic!("expected the chip to refuse the write, got {other:?}"),
        }
    }

    #[test]
    fn requesting_an_interface_the_device_lacks_is_refused() {
        let mut device = connected();
        match device.set_interface(FlashInterface::Emmc) {
            Err(DeviceError::InterfaceUnsupported { interface }) => {
                assert_eq!(interface, FlashInterface::Emmc)
            }
            other => panic!("expected the interface to be refused, got {other:?}"),
        }
        device.set_interface(FlashInterface::SpiNor).unwrap();
    }

    #[test]
    fn a_device_speaking_another_protocol_revision_is_refused_at_connect() {
        // A transport that answers GetVersion with a different revision.
        struct OldFirmware;

        impl Transport for OldFirmware {
            fn kind(&self) -> TransportKind {
                TransportKind::Emulator { chip: "old".into() }
            }

            fn exchange(
                &mut self,
                request: &[u8],
                _timeout: Duration,
            ) -> crate::transport::TransportResult<Vec<u8>> {
                let frame = openflash_protocol::Frame::decode(request).unwrap();
                let command = frame.decoded_command().unwrap();
                let payload = if command == Command::GetVersion {
                    VersionInfo {
                        protocol: 1,
                        firmware: (2, 3, 0),
                        platform: Some(openflash_protocol::Platform::Rp2040),
                        interfaces: 0xFF,
                    }
                    .to_bytes()
                    .to_vec()
                } else {
                    Vec::new()
                };
                Ok(openflash_protocol::Frame::response(
                    command,
                    openflash_protocol::Status::Ok,
                    &payload,
                )
                .encode_to_vec()
                .unwrap())
            }
        }

        match Device::connect(OldFirmware) {
            Err(DeviceError::ProtocolMismatch { device, host }) => {
                assert_eq!(device, 1);
                assert_eq!(host, PROTOCOL_VERSION);
            }
            Err(other) => panic!("expected a protocol mismatch, got {other:?}"),
            Ok(_) => panic!("a v1 device must not be accepted by a v2 host"),
        }
    }

    #[test]
    fn read_report_computes_throughput() {
        let report = ReadReport {
            bytes_read: 1024,
            chunks: 1,
            duration: Duration::from_millis(500),
        };
        assert_eq!(report.bytes_per_second(), Some(2048));

        let instant = ReadReport {
            bytes_read: 1024,
            chunks: 1,
            duration: Duration::ZERO,
        };
        assert_eq!(instant.bytes_per_second(), None);
    }

    #[test]
    fn programming_the_whole_chip_round_trips() {
        // A small chip so the test stays quick, but exercising every sector and
        // the chunking logic end to end.
        let mut device =
            Device::connect(EmulatedDevice::new("SMALL", [0xEF, 0x40, 0x15], 64 * 1024)).unwrap();

        let payload: Vec<u8> = (0..64 * 1024u32).map(|i| (i * 31 % 257) as u8).collect();
        let report = device
            .program(0, &payload, ProgramOptions::default(), None)
            .unwrap();

        assert_eq!(report.sectors_erased, 16);
        assert!(report.verified);
        assert_eq!(
            device.read_to_vec(0, payload.len() as u64).unwrap(),
            payload
        );
    }
}
