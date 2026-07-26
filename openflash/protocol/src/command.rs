//! The canonical device command set.
//!
//! This table is the single source of truth. The host tools and *every* device
//! firmware must use these values and nothing else — before this crate existed
//! each firmware declared its own table and they had silently drifted apart
//! (see `docs/PROTOCOL.md`), which made the host unable to talk to most of the
//! supported boards.
//!
//! # Reserved ranges
//!
//! Only commands a device can actually execute belong here. Host-side and
//! server-side concepts deliberately do not:
//!
//! | Range       | Use                                                       |
//! |-------------|-----------------------------------------------------------|
//! | `0x00`      | never a command (an idle bus reads as all-zeroes)          |
//! | `0x03-0x07` | deprecated v1 parallel-NAND aliases, accepted on receive   |
//! | `0xB0-0xDF` | reserved: host-side batching, scripting, job/server control|
//! | `0xF0-0xFE` | reserved: cloud features, host-to-server only              |
//! | `0xFF`      | never a command (an idle/floating bus reads as all-ones)   |

/// Protocol revision implemented by this crate.
///
/// Bumped to `2` when framing (magic, length, CRC) was introduced; revision 1
/// was the unframed `[command][args…]` layout that could not detect a dropped
/// byte. A device reports the revision it speaks in its
/// [`Command::GetVersion`] response.
pub const PROTOCOL_VERSION: u8 = 2;

