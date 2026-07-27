//! The SPI bus a board provides.
//!
//! This is the only thing a board-specific agent has to implement. Everything
//! above it — opcodes, address byte order, page boundaries, erase alignment,
//! busy polling — lives in [`crate::nor`] so it exists once and is tested once,
//! instead of being written again per board.

use std::fmt;

/// Something went wrong on the bus.
#[derive(Debug)]
pub enum BusError {
    /// The SPI device could not be opened, or is not open yet.
    Unavailable(String),
    /// A transfer failed.
    Transfer(String),
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(why) => write!(f, "SPI bus unavailable: {why}"),
            Self::Transfer(why) => write!(f, "SPI transfer failed: {why}"),
        }
    }
}

impl std::error::Error for BusError {}

/// Result of a bus operation.
pub type BusResult<T> = Result<T, BusError>;

/// A SPI bus with the chip select wired to one flash chip.
///
/// Both methods must keep chip select asserted for the whole call and release it
/// afterwards — one call is one SPI transaction. A flash chip requires the
/// command, the address and the data to arrive without chip select going high in
/// between, so an implementation that issues a `write` syscall followed by a
/// separate `read` syscall on `/dev/spidev*` does *not* satisfy this: the kernel
/// deasserts chip select between the two, and the chip discards the command.
/// That is what the Orange Pi and Banana Pi agents used to do.
pub trait SpiBus {
    /// Clock `write` out while clocking the same number of bytes into `read`.
    ///
    /// `read` and `write` are the same length; byte *n* of `read` is what the
    /// chip returned while byte *n* of `write` went out, so a response to a
    /// command begins after the command and address bytes.
    fn transfer(&mut self, write: &[u8], read: &mut [u8]) -> BusResult<()>;

    /// Clock `data` out, discarding whatever comes back.
    fn write(&mut self, data: &[u8]) -> BusResult<()>;

    /// Whether the bus is usable.
    ///
    /// An agent that cannot open its SPI device still starts and still answers,
    /// so a host is told "no interfaces" rather than having every operation fail
    /// one at a time.
    fn is_available(&self) -> bool;
}

#[cfg(test)]
pub(crate) mod fake {
    //! A SPI NOR chip that answers on the bus.
    //!
    //! Tests drive the real command sequences through this, so a wrong opcode,
    //! a reversed address, or a missing write-enable shows up as wrong data
    //! rather than passing unnoticed. Semantics match hardware: programming only
    //! clears bits, a program wrapping the page boundary wraps within the page,
    //! and program and erase need the write-enable latch, which clears after.

    use super::{BusError, BusResult, SpiBus};

    const READ_JEDEC_ID: u8 = 0x9F;
    const READ: u8 = 0x03;
    const PAGE_PROGRAM: u8 = 0x02;
    const SECTOR_ERASE_4K: u8 = 0x20;
    const BLOCK_ERASE_32K: u8 = 0x52;
    const BLOCK_ERASE_64K: u8 = 0xD8;
    const CHIP_ERASE: u8 = 0xC7;
    const READ_STATUS_1: u8 = 0x05;
    const WRITE_ENABLE: u8 = 0x06;
    const WRITE_DISABLE: u8 = 0x04;

    const PAGE: usize = 256;

    pub struct FakeChip {
        pub image: Vec<u8>,
        pub jedec_id: [u8; 3],
        pub write_enabled: bool,
        pub available: bool,
        /// Commands the host issued, in order, for tests that check sequencing.
        pub log: Vec<u8>,
    }

    impl FakeChip {
        pub fn new(size: usize) -> Self {
            Self {
                image: vec![0xFF; size],
                jedec_id: [0xEF, 0x40, size.trailing_zeros() as u8],
                write_enabled: false,
                available: true,
                log: Vec::new(),
            }
        }

        fn address(bytes: &[u8]) -> usize {
            // Three address bytes, most significant first, as SPI NOR expects.
            ((bytes[0] as usize) << 16) | ((bytes[1] as usize) << 8) | bytes[2] as usize
        }

        fn erase(&mut self, address: usize, granularity: usize) {
            if !self.write_enabled || address % granularity != 0 {
                return;
            }
            let end = (address + granularity).min(self.image.len());
            if address < self.image.len() {
                self.image[address..end].fill(0xFF);
            }
            self.write_enabled = false;
        }

        fn program(&mut self, address: usize, data: &[u8]) {
            if !self.write_enabled {
                return;
            }
            let base = address & !(PAGE - 1);
            for (index, &byte) in data.iter().enumerate() {
                let offset = base + ((address - base + index) % PAGE);
                if offset < self.image.len() {
                    // Programming clears bits; it cannot set them.
                    self.image[offset] &= byte;
                }
            }
            self.write_enabled = false;
        }
    }

    impl SpiBus for FakeChip {
        fn transfer(&mut self, write: &[u8], read: &mut [u8]) -> BusResult<()> {
            if !self.available {
                return Err(BusError::Unavailable("fake chip is offline".into()));
            }
            assert_eq!(write.len(), read.len(), "a transfer is symmetric");
            read.fill(0xFF);

            let Some(&opcode) = write.first() else {
                return Ok(());
            };
            self.log.push(opcode);

            match opcode {
                READ_JEDEC_ID => {
                    for (index, slot) in read.iter_mut().enumerate().skip(1).take(3) {
                        *slot = self.jedec_id[index - 1];
                    }
                }
                READ_STATUS_1 => {
                    // Never busy: this chip completes synchronously.
                    let status = if self.write_enabled { 0x02 } else { 0x00 };
                    if read.len() > 1 {
                        read[1] = status;
                    }
                }
                READ if write.len() >= 4 => {
                    let address = Self::address(&write[1..4]);
                    for (index, slot) in read.iter_mut().enumerate().skip(4) {
                        let offset = address + index - 4;
                        *slot = self.image.get(offset).copied().unwrap_or(0xFF);
                    }
                }
                _ => {}
            }
            Ok(())
        }

        fn write(&mut self, data: &[u8]) -> BusResult<()> {
            if !self.available {
                return Err(BusError::Unavailable("fake chip is offline".into()));
            }
            let Some(&opcode) = data.first() else {
                return Ok(());
            };
            self.log.push(opcode);

            match opcode {
                WRITE_ENABLE => self.write_enabled = true,
                WRITE_DISABLE => self.write_enabled = false,
                PAGE_PROGRAM if data.len() >= 4 => {
                    let address = Self::address(&data[1..4]);
                    self.program(address, &data[4..]);
                }
                SECTOR_ERASE_4K if data.len() >= 4 => {
                    let address = Self::address(&data[1..4]);
                    self.erase(address, 4096);
                }
                BLOCK_ERASE_32K if data.len() >= 4 => {
                    let address = Self::address(&data[1..4]);
                    self.erase(address, 32 * 1024);
                }
                BLOCK_ERASE_64K if data.len() >= 4 => {
                    let address = Self::address(&data[1..4]);
                    self.erase(address, 64 * 1024);
                }
                CHIP_ERASE => {
                    if self.write_enabled {
                        self.image.fill(0xFF);
                        self.write_enabled = false;
                    }
                }
                _ => {}
            }
            Ok(())
        }

        fn is_available(&self) -> bool {
            self.available
        }
    }
}
