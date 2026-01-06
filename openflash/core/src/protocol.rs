//! USB Protocol definitions for OpenFlash
//! Defines command packets for communication between host and firmware

use serde::{Deserialize, Serialize};

/// Flash interface type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashInterface {
    ParallelNand = 0x00,
    SpiNand = 0x01,
}

/// USB Protocol Commands
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    // General commands (0x01-0x0F)
    Ping = 0x01,
    BusConfig = 0x02,
    Reset = 0x08,
    SetInterface = 0x09,      // Set flash interface type
    
    // Parallel NAND commands (0x10-0x1F)
    NandCmd = 0x10,
    NandAddr = 0x11,
    NandReadPage = 0x12,
    NandWritePage = 0x13,
    NandReadId = 0x14,
    NandErase = 0x15,
    NandReadStatus = 0x16,
    
    // SPI NAND commands (0x20-0x3F)
    SpiNandReadId = 0x20,
    SpiNandReset = 0x21,
    SpiNandGetFeature = 0x22,
    SpiNandSetFeature = 0x23,
    SpiNandPageRead = 0x24,       // Load page to cache
    SpiNandReadCache = 0x25,      // Read from cache
    SpiNandReadCacheX4 = 0x26,    // Read from cache (Quad)
    SpiNandProgramLoad = 0x27,    // Load data to cache
    SpiNandProgramLoadX4 = 0x28,  // Load data to cache (Quad)
    SpiNandProgramExec = 0x29,    // Program cache to array
    SpiNandBlockErase = 0x2A,
    SpiNandWriteEnable = 0x2B,
    SpiNandWriteDisable = 0x2C,
}

impl Command {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            // General
            0x01 => Some(Command::Ping),
            0x02 => Some(Command::BusConfig),
            0x08 => Some(Command::Reset),
            0x09 => Some(Command::SetInterface),
            
            // Parallel NAND (legacy 0x03-0x07 mapped to new values)
            0x03 | 0x10 => Some(Command::NandCmd),
            0x04 | 0x11 => Some(Command::NandAddr),
            0x05 | 0x12 => Some(Command::NandReadPage),
            0x06 | 0x13 => Some(Command::NandWritePage),
            0x07 | 0x14 => Some(Command::NandReadId),
            0x15 => Some(Command::NandErase),
            0x16 => Some(Command::NandReadStatus),
            
            // SPI NAND
            0x20 => Some(Command::SpiNandReadId),
            0x21 => Some(Command::SpiNandReset),
            0x22 => Some(Command::SpiNandGetFeature),
            0x23 => Some(Command::SpiNandSetFeature),
            0x24 => Some(Command::SpiNandPageRead),
            0x25 => Some(Command::SpiNandReadCache),
            0x26 => Some(Command::SpiNandReadCacheX4),
            0x27 => Some(Command::SpiNandProgramLoad),
            0x28 => Some(Command::SpiNandProgramLoadX4),
            0x29 => Some(Command::SpiNandProgramExec),
            0x2A => Some(Command::SpiNandBlockErase),
            0x2B => Some(Command::SpiNandWriteEnable),
            0x2C => Some(Command::SpiNandWriteDisable),
            
            _ => None,
        }
    }
    
    /// Check if command is for SPI NAND interface
    pub fn is_spi_nand(&self) -> bool {
        matches!(self, 
            Command::SpiNandReadId |
            Command::SpiNandReset |
            Command::SpiNandGetFeature |
            Command::SpiNandSetFeature |
            Command::SpiNandPageRead |
            Command::SpiNandReadCache |
            Command::SpiNandReadCacheX4 |
            Command::SpiNandProgramLoad |
            Command::SpiNandProgramLoadX4 |
            Command::SpiNandProgramExec |
            Command::SpiNandBlockErase |
            Command::SpiNandWriteEnable |
            Command::SpiNandWriteDisable
        )
    }
}

/// Protocol packet structure (64 bytes total)
#[derive(Debug, Clone)]
pub struct Packet {
    pub cmd: Command,
    pub args: [u8; 63],
}

impl Packet {
    pub fn new(cmd: Command, args: &[u8]) -> Self {
        let mut packet_args = [0u8; 63];
        let copy_len = args.len().min(63);
        packet_args[..copy_len].copy_from_slice(&args[..copy_len]);
        
        Self {
            cmd,
            args: packet_args,
        }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[0] = self.cmd as u8;
        bytes[1..].copy_from_slice(&self.args);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 64 {
            return None;
        }

        let cmd = Command::from_u8(bytes[0])?;
        let mut args = [0u8; 63];
        args.copy_from_slice(&bytes[1..64]);

        Some(Self { cmd, args })
    }
}

/// Common parallel NAND commands
pub mod nand_commands {
    pub const READ1: u8 = 0x00;
    pub const READ2: u8 = 0x30;
    pub const READID: u8 = 0x90;
    pub const PAGEPROG: u8 = 0x80;
    pub const PROGSTART: u8 = 0x10;
    pub const BLOCKERASE: u8 = 0x60;
    pub const ERASESTART: u8 = 0xD0;
    pub const READSTATUS: u8 = 0x70;
    pub const RESET: u8 = 0xFF;
}

/// SPI NAND flash commands (re-exported from spi_nand module)
pub mod spi_nand_commands {
    pub use crate::spi_nand::commands::*;
}

/// SPI configuration for SPI NAND
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpiConfig {
    /// Clock frequency in MHz
    pub clock_mhz: u8,
    /// SPI mode (0-3)
    pub mode: u8,
    /// Enable Quad SPI
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
    pub fn fast() -> Self {
        Self {
            clock_mhz: 80,
            mode: 0,
            quad_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let packet = Packet::new(Command::Ping, &[0x01, 0x02, 0x03]);
        let bytes = packet.to_bytes();
        let parsed = Packet::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.cmd, Command::Ping);
        assert_eq!(parsed.args[0], 0x01);
        assert_eq!(parsed.args[1], 0x02);
        assert_eq!(parsed.args[2], 0x03);
    }

    #[test]
    fn test_command_from_u8() {
        assert_eq!(Command::from_u8(0x01), Some(Command::Ping));
        assert_eq!(Command::from_u8(0x14), Some(Command::NandReadId));
        assert_eq!(Command::from_u8(0x20), Some(Command::SpiNandReadId));
        assert_eq!(Command::from_u8(0xFF), None);
    }
    
    #[test]
    fn test_legacy_command_mapping() {
        // Old command values should still work
        assert_eq!(Command::from_u8(0x03), Some(Command::NandCmd));
        assert_eq!(Command::from_u8(0x05), Some(Command::NandReadPage));
        assert_eq!(Command::from_u8(0x07), Some(Command::NandReadId));
    }
    
    #[test]
    fn test_spi_nand_command_detection() {
        assert!(Command::SpiNandReadId.is_spi_nand());
        assert!(Command::SpiNandPageRead.is_spi_nand());
        assert!(!Command::NandReadPage.is_spi_nand());
        assert!(!Command::Ping.is_spi_nand());
    }
}
