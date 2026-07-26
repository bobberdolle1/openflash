//! Core library for the OpenFlash flash programmer.
//!
//! # Layers
//!
//! The modules that talk to hardware form a stack, and everything that performs
//! real I/O lives in it:
//!
//! - [`protocol`] re-exports the wire format shared with firmware.
//! - [`transport`] moves protocol frames over USB, TCP or a Unix socket.
//! - [`device`] turns frames into chip operations: identify, read, erase,
//!   program, verify.
//! - [`emulator`] is a device-side implementation of the protocol backed by a
//!   byte array, with real flash semantics, so the stack above it can be tested
//!   without hardware.
//!
//! The remaining modules analyse dumps that are already on disk ([`analysis`],
//! [`ai`], [`ai_advanced`], [`ecc`]) or describe chips and formats ([`onfi`],
//! [`spi_nor`], [`spi_nand`], [`emmc`], [`ufs`]).

pub mod ai;
pub mod ai_advanced;
pub mod analysis;
pub mod cloud;
pub mod device;
pub mod ecc;
pub mod emmc;
pub mod emulator;
pub mod hardware;
pub mod onfi;
pub mod protocol;
pub mod scripting;
pub mod server;
pub mod spi_nand;
pub mod spi_nor;
pub mod transport;
pub mod ufs;
pub mod write_ops;

pub use ai::*;
pub use analysis::*;
pub use ecc::*;
pub use onfi::*;
pub use protocol::*;

// `ProgramOptions`/`ProgramReport` are intentionally not re-exported here:
// `write_ops` already exports types by those names for its offline planning
// API. Reach the device-layer ones through `device::`.
pub use device::{DetectedChip, Device, DeviceError, DeviceResult, ReadReport};
pub use emulator::EmulatedDevice;
pub use transport::{Transport, TransportError, TransportKind, TransportResult};