/// Commands a device firmware can be asked to execute.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Command {
    // ---- General (0x01-0x0F) ----
    /// Liveness check. Responds with no payload.
    Ping = 0x01,
    /// Configure bus timing/width for the active interface.
    BusConfig = 0x02,
    /// Reset the active interface.
    Reset = 0x08,
    /// Select which flash interface subsequent commands address.
    SetInterface = 0x09,
    /// Report protocol revision, firmware version and platform id.
    GetVersion = 0x0A,
    /// Report which interfaces this device actually implements.
    GetCapabilities = 0x0B,

    // ---- Parallel NAND (0x10-0x1F) ----
    /// Drive a raw command byte onto the NAND bus.
    NandCmd = 0x10,
    /// Drive a raw address byte onto the NAND bus.
    NandAddr = 0x11,
    /// Read one page (plus OOB when requested).
    NandReadPage = 0x12,
    /// Program one page.
    NandWritePage = 0x13,
    /// Read the 5-byte ONFI/JEDEC id.
    NandReadId = 0x14,
    /// Erase one block.
    NandErase = 0x15,
    /// Read the NAND status register.
    NandReadStatus = 0x16,

    // ---- SPI NAND (0x20-0x3F) ----
    /// Read the SPI NAND id.
    SpiNandReadId = 0x20,
    /// Software reset.
    SpiNandReset = 0x21,
    /// Read a feature register.
    SpiNandGetFeature = 0x22,
    /// Write a feature register.
    SpiNandSetFeature = 0x23,
    /// Load a page from the array into the device cache.
    SpiNandPageRead = 0x24,
    /// Read out of the device cache.
    SpiNandReadCache = 0x25,
    /// Read out of the device cache over four data lines.
    SpiNandReadCacheX4 = 0x26,
    /// Load data into the device cache.
    SpiNandProgramLoad = 0x27,
    /// Load data into the device cache over four data lines.
    SpiNandProgramLoadX4 = 0x28,
    /// Commit the device cache to the array.
    SpiNandProgramExec = 0x29,
    /// Erase one block.
    SpiNandBlockErase = 0x2A,
    /// Set the write-enable latch.
    SpiNandWriteEnable = 0x2B,
    /// Clear the write-enable latch.
    SpiNandWriteDisable = 0x2C,

    // ---- eMMC (0x40-0x5F) ----
    /// Run the eMMC initialisation sequence.
    EmmcInit = 0x40,
    /// Read the CID register.
    EmmcReadCid = 0x41,
    /// Read the CSD register.
    EmmcReadCsd = 0x42,
    /// Read the extended CSD register.
    EmmcReadExtCsd = 0x43,
    /// Read a single 512-byte block.
    EmmcReadBlock = 0x44,
    /// Read consecutive blocks.
    EmmcReadMultiple = 0x45,
    /// Write a single 512-byte block.
    EmmcWriteBlock = 0x46,
    /// Write consecutive blocks.
    EmmcWriteMultiple = 0x47,
    /// Erase a block range.
    EmmcErase = 0x48,
    /// Read the card status register.
    EmmcGetStatus = 0x49,
    /// Select the user/boot/RPMB partition.
    EmmcSetPartition = 0x4A,

    // ---- SPI NOR (0x60-0x7F) ----
    /// Read the 3-byte JEDEC id.
    SpiNorReadJedecId = 0x60,
    /// Read the SFDP parameter tables.
    SpiNorReadSfdp = 0x61,
    /// Read at the standard clock rate.
    SpiNorRead = 0x62,
    /// Read with a dummy cycle.
    SpiNorFastRead = 0x63,
    /// Read over two data lines.
    SpiNorDualRead = 0x64,
    /// Read over four data lines.
    SpiNorQuadRead = 0x65,
    /// Program up to one page (256 bytes).
    SpiNorPageProgram = 0x66,
    /// Erase a 4 KiB sector.
    SpiNorSectorErase = 0x67,
    /// Erase a 32 KiB block.
    SpiNorBlockErase32K = 0x68,
    /// Erase a 64 KiB block.
    SpiNorBlockErase64K = 0x69,
    /// Erase the whole chip.
    SpiNorChipErase = 0x6A,
    /// Read status register 1.
    SpiNorReadStatus1 = 0x6B,
    /// Read status register 2.
    SpiNorReadStatus2 = 0x6C,
    /// Read status register 3.
    SpiNorReadStatus3 = 0x6D,
    /// Write status register 1.
    SpiNorWriteStatus1 = 0x6E,
    /// Write status register 2.
    SpiNorWriteStatus2 = 0x6F,
    /// Write status register 3.
    SpiNorWriteStatus3 = 0x70,
    /// Set the write-enable latch.
    SpiNorWriteEnable = 0x71,
    /// Clear the write-enable latch.
    SpiNorWriteDisable = 0x72,
    /// Software reset.
    SpiNorReset = 0x73,

    // ---- UFS (0x80-0x9F) ----
    /// Run the UFS link startup sequence.
    UfsInit = 0x80,
    /// Read a UFS descriptor.
    UfsReadDescriptor = 0x81,
    /// Read the device capacity.
    UfsReadCapacity = 0x82,
    /// SCSI READ(10).
    UfsRead10 = 0x83,
    /// SCSI READ(16).
    UfsRead16 = 0x84,
    /// SCSI WRITE(10).
    UfsWrite10 = 0x85,
    /// SCSI WRITE(16).
    UfsWrite16 = 0x86,
    /// Select the active logical unit.
    UfsSelectLun = 0x87,
    /// Read the device status.
    UfsGetStatus = 0x88,

    // ---- Bad block and wear management (0xA0-0xAF) ----
    /// Program a whole image with per-page verification.
    FullChipProgram = 0xA0,
    /// Read the stored bad block table.
    ReadBadBlockTable = 0xA1,
    /// Write the bad block table.
    WriteBadBlockTable = 0xA2,
    /// Scan the chip for bad blocks.
    ScanBadBlocks = 0xA3,
    /// Mark one block bad.
    MarkBadBlock = 0xA4,
    /// Read erase counters.
    GetWearInfo = 0xA5,
    /// Program one page and read it back.
    ProgramWithVerify = 0xA6,
    /// Erase one block and confirm it reads as erased.
    EraseWithVerify = 0xA7,

    // ---- Hardware expansion (0xE0-0xEF) ----
    /// Detect the OpenFlash PCB.
    PcbDetect = 0xE0,
    /// Report PCB capabilities.
    PcbCapabilities = 0xE1,
    /// Declare the installed socket type.
    SetSocket = 0xE2,
    /// Report adapter info.
    AdapterInfo = 0xE3,
    /// Configure the adapter pinout.
    SetPinout = 0xE4,
    /// Arm the logic analyzer.
    LogicArm = 0xE5,
    /// Start a logic analyzer capture.
    LogicCapture = 0xE6,
    /// Retrieve captured logic analyzer samples.
    LogicGetData = 0xE7,
    /// Scan the JTAG chain.
    JtagScan = 0xE8,
    /// Perform a JTAG transfer.
    JtagTransfer = 0xE9,
    /// Connect over SWD.
    SwdConnect = 0xEA,
    /// Perform an SWD transfer.
    SwdTransfer = 0xEB,
    /// Update the OLED display.
    OledUpdate = 0xEC,
    /// Set the I/O voltage level.
    SetVoltage = 0xED,
    /// Control an attached BGA rework station.
    BgaControl = 0xEE,
    /// Report hardware status.
    HardwareStatus = 0xEF,
}

