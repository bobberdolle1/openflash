//! SPI NAND Flash driver for RP2040
//! Uses hardware SPI peripheral for high-speed communication

use embassy_rp::gpio::{Level, Output, AnyPin};
use embassy_rp::spi::{Spi, Config as SpiConfig, Phase, Polarity};
use embassy_rp::peripherals::SPI0;
use embassy_time::{Duration, Timer};

/// SPI NAND standard commands
pub mod commands {
    pub const RESET: u8 = 0xFF;
    pub const READ_ID: u8 = 0x9F;
    pub const GET_FEATURE: u8 = 0x0F;
    pub const SET_FEATURE: u8 = 0x1F;
    pub const PAGE_READ: u8 = 0x13;
    pub const READ_FROM_CACHE: u8 = 0x03;
    pub const READ_FROM_CACHE_X4: u8 = 0x6B;
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const WRITE_DISABLE: u8 = 0x04;
    pub const PROGRAM_LOAD: u8 = 0x02;
    pub const PROGRAM_LOAD_X4: u8 = 0x32;
    pub const PROGRAM_EXECUTE: u8 = 0x10;
    pub const BLOCK_ERASE: u8 = 0xD8;
}

/// Feature register addresses
pub mod features {
    pub const PROTECTION: u8 = 0xA0;
    pub const FEATURE: u8 = 0xB0;
    pub const STATUS: u8 = 0xC0;
}

/// Status register bits
pub mod status {
    pub const OIP: u8 = 0x01;      // Operation In Progress
    pub const WEL: u8 = 0x02;      // Write Enable Latch
    pub const E_FAIL: u8 = 0x04;   // Erase Fail
    pub const P_FAIL: u8 = 0x08;   // Program Fail
}

/// SPI NAND controller
pub struct SpiNandController<'d, SPI: embassy_rp::spi::Instance> {
    spi: Spi<'d, SPI, embassy_rp::spi::Blocking>,
    cs: Output<'d>,
    page_size: u32,
    oob_size: u32,
}

impl<'d, SPI: embassy_rp::spi::Instance> SpiNandController<'d, SPI> {
    /// Create a new SPI NAND controller
    pub fn new(
        spi: Spi<'d, SPI, embassy_rp::spi::Blocking>,
        cs: Output<'d>,
    ) -> Self {
        Self {
            spi,
            cs,
            page_size: 2048,
            oob_size: 64,
        }
    }

    /// Set page geometry after chip identification
    pub fn set_geometry(&mut self, page_size: u32, oob_size: u32) {
        self.page_size = page_size;
        self.oob_size = oob_size;
    }

    /// Assert chip select (active low)
    fn cs_low(&mut self) {
        self.cs.set_low();
    }

    /// Deassert chip select
    fn cs_high(&mut self) {
        self.cs.set_high();
    }

