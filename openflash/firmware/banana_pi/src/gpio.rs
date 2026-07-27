//! GPIO control for Banana Pi boards
//!
//! Supports memory-mapped GPIO for Allwinner and SpacemiT SoCs.
//! Note: Parallel NAND via GPIO is not recommended on Linux SBCs
//! due to timing constraints. Use SPI interfaces instead.
//!
//! Not reachable from the command handler: parallel NAND over Linux GPIO cannot
//! meet the chip's timing requirements reliably, and the agent advertises only
//! SPI NOR. Kept compiled and type-checked so the register maps stay reviewable
//! rather than rotting.
#![allow(dead_code)]

use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::io;

/// GPIO controller for Allwinner H3/H618
pub struct AllwinnerGpio {
    mmap: Option<MmapMut>,
    base: u32,
}

impl AllwinnerGpio {
    /// Create new GPIO controller
    pub fn new(base: u32) -> Self {
        Self { mmap: None, base }
    }

    /// Initialize memory-mapped GPIO
    pub fn init(&mut self) -> io::Result<()> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/mem")?;

        let mmap = unsafe {
            MmapOptions::new()
                .offset(self.base as u64)
                .len(0x1000) // 4KB page
                .map_mut(&file)?
        };

        self.mmap = Some(mmap);
        Ok(())
    }

    /// Set pin as output
    pub fn set_output(&mut self, port: u8, pin: u8) {
        if let Some(ref mut mmap) = self.mmap {
            let cfg_offset = (port as usize) * 0x24 + (pin as usize / 8) * 4;
            let bit_offset = (pin % 8) * 4;

            if cfg_offset + 4 <= mmap.len() {
                let mut val = u32::from_le_bytes([
                    mmap[cfg_offset],
                    mmap[cfg_offset + 1],
                    mmap[cfg_offset + 2],
                    mmap[cfg_offset + 3],
                ]);
                val &= !(0xF << bit_offset);
                val |= 0x1 << bit_offset; // Output mode
                mmap[cfg_offset..cfg_offset + 4].copy_from_slice(&val.to_le_bytes());
            }
        }
    }

    /// Set pin as input
    pub fn set_input(&mut self, port: u8, pin: u8) {
        if let Some(ref mut mmap) = self.mmap {
            let cfg_offset = (port as usize) * 0x24 + (pin as usize / 8) * 4;
            let bit_offset = (pin % 8) * 4;

            if cfg_offset + 4 <= mmap.len() {
                let mut val = u32::from_le_bytes([
                    mmap[cfg_offset],
                    mmap[cfg_offset + 1],
                    mmap[cfg_offset + 2],
                    mmap[cfg_offset + 3],
                ]);
                val &= !(0xF << bit_offset); // Input mode (0)
                mmap[cfg_offset..cfg_offset + 4].copy_from_slice(&val.to_le_bytes());
            }
        }
    }

    /// Write pin value
    pub fn write(&mut self, port: u8, pin: u8, high: bool) {
        if let Some(ref mut mmap) = self.mmap {
            let data_offset = (port as usize) * 0x24 + 0x10;

            if data_offset + 4 <= mmap.len() {
                let mut val = u32::from_le_bytes([
                    mmap[data_offset],
                    mmap[data_offset + 1],
                    mmap[data_offset + 2],
                    mmap[data_offset + 3],
                ]);
                if high {
                    val |= 1 << pin;
                } else {
                    val &= !(1 << pin);
                }
                mmap[data_offset..data_offset + 4].copy_from_slice(&val.to_le_bytes());
            }
        }
    }

    /// Read pin value
    pub fn read(&self, port: u8, pin: u8) -> bool {
        if let Some(ref mmap) = self.mmap {
            let data_offset = (port as usize) * 0x24 + 0x10;

            if data_offset + 4 <= mmap.len() {
                let val = u32::from_le_bytes([
                    mmap[data_offset],
                    mmap[data_offset + 1],
                    mmap[data_offset + 2],
                    mmap[data_offset + 3],
                ]);
                return (val >> pin) & 1 == 1;
            }
        }
        false
    }
}

/// GPIO controller using libgpiod (safer, works without root)
pub struct GpiodController {
    chip_path: String,
}

impl GpiodController {
    pub fn new(chip: &str) -> Self {
        Self {
            chip_path: format!("/dev/{}", chip),
        }
    }

    /// Check if gpiod is available
    pub fn is_available(&self) -> bool {
        std::path::Path::new(&self.chip_path).exists()
    }
}

/// Port identifiers for Allwinner SoCs
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AllwinnerPort {
    PA = 0,
    PB = 1,
    PC = 2,
    PD = 3,
    PE = 4,
    PF = 5,
    PG = 6,
    PH = 7,
    PI = 8,
}