// Re-export chip info types (avoid glob conflicts)
pub use ai_advanced::{
    AiAdvancedError,
    AiAdvancedResult,
    ArchiveFormat,
    ChipPrediction,
    CompressionFormat,
    CustomSignature,
    CvssScore,
    ExtractedFile,
    ExtractedSection,
    FeatureVector,
    FilesystemType,
    // Firmware Unpacking
    FirmwareUnpacker,
    // ML Chip Identification
    MlChipIdentifier,
    MlModelInfo,
    PatternType,
    // Rootfs Extraction
    RootfsExtractor,
    RootfsResult,
    Severity,
    SignatureCategory,
    // Custom Signatures
    SignatureDatabase,
    SignatureDatabaseInfo,
    SignatureMatch,
    UnpackResult,
    VulnScanResult,
    // Vulnerability Scanning
    VulnScanner,
    Vulnerability,
};
pub use emmc::{
    crc16, crc7, get_emmc_chip_info, get_emmc_manufacturer_name, parse_boot_size_from_ext_csd,
    parse_capacity_from_ext_csd, CardState, EmmcChipInfo, EmmcReadResult, ResponseType,
};
pub use hardware::{
    BgaProfile,
    BgaReworkStation,
    // BGA Rework
    BgaStationType,
    BusWidth,
    HardwareCommand,
    HardwareError,
    HardwareResult,
    JtagController,
    JtagDevice,
    // JTAG/SWD
    JtagState,
    LogicAnalyzer,
    LogicAnalyzerConfig,
    LogicAnalyzerState,
    LogicCapture,
    LogicChannel,
    OledDisplay,
    // OLED
    OledType,
    OpenFlashPcb,
    PcbCapabilities,
    // PCB
    PcbRevision,
    PcbStatus,
    SocketType,
    SwdController,
    // Logic Analyzer
    TriggerType,
    Tsop48Adapter,
    Tsop48PinMapping,
    // TSOP-48 Adapter
    Tsop48Pinout,
    VoltageLevel,
};
pub use scripting::{
    AnalysisOptions, AnomalyInfo, BatchJob, BatchJobConfig, BatchJobResult, BatchJobStatus,
    BatchJobType, BatchProcessor, ChipDetectionResult, CiArtifact, CiArtifactType, CiJobConfig,
    CiJobResult, CiOperation, CiOperationResult, CliCommand, CliConfig, CliOutputFormat,
    ConnectionConfig, ConnectionTarget, DeviceInfo, DumpResult, FilesystemInfo, KeyCandidate,
    OpenFlash, PatternInfo, PluginContext, PluginHook, PluginManager, PluginMetadata, PluginResult,
    ReadOptions, ReadStats, RecoverySuggestion, ReportFormat, ReportOptions, ScriptAnalysisResult,
    ScriptError, ScriptResult, WriteOptions,
};
pub use server::{
    // REST API
    AuthMethod,
    ChunkJob,
    ChunkStatus,
    DeviceCapabilities,
    DeviceInfo as ServerDeviceInfo,
    DeviceListResponse,
    DevicePlatform,
    DevicePool,
    // Device Pool
    DeviceStatus,
    // gRPC
    GrpcConfig,
    Job,
    // Job Queue
    JobPriority,
    JobQueue,
    JobResult,
    JobStatus,
    JobStatusResponse,
    JobType,
    OpenFlashServer,
    // Parallel Dumping
    ParallelDumpConfig,
    ParallelDumpJob,
    ParallelJobStatus,
    PassCriteria,
    PoolDevice,
    PoolStats,
    // Production Line
    ProductionLineConfig,
    ProductionLogging,
    ProductionStats,
    ProductionUnitResult,
    QueueStats,
    RateLimitConfig,
    RestApiConfig,
    // Server
    ServerConfig,
    ServerError,
    ServerInfo,
    ServerResult,
    StationConfig,
    StationOperation,
    SubmitJobRequest,
    SubmitJobResponse,
    VerificationMode,
    WebSocketConfig,
    // WebSocket
    WsMessage,
};
pub use spi_nand::{
    calculate_column_address, calculate_row_address, get_spi_nand_chip_info,
    get_spi_nand_manufacturer_name, EccStatus, SpiNandCellType, SpiNandChipInfo, SpiNandReadResult,
};
pub use spi_nor::{
    get_spi_nor_chip_info, get_spi_nor_manufacturer_name, FastReadSupport, ProtectionStatus,
    QuadEnableMethod, SfdpInfo, SfdpParser, SpiNorChipInfo, SpiNorError,
};
pub use ufs::{
    get_ufs_manufacturer_name, select_read_command, DeviceDescriptor, GeometryDescriptor,
    ReadCommandType, ScsiCdbBuilder, UfsDeviceInfo, UfsError, UfsLun, UfsVersion, UnitDescriptor,
};
pub use write_ops::{
    BackupMetadata, BadBlockEntry, BadBlockReason, BadBlockTable, BlockWearInfo, ChangeTracker,
    ChipCloner, ChipProgrammer, CloneMode, CloneOptions, ClonePhase, CloneProgress,
    ProgramOperation, ProgramOptions, ProgramProgress, WearLevelingManager, WearStatistics,
    WriteError, WriteResult,
};
// Cloud & Pro features (v3.0)
pub use cloud::{
    AiModelInfo, AiModelType, AiModelUpdate, AiUpdateConfig, AuthProvider, AuthToken,
    ChipContribution, ChipType, CloudCommand, CloudConfig, CloudError, CloudResult, CloudState,
    CommunityChipDatabase, ConflictResolution, ContributionStatus, ContributorStats,
    OpenFlashCloud, Organization, ProjectAccess, ProjectMember, SharedProject, SubscriptionTier,
    SupportTicket, SyncConfig, SyncItem, SyncItemType, SyncStatus, TeamMember, TeamRole,
    TicketMessage, TicketPriority, TicketStatus, TimingInfo, UserProfile, VerificationData,
};
