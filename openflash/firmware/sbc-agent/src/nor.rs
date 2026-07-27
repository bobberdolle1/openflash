//! SPI NOR command sequencing, shared by every board.
//!
//! Written once, over the [`SpiBus`] abstraction, and tested against a chip that
//! answers on the bus — so a wrong opcode, a reversed address or a missing
//! write-enable shows up as wrong data in a test rather than as a corrupted chip
//! on someone's desk.

use std::time::{Duration, Instant};

use crate::bus::{BusError, SpiBus};

/// Bytes in a program page. A program operation may not cross this boundary.
pub const PAGE_SIZE: usize = 256;

/// Standard SPI NOR opcodes, as driven onto the bus.
pub mod opcode {
    /// Read the three-byte JEDEC id.
    pub const READ_JEDEC_ID: u8 = 0x9F;
    /// Read at the standard clock rate.
    pub const READ: u8 = 0x03;
    /// Program up to one page.
    pub const PAGE_PROGRAM: u8 = 0x02;
    /// Erase a 4 KiB sector.
    pub const SECTOR_ERASE_4K: u8 = 0x20;
    /// Erase a 32 KiB block.
    pub const BLOCK_ERASE_32K: u8 = 0x52;
    /// Erase a 64 KiB block.
    pub const BLOCK_ERASE_64K: u8 = 0xD8;
    /// Erase the whole chip.
    pub const CHIP_ERASE: u8 = 0xC7;
    /// Read status register 1.
    pub const READ_STATUS_1: u8 = 0x05;
    /// Set the write-enable latch.
    pub const WRITE_ENABLE: u8 = 0x06;
    /// Clear the write-enable latch.
    pub const WRITE_DISABLE: u8 = 0x04;
}

/// Status register 1, bit 0: a program or erase is in progress.
pub const STATUS_BUSY: u8 = 0x01;

/// Status register 1, bit 1: the write-enable latch.
pub const STATUS_WEL: u8 = 0x02;

/// Why a SPI NOR operation was refused or failed.
#[derive(Debug)]
pub enum NorError {
    /// The bus itself failed.
    Bus(BusError),
    /// A program would have crossed a page boundary.
    ///
    /// A chip asked to do this wraps to the start of the same page and corrupts
    /// whatever is there, so the request is refused before it reaches the bus.
    PageOverrun {
        /// Requested start address.
        address: u32,
        /// Requested length.
        length: usize,
    },
    /// An erase address is not aligned to the erase unit.
    Unaligned {
        /// Requested address.
        address: u32,
        /// Erase unit size.
        granularity: usize,
    },
    /// No erase opcode exists for the requested size.
    UnsupportedGranularity {
        /// Requested erase unit size.
        granularity: usize,
    },
    /// The chip stayed busy for longer than allowed.
    Timeout {
        /// How long the agent waited, in milliseconds.
        waited_ms: u64,
    },
}

impl std::fmt::Display for NorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bus(error) => write!(f, "{error}"),
            Self::PageOverrun { address, length } => write!(
                f,
                "a program of {length} bytes at {address:#x} would cross a \
                 {PAGE_SIZE}-byte page boundary"
            ),
            Self::Unaligned {
                address,
                granularity,
            } => write!(
                f,
                "address {address:#x} is not aligned to a {granularity}-byte erase unit"
            ),
            Self::UnsupportedGranularity { granularity } => {
                write!(f, "no erase opcode for a {granularity}-byte unit")
            }
            Self::Timeout { waited_ms } => {
                write!(f, "chip stayed busy for longer than {waited_ms} ms")
            }
        }
    }
}

impl std::error::Error for NorError {}

impl From<BusError> for NorError {
    fn from(error: BusError) -> Self {
        Self::Bus(error)
    }
}

/// Result of a SPI NOR operation.
pub type NorResult<T> = Result<T, NorError>;

/// How long to wait for each class of operation.
///
/// Times are the worst case a slow part specifies, not the typical case: waiting
/// too briefly reports a timeout on a chip that was about to finish.
mod timeout {
    /// A page program.
    pub const PROGRAM_MS: u64 = 5_000;
    /// A sector or block erase.
    pub const ERASE_MS: u64 = 10_000;
    /// A full chip erase, the slowest thing a NOR part does.
    pub const CHIP_ERASE_MS: u64 = 300_000;
}

