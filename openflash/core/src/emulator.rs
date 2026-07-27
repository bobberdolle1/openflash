//! An in-process device that speaks the real protocol.
//!
//! This is a test double, not a stand-in for a missing feature. It implements
//! the device side of [`crate::transport`] against a byte array, with the
//! semantics an actual SPI NOR part has:
//!
//! - erased flash reads as `0xFF`,
//! - a program operation can only clear bits, never set them, so programming
//!   `0xF0` over `0x0F` yields `0x00` — it does not overwrite,
//! - a page program that runs past a 256-byte page boundary wraps to the start
//!   of that page instead of continuing into the next one,
//! - programming and erasing require the write-enable latch, which the chip
//!   clears again afterwards,
//! - erase works on 4 KiB, 32 KiB and 64 KiB granularities and only at aligned
//!   addresses.
//!
//! Getting those details right is the point: a host that passes against this
//! emulator has been shown to erase before writing, to respect page boundaries
//! and to verify what it wrote, rather than merely to have called the right
//! functions in the right order.
//!
//! It is exposed outside tests too, because it is what `openflash --emulate`
//! drives — a self-check of the whole host stack that needs no hardware, and is
//! labelled as emulated everywhere it appears.

use std::path::{Path, PathBuf};
use std::time::Duration;

use openflash_protocol::frame::Frame;
use openflash_protocol::{
    Command, FlashInterface, Platform, Status, VersionInfo, PROTOCOL_VERSION,
};

use crate::transport::{Transport, TransportKind, TransportResult};

/// Page size for programming: a program may not cross this boundary.
pub const PAGE_SIZE: usize = 256;

/// Smallest erasable unit.
pub const SECTOR_SIZE: usize = 4 * 1024;

/// Status register 1, bit 0: write in progress.
pub const STATUS_WIP: u8 = 0x01;

/// Status register 1, bit 1: write-enable latch.
pub const STATUS_WEL: u8 = 0x02;

/// Counters describing what a host actually did to the emulated chip.
///
/// Tests assert on these to catch a host that "succeeds" without doing the
/// work — programming without erasing first, for instance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmulatorStats {
    /// Number of read operations served.
    pub reads: u64,
    /// Number of bytes read.
    pub bytes_read: u64,
    /// Number of page program operations performed.
    pub programs: u64,
    /// Number of bytes programmed.
    pub bytes_programmed: u64,
    /// Number of erase operations performed.
    pub erases: u64,
    /// Number of program attempts refused because the latch was not set.
    pub refused_without_write_enable: u64,
    /// Number of commands refused as unsupported.
    pub unsupported_commands: u64,
}

/// An emulated SPI NOR device.
pub struct EmulatedDevice {
    image: Vec<u8>,
    jedec_id: [u8; 3],
    model: String,
    write_enabled: bool,
    write_protected: bool,
    interface: FlashInterface,
    stats: EmulatorStats,
    backing_file: Option<PathBuf>,
}

/// Capacity byte of a JEDEC id for a chip of `size` bytes.
///
/// The third id byte is the base-2 logarithm of the size, so it has to be
/// derived from the array rather than chosen independently: an id that claims a
/// different capacity than the emulated array would make the host compute
/// addresses the emulated chip rejects, and the emulator would be testing the
/// host against a chip that could not exist.
fn capacity_byte_for(size: usize) -> u8 {
    debug_assert!(size.is_power_of_two(), "size must be a power of two");
    size.trailing_zeros() as u8
}

impl EmulatedDevice {
    /// Create an erased device of `size` bytes.
    ///
    /// `size` must be a power of two and at least [`SECTOR_SIZE`]. The JEDEC id's
    /// capacity byte is derived from it, so the id the host reads always agrees
    /// with how much memory there actually is.
    pub fn new(model: &str, jedec_id: [u8; 3], size: usize) -> Self {
        Self::with_image(model, jedec_id, vec![0xFF; size])
    }

    /// A 2 MiB Winbond W25Q16JV, the part most commonly wired up for testing.
    pub fn w25q16jv() -> Self {
        Self::new("W25Q16JV", [0xEF, 0x40, 0x15], 2 * 1024 * 1024)
    }