/// Deprecated protocol v1 parallel-NAND opcodes.
///
/// Firmware built before the tables were unified used these values. Hosts must
/// never send them; [`Command::from_u8`] still maps them so that a v1 device's
/// response can be attributed to the right command.
pub const LEGACY_NAND_ALIASES: [(u8, Command); 5] = [
    (0x03, Command::NandCmd),
    (0x04, Command::NandAddr),
    (0x05, Command::NandReadPage),
    (0x06, Command::NandWritePage),
    (0x07, Command::NandReadId),
];

impl Command {
    /// Decode a command byte, accepting the deprecated v1 NAND aliases.
    pub fn from_u8(value: u8) -> Option<Self> {
        if let Some(cmd) = Self::from_u8_strict(value) {
            return Some(cmd);
        }
        let mut i = 0;
        while i < LEGACY_NAND_ALIASES.len() {
            let (byte, cmd) = LEGACY_NAND_ALIASES[i];
            if byte == value {
                return Some(cmd);
            }
            i += 1;
        }
        None
    }

    /// Decode a command byte, rejecting the deprecated v1 NAND aliases.
    pub fn from_u8_strict(value: u8) -> Option<Self> {
        use Command::*;
        Some(match value {
            0x01 => Ping,
            0x02 => BusConfig,
            0x08 => Reset,
            0x09 => SetInterface,
            0x0A => GetVersion,
            0x0B => GetCapabilities,

            0x10 => NandCmd,
            0x11 => NandAddr,
            0x12 => NandReadPage,
            0x13 => NandWritePage,
            0x14 => NandReadId,
            0x15 => NandErase,
            0x16 => NandReadStatus,

            0x20 => SpiNandReadId,
            0x21 => SpiNandReset,
            0x22 => SpiNandGetFeature,
            0x23 => SpiNandSetFeature,
            0x24 => SpiNandPageRead,
            0x25 => SpiNandReadCache,
            0x26 => SpiNandReadCacheX4,
            0x27 => SpiNandProgramLoad,
            0x28 => SpiNandProgramLoadX4,
            0x29 => SpiNandProgramExec,
            0x2A => SpiNandBlockErase,
            0x2B => SpiNandWriteEnable,
            0x2C => SpiNandWriteDisable,

            0x40 => EmmcInit,
            0x41 => EmmcReadCid,
            0x42 => EmmcReadCsd,
            0x43 => EmmcReadExtCsd,
            0x44 => EmmcReadBlock,
            0x45 => EmmcReadMultiple,
            0x46 => EmmcWriteBlock,
            0x47 => EmmcWriteMultiple,
            0x48 => EmmcErase,
            0x49 => EmmcGetStatus,
            0x4A => EmmcSetPartition,

            0x60 => SpiNorReadJedecId,
            0x61 => SpiNorReadSfdp,
            0x62 => SpiNorRead,
            0x63 => SpiNorFastRead,
            0x64 => SpiNorDualRead,
            0x65 => SpiNorQuadRead,
            0x66 => SpiNorPageProgram,
            0x67 => SpiNorSectorErase,
            0x68 => SpiNorBlockErase32K,
            0x69 => SpiNorBlockErase64K,
            0x6A => SpiNorChipErase,
            0x6B => SpiNorReadStatus1,
            0x6C => SpiNorReadStatus2,
            0x6D => SpiNorReadStatus3,
            0x6E => SpiNorWriteStatus1,
            0x6F => SpiNorWriteStatus2,
            0x70 => SpiNorWriteStatus3,
            0x71 => SpiNorWriteEnable,
            0x72 => SpiNorWriteDisable,
            0x73 => SpiNorReset,

            0x80 => UfsInit,
            0x81 => UfsReadDescriptor,
            0x82 => UfsReadCapacity,
            0x83 => UfsRead10,
            0x84 => UfsRead16,
            0x85 => UfsWrite10,
            0x86 => UfsWrite16,
            0x87 => UfsSelectLun,
            0x88 => UfsGetStatus,

            0xA0 => FullChipProgram,
            0xA1 => ReadBadBlockTable,
            0xA2 => WriteBadBlockTable,
            0xA3 => ScanBadBlocks,
            0xA4 => MarkBadBlock,
            0xA5 => GetWearInfo,
            0xA6 => ProgramWithVerify,
            0xA7 => EraseWithVerify,

            0xE0 => PcbDetect,
            0xE1 => PcbCapabilities,
            0xE2 => SetSocket,
            0xE3 => AdapterInfo,
            0xE4 => SetPinout,
            0xE5 => LogicArm,
            0xE6 => LogicCapture,
            0xE7 => LogicGetData,
            0xE8 => JtagScan,
            0xE9 => JtagTransfer,
            0xEA => SwdConnect,
            0xEB => SwdTransfer,
            0xEC => OledUpdate,
            0xED => SetVoltage,
            0xEE => BgaControl,
            0xEF => HardwareStatus,

            _ => return None,
        })
    }

