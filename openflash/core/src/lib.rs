pub mod onfi;
pub mod protocol;
pub mod ecc;
pub mod analysis;
pub mod spi_nand;
pub mod spi_nor;
pub mod emmc;
pub mod ufs;
pub mod ai;
pub mod write_ops;

pub use onfi::*;
pub use protocol::*;
pub use ecc::*;
pub use analysis::*;
pub use ai::*;

// Re-export chip info types (avoid glob conflicts)
pub use spi_nand::{
    SpiNandChipInfo, SpiNandCellType, SpiNandReadResult, EccStatus,
    get_spi_nand_chip_info, get_spi_nand_manufacturer_name,
    calculate_row_address, calculate_column_address,
};
pub use emmc::{
    EmmcChipInfo, EmmcReadResult, CardState, ResponseType,
    get_emmc_chip_info, get_emmc_manufacturer_name,
    parse_capacity_from_ext_csd, parse_boot_size_from_ext_csd,
    crc7, crc16,
};
pub use spi_nor::{
    SpiNorChipInfo, SpiNorError, ProtectionStatus, SfdpInfo, SfdpParser,
    QuadEnableMethod, FastReadSupport,
    get_spi_nor_chip_info, get_spi_nor_manufacturer_name,
};
pub use ufs::{
    UfsDeviceInfo, UfsVersion, UfsLun, UfsError,
    DeviceDescriptor, UnitDescriptor, GeometryDescriptor,
    ScsiCdbBuilder, ReadCommandType, select_read_command,
    get_ufs_manufacturer_name,
};
pub use write_ops::{
    WriteError, WriteResult,
    BadBlockTable, BadBlockEntry, BadBlockReason,
    WearLevelingManager, BlockWearInfo, WearStatistics,
    ChipProgrammer, ProgramOptions, ProgramProgress, ProgramOperation,
    ChangeTracker, BackupMetadata,
    ChipCloner, CloneOptions, CloneMode, CloneProgress, ClonePhase,
};