    /// Create a device preloaded with `image`.
    pub fn with_image(model: &str, jedec_id: [u8; 3], image: Vec<u8>) -> Self {
        assert!(
            image.len() >= SECTOR_SIZE && image.len().is_power_of_two(),
            "emulated image length {} must be a power of two of at least {SECTOR_SIZE} bytes",
            image.len()
        );

        let mut jedec_id = jedec_id;
        jedec_id[2] = capacity_byte_for(image.len());

        Self {
            image,
            jedec_id,
            model: model.to_string(),
            write_enabled: false,
            write_protected: false,
            interface: FlashInterface::SpiNor,
            stats: EmulatorStats::default(),
            backing_file: None,
        }
    }

    /// Create a device whose contents live in a file.
    ///
    /// The image is loaded if the file exists, and created blank at `size` bytes
    /// if it does not. Changes are written back when the device is dropped, so
    /// the emulated chip keeps its contents between separate runs of the CLI —
    /// without that, a `write` followed by a `read` in two processes would talk
    /// to two different blank chips.
    pub fn with_backing_file(
        model: &str,
        jedec_id: [u8; 3],
        path: &Path,
        size: usize,
    ) -> std::io::Result<Self> {
        let image = if path.exists() {
            let loaded = std::fs::read(path)?;
            if loaded.len() < SECTOR_SIZE || !loaded.len().is_power_of_two() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} is {} bytes; an emulated image must be a power of two of at \
                         least {SECTOR_SIZE} bytes",
                        path.display(),
                        loaded.len()
                    ),
                ));
            }
            loaded
        } else {
            vec![0xFF; size]
        };

        let mut device = Self::with_image(model, jedec_id, image);
        device.backing_file = Some(path.to_path_buf());
        device.persist()?;
        Ok(device)
    }

    /// Write the current contents to the backing file, if there is one.
    pub fn persist(&self) -> std::io::Result<()> {
        match &self.backing_file {
            Some(path) => std::fs::write(path, &self.image),
            None => Ok(()),
        }
    }

    /// Simulate the WP# pin being held low: every write and erase is refused.
    pub fn set_write_protected(&mut self, protected: bool) {
        self.write_protected = protected;
    }

    /// The current chip contents.
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// Chip size in bytes.
    pub fn size(&self) -> usize {
        self.image.len()
    }

    /// Model name, for display.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// What the host has done to this chip so far.
    pub fn stats(&self) -> EmulatorStats {
        self.stats
    }

    fn version_info(&self) -> VersionInfo {
        VersionInfo {
            protocol: PROTOCOL_VERSION,
            firmware: (3, 1, 0),
            platform: Some(Platform::Rp2040),
            interfaces: VersionInfo::bitmap_of(&[FlashInterface::SpiNor]),
        }
    }

    /// Handle one request frame and produce the response frame bytes.
    ///
    /// Never panics on bad input: a malformed request gets an error status, the
    /// way firmware must behave.
    pub fn handle_frame(&mut self, request: &[u8]) -> Vec<u8> {
        let frame = match Frame::decode(request) {
            Ok(frame) => frame,
            // A frame the device cannot parse cannot be attributed to a command,
            // so answer with a zero command byte and a generic error. A real host
            // will report this as a command mismatch, which is the truth.
            Err(_) => {
                return Frame::response_raw(0x00, Status::Error, &[])
                    .encode_to_vec()
                    .expect("empty payload always encodes")
            }
        };

        let Some(command) = frame.decoded_command() else {
            self.stats.unsupported_commands += 1;
            return Frame::response_raw(frame.command, Status::UnsupportedCommand, &[])
                .encode_to_vec()
                .expect("empty payload always encodes");
        };

        let (status, payload) = self.dispatch(command, frame.payload);
        Frame::response(command, status, &payload)
            .encode_to_vec()
            .expect("payloads produced here are within the frame limit")
    }

    fn dispatch(&mut self, command: Command, payload: &[u8]) -> (Status, Vec<u8>) {
        match command {
            Command::Ping | Command::Reset => {
                self.write_enabled = false;
                (Status::Ok, Vec::new())
            }
            Command::GetVersion => (Status::Ok, self.version_info().to_bytes().to_vec()),
            Command::GetCapabilities => (Status::Ok, vec![self.version_info().interfaces]),
            Command::BusConfig => (Status::Ok, Vec::new()),
            Command::SetInterface => {
                match payload.first().copied().and_then(FlashInterface::from_u8) {
                    Some(FlashInterface::SpiNor) => {
                        self.interface = FlashInterface::SpiNor;
                        (Status::Ok, Vec::new())
                    }
                    // Honest about being a SPI NOR part only, rather than accepting
                    // an interface it cannot serve.
                    Some(_) => (Status::UnsupportedOnInterface, Vec::new()),
                    None => (Status::InvalidArgument, Vec::new()),
                }
            }

            Command::SpiNorReadJedecId => (Status::Ok, self.jedec_id.to_vec()),
            Command::SpiNorRead | Command::SpiNorFastRead => self.read(payload),
            Command::SpiNorPageProgram => self.page_program(payload),
            Command::SpiNorSectorErase => self.erase(payload, SECTOR_SIZE),
            Command::SpiNorBlockErase32K => self.erase(payload, 32 * 1024),
            Command::SpiNorBlockErase64K => self.erase(payload, 64 * 1024),
            Command::SpiNorChipErase => self.chip_erase(),
            Command::SpiNorReadStatus1 => (Status::Ok, vec![self.status_register()]),
            Command::SpiNorReadStatus2 | Command::SpiNorReadStatus3 => (Status::Ok, vec![0x00]),
            Command::SpiNorWriteEnable => {
                self.write_enabled = true;
                (Status::Ok, Vec::new())
            }
            Command::SpiNorWriteDisable => {
                self.write_enabled = false;
                (Status::Ok, Vec::new())
            }
            Command::SpiNorReset => {
                self.write_enabled = false;
                (Status::Ok, Vec::new())
            }

            // Commands for interfaces this emulated part does not have.
            other if other.interface().is_some() => {
                self.stats.unsupported_commands += 1;
                (Status::UnsupportedOnInterface, Vec::new())
            }
            _ => {
                self.stats.unsupported_commands += 1;
                (Status::UnsupportedCommand, Vec::new())
            }
        }
    }

    fn status_register(&self) -> u8 {
        // WIP is never set: the emulator completes operations synchronously.
        if self.write_enabled {
            STATUS_WEL
        } else {
            0
        }
    }

    /// `[address: u32 LE][length: u16 LE]`
    fn read(&mut self, payload: &[u8]) -> (Status, Vec<u8>) {
        if payload.len() < 6 {
            return (Status::InvalidArgument, Vec::new());
        }
        let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        let length = u16::from_le_bytes([payload[4], payload[5]]) as usize;

        if address >= self.image.len() || address + length > self.image.len() {
            return (Status::InvalidArgument, Vec::new());
        }

        self.stats.reads += 1;
        self.stats.bytes_read += length as u64;
        (Status::Ok, self.image[address..address + length].to_vec())
    }

    /// `[address: u32 LE][data…]`
    fn page_program(&mut self, payload: &[u8]) -> (Status, Vec<u8>) {
        if payload.len() < 5 {
            return (Status::InvalidArgument, Vec::new());
        }
        let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        let data = &payload[4..];

        if data.len() > PAGE_SIZE {
            return (Status::InvalidArgument, Vec::new());
        }
        if address >= self.image.len() {
            return (Status::InvalidArgument, Vec::new());
        }
        if self.write_protected {
            return (Status::WriteProtected, Vec::new());
        }
        if !self.write_enabled {
            self.stats.refused_without_write_enable += 1;
            return (Status::WriteProtected, Vec::new());
        }

        // Real parts wrap within the page rather than spilling into the next one.
        let page_base = address & !(PAGE_SIZE - 1);
        for (index, &byte) in data.iter().enumerate() {
            let offset = page_base + ((address - page_base + index) % PAGE_SIZE);
            // Programming clears bits; it cannot set them.
            self.image[offset] &= byte;
        }

        self.stats.programs += 1;
        self.stats.bytes_programmed += data.len() as u64;
        self.write_enabled = false;
        (Status::Ok, Vec::new())
    }

    /// `[address: u32 LE]`
    fn erase(&mut self, payload: &[u8], granularity: usize) -> (Status, Vec<u8>) {
        if payload.len() < 4 {
            return (Status::InvalidArgument, Vec::new());
        }
        let address = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;

        if address % granularity != 0 || address + granularity > self.image.len() {
            return (Status::InvalidArgument, Vec::new());
        }
        if self.write_protected {
            return (Status::WriteProtected, Vec::new());
        }
        if !self.write_enabled {
            self.stats.refused_without_write_enable += 1;
            return (Status::WriteProtected, Vec::new());
        }

        self.image[address..address + granularity].fill(0xFF);
        self.stats.erases += 1;
        self.write_enabled = false;
        (Status::Ok, Vec::new())
    }

    fn chip_erase(&mut self) -> (Status, Vec<u8>) {
        if self.write_protected {
            return (Status::WriteProtected, Vec::new());
        }
        if !self.write_enabled {
            self.stats.refused_without_write_enable += 1;
            return (Status::WriteProtected, Vec::new());
        }
        self.image.fill(0xFF);
        self.stats.erases += 1;
        self.write_enabled = false;
        (Status::Ok, Vec::new())
    }
}