    /// The interface this command operates on, or `None` for general commands.
    pub fn interface(&self) -> Option<FlashInterface> {
        let byte = *self as u8;
        match byte {
            0x10..=0x1F => Some(FlashInterface::ParallelNand),
            0x20..=0x3F => Some(FlashInterface::SpiNand),
            0x40..=0x5F => Some(FlashInterface::Emmc),
            0x60..=0x7F => Some(FlashInterface::SpiNor),
            0x80..=0x9F => Some(FlashInterface::Ufs),
            _ => None,
        }
    }

    /// Whether this command can modify or destroy data on the chip.
    ///
    /// The CLI refuses to run these without an explicit confirmation, and a
    /// write-blocked session rejects them outright.
    pub fn is_destructive(&self) -> bool {
        use Command::*;
        matches!(
            self,
            NandWritePage
                | NandErase
                | SpiNandProgramLoad
                | SpiNandProgramLoadX4
                | SpiNandProgramExec
                | SpiNandBlockErase
                | SpiNandSetFeature
                | EmmcWriteBlock
                | EmmcWriteMultiple
                | EmmcErase
                | EmmcSetPartition
                | SpiNorPageProgram
                | SpiNorSectorErase
                | SpiNorBlockErase32K
                | SpiNorBlockErase64K
                | SpiNorChipErase
                | SpiNorWriteStatus1
                | SpiNorWriteStatus2
                | SpiNorWriteStatus3
                | UfsWrite10
                | UfsWrite16
                | FullChipProgram
                | WriteBadBlockTable
                | MarkBadBlock
                | ProgramWithVerify
                | EraseWithVerify
        )
    }