/// SPI NOR operations over a bus.
pub struct SpiNor<B: SpiBus> {
    bus: B,
    /// Time to sleep between busy-bit polls.
    poll_interval: Duration,
}

impl<B: SpiBus> SpiNor<B> {
    /// Wrap a bus.
    pub fn new(bus: B) -> Self {
        Self {
            bus,
            poll_interval: Duration::from_micros(200),
        }
    }

    /// Poll the busy bit this often. Tests set it to zero.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Whether the underlying bus is usable.
    pub fn is_available(&self) -> bool {
        self.bus.is_available()
    }

    /// Borrow the bus, for board code that needs it.
    pub fn bus(&self) -> &B {
        &self.bus
    }

    /// Three address bytes, most significant first, as SPI NOR expects.
    fn address_bytes(address: u32) -> [u8; 3] {
        [(address >> 16) as u8, (address >> 8) as u8, address as u8]
    }

    /// Read the three-byte JEDEC id.
    pub fn read_jedec_id(&mut self) -> NorResult<[u8; 3]> {
        let write = [opcode::READ_JEDEC_ID, 0, 0, 0];
        let mut read = [0u8; 4];
        self.bus.transfer(&write, &mut read)?;
        Ok([read[1], read[2], read[3]])
    }

    /// Read status register 1.
    pub fn read_status(&mut self) -> NorResult<u8> {
        let write = [opcode::READ_STATUS_1, 0];
        let mut read = [0u8; 2];
        self.bus.transfer(&write, &mut read)?;
        Ok(read[1])
    }

    /// Set the write-enable latch.
    pub fn write_enable(&mut self) -> NorResult<()> {
        self.bus.write(&[opcode::WRITE_ENABLE])?;
        Ok(())
    }

    /// Clear the write-enable latch.
    pub fn write_disable(&mut self) -> NorResult<()> {
        self.bus.write(&[opcode::WRITE_DISABLE])?;
        Ok(())
    }

    /// Read `output.len()` bytes starting at `address`.
    pub fn read(&mut self, address: u32, output: &mut [u8]) -> NorResult<()> {
        // One transaction: opcode, three address bytes, then the data clocked in.
        let mut write = vec![0u8; 4 + output.len()];
        write[0] = opcode::READ;
        write[1..4].copy_from_slice(&Self::address_bytes(address));

        let mut read = vec![0u8; write.len()];
        self.bus.transfer(&write, &mut read)?;
        output.copy_from_slice(&read[4..]);
        Ok(())
    }

    /// Program up to one page, without crossing the page boundary.
    pub fn page_program(&mut self, address: u32, data: &[u8]) -> NorResult<()> {
        let offset = address as usize % PAGE_SIZE;
        if data.len() > PAGE_SIZE - offset {
            return Err(NorError::PageOverrun {
                address,
                length: data.len(),
            });
        }
        if data.is_empty() {
            return Ok(());
        }

        self.write_enable()?;

        let mut command = Vec::with_capacity(4 + data.len());
        command.push(opcode::PAGE_PROGRAM);
        command.extend_from_slice(&Self::address_bytes(address));
        command.extend_from_slice(data);
        self.bus.write(&command)?;

        self.wait_ready(timeout::PROGRAM_MS)
    }

    /// Erase the unit of `granularity` bytes containing `address`.
    pub fn erase(&mut self, address: u32, granularity: usize) -> NorResult<()> {
        let opcode = match granularity {
            4096 => opcode::SECTOR_ERASE_4K,
            32_768 => opcode::BLOCK_ERASE_32K,
            65_536 => opcode::BLOCK_ERASE_64K,
            other => return Err(NorError::UnsupportedGranularity { granularity: other }),
        };
        if address as usize % granularity != 0 {
            return Err(NorError::Unaligned {
                address,
                granularity,
            });
        }

        self.write_enable()?;

        let mut command = [opcode, 0, 0, 0];
        command[1..4].copy_from_slice(&Self::address_bytes(address));
        self.bus.write(&command)?;

        self.wait_ready(timeout::ERASE_MS)
    }

    /// Erase the whole chip.
    pub fn chip_erase(&mut self) -> NorResult<()> {
        self.write_enable()?;
        self.bus.write(&[opcode::CHIP_ERASE])?;
        self.wait_ready(timeout::CHIP_ERASE_MS)
    }