impl Drop for EmulatedDevice {
    fn drop(&mut self) {
        // Reported rather than ignored: silently losing the emulated image would
        // make a later read look like the write never happened.
        if let Err(error) = self.persist() {
            eprintln!(
                "warning: cannot save the emulated chip image to {}: {error}",
                self.backing_file
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
        }
    }
}

impl Transport for EmulatedDevice {
    fn kind(&self) -> TransportKind {
        TransportKind::Emulator {
            chip: format!("{} ({} KiB)", self.model, self.size() / 1024),
        }
    }

    fn exchange(&mut self, request: &[u8], _timeout: Duration) -> TransportResult<Vec<u8>> {
        Ok(self.handle_frame(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{TransportError, DEFAULT_TIMEOUT};

    fn small_device() -> EmulatedDevice {
        EmulatedDevice::new("TEST16K", [0xEF, 0x40, 0x15], 16 * 1024)
    }

    fn read(device: &mut EmulatedDevice, address: u32, length: u16) -> Vec<u8> {
        let mut payload = address.to_le_bytes().to_vec();
        payload.extend_from_slice(&length.to_le_bytes());
        device
            .transact(Command::SpiNorRead, &payload, DEFAULT_TIMEOUT)
            .unwrap()
    }

    fn write_enable(device: &mut EmulatedDevice) {
        device
            .transact(Command::SpiNorWriteEnable, &[], DEFAULT_TIMEOUT)
            .unwrap();
    }

    fn program(
        device: &mut EmulatedDevice,
        address: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, TransportError> {
        let mut payload = address.to_le_bytes().to_vec();
        payload.extend_from_slice(data);
        device.transact(Command::SpiNorPageProgram, &payload, DEFAULT_TIMEOUT)
    }

    fn erase_sector(device: &mut EmulatedDevice, address: u32) -> Result<Vec<u8>, TransportError> {
        device.transact(
            Command::SpiNorSectorErase,
            &address.to_le_bytes(),
            DEFAULT_TIMEOUT,
        )
    }

    #[test]
    fn a_fresh_device_reads_as_erased() {
        let mut device = small_device();
        assert_eq!(read(&mut device, 0, 64), vec![0xFF; 64]);
    }

    #[test]
    fn it_reports_its_jedec_id() {
        let mut device = EmulatedDevice::w25q16jv();
        let id = device
            .transact(Command::SpiNorReadJedecId, &[], DEFAULT_TIMEOUT)
            .unwrap();
        assert_eq!(id, vec![0xEF, 0x40, 0x15]);
        assert_eq!(device.size(), 2 * 1024 * 1024);
    }

    #[test]
    fn it_reports_a_version_the_host_can_parse() {
        let mut device = small_device();
        let payload = device
            .transact(Command::GetVersion, &[], DEFAULT_TIMEOUT)
            .unwrap();
        let info = VersionInfo::from_bytes(&payload).expect("well-formed version payload");
        assert_eq!(info.protocol, PROTOCOL_VERSION);
        assert!(info.supports(FlashInterface::SpiNor));
        assert!(!info.supports(FlashInterface::ParallelNand));
    }

    #[test]
    fn programming_needs_the_write_enable_latch() {
        let mut device = small_device();

        match program(&mut device, 0, &[0x00]) {
            Err(TransportError::Device {
                status: Status::WriteProtected,
                ..
            }) => {}
            other => panic!("expected a write-protected error, got {other:?}"),
        }
        assert_eq!(device.stats().refused_without_write_enable, 1);
        assert_eq!(
            read(&mut device, 0, 1),
            vec![0xFF],
            "chip must be untouched"
        );

        write_enable(&mut device);
        program(&mut device, 0, &[0x00]).unwrap();
        assert_eq!(read(&mut device, 0, 1), vec![0x00]);
    }

    #[test]
    fn the_latch_clears_after_each_program() {
        let mut device = small_device();
        write_enable(&mut device);
        program(&mut device, 0, &[0xAA]).unwrap();

        // A second program without re-enabling must be refused, as on real parts.
        assert!(program(&mut device, 1, &[0xAA]).is_err());
        assert_eq!(read(&mut device, 1, 1), vec![0xFF]);
    }

    /// The defining property of flash: a program clears bits and cannot set
    /// them. A host that writes without erasing first will produce corrupt
    /// contents, and this is what makes that visible in tests.
    #[test]
    fn programming_only_clears_bits() {
        let mut device = small_device();

        write_enable(&mut device);
        program(&mut device, 0, &[0b1111_0000]).unwrap();
        assert_eq!(read(&mut device, 0, 1), vec![0b1111_0000]);

        // Programming 0b0000_1111 over it cannot restore the high bits.
        write_enable(&mut device);
        program(&mut device, 0, &[0b0000_1111]).unwrap();
        assert_eq!(
            read(&mut device, 0, 1),
            vec![0b0000_0000],
            "program must AND with the existing contents, not replace them"
        );
    }

    #[test]
    fn writing_over_unerased_data_corrupts_it_exactly_as_hardware_would() {
        let mut device = small_device();
        write_enable(&mut device);
        program(&mut device, 0, &[0x0F, 0x0F, 0x0F, 0x0F]).unwrap();

        write_enable(&mut device);
        program(&mut device, 0, &[0xF0, 0xF0, 0xF0, 0xF0]).unwrap();
        assert_eq!(read(&mut device, 0, 4), vec![0x00; 4]);

        // After an erase the same write lands correctly.
        write_enable(&mut device);
        erase_sector(&mut device, 0).unwrap();
        write_enable(&mut device);
        program(&mut device, 0, &[0xF0, 0xF0, 0xF0, 0xF0]).unwrap();
        assert_eq!(read(&mut device, 0, 4), vec![0xF0; 4]);
    }

    #[test]
    fn a_program_crossing_a_page_boundary_wraps_within_the_page() {
        let mut device = small_device();
        // Start 4 bytes before the end of page 0 and write 8 bytes.
        let start = (PAGE_SIZE - 4) as u32;
        let data: Vec<u8> = (0..8u8).map(|i| !(1 << (i % 8))).collect();

        write_enable(&mut device);
        program(&mut device, start, &data).unwrap();

        // The last 4 bytes wrapped to the start of the same page…
        assert_eq!(read(&mut device, start, 4), data[..4].to_vec());
        assert_eq!(read(&mut device, 0, 4), data[4..].to_vec());
        // …and page 1 was left alone.
        assert_eq!(read(&mut device, PAGE_SIZE as u32, 4), vec![0xFF; 4]);
    }

    #[test]
    fn a_program_larger_than_a_page_is_refused() {
        let mut device = small_device();
        write_enable(&mut device);
        match program(&mut device, 0, &vec![0x00; PAGE_SIZE + 1]) {
            Err(TransportError::Device {
                status: Status::InvalidArgument,
                ..
            }) => {}
            other => panic!("expected an invalid-argument error, got {other:?}"),
        }
    }

    #[test]
    fn sector_erase_restores_only_its_own_sector() {
        let mut device = small_device();

        // Dirty two adjacent sectors.
        for sector in 0..2u32 {
            write_enable(&mut device);
            program(&mut device, sector * SECTOR_SIZE as u32, &[0x00; 4]).unwrap();
        }

        write_enable(&mut device);
        erase_sector(&mut device, 0).unwrap();

        assert_eq!(read(&mut device, 0, 4), vec![0xFF; 4]);
        assert_eq!(
            read(&mut device, SECTOR_SIZE as u32, 4),
            vec![0x00; 4],
            "the neighbouring sector must be untouched"
        );
    }

    #[test]
    fn an_unaligned_erase_is_refused() {
        let mut device = small_device();
        write_enable(&mut device);
        match erase_sector(&mut device, 1) {
            Err(TransportError::Device {
                status: Status::InvalidArgument,
                ..
            }) => {}
            other => panic!("expected an invalid-argument error, got {other:?}"),
        }
    }

    #[test]
    fn chip_erase_clears_everything() {
        let mut device = small_device();
        write_enable(&mut device);
        program(&mut device, 0, &[0x00; 16]).unwrap();
        write_enable(&mut device);
        program(&mut device, 8000, &[0x00; 16]).unwrap();

        write_enable(&mut device);
        device
            .transact(Command::SpiNorChipErase, &[], DEFAULT_TIMEOUT)
            .unwrap();

        assert!(device.image().iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn a_write_protected_device_refuses_every_change() {
        let mut device = small_device();
        device.set_write_protected(true);

        write_enable(&mut device);
        assert!(program(&mut device, 0, &[0x00]).is_err());
        write_enable(&mut device);
        assert!(erase_sector(&mut device, 0).is_err());
        write_enable(&mut device);
        assert!(device
            .transact(Command::SpiNorChipErase, &[], DEFAULT_TIMEOUT)
            .is_err());

        assert!(device.image().iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn reads_past_the_end_of_the_chip_are_refused() {
        let mut device = small_device();
        let size = device.size() as u32;
        let mut payload = (size - 2).to_le_bytes().to_vec();
        payload.extend_from_slice(&16u16.to_le_bytes());

        match device.transact(Command::SpiNorRead, &payload, DEFAULT_TIMEOUT) {
            Err(TransportError::Device {
                status: Status::InvalidArgument,
                ..
            }) => {}
            other => panic!("expected an invalid-argument error, got {other:?}"),
        }
    }

    #[test]
    fn the_status_register_reflects_the_latch() {
        let mut device = small_device();
        let status = device
            .transact(Command::SpiNorReadStatus1, &[], DEFAULT_TIMEOUT)
            .unwrap();
        assert_eq!(status, vec![0x00]);

        write_enable(&mut device);
        let status = device
            .transact(Command::SpiNorReadStatus1, &[], DEFAULT_TIMEOUT)
            .unwrap();
        assert_eq!(status, vec![STATUS_WEL]);
    }

    #[test]
    fn commands_for_other_interfaces_are_refused_rather_than_faked() {
        let mut device = small_device();
        for command in [
            Command::NandReadPage,
            Command::SpiNandReadId,
            Command::EmmcReadBlock,
            Command::UfsRead10,
        ] {
            match device.transact(command, &[0; 8], DEFAULT_TIMEOUT) {
                Err(TransportError::Device {
                    status: Status::UnsupportedOnInterface,
                    ..
                }) => {}
                other => panic!("{command:?} should be unsupported, got {other:?}"),
            }
        }
    }

    #[test]
    fn an_unknown_opcode_gets_an_unsupported_status_not_silence() {
        let mut device = small_device();
        // 0xB5 is in the reserved host-side range.
        let request = Frame::response_raw(0xB5, Status::Ok, &[])
            .encode_to_vec()
            .unwrap();
        let response = device.handle_frame(&request);
        let frame = Frame::decode(&response).unwrap();
        assert_eq!(frame.command, 0xB5);
        assert_eq!(frame.decoded_status(), Some(Status::UnsupportedCommand));
    }

    #[test]
    fn a_corrupt_request_is_answered_rather_than_dropped() {
        let mut device = small_device();
        let mut request = Frame::request(Command::Ping, &[]).encode_to_vec().unwrap();
        request[3] ^= 0xFF; // break the header CRC

        let response = device.handle_frame(&request);
        let frame = Frame::decode(&response).unwrap();
        assert_eq!(frame.decoded_status(), Some(Status::Error));
    }

    #[test]
    fn preloaded_images_are_readable_and_reported_by_size() {
        let image: Vec<u8> = (0..SECTOR_SIZE).map(|i| (i % 251) as u8).collect();
        let mut device = EmulatedDevice::with_image("PRELOAD", [0xC2, 0x20, 0x14], image.clone());

        assert_eq!(device.size(), SECTOR_SIZE);
        assert_eq!(read(&mut device, 0, 256), image[..256].to_vec());
    }

    #[test]
    fn stats_count_the_work_that_was_actually_done() {
        let mut device = small_device();
        read(&mut device, 0, 100);
        write_enable(&mut device);
        program(&mut device, 0, &[0x00; 10]).unwrap();
        write_enable(&mut device);
        erase_sector(&mut device, 0).unwrap();

        let stats = device.stats();
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.bytes_read, 100);
        assert_eq!(stats.programs, 1);
        assert_eq!(stats.bytes_programmed, 10);
        assert_eq!(stats.erases, 1);
    }
}
