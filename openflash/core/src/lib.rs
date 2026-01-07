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
pub mod scripting;
pub mod ai_advanced;
pub mod server;
pub mod hardware;

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
pub use scripting::{
    ScriptError, ScriptResult,
    ConnectionConfig, DeviceInfo, DeviceHandle,
    ReadOptions, WriteOptions, DumpResult, ReadStats,
    ChipDetectionResult, AnalysisOptions, ScriptAnalysisResult,
    PatternInfo, FilesystemInfo, AnomalyInfo, RecoverySuggestion, KeyCandidate,
    ReportFormat, ReportOptions,
    BatchJob, BatchJobType, BatchJobConfig, BatchJobStatus, BatchJobResult, BatchProcessor,
    PluginMetadata, PluginHook, PluginContext, PluginResult, PluginManager,
    CliCommand, CliOutputFormat, CliConfig,
    CiJobConfig, CiOperation, CiArtifact, CiArtifactType, CiJobResult, CiOperationResult,
    OpenFlash,
};
pub use ai_advanced::{
    AiAdvancedError, AiAdvancedResult,
    // ML Chip Identification
    MlChipIdentifier, ChipPrediction, FeatureVector, MlModelInfo,
    // Firmware Unpacking
    FirmwareUnpacker, UnpackResult, ExtractedSection, CompressionFormat, ArchiveFormat,
    // Rootfs Extraction
    RootfsExtractor, RootfsResult, ExtractedFile, FilesystemType,
    // Vulnerability Scanning
    VulnScanner, VulnScanResult, Vulnerability, CvssScore, Severity,
    // Custom Signatures
    SignatureDatabase, CustomSignature, SignatureMatch, SignatureCategory, PatternType,
    SignatureDatabaseInfo,
};
pub use server::{
    ServerError, ServerResult,
    // Device Pool
    DeviceStatus, DevicePlatform, DeviceCapabilities, PoolDevice, DevicePool, PoolStats,
    // Job Queue
    JobPriority, JobStatus, JobType, Job, JobResult, JobQueue, QueueStats,
    // REST API
    AuthMethod, RateLimitConfig, RestApiConfig,
    SubmitJobRequest, SubmitJobResponse, JobStatusResponse, DeviceListResponse,
    DeviceInfo as ServerDeviceInfo,
    // WebSocket
    WsMessage, WebSocketConfig,
    // gRPC
    GrpcConfig,
    // Server
    ServerConfig, OpenFlashServer, ServerInfo,
    // Parallel Dumping
    ParallelDumpConfig, ParallelDumpJob, ChunkJob, ChunkStatus, ParallelJobStatus,
    // Production Line
    ProductionLineConfig, StationConfig, StationOperation, PassCriteria,
    VerificationMode, ProductionLogging, ProductionUnitResult, ProductionStats,
};
pub use hardware::{
    HardwareError, HardwareResult, HardwareCommand,
    // PCB
    PcbRevision, SocketType, OpenFlashPcb, PcbCapabilities, PcbStatus,
    // TSOP-48 Adapter
    Tsop48Pinout, Tsop48Adapter, Tsop48PinMapping, VoltageLevel, BusWidth,
    // BGA Rework
    BgaStationType, BgaReworkStation, BgaProfile,
    // Logic Analyzer
    TriggerType, LogicChannel, LogicAnalyzerConfig, LogicCapture,
    LogicAnalyzer, LogicAnalyzerState,
    // JTAG/SWD
    JtagState, JtagDevice, JtagController, SwdController,
    // OLED
    OledType, OledDisplay,
};