    /// Poll the busy bit until the chip finishes, or the timeout expires.
    pub fn wait_ready(&mut self, timeout_ms: u64) -> NorResult<()> {
        let deadline = Duration::from_millis(timeout_ms);
        let started = Instant::now();

        loop {
            if self.read_status()? & STATUS_BUSY == 0 {
                return Ok(());
            }
            if started.elapsed() >= deadline {
                return Err(NorError::Timeout {
                    waited_ms: timeout_ms,
                });
            }
            if !self.poll_interval.is_zero() {
                std::thread::sleep(self.poll_interval);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::fake::FakeChip;

    fn chip(size: usize) -> SpiNor<FakeChip> {
        SpiNor::new(FakeChip::new(size)).with_poll_interval(Duration::ZERO)
    }

    #[test]
    fn the_jedec_id_comes_back_in_the_right_order() {
        let mut nor = chip(64 * 1024);
        // 64 KiB is 2^16, so the capacity byte is 0x10.
        assert_eq!(nor.read_jedec_id().unwrap(), [0xEF, 0x40, 0x10]);
    }

    #[test]
    fn a_fresh_chip_reads_as_erased() {
        let mut nor = chip(64 * 1024);
        let mut data = [0u8; 32];
        nor.read(0, &mut data).unwrap();
        assert_eq!(data, [0xFF; 32]);
    }

    /// Proves the address goes out most significant byte first.
    ///
    /// 0x102030 and its byte-reversed form 0x302010 both fall inside a 4 MiB
    /// chip, so if the order were wrong the data would land at the other address
    /// and both assertions would catch it.
    #[test]
    fn the_address_is_sent_most_significant_byte_first() {
        let mut nor = chip(4 * 1024 * 1024);
        let payload = [0x11u8, 0x22, 0x33, 0x44];

        nor.page_program(0x10_2030, &payload).unwrap();

        let mut read_back = [0u8; 4];
        nor.read(0x10_2030, &mut read_back).unwrap();
        assert_eq!(read_back, payload, "the data is not where it was addressed");

        let mut reversed = [0u8; 4];
        nor.read(0x30_2010, &mut reversed).unwrap();
        assert_eq!(
            reversed, [0xFF; 4],
            "the data landed at the byte-reversed address"
        );
    }

    /// A program must be preceded by a write-enable in the same sequence; a chip
    /// ignores the program otherwise. The fake chip enforces that, so this test
    /// fails if the sequencing is dropped.
    #[test]
    fn a_program_is_preceded_by_write_enable() {
        let mut nor = chip(64 * 1024);
        nor.page_program(0, &[0x00; 4]).unwrap();

        let log = &nor.bus().log;
        let program = log
            .iter()
            .position(|&op| op == opcode::PAGE_PROGRAM)
            .expect("a program was issued");
        let enable = log
            .iter()
            .position(|&op| op == opcode::WRITE_ENABLE)
            .expect("a write-enable was issued");
        assert!(enable < program, "write-enable must come first: {log:02X?}");

        let mut read_back = [0xFFu8; 4];
        nor.read(0, &mut read_back).unwrap();
        assert_eq!(read_back, [0x00; 4]);
    }

    #[test]
    fn programming_only_clears_bits() {
        let mut nor = chip(64 * 1024);
        nor.page_program(0, &[0b1111_0000]).unwrap();
        nor.page_program(0, &[0b0000_1111]).unwrap();

        let mut read_back = [0u8; 1];
        nor.read(0, &mut read_back).unwrap();
        assert_eq!(read_back[0], 0b0000_0000);
    }

    #[test]
    fn a_program_crossing_a_page_boundary_is_refused_before_the_bus() {
        let mut nor = chip(64 * 1024);
        match nor.page_program((PAGE_SIZE - 4) as u32, &[0u8; 10]) {
            Err(NorError::PageOverrun { address, length }) => {
                assert_eq!(address, (PAGE_SIZE - 4) as u32);
                assert_eq!(length, 10);
            }
            other => panic!("expected a page overrun, got {other:?}"),
        }
        // Nothing was sent.
        assert!(nor.bus().log.is_empty());
    }

    #[test]
    fn a_program_that_exactly_fills_a_page_is_allowed() {
        let mut nor = chip(64 * 1024);
        let payload: Vec<u8> = (0..PAGE_SIZE).map(|i| !(i as u8)).collect();
        nor.page_program(0, &payload).unwrap();

        let mut read_back = vec![0u8; PAGE_SIZE];
        nor.read(0, &mut read_back).unwrap();
        assert_eq!(read_back, payload);
    }

    #[test]
    fn erase_restores_a_sector_and_leaves_its_neighbour_alone() {
        let mut nor = chip(64 * 1024);
        nor.page_program(0, &[0x00; 4]).unwrap();
        nor.page_program(4096, &[0x00; 4]).unwrap();

        nor.erase(0, 4096).unwrap();

        let mut first = [0u8; 4];
        nor.read(0, &mut first).unwrap();
        assert_eq!(first, [0xFF; 4]);

        let mut second = [0u8; 4];
        nor.read(4096, &mut second).unwrap();
        assert_eq!(second, [0x00; 4], "the next sector must be untouched");
    }

    #[test]
    fn each_erase_size_uses_its_own_opcode() {
        for (granularity, expected) in [
            (4096usize, opcode::SECTOR_ERASE_4K),
            (32_768, opcode::BLOCK_ERASE_32K),
            (65_536, opcode::BLOCK_ERASE_64K),
        ] {
            let mut nor = chip(128 * 1024);
            nor.erase(0, granularity).unwrap();
            assert!(
                nor.bus().log.contains(&expected),
                "a {granularity}-byte erase should use {expected:#04X}: {:02X?}",
                nor.bus().log
            );
        }
    }

    #[test]
    fn an_unaligned_erase_is_refused() {
        let mut nor = chip(64 * 1024);
        match nor.erase(100, 4096) {
            Err(NorError::Unaligned {
                address,
                granularity,
            }) => {
                assert_eq!(address, 100);
                assert_eq!(granularity, 4096);
            }
            other => panic!("expected an alignment error, got {other:?}"),
        }
        assert!(nor.bus().log.is_empty(), "nothing should have been sent");
    }

    #[test]
    fn an_unsupported_erase_size_is_refused() {
        let mut nor = chip(64 * 1024);
        assert!(matches!(
            nor.erase(0, 1024),
            Err(NorError::UnsupportedGranularity { granularity: 1024 })
        ));
    }

    #[test]
    fn chip_erase_blanks_everything() {
        let mut nor = chip(64 * 1024);
        nor.page_program(0, &[0x00; 16]).unwrap();
        nor.page_program(32_768, &[0x00; 16]).unwrap();

        nor.chip_erase().unwrap();

        assert!(nor.bus().image.iter().all(|&byte| byte == 0xFF));
    }

    #[test]
    fn the_status_register_reflects_the_latch() {
        let mut nor = chip(64 * 1024);
        assert_eq!(nor.read_status().unwrap() & STATUS_WEL, 0);

        nor.write_enable().unwrap();
        assert_eq!(nor.read_status().unwrap() & STATUS_WEL, STATUS_WEL);

        nor.write_disable().unwrap();
        assert_eq!(nor.read_status().unwrap() & STATUS_WEL, 0);
    }

    #[test]
    fn a_read_spanning_several_pages_is_contiguous() {
        let mut nor = chip(64 * 1024);
        // Fill three pages with distinguishable content.
        for page in 0..3u32 {
            let payload: Vec<u8> = (0..PAGE_SIZE)
                .map(|i| (page as usize * 3 + i) as u8)
                .collect();
            nor.page_program(page * PAGE_SIZE as u32, &payload).unwrap();
        }

        let mut read_back = vec![0u8; 3 * PAGE_SIZE];
        nor.read(0, &mut read_back).unwrap();

        for page in 0..3usize {
            for index in 0..PAGE_SIZE {
                assert_eq!(
                    read_back[page * PAGE_SIZE + index],
                    (page * 3 + index) as u8,
                    "page {page} byte {index}"
                );
            }
        }
    }

    #[test]
    fn an_unavailable_bus_surfaces_as_an_error() {
        let mut fake = FakeChip::new(64 * 1024);
        fake.available = false;
        let mut nor = SpiNor::new(fake).with_poll_interval(Duration::ZERO);

        assert!(!nor.is_available());
        assert!(matches!(
            nor.read_jedec_id(),
            Err(NorError::Bus(BusError::Unavailable(_)))
        ));
    }
}
