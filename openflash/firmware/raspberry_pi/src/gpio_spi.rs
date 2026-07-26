//! SPI NOR access on a Raspberry Pi, over the hardware SPI controller.
//!
//! The Pi drives the chip directly through `/dev/spidev*`, so this is where the
//! agent's SPI NOR commands actually reach silicon.

use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use thiserror::Error;

/// Bytes in a program page: a program operation may not cross this boundary.
pub const PAGE_SIZE: usize = 256;

/// Standard SPI NOR opcodes, as driven onto the bus.
mod opcode {
    pub const READ_JEDEC_ID: u8 = 0x9F;
    pub const READ: u8 = 0x03;
    pub const PAGE_PROGRAM: u8 = 0x02;
    pub const SECTOR_ERASE_4K: u8 = 0x20;
    pub const BLOCK_ERASE_32K: u8 = 0x52;
    pub const BLOCK_ERASE_64K: u8 = 0xD8;
    pub const CHIP_ERASE: u8 = 0xC7;
    pub const READ_STATUS_1: u8 = 0x05;
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const WRITE_DISABLE: u8 = 0x04;
}

/// Status register 1, bit 0: a program or erase is in progress.
const STATUS_BUSY: u8 = 0x01;

/// SPI bus settings.
pub struct SpiConfig {
    pub bus: Bus,
    pub slave_select: SlaveSelect,
    pub clock_speed: u32,
    pub mode: Mode,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            bus: Bus::Spi0,
            slave_select: SlaveSelect::Ss0,
            clock_speed: 10_000_000,
            mode: Mode::Mode0,
        }
    }
}

#[derive(Error, Debug)]
pub enum SpiError {
    #[error("SPI error: {0}")]
    Spi(#[from] rppal::spi::Error),

    #[error("SPI is not initialised; call init() first")]
    NotInitialized,

    #[error("chip stayed busy for longer than {0} ms")]
    Timeout(u64),

    #[error("a program of {length} bytes at {address:#x} would cross a {PAGE_SIZE}-byte page")]
    PageOverrun { address: u32, length: usize },

    #[error("address {address:#x} is not aligned to a {granularity}-byte erase unit")]
    Unaligned { address: u32, granularity: usize },
}

/// The Pi's SPI controller, wired to a flash chip.
pub struct GpioSpi {
    spi: Option<Spi>,
    config: SpiConfig,
}

impl GpioSpi {
    pub fn new(config: SpiConfig) -> Self {
        Self { spi: None, config }
    }

    /// Open the SPI device.
    pub fn init(&mut self) -> Result<(), SpiError> {
        self.spi = Some(Spi::new(
            self.config.bus,
            self.config.slave_select,
            self.config.clock_speed,
            self.config.mode,
        )?);
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.spi.is_some()
    }

    fn spi(&self) -> Result<&Spi, SpiError> {
        self.spi.as_ref().ok_or(SpiError::NotInitialized)
    }

    fn spi_mut(&mut self) -> Result<&mut Spi, SpiError> {
        self.spi.as_mut().ok_or(SpiError::NotInitialized)
    }

    /// Full-duplex transfer: shift `write` out while shifting `read` in.
    fn transfer(&self, read: &mut [u8], write: &[u8]) -> Result<(), SpiError> {
        self.spi()?.transfer(read, write)?;
        Ok(())
    }

    /// Send bytes, ignoring what comes back.
    fn write_only(&mut self, data: &[u8]) -> Result<(), SpiError> {
        self.spi_mut()?.write(data)?;
        Ok(())
    }

    /// Read the three-byte JEDEC id.
    pub fn read_jedec_id(&self) -> Result<[u8; 3], SpiError> {
        let mut read = [0u8; 4];
        self.transfer(&mut read, &[opcode::READ_JEDEC_ID, 0, 0, 0])?;
        Ok([read[1], read[2], read[3]])
    }

    /// Read status register 1.
    pub fn read_status(&self) -> Result<u8, SpiError> {
        let mut read = [0u8; 2];
        self.transfer(&mut read, &[opcode::READ_STATUS_1, 0])?;
        Ok(read[1])
    }

    pub fn write_enable(&mut self) -> Result<(), SpiError> {
        self.write_only(&[opcode::WRITE_ENABLE])
    }

    pub fn write_disable(&mut self) -> Result<(), SpiError> {
        self.write_only(&[opcode::WRITE_DISABLE])
    }

    /// Read `output.len()` bytes starting at `address`.
    pub fn read(&self, address: u32, output: &mut [u8]) -> Result<(), SpiError> {
        // One transfer: four command bytes, then the data clocked in.
        let mut write = vec![0u8; 4 + output.len()];
        write[0] = opcode::READ;
        write[1..4].copy_from_slice(&address.to_be_bytes()[1..]);

        let mut read = vec![0u8; write.len()];
        self.transfer(&mut read, &write)?;
        output.copy_from_slice(&read[4..]);
        Ok(())
    }

