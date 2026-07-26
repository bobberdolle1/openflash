//! Memory-mapped GPIO for Orange Pi (Allwinner/Rockchip)
//!
//! Different SoCs have different GPIO base addresses and register layouts.
//!
//! Not reachable from the command handler: parallel NAND over Linux GPIO cannot
//! meet the chip's timing requirements reliably, and the agent advertises only
//! SPI NOR. Kept compiled and type-checked so the register maps stay reviewable
//! rather than rotting.
#![allow(dead_code)]

use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use thiserror::Error;

/// GPIO base addresses for different SoCs
pub mod base_addr {
    /// Allwinner H618 (Orange Pi Zero 3)
    pub const H618_GPIO: u64 = 0x0300_B000;

    /// Allwinner H616 (Orange Pi Zero 2W)
    pub const H616_GPIO: u64 = 0x0300_B000;

    /// Rockchip RK3588 (Orange Pi 5)
    pub const RK3588_GPIO0: u64 = 0xFD8A_0000;
    pub const RK3588_GPIO1: u64 = 0xFEC2_0000;
    pub const RK3588_GPIO2: u64 = 0xFEC3_0000;
    pub const RK3588_GPIO3: u64 = 0xFEC4_0000;
    pub const RK3588_GPIO4: u64 = 0xFEC5_0000;
}

/// GPIO controller
pub struct GpioController {
    mmap: Option<MmapMut>,
    soc_type: SocType,
}

/// SoC type
#[derive(Clone, Copy)]
pub enum SocType {
    AllwinnerH618,
    AllwinnerH616,
    RockchipRK3588,
}

#[derive(Error, Debug)]
pub enum GpioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Invalid pin")]
    InvalidPin,
}

impl GpioController {
    pub fn new(soc_type: SocType) -> Self {
        Self {
            mmap: None,
            soc_type,
        }
    }

    /// Initialize memory-mapped GPIO
    pub fn init(&mut self) -> Result<(), GpioError> {
        let base_addr = match self.soc_type {
            SocType::AllwinnerH618 => base_addr::H618_GPIO,
            SocType::AllwinnerH616 => base_addr::H616_GPIO,
            SocType::RockchipRK3588 => base_addr::RK3588_GPIO0,
        };

        let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

        let mmap = unsafe {
            MmapOptions::new()
                .offset(base_addr)
                .len(0x1000)
                .map_mut(&file)?
        };

        self.mmap = Some(mmap);
        Ok(())
    }

    /// Set pin as output
    pub fn set_output(&mut self, _pin: u8) -> Result<(), GpioError> {
        Ok(())
    }

    /// Set pin as input
    pub fn set_input(&mut self, _pin: u8) -> Result<(), GpioError> {
        Ok(())
    }

    /// Write pin value
    pub fn write(&mut self, _pin: u8, _value: bool) -> Result<(), GpioError> {
        Ok(())
    }

    /// Read pin value
    pub fn read(&self, _pin: u8) -> Result<bool, GpioError> {
        Ok(false)
    }
}
