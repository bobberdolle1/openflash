//! USB Protocol definitions for OpenFlash
//! Defines command packets for communication between host and firmware

use serde::{Deserialize, Serialize};

/// Flash interface type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashInterface {
    ParallelNand = 0x00,
    SpiNand = 0x01,
    Emmc = 0x02,
    SpiNor = 0x03,          // NEW: SPI NOR Flash
    Ufs = 0x04,             // NEW: Universal Flash Storage
    ParallelNand16 = 0x05,  // NEW: 16-bit parallel NAND
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
    
    // eMMC commands (0x40-0x5F)
    EmmcInit = 0x40,              // Initialize eMMC card
    EmmcReadCid = 0x41,           // Read CID register
    EmmcReadCsd = 0x42,           // Read CSD register
    EmmcReadExtCsd = 0x43,        // Read Extended CSD
    EmmcReadBlock = 0x44,         // Read single block
    EmmcReadMultiple = 0x45,      // Read multiple blocks
    EmmcWriteBlock = 0x46,        // Write single block
    EmmcWriteMultiple = 0x47,     // Write multiple blocks
    EmmcErase = 0x48,             // Erase blocks
    EmmcGetStatus = 0x49,         // Get card status
    EmmcSetPartition = 0x4A,      // Select partition (user/boot/rpmb)
    
    // SPI NOR commands (0x60-0x7F)
    SpiNorReadJedecId = 0x60,     // Read JEDEC ID
    SpiNorReadSfdp = 0x61,        // Read SFDP data
    SpiNorRead = 0x62,            // Standard read
    SpiNorFastRead = 0x63,        // Fast read with dummy cycle
    SpiNorDualRead = 0x64,        // Dual SPI read
    SpiNorQuadRead = 0x65,        // Quad SPI read
    SpiNorPageProgram = 0x66,     // Page program (256 bytes)
    SpiNorSectorErase = 0x67,     // Sector erase (4KB)
    SpiNorBlockErase32K = 0x68,   // Block erase (32KB)
    SpiNorBlockErase64K = 0x69,   // Block erase (64KB)
    SpiNorChipErase = 0x6A,       // Chip erase
    SpiNorReadStatus1 = 0x6B,     // Read status register 1
    SpiNorReadStatus2 = 0x6C,     // Read status register 2
    SpiNorReadStatus3 = 0x6D,     // Read status register 3
    SpiNorWriteStatus1 = 0x6E,    // Write status register 1
    SpiNorWriteStatus2 = 0x6F,    // Write status register 2
    SpiNorWriteStatus3 = 0x70,    // Write status register 3
    SpiNorWriteEnable = 0x71,     // Write enable
    SpiNorWriteDisable = 0x72,    // Write disable
    SpiNorReset = 0x73,           // Software reset
    
    // UFS commands (0x80-0x9F)
    UfsInit = 0x80,               // Initialize UFS device
    UfsReadDescriptor = 0x81,     // Read UFS descriptor
    UfsReadCapacity = 0x82,       // Read device capacity
    UfsRead10 = 0x83,             // SCSI READ(10) command
    UfsRead16 = 0x84,             // SCSI READ(16) command
    UfsWrite10 = 0x85,            // SCSI WRITE(10) command
    UfsWrite16 = 0x86,            // SCSI WRITE(16) command
    UfsSelectLun = 0x87,          // Select logical unit
    UfsGetStatus = 0x88,          // Get device status
    
    // Advanced Write Operations (0xA0-0xBF) - v1.7
    FullChipProgram = 0xA0,       // Full chip programming with verify
    ReadBadBlockTable = 0xA1,     // Read bad block table
    WriteBadBlockTable = 0xA2,    // Write bad block table
    ScanBadBlocks = 0xA3,         // Scan for bad blocks
    MarkBadBlock = 0xA4,          // Mark block as bad
    GetWearInfo = 0xA5,           // Get wear leveling info
    ProgramWithVerify = 0xA6,     // Program page with verification
    EraseWithVerify = 0xA7,       // Erase block with verification
    IncrementalRead = 0xA8,       // Read only changed blocks
    CloneStart = 0xA9,            // Start chip-to-chip clone
    CloneStatus = 0xAA,           // Get clone operation status
    CloneAbort = 0xAB,            // Abort clone operation
    
    // Scripting & Automation Commands (0xB0-0xBF) - v1.8
    BatchStart = 0xB0,            // Start batch operation
    BatchStatus = 0xB1,           // Get batch operation status
    BatchAbort = 0xB2,            // Abort batch operation
    ScriptLoad = 0xB3,            // Load script to device
    ScriptRun = 0xB4,             // Run loaded script
    ScriptStatus = 0xB5,          // Get script execution status
    PluginList = 0xB6,            // List loaded plugins
    PluginLoad = 0xB7,            // Load plugin
    PluginUnload = 0xB8,          // Unload plugin
    RemoteConnect = 0xB9,         // Remote connection (server mode)
    RemoteDisconnect = 0xBA,      // Disconnect remote
    GetDeviceInfo = 0xBB,         // Get detailed device info
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
            
            // eMMC
            0x40 => Some(Command::EmmcInit),
            0x41 => Some(Command::EmmcReadCid),
            0x42 => Some(Command::EmmcReadCsd),
            0x43 => Some(Command::EmmcReadExtCsd),
            0x44 => Some(Command::EmmcReadBlock),
            0x45 => Some(Command::EmmcReadMultiple),
            0x46 => Some(Command::EmmcWriteBlock),
            0x47 => Some(Command::EmmcWriteMultiple),
            0x48 => Some(Command::EmmcErase),
            0x49 => Some(Command::EmmcGetStatus),
            0x4A => Some(Command::EmmcSetPartition),
            
            // SPI NOR
            0x60 => Some(Command::SpiNorReadJedecId),
            0x61 => Some(Command::SpiNorReadSfdp),
            0x62 => Some(Command::SpiNorRead),
            0x63 => Some(Command::SpiNorFastRead),
            0x64 => Some(Command::SpiNorDualRead),
            0x65 => Some(Command::SpiNorQuadRead),
            0x66 => Some(Command::SpiNorPageProgram),
            0x67 => Some(Command::SpiNorSectorErase),
            0x68 => Some(Command::SpiNorBlockErase32K),
            0x69 => Some(Command::SpiNorBlockErase64K),
            0x6A => Some(Command::SpiNorChipErase),
            0x6B => Some(Command::SpiNorReadStatus1),
            0x6C => Some(Command::SpiNorReadStatus2),
            0x6D => Some(Command::SpiNorReadStatus3),
            0x6E => Some(Command::SpiNorWriteStatus1),
            0x6F => Some(Command::SpiNorWriteStatus2),
            0x70 => Some(Command::SpiNorWriteStatus3),
            0x71 => Some(Command::SpiNorWriteEnable),
            0x72 => Some(Command::SpiNorWriteDisable),
            0x73 => Some(Command::SpiNorReset),
            
            // UFS
            0x80 => Some(Command::UfsInit),
            0x81 => Some(Command::UfsReadDescriptor),
            0x82 => Some(Command::UfsReadCapacity),
            0x83 => Some(Command::UfsRead10),
            0x84 => Some(Command::UfsRead16),
            0x85 => Some(Command::UfsWrite10),
            0x86 => Some(Command::UfsWrite16),
            0x87 => Some(Command::UfsSelectLun),
            0x88 => Some(Command::UfsGetStatus),
            
            // Advanced Write Operations (v1.7)
            0xA0 => Some(Command::FullChipProgram),
            0xA1 => Some(Command::ReadBadBlockTable),
            0xA2 => Some(Command::WriteBadBlockTable),
            0xA3 => Some(Command::ScanBadBlocks),
            0xA4 => Some(Command::MarkBadBlock),
            0xA5 => Some(Command::GetWearInfo),
            0xA6 => Some(Command::ProgramWithVerify),
            0xA7 => Some(Command::EraseWithVerify),
            0xA8 => Some(Command::IncrementalRead),
            0xA9 => Some(Command::CloneStart),
            0xAA => Some(Command::CloneStatus),
            0xAB => Some(Command::CloneAbort),
            
            // Scripting & Automation (v1.8)
            0xB0 => Some(Command::BatchStart),
            0xB1 => Some(Command::BatchStatus),
            0xB2 => Some(Command::BatchAbort),
            0xB3 => Some(Command::ScriptLoad),
            0xB4 => Some(Command::ScriptRun),
            0xB5 => Some(Command::ScriptStatus),
            0xB6 => Some(Command::PluginList),
            0xB7 => Some(Command::PluginLoad),
            0xB8 => Some(Command::PluginUnload),
            0xB9 => Some(Command::RemoteConnect),
            0xBA => Some(Command::RemoteDisconnect),
            0xBB => Some(Command::GetDeviceInfo),
            
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
    
    /// Check if command is for eMMC interface
    pub fn is_emmc(&self) -> bool {
        matches!(self,
            Command::EmmcInit |
            Command::EmmcReadCid |
            Command::EmmcReadCsd |
            Command::EmmcReadExtCsd |
            Command::EmmcReadBlock |
            Command::EmmcReadMultiple |
            Command::EmmcWriteBlock |
            Command::EmmcWriteMultiple |
            Command::EmmcErase |
            Command::EmmcGetStatus |
            Command::EmmcSetPartition
        )
    }
    
    /// Check if command is for SPI NOR interface
    pub fn is_spi_nor(&self) -> bool {
        matches!(self,
            Command::SpiNorReadJedecId |
            Command::SpiNorReadSfdp |
            Command::SpiNorRead |
            Command::SpiNorFastRead |
            Command::SpiNorDualRead |
            Command::SpiNorQuadRead |
            Command::SpiNorPageProgram |
            Command::SpiNorSectorErase |
            Command::SpiNorBlockErase32K |
            Command::SpiNorBlockErase64K |
            Command::SpiNorChipErase |
            Command::SpiNorReadStatus1 |
            Command::SpiNorReadStatus2 |
            Command::SpiNorReadStatus3 |
            Command::SpiNorWriteStatus1 |
            Command::SpiNorWriteStatus2 |
            Command::SpiNorWriteStatus3 |
            Command::SpiNorWriteEnable |
            Command::SpiNorWriteDisable |
            Command::SpiNorReset
        )
    }
    
    /// Check if command is for UFS interface
    pub fn is_ufs(&self) -> bool {
        matches!(self,
            Command::UfsInit |
            Command::UfsReadDescriptor |
            Command::UfsReadCapacity |
            Command::UfsRead10 |
            Command::UfsRead16 |
            Command::UfsWrite10 |
            Command::UfsWrite16 |
            Command::UfsSelectLun |
            Command::UfsGetStatus
        )
    }
    
    /// Check if command is for advanced write operations (v1.7)
    pub fn is_write_ops(&self) -> bool {
        matches!(self,
            Command::FullChipProgram |
            Command::ReadBadBlockTable |
            Command::WriteBadBlockTable |
            Command::ScanBadBlocks |
            Command::MarkBadBlock |
            Command::GetWearInfo |
            Command::ProgramWithVerify |
            Command::EraseWithVerify |
            Command::IncrementalRead |
            Command::CloneStart |
            Command::CloneStatus |
            Command::CloneAbort
        )
    }
    
    /// Check if command is for scripting & automation (v1.8)
    pub fn is_scripting(&self) -> bool {
        matches!(self,
            Command::BatchStart |
            Command::BatchStatus |
            Command::BatchAbort |
            Command::ScriptLoad |
            Command::ScriptRun |
            Command::ScriptStatus |
            Command::PluginList |
            Command::PluginLoad |
            Command::PluginUnload |
            Command::RemoteConnect |
            Command::RemoteDisconnect |
            Command::GetDeviceInfo
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

/// eMMC commands (re-exported from emmc module)
pub mod emmc_commands {
    pub use crate::emmc::commands::*;
}

/// SPI NOR commands (re-exported from spi_nor module)
pub mod spi_nor_commands {
    pub use crate::spi_nor::commands::*;
}

/// UFS SCSI commands (re-exported from ufs module)
pub mod ufs_commands {
    pub use crate::ufs::scsi::*;
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
    
    #[test]
    fn test_emmc_command_detection() {
        assert!(Command::EmmcInit.is_emmc());
        assert!(Command::EmmcReadBlock.is_emmc());
        assert!(!Command::SpiNandReadId.is_emmc());
        assert!(!Command::NandReadPage.is_emmc());
    }
    
    #[test]
    fn test_emmc_command_from_u8() {
        assert_eq!(Command::from_u8(0x40), Some(Command::EmmcInit));
        assert_eq!(Command::from_u8(0x44), Some(Command::EmmcReadBlock));
        assert_eq!(Command::from_u8(0x48), Some(Command::EmmcErase));
    }
    
    #[test]
    fn test_spi_nor_command_from_u8() {
        assert_eq!(Command::from_u8(0x60), Some(Command::SpiNorReadJedecId));
        assert_eq!(Command::from_u8(0x62), Some(Command::SpiNorRead));
        assert_eq!(Command::from_u8(0x66), Some(Command::SpiNorPageProgram));
        assert_eq!(Command::from_u8(0x67), Some(Command::SpiNorSectorErase));
        assert_eq!(Command::from_u8(0x6A), Some(Command::SpiNorChipErase));
        assert_eq!(Command::from_u8(0x73), Some(Command::SpiNorReset));
    }
    
    #[test]
    fn test_spi_nor_command_detection() {
        assert!(Command::SpiNorReadJedecId.is_spi_nor());
        assert!(Command::SpiNorRead.is_spi_nor());
        assert!(Command::SpiNorPageProgram.is_spi_nor());
        assert!(!Command::SpiNandReadId.is_spi_nor());
        assert!(!Command::EmmcInit.is_spi_nor());
        assert!(!Command::Ping.is_spi_nor());
    }
    
    #[test]
    fn test_ufs_command_from_u8() {
        assert_eq!(Command::from_u8(0x80), Some(Command::UfsInit));
        assert_eq!(Command::from_u8(0x81), Some(Command::UfsReadDescriptor));
        assert_eq!(Command::from_u8(0x83), Some(Command::UfsRead10));
        assert_eq!(Command::from_u8(0x84), Some(Command::UfsRead16));
        assert_eq!(Command::from_u8(0x87), Some(Command::UfsSelectLun));
        assert_eq!(Command::from_u8(0x88), Some(Command::UfsGetStatus));
    }
    
    #[test]
    fn test_ufs_command_detection() {
        assert!(Command::UfsInit.is_ufs());
        assert!(Command::UfsRead10.is_ufs());
        assert!(Command::UfsSelectLun.is_ufs());
        assert!(!Command::SpiNorRead.is_ufs());
        assert!(!Command::EmmcInit.is_ufs());
        assert!(!Command::Ping.is_ufs());
    }
    
    #[test]
    fn test_flash_interface_values() {
        assert_eq!(FlashInterface::ParallelNand as u8, 0x00);
        assert_eq!(FlashInterface::SpiNand as u8, 0x01);
        assert_eq!(FlashInterface::Emmc as u8, 0x02);
        assert_eq!(FlashInterface::SpiNor as u8, 0x03);
        assert_eq!(FlashInterface::Ufs as u8, 0x04);
        assert_eq!(FlashInterface::ParallelNand16 as u8, 0x05);
    }
    
    #[test]
    fn test_write_ops_command_from_u8() {
        assert_eq!(Command::from_u8(0xA0), Some(Command::FullChipProgram));
        assert_eq!(Command::from_u8(0xA1), Some(Command::ReadBadBlockTable));
        assert_eq!(Command::from_u8(0xA3), Some(Command::ScanBadBlocks));
        assert_eq!(Command::from_u8(0xA6), Some(Command::ProgramWithVerify));
        assert_eq!(Command::from_u8(0xA9), Some(Command::CloneStart));
        assert_eq!(Command::from_u8(0xAB), Some(Command::CloneAbort));
    }
    
    #[test]
    fn test_write_ops_command_detection() {
        assert!(Command::FullChipProgram.is_write_ops());
        assert!(Command::ReadBadBlockTable.is_write_ops());
        assert!(Command::ScanBadBlocks.is_write_ops());
        assert!(Command::CloneStart.is_write_ops());
        assert!(!Command::SpiNorRead.is_write_ops());
        assert!(!Command::EmmcInit.is_write_ops());
        assert!(!Command::Ping.is_write_ops());
    }
    
    #[test]
    fn test_scripting_command_from_u8() {
        assert_eq!(Command::from_u8(0xB0), Some(Command::BatchStart));
        assert_eq!(Command::from_u8(0xB1), Some(Command::BatchStatus));
        assert_eq!(Command::from_u8(0xB3), Some(Command::ScriptLoad));
        assert_eq!(Command::from_u8(0xB4), Some(Command::ScriptRun));
        assert_eq!(Command::from_u8(0xB6), Some(Command::PluginList));
        assert_eq!(Command::from_u8(0xB9), Some(Command::RemoteConnect));
        assert_eq!(Command::from_u8(0xBB), Some(Command::GetDeviceInfo));
    }
    
    #[test]
    fn test_scripting_command_detection() {
        assert!(Command::BatchStart.is_scripting());
        assert!(Command::ScriptRun.is_scripting());
        assert!(Command::PluginList.is_scripting());
        assert!(Command::RemoteConnect.is_scripting());
        assert!(Command::GetDeviceInfo.is_scripting());
        assert!(!Command::FullChipProgram.is_scripting());
        assert!(!Command::SpiNorRead.is_scripting());
        assert!(!Command::Ping.is_scripting());
    }
}