    /// Every command in the table, for exhaustive tests and documentation.
    pub const ALL: &'static [Command] = &[
        Command::Ping,
        Command::BusConfig,
        Command::Reset,
        Command::SetInterface,
        Command::GetVersion,
        Command::GetCapabilities,
        Command::NandCmd,
        Command::NandAddr,
        Command::NandReadPage,
        Command::NandWritePage,
        Command::NandReadId,
        Command::NandErase,
        Command::NandReadStatus,
        Command::SpiNandReadId,
        Command::SpiNandReset,
        Command::SpiNandGetFeature,
        Command::SpiNandSetFeature,
        Command::SpiNandPageRead,
        Command::SpiNandReadCache,
        Command::SpiNandReadCacheX4,
        Command::SpiNandProgramLoad,
        Command::SpiNandProgramLoadX4,
        Command::SpiNandProgramExec,
        Command::SpiNandBlockErase,
        Command::SpiNandWriteEnable,
        Command::SpiNandWriteDisable,
        Command::EmmcInit,
        Command::EmmcReadCid,
        Command::EmmcReadCsd,
        Command::EmmcReadExtCsd,
        Command::EmmcReadBlock,
        Command::EmmcReadMultiple,
        Command::EmmcWriteBlock,
        Command::EmmcWriteMultiple,
        Command::EmmcErase,
        Command::EmmcGetStatus,
        Command::EmmcSetPartition,
        Command::SpiNorReadJedecId,
        Command::SpiNorReadSfdp,
        Command::SpiNorRead,
        Command::SpiNorFastRead,
        Command::SpiNorDualRead,
        Command::SpiNorQuadRead,
        Command::SpiNorPageProgram,
        Command::SpiNorSectorErase,
        Command::SpiNorBlockErase32K,
        Command::SpiNorBlockErase64K,
        Command::SpiNorChipErase,
        Command::SpiNorReadStatus1,
        Command::SpiNorReadStatus2,
        Command::SpiNorReadStatus3,
        Command::SpiNorWriteStatus1,
        Command::SpiNorWriteStatus2,
        Command::SpiNorWriteStatus3,
        Command::SpiNorWriteEnable,
        Command::SpiNorWriteDisable,
        Command::SpiNorReset,
        Command::UfsInit,
        Command::UfsReadDescriptor,
        Command::UfsReadCapacity,
        Command::UfsRead10,
        Command::UfsRead16,
        Command::UfsWrite10,
        Command::UfsWrite16,
        Command::UfsSelectLun,
        Command::UfsGetStatus,
        Command::FullChipProgram,
        Command::ReadBadBlockTable,
        Command::WriteBadBlockTable,
        Command::ScanBadBlocks,
        Command::MarkBadBlock,
        Command::GetWearInfo,
        Command::ProgramWithVerify,
        Command::EraseWithVerify,
        Command::PcbDetect,
        Command::PcbCapabilities,
        Command::SetSocket,
        Command::AdapterInfo,
        Command::SetPinout,
        Command::LogicArm,
        Command::LogicCapture,
        Command::LogicGetData,
        Command::JtagScan,
        Command::JtagTransfer,
        Command::SwdConnect,
        Command::SwdTransfer,
        Command::OledUpdate,
        Command::SetVoltage,
        Command::BgaControl,
        Command::HardwareStatus,
    ];
}

/// Which flash interface a device should route commands to.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlashInterface {
    /// 8-bit parallel NAND (TSOP-48 and friends).
    #[default]
    ParallelNand = 0x00,
    /// SPI NAND.
    SpiNand = 0x01,
    /// eMMC.
    Emmc = 0x02,
    /// SPI NOR.
    SpiNor = 0x03,
    /// Universal Flash Storage.
    Ufs = 0x04,
    /// 16-bit parallel NAND.
    ParallelNand16 = 0x05,
}

impl FlashInterface {
    /// Decode an interface byte.
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::ParallelNand,
            0x01 => Self::SpiNand,
            0x02 => Self::Emmc,
            0x03 => Self::SpiNor,
            0x04 => Self::Ufs,
            0x05 => Self::ParallelNand16,
            _ => return None,
        })
    }

    /// Human-readable name, also used as the CLI's `--interface` value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParallelNand => "parallel-nand",
            Self::SpiNand => "spi-nand",
            Self::Emmc => "emmc",
            Self::SpiNor => "spi-nor",
            Self::Ufs => "ufs",
            Self::ParallelNand16 => "parallel-nand16",
        }
    }

    /// Parse the name produced by [`FlashInterface::as_str`].
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "parallel-nand" | "parallel_nand" | "nand" => Self::ParallelNand,
            "spi-nand" | "spi_nand" => Self::SpiNand,
            "emmc" => Self::Emmc,
            "spi-nor" | "spi_nor" | "nor" => Self::SpiNor,
            "ufs" => Self::Ufs,
            "parallel-nand16" | "parallel_nand16" | "nand16" => Self::ParallelNand16,
            _ => return None,
        })
    }

    /// Every interface, for exhaustive tests and CLI help text.
    pub const ALL: &'static [FlashInterface] = &[
        Self::ParallelNand,
        Self::SpiNand,
        Self::Emmc,
        Self::SpiNor,
        Self::Ufs,
        Self::ParallelNand16,
    ];
}

