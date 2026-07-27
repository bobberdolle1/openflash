//! Host-side view of the OpenFlash wire protocol.
//!
//! The opcode table, framing and status codes live in the `openflash-protocol`
//! crate, which host and firmware share verbatim, and are re-exported here so
//! `openflash_core::protocol::Command` keeps working. This module adds the
//! chip-level command bytes that only the host needs — the bytes a host asks a
//! device to drive onto the NAND/SPI bus, as opposed to the commands it sends
//! *to* the device.
//!
//! Note the two are different layers and use overlapping numbers:
//! [`Command::NandReadId`] (`0x14`) is "device, read the chip id for me", while
//! [`nand_commands::READID`] (`0x90`) is the byte the device then drives onto
//! the NAND bus.

pub use openflash_protocol::command::{Command, FlashInterface, Status, LEGACY_NAND_ALIASES};
pub use openflash_protocol::frame::{Frame, FrameError, MAX_PAYLOAD};
pub use openflash_protocol::{
    Platform, VersionInfo, PROTOCOL_VERSION, USB_ENDPOINT_IN, USB_ENDPOINT_OUT, USB_PRODUCT_ID,
    USB_VENDOR_ID,
};

use serde::{Deserialize, Serialize};

/// Command bytes driven onto a parallel NAND bus, per the ONFI specification.
pub mod nand_commands {
    /// Read cycle, first phase.
    pub const READ1: u8 = 0x00;
    /// Read cycle, confirm phase.
    pub const READ2: u8 = 0x30;
    /// Read the chip id.
    pub const READID: u8 = 0x90;
    /// Begin a page program.
    pub const PAGEPROG: u8 = 0x80;
    /// Confirm a page program.
    pub const PROGSTART: u8 = 0x10;
    /// Begin a block erase.
    pub const BLOCKERASE: u8 = 0x60;
    /// Confirm a block erase.
    pub const ERASESTART: u8 = 0xD0;
    /// Read the status register.
    pub const READSTATUS: u8 = 0x70;
    /// Reset the chip.
    pub const RESET: u8 = 0xFF;
    /// Read the ONFI parameter page.
    pub const READ_PARAM_PAGE: u8 = 0xEC;
}

/// SPI NAND chip command bytes.
pub mod spi_nand_commands {
    pub use crate::spi_nand::commands::*;
}

/// eMMC chip command indices.
pub mod emmc_commands {
    pub use crate::emmc::commands::*;
}

/// SPI NOR chip command bytes.
pub mod spi_nor_commands {
    pub use crate::spi_nor::commands::*;
}

/// UFS SCSI operation codes.
pub mod ufs_commands {
    pub use crate::ufs::scsi::*;
}

/// SPI bus settings a host asks a device to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpiConfig {
    /// Clock frequency in MHz.
    pub clock_mhz: u8,
    /// SPI mode, 0 to 3.
    pub mode: u8,
    /// Whether to use four data lines.
    pub quad_enabled: bool,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            clock_mhz: 40,
            mode: 0,
            quad_enabled: false,
        }
    }
}

impl SpiConfig {
    /// Conservative settings that work on essentially any part.
    pub fn safe() -> Self {
        Self {
            clock_mhz: 10,
            mode: 0,
            quad_enabled: false,
        }
    }

    /// Fast settings for parts that advertise quad support.
    pub fn fast() -> Self {
        Self {
            clock_mhz: 80,
            mode: 0,
            quad_enabled: true,
        }
    }

    /// Encode as the payload of a [`Command::BusConfig`] request.
    pub fn to_bytes(self) -> [u8; 3] {
        [self.clock_mhz, self.mode, self.quad_enabled as u8]
    }

    /// Decode a [`Command::BusConfig`] payload.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 {
            return None;
        }
        Some(Self {
            clock_mhz: bytes[0],
            mode: bytes[1],
            quad_enabled: bytes[2] != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reexported_table_is_the_shared_one() {
        assert_eq!(Command::Ping as u8, 0x01);
        assert_eq!(Command::NandReadPage as u8, 0x12);
        assert_eq!(Command::SpiNorReadJedecId as u8, 0x60);
        assert_eq!(PROTOCOL_VERSION, openflash_protocol::PROTOCOL_VERSION);
    }

    /// The chip-level and device-level command spaces are distinct layers. NAND
    /// RESET is 0xFF on the chip bus, which is exactly why 0xFF may not be a
    /// device command: the two must never be confused.
    #[test]
    fn chip_level_reset_does_not_decode_as_a_device_command() {
        assert_eq!(nand_commands::RESET, 0xFF);
        assert_eq!(Command::from_u8(nand_commands::RESET), None);
    }

    #[test]
    fn bus_config_round_trips() {
        for config in [SpiConfig::default(), SpiConfig::safe(), SpiConfig::fast()] {
            let bytes = config.to_bytes();
            assert_eq!(SpiConfig::from_bytes(&bytes), Some(config));
        }
        assert_eq!(SpiConfig::from_bytes(&[1, 2]), None);
    }
}