    /// Program up to one page, without crossing the page boundary.
    ///
    /// A chip that is asked to program across a page boundary wraps to the start
    /// of the same page and silently corrupts data, so the overrun is rejected
    /// here rather than passed on.
    pub fn page_program(&mut self, address: u32, data: &[u8]) -> Result<(), SpiError> {
        let offset = address as usize % PAGE_SIZE;
        if data.len() > PAGE_SIZE - offset {
            return Err(SpiError::PageOverrun {
                address,
                length: data.len(),
            });
        }

        self.write_enable()?;

        let mut command = Vec::with_capacity(4 + data.len());
        command.push(opcode::PAGE_PROGRAM);
        command.extend_from_slice(&address.to_be_bytes()[1..]);
        command.extend_from_slice(data);
        self.write_only(&command)?;

        self.wait_busy(5_000)
    }

    /// Erase the unit of `granularity` bytes containing `address`.
    pub fn erase(&mut self, address: u32, granularity: usize) -> Result<(), SpiError> {
        let opcode = match granularity {
            4096 => opcode::SECTOR_ERASE_4K,
            32768 => opcode::BLOCK_ERASE_32K,
            65536 => opcode::BLOCK_ERASE_64K,
            other => {
                return Err(SpiError::Unaligned {
                    address,
                    granularity: other,
                })
            }
        };
        if address as usize % granularity != 0 {
            return Err(SpiError::Unaligned {
                address,
                granularity,
            });
        }

        self.write_enable()?;
        let mut command = [opcode, 0, 0, 0];
        command[1..4].copy_from_slice(&address.to_be_bytes()[1..]);
        self.write_only(&command)?;

        // A 64 KiB block erase takes a couple of seconds on a slow part.
        self.wait_busy(10_000)
    }

    /// Erase the whole chip.
    pub fn chip_erase(&mut self) -> Result<(), SpiError> {
        self.write_enable()?;
        self.write_only(&[opcode::CHIP_ERASE])?;
        // Chip erase is the slowest operation a NOR part performs.
        self.wait_busy(300_000)
    }

    /// Poll the busy bit until the chip finishes, or the timeout expires.
    pub fn wait_busy(&self, timeout_ms: u64) -> Result<(), SpiError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            if self.read_status()? & STATUS_BUSY == 0 {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
        Err(SpiError::Timeout(timeout_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bus cannot be opened in CI, so the checks that run here are the ones
    /// that happen before any transfer: they are also the ones that protect the
    /// chip from a malformed request.
    #[test]
    fn operations_without_init_are_refused() {
        let spi = GpioSpi::new(SpiConfig::default());
        assert!(!spi.is_initialized());
        assert!(matches!(spi.read_jedec_id(), Err(SpiError::NotInitialized)));
        assert!(matches!(spi.read_status(), Err(SpiError::NotInitialized)));
    }

    #[test]
    fn a_program_crossing_a_page_boundary_is_refused_before_touching_the_bus() {
        let mut spi = GpioSpi::new(SpiConfig::default());
        // Ten bytes from four before the page end would wrap on real hardware.
        match spi.page_program((PAGE_SIZE - 4) as u32, &[0u8; 10]) {
            Err(SpiError::PageOverrun { address, length }) => {
                assert_eq!(address, (PAGE_SIZE - 4) as u32);
                assert_eq!(length, 10);
            }
            other => panic!("expected a page overrun, got {other:?}"),
        }
    }

    #[test]
    fn a_program_that_fits_the_page_gets_as_far_as_the_bus() {
        let mut spi = GpioSpi::new(SpiConfig::default());
        // Reaches the uninitialised-bus error, which means the page check passed.
        assert!(matches!(
            spi.page_program(0, &[0u8; PAGE_SIZE]),
            Err(SpiError::NotInitialized)
        ));
    }

    #[test]
    fn an_unaligned_erase_is_refused() {
        let mut spi = GpioSpi::new(SpiConfig::default());
        match spi.erase(100, 4096) {
            Err(SpiError::Unaligned {
                address,
                granularity,
            }) => {
                assert_eq!(address, 100);
                assert_eq!(granularity, 4096);
            }
            other => panic!("expected an alignment error, got {other:?}"),
        }
    }

    #[test]
    fn an_unsupported_erase_granularity_is_refused() {
        let mut spi = GpioSpi::new(SpiConfig::default());
        assert!(matches!(
            spi.erase(0, 1024),
            Err(SpiError::Unaligned { .. })
        ));
    }

    #[test]
    fn aligned_erases_pass_the_alignment_check() {
        let mut spi = GpioSpi::new(SpiConfig::default());
        for (address, granularity) in [(0, 4096), (4096, 4096), (32768, 32768), (65536, 65536)] {
            assert!(
                matches!(
                    spi.erase(address, granularity),
                    Err(SpiError::NotInitialized)
                ),
                "{address:#x}/{granularity} should have passed the alignment check"
            );
        }
    }
}