    /// Send command and receive response
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) {
        self.cs_low();
        if !tx.is_empty() {
            let _ = self.spi.blocking_write(tx);
        }
        if !rx.is_empty() {
            let _ = self.spi.blocking_read(rx);
        }
        self.cs_high();
    }

    /// Send command only
    fn write_cmd(&mut self, cmd: &[u8]) {
        self.cs_low();
        let _ = self.spi.blocking_write(cmd);
        self.cs_high();
    }


    /// Reset the SPI NAND chip
    pub async fn reset(&mut self) {
        self.write_cmd(&[commands::RESET]);
        // Wait for reset to complete (tRST typically 5-500us)
        Timer::after(Duration::from_micros(500)).await;
    }

    /// Read chip ID (manufacturer + device ID)
    pub fn read_id(&mut self) -> [u8; 3] {
        let mut id = [0u8; 3];
        // READ_ID command + dummy byte + 3 ID bytes
        self.cs_low();
        let _ = self.spi.blocking_write(&[commands::READ_ID, 0x00]);
        let _ = self.spi.blocking_read(&mut id);
        self.cs_high();
        id
    }

    /// Get feature register value
    pub fn get_feature(&mut self, addr: u8) -> u8 {
        let mut value = [0u8; 1];
        self.cs_low();
        let _ = self.spi.blocking_write(&[commands::GET_FEATURE, addr]);
        let _ = self.spi.blocking_read(&mut value);
        self.cs_high();
        value[0]
    }

    /// Set feature register value
    pub fn set_feature(&mut self, addr: u8, value: u8) {
        self.write_cmd(&[commands::SET_FEATURE, addr, value]);
    }

    /// Read status register
    pub fn read_status(&mut self) -> u8 {
        self.get_feature(features::STATUS)
    }

    /// Wait for operation to complete
    pub async fn wait_ready(&mut self) -> u8 {
        loop {
            let status = self.read_status();
            if (status & status::OIP) == 0 {
                return status;
            }
            Timer::after(Duration::from_micros(10)).await;
        }
    }

    /// Enable write operations
    pub fn write_enable(&mut self) {
        self.write_cmd(&[commands::WRITE_ENABLE]);
    }

    /// Disable write operations
    pub fn write_disable(&mut self) {
        self.write_cmd(&[commands::WRITE_DISABLE]);
    }

    /// Load page from array to cache (first step of read)
    pub async fn page_read_to_cache(&mut self, row_addr: u32) {
        let addr_bytes = [
            commands::PAGE_READ,
            0x00, // dummy
            ((row_addr >> 16) & 0xFF) as u8,
            ((row_addr >> 8) & 0xFF) as u8,
            (row_addr & 0xFF) as u8,
        ];
        self.write_cmd(&addr_bytes);
        self.wait_ready().await;
    }

    /// Read data from cache (second step of read)
    pub fn read_from_cache(&mut self, col_addr: u16, buf: &mut [u8]) {
        self.cs_low();
        let cmd = [
            commands::READ_FROM_CACHE,
            ((col_addr >> 8) & 0xFF) as u8,
            (col_addr & 0xFF) as u8,
            0x00, // dummy byte
        ];
        let _ = self.spi.blocking_write(&cmd);
        let _ = self.spi.blocking_read(buf);
        self.cs_high();
    }

    /// Read a full page (data + OOB)
    pub async fn read_page(&mut self, row_addr: u32, buf: &mut [u8]) {
        self.page_read_to_cache(row_addr).await;
        let read_len = buf.len().min((self.page_size + self.oob_size) as usize);
        self.read_from_cache(0, &mut buf[..read_len]);
    }

    /// Load data to cache for programming
    pub fn program_load(&mut self, col_addr: u16, data: &[u8]) {
        self.cs_low();
        let cmd = [
            commands::PROGRAM_LOAD,
            ((col_addr >> 8) & 0xFF) as u8,
            (col_addr & 0xFF) as u8,
        ];
        let _ = self.spi.blocking_write(&cmd);
        let _ = self.spi.blocking_write(data);
        self.cs_high();
    }

    /// Execute program operation (write cache to array)
    pub async fn program_execute(&mut self, row_addr: u32) -> bool {
        let addr_bytes = [
            commands::PROGRAM_EXECUTE,
            ((row_addr >> 16) & 0xFF) as u8,
            ((row_addr >> 8) & 0xFF) as u8,
            (row_addr & 0xFF) as u8,
        ];
        self.write_cmd(&addr_bytes);
        let status = self.wait_ready().await;
        (status & status::P_FAIL) == 0
    }

    /// Write a full page
    pub async fn write_page(&mut self, row_addr: u32, data: &[u8]) -> bool {
        self.write_enable();
        self.program_load(0, data);
        self.program_execute(row_addr).await
    }

    /// Erase a block
    pub async fn erase_block(&mut self, row_addr: u32) -> bool {
        self.write_enable();
        let addr_bytes = [
            commands::BLOCK_ERASE,
            ((row_addr >> 16) & 0xFF) as u8,
            ((row_addr >> 8) & 0xFF) as u8,
            (row_addr & 0xFF) as u8,
        ];
        self.write_cmd(&addr_bytes);
        let status = self.wait_ready().await;
        (status & status::E_FAIL) == 0
    }

    /// Enable internal ECC
    pub fn enable_ecc(&mut self) {
        let feature = self.get_feature(features::FEATURE);
        self.set_feature(features::FEATURE, feature | 0x10);
    }

    /// Disable internal ECC
    pub fn disable_ecc(&mut self) {
        let feature = self.get_feature(features::FEATURE);
        self.set_feature(features::FEATURE, feature & !0x10);
    }

    /// Enable Quad SPI mode
    pub fn enable_quad(&mut self) {
        let feature = self.get_feature(features::FEATURE);
        self.set_feature(features::FEATURE, feature | 0x01);
    }

    /// Unlock all blocks (disable write protection)
    pub fn unlock_all(&mut self) {
        self.set_feature(features::PROTECTION, 0x00);
    }

    /// Get ECC status from last read
    pub fn get_ecc_status(&mut self) -> EccStatus {
        let status = self.read_status();
        let ecc_bits = (status >> 4) & 0x03;
        match ecc_bits {
            0b00 => EccStatus::NoError,
            0b01 => EccStatus::Corrected,
            0b10 => EccStatus::CorrectedMax,
            0b11 => EccStatus::Uncorrectable,
            _ => EccStatus::NoError,
        }
    }
}

/// ECC status from internal ECC engine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EccStatus {
    NoError,
    Corrected,
    CorrectedMax,
    Uncorrectable,
}