/// Status byte a device returns in every response frame.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Status {
    /// The command completed.
    Ok = 0x00,
    /// The command failed for a reason with no more specific code.
    Error = 0x01,
    /// The device is busy with a previous command.
    Busy = 0x02,
    /// The operation timed out on the device side.
    Timeout = 0x03,
    /// The command byte is not implemented by this firmware.
    UnsupportedCommand = 0x04,
    /// The payload was malformed or out of range.
    InvalidArgument = 0x05,
    /// The command is valid but not supported on the active interface.
    UnsupportedOnInterface = 0x06,
    /// No chip responded.
    ChipNotFound = 0x07,
    /// Data was read but ECC could not correct it.
    EccFailure = 0x08,
    /// The chip or the device refused a write.
    WriteProtected = 0x09,
    /// A verify-after-write comparison failed.
    VerifyFailed = 0x0A,
}

impl Status {
    /// Decode a status byte.
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::Ok,
            0x01 => Self::Error,
            0x02 => Self::Busy,
            0x03 => Self::Timeout,
            0x04 => Self::UnsupportedCommand,
            0x05 => Self::InvalidArgument,
            0x06 => Self::UnsupportedOnInterface,
            0x07 => Self::ChipNotFound,
            0x08 => Self::EccFailure,
            0x09 => Self::WriteProtected,
            0x0A => Self::VerifyFailed,
            _ => return None,
        })
    }

    /// Human-readable description, used in host error messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "device reported a generic error",
            Self::Busy => "device is busy",
            Self::Timeout => "device timed out talking to the chip",
            Self::UnsupportedCommand => "command not implemented by this firmware",
            Self::InvalidArgument => "device rejected the arguments",
            Self::UnsupportedOnInterface => "command not supported on the active interface",
            Self::ChipNotFound => "no chip responded",
            Self::EccFailure => "uncorrectable ECC error",
            Self::WriteProtected => "chip or device is write protected",
            Self::VerifyFailed => "verification after write failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_listed_commands_round_trip_through_their_byte() {
        for &cmd in Command::ALL {
            let byte = cmd as u8;
            assert_eq!(
                Command::from_u8_strict(byte),
                Some(cmd),
                "{cmd:?} (0x{byte:02X}) did not decode back to itself"
            );
        }
    }

    #[test]
    fn all_is_complete_and_has_no_duplicate_opcodes() {
        // Every byte that decodes must be present in ALL, and no two variants
        // may share an opcode. Together these catch a variant added to the enum
        // but forgotten in ALL, and a copy-paste opcode collision.
        let mut decoded = 0;
        for byte in 0..=u8::MAX {
            if let Some(cmd) = Command::from_u8_strict(byte) {
                decoded += 1;
                assert!(
                    Command::ALL.contains(&cmd),
                    "{cmd:?} decodes from 0x{byte:02X} but is missing from Command::ALL"
                );
            }
        }
        assert_eq!(
            decoded,
            Command::ALL.len(),
            "Command::ALL has entries that do not decode, or contains duplicates"
        );
    }

    /// `0x00` and `0xFF` are what a host reads from an idle, floating or
    /// disconnected bus. Neither may ever be a valid command, otherwise noise
    /// decodes as a real operation.
    #[test]
    fn idle_bus_values_are_not_commands() {
        assert_eq!(Command::from_u8(0x00), None);
        assert_eq!(Command::from_u8(0xFF), None);
        assert_eq!(Command::from_u8_strict(0x00), None);
        assert_eq!(Command::from_u8_strict(0xFF), None);
    }

    /// The host-side ranges must stay out of the device command space.
    #[test]
    fn host_side_ranges_are_reserved() {
        for byte in 0xB0..=0xDF {
            assert_eq!(
                Command::from_u8(byte),
                None,
                "0x{byte:02X} is reserved for host-side orchestration"
            );
        }
        for byte in 0xF0..=0xFF {
            assert_eq!(
                Command::from_u8(byte),
                None,
                "0x{byte:02X} is reserved for cloud/host-side use"
            );
        }
    }

    #[test]
    fn legacy_nand_aliases_are_accepted_loosely_and_rejected_strictly() {
        for (byte, expected) in LEGACY_NAND_ALIASES {
            assert_eq!(Command::from_u8(byte), Some(expected));
            assert_eq!(
                Command::from_u8_strict(byte),
                None,
                "0x{byte:02X} is a deprecated v1 alias and must not decode strictly"
            );
        }
    }

    #[test]
    fn commands_are_grouped_into_the_interface_their_range_implies() {
        assert_eq!(
            Command::NandReadPage.interface(),
            Some(FlashInterface::ParallelNand)
        );
        assert_eq!(
            Command::SpiNorRead.interface(),
            Some(FlashInterface::SpiNor)
        );
        assert_eq!(
            Command::EmmcReadBlock.interface(),
            Some(FlashInterface::Emmc)
        );
        assert_eq!(Command::UfsRead10.interface(), Some(FlashInterface::Ufs));
        assert_eq!(Command::Ping.interface(), None);
        assert_eq!(Command::PcbDetect.interface(), None);
    }

    /// A read-only command classified as destructive would block legitimate
    /// forensic reads; a destructive one classified as read-only would let the
    /// CLI erase a chip without asking. The second direction is the dangerous
    /// one, so it is checked by name: anything that writes, programs, erases or
    /// marks a block must be flagged.
    #[test]
    fn every_write_or_erase_command_is_flagged_destructive() {
        for &cmd in Command::ALL {
            let name = format!("{cmd:?}");
            let writes_something = name.contains("Write")
                || name.contains("Program")
                || name.contains("Erase")
                || name.contains("MarkBad");
            // WriteEnable/WriteDisable only toggle the write-enable latch; they
            // cannot change stored data on their own.
            let latch_only = matches!(
                cmd,
                Command::SpiNandWriteEnable
                    | Command::SpiNandWriteDisable
                    | Command::SpiNorWriteEnable
                    | Command::SpiNorWriteDisable
            );
            if writes_something && !latch_only {
                assert!(
                    cmd.is_destructive(),
                    "{cmd:?} modifies the chip but is_destructive() is false"
                );
            }
            if latch_only {
                assert!(
                    !cmd.is_destructive(),
                    "{cmd:?} only touches the write-enable latch"
                );
            }
        }
    }

    /// Commands that are destructive for reasons their name does not reveal.
    /// Listing them explicitly keeps the classification reviewable: anything
    /// flagged destructive is either an obvious write/erase or is named here
    /// with a reason.
    #[test]
    fn destructive_commands_beyond_the_obvious_ones_are_accounted_for() {
        // SpiNandSetFeature writes the configuration/protection registers, which
        // persists across power cycles on most parts. EmmcSetPartition switches
        // which partition subsequent writes land in, including boot and RPMB.
        let documented_extras = [Command::SpiNandSetFeature, Command::EmmcSetPartition];

        for &cmd in Command::ALL {
            if !cmd.is_destructive() {
                continue;
            }
            let name = format!("{cmd:?}");
            let obvious = name.contains("Write")
                || name.contains("Program")
                || name.contains("Erase")
                || name.contains("MarkBad");
            assert!(
                obvious || documented_extras.contains(&cmd),
                "{cmd:?} is flagged destructive but is neither an obvious write \
                 nor a documented exception"
            );
        }

        for cmd in documented_extras {
            assert!(cmd.is_destructive(), "{cmd:?} must stay flagged");
        }
    }

    #[test]
    fn interface_names_round_trip() {
        for &iface in FlashInterface::ALL {
            assert_eq!(FlashInterface::parse(iface.as_str()), Some(iface));
            assert_eq!(FlashInterface::from_u8(iface as u8), Some(iface));
        }
        assert_eq!(FlashInterface::from_u8(0x06), None);
        assert_eq!(FlashInterface::parse("floppy"), None);
    }

    #[test]
    fn status_codes_round_trip_and_ok_is_zero() {
        assert_eq!(Status::Ok as u8, 0x00);
        for byte in 0x00..=0x0A {
            let status = Status::from_u8(byte).expect("0x00..=0x0A are all defined");
            assert_eq!(status as u8, byte);
        }
        assert_eq!(Status::from_u8(0x0B), None);
    }
}
