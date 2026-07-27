//! Scripting & Automation module for OpenFlash v1.8
//! Provides Python API bindings, CLI support, batch processing, and plugin system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use openflash_protocol::FlashInterface;

use crate::device::{Device, DeviceError, ProgramOptions as DeviceProgramOptions, ProgramReport};
use crate::emulator::EmulatedDevice;
#[cfg(unix)]
use crate::transport::UnixTransport;
#[cfg(feature = "usb")]
use crate::transport::UsbTransport;
use crate::transport::{TcpTransport, Transport, TransportKind};

// ============================================================================
// Error Types
// ============================================================================

/// Scripting module errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptError {
    /// Device connection failed
    ConnectionFailed(String),
    /// Device not connected
    NotConnected,
    /// Invalid operation
    InvalidOperation(String),
    /// Read operation failed
    ReadFailed { address: u64, reason: String },
    /// Write operation failed
    WriteFailed { address: u64, reason: String },
    /// Analysis failed
    AnalysisFailed(String),
    /// Plugin error
    PluginError { plugin: String, message: String },
    /// Batch processing error
    BatchError { job_id: usize, message: String },
    /// Script execution error
    ScriptExecutionError(String),
    /// Export failed
    ExportFailed(String),
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(s) => write!(f, "Connection failed: {}", s),
            Self::NotConnected => write!(f, "Device not connected"),
            Self::InvalidOperation(s) => write!(f, "Invalid operation: {}", s),
            Self::ReadFailed { address, reason } => {
                write!(f, "Read failed at 0x{:X}: {}", address, reason)
            }
            Self::WriteFailed { address, reason } => {
                write!(f, "Write failed at 0x{:X}: {}", address, reason)
            }
            Self::AnalysisFailed(s) => write!(f, "Analysis failed: {}", s),
            Self::PluginError { plugin, message } => {
                write!(f, "Plugin '{}' error: {}", plugin, message)
            }
            Self::BatchError { job_id, message } => {
                write!(f, "Batch job {} error: {}", job_id, message)
            }
            Self::ScriptExecutionError(s) => write!(f, "Script execution error: {}", s),
            Self::ExportFailed(s) => write!(f, "Export failed: {}", s),
            Self::InvalidConfig(s) => write!(f, "Invalid config: {}", s),
        }
    }
}

impl std::error::Error for ScriptError {}

pub type ScriptResult<T> = Result<T, ScriptError>;

// ============================================================================
// Device Connection API
// ============================================================================

/// Which device to connect to, and how long to wait for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionConfig {
    /// Where to look for the device.
    pub target: ConnectionTarget,
    /// How long to wait for each exchange, in milliseconds.
    pub timeout_ms: u32,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            target: ConnectionTarget::AutoUsb,
            timeout_ms: 5000,
        }
    }
}

impl ConnectionConfig {
    /// Connect to the in-process emulator with a chip of `size` bytes.
    pub fn emulated(size: usize) -> Self {
        Self {
            target: ConnectionTarget::Emulator { size },
            ..Self::default()
        }
    }
}

/// Connected device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device port
    pub port: String,
    /// Firmware version
    pub firmware_version: String,
    /// Hardware platform (RP2040, STM32F1, STM32F4, ESP32)
    pub platform: String,
    /// Serial number
    pub serial_number: String,
    /// Supported interfaces
    pub interfaces: Vec<String>,
}

// ============================================================================
// Read/Write Operations API
// ============================================================================

/// Read options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOptions {
    /// Start address
    pub start_address: u64,
    /// Length in bytes (None = full chip)
    pub length: Option<u64>,
    /// Include OOB/spare area
    pub include_oob: bool,
    /// Skip bad blocks
    pub skip_bad_blocks: bool,
    /// Progress callback interval (bytes)
    pub progress_interval: u64,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            start_address: 0,
            length: None,
            include_oob: false,
            skip_bad_blocks: true,
            progress_interval: 1024 * 1024, // 1MB
        }
    }
}

/// Write options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOptions {
    /// Start address
    pub start_address: u64,
    /// Verify after write
    pub verify: bool,
    /// Skip bad blocks
    pub skip_bad_blocks: bool,
    /// Erase before write
    pub erase_before_write: bool,
    /// Progress callback interval (bytes)
    pub progress_interval: u64,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            start_address: 0,
            verify: true,
            skip_bad_blocks: true,
            erase_before_write: true,
            progress_interval: 1024 * 1024,
        }
    }
}

/// Dump result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpResult {
    /// Raw data
    pub data: Vec<u8>,
    /// OOB data (if requested)
    pub oob_data: Option<Vec<u8>>,
    /// Bad blocks encountered
    pub bad_blocks: Vec<u32>,
    /// Read statistics
    pub stats: ReadStats,
}

/// Read statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadStats {
    /// Total bytes read
    pub bytes_read: u64,
    /// Pages read
    pub pages_read: u32,
    /// Blocks read
    pub blocks_read: u32,
    /// ECC corrections
    pub ecc_corrections: u32,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Transfer speed (bytes/sec)
    pub speed_bps: u64,
}

// ============================================================================
// Chip Detection API
// ============================================================================

/// Detected chip information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipDetectionResult {
    /// Chip manufacturer
    pub manufacturer: String,
    /// Chip model/part number
    pub model: String,
    /// Chip capacity in bytes
    pub capacity: u64,
    /// Page size in bytes
    pub page_size: u32,
    /// Block size in bytes
    pub block_size: u32,
    /// OOB size per page
    pub oob_size: u16,
    /// Raw ID bytes
    pub id_bytes: Vec<u8>,
    /// Interface type
    pub interface: String,
    /// Additional properties
    pub properties: HashMap<String, String>,
}

// ============================================================================
// AI Analysis API
// ============================================================================

/// AI analysis options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Enable deep scan (slower but more thorough)
    pub deep_scan: bool,
    /// OOB size for analysis
    pub oob_size: Option<u16>,
    /// Search for encryption keys
    pub search_keys: bool,
    /// Detect filesystems
    pub detect_filesystems: bool,
    /// Analyze wear leveling
    pub analyze_wear: bool,
    /// Generate memory map
    pub generate_memory_map: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            deep_scan: false,
            oob_size: None,
            search_keys: true,
            detect_filesystems: true,
            analyze_wear: true,
            generate_memory_map: true,
        }
    }
}

/// AI analysis result (simplified for scripting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptAnalysisResult {
    /// Data quality score (0.0 - 1.0)
    pub quality_score: f32,
    /// Encryption probability (0.0 - 1.0)
    pub encryption_probability: f32,
    /// Compression probability (0.0 - 1.0)
    pub compression_probability: f32,
    /// Detected patterns
    pub patterns: Vec<PatternInfo>,
    /// Detected filesystems
    pub filesystems: Vec<FilesystemInfo>,
    /// Detected anomalies
    pub anomalies: Vec<AnomalyInfo>,
    /// Recovery suggestions
    pub recovery_suggestions: Vec<RecoverySuggestion>,
    /// Key candidates (if search_keys enabled)
    pub key_candidates: Vec<KeyCandidate>,
    /// Summary text
    pub summary: String,
}

/// Pattern information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInfo {
    pub pattern_type: String,
    pub offset: u64,
    pub size: u64,
    pub confidence: f32,
}

/// Filesystem information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub fs_type: String,
    pub offset: u64,
    pub size: Option<u64>,
    pub confidence: f32,
}

/// Anomaly information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyInfo {
    pub anomaly_type: String,
    pub severity: String,
    pub offset: u64,
    pub description: String,
}

/// Recovery suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySuggestion {
    pub action: String,
    pub description: String,
    pub success_probability: f32,
    pub priority: u8,
}

/// Encryption key candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCandidate {
    pub key_type: String,
    pub offset: u64,
    pub key_data: Vec<u8>,
    pub confidence: f32,
}

// ============================================================================
// Report Export API
// ============================================================================

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    Markdown,
    Html,
    Json,
    Pdf,
}

/// Report options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOptions {
    /// Output format
    pub format: ReportFormat,
    /// Include hex dump samples
    pub include_hex_samples: bool,
    /// Include memory map visualization
    pub include_memory_map: bool,
    /// Include detailed pattern analysis
    pub include_patterns: bool,
    /// Maximum hex sample size
    pub max_hex_sample_size: usize,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            format: ReportFormat::Markdown,
            include_hex_samples: true,
            include_memory_map: true,
            include_patterns: true,
            max_hex_sample_size: 256,
        }
    }
}

// ============================================================================
// Batch Processing API
// ============================================================================

/// Batch job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJob {
    /// Job ID
    pub id: usize,
    /// Job name
    pub name: String,
    /// Job type
    pub job_type: BatchJobType,
    /// Job configuration
    pub config: BatchJobConfig,
    /// Dependencies (job IDs that must complete first)
    pub depends_on: Vec<usize>,
}

/// Batch job types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchJobType {
    /// Read/dump chip
    Read,
    /// Write/program chip
    Write,
    /// Erase chip
    Erase,
    /// Verify chip contents
    Verify,
    /// AI analysis
    Analyze,
    /// Clone chip-to-chip
    Clone,
    /// Export report
    ExportReport,
    /// Custom script
    CustomScript,
}

/// Batch job configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchJobConfig {
    /// Input file (for write operations)
    pub input_file: Option<String>,
    /// Output file (for read/export operations)
    pub output_file: Option<String>,
    /// Read options
    pub read_options: Option<ReadOptions>,
    /// Write options
    pub write_options: Option<WriteOptions>,
    /// Analysis options
    pub analysis_options: Option<AnalysisOptions>,
    /// Report options
    pub report_options: Option<ReportOptions>,
    /// Custom script path
    pub script_path: Option<String>,
    /// Custom parameters
    pub params: HashMap<String, String>,
}

/// Batch job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchJobStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Skipped,
}

/// Batch job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobResult {
    /// Job ID
    pub job_id: usize,
    /// Job status
    pub status: BatchJobStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Output data (if applicable)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Batch processor
#[derive(Debug, Clone)]
pub struct BatchProcessor {
    /// Jobs queue
    pub jobs: Vec<BatchJob>,
    /// Results
    pub results: Vec<BatchJobResult>,
    /// Stop on first error
    pub stop_on_error: bool,
    /// Parallel execution (for independent jobs)
    pub parallel: bool,
    /// Max parallel jobs
    pub max_parallel: usize,
}

impl BatchProcessor {
    /// Create new batch processor
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            results: Vec::new(),
            stop_on_error: false,
            parallel: false,
            max_parallel: 4,
        }
    }

    /// Add a job to the queue
    pub fn add_job(&mut self, job: BatchJob) -> usize {
        let id = self.jobs.len();
        self.jobs.push(job);
        id
    }

    /// Add a read job
    pub fn add_read_job(&mut self, name: &str, output_file: &str, options: ReadOptions) -> usize {
        let job = BatchJob {
            id: self.jobs.len(),
            name: name.to_string(),
            job_type: BatchJobType::Read,
            config: BatchJobConfig {
                output_file: Some(output_file.to_string()),
                read_options: Some(options),
                ..Default::default()
            },
            depends_on: Vec::new(),
        };
        self.add_job(job)
    }

    /// Add a write job
    pub fn add_write_job(&mut self, name: &str, input_file: &str, options: WriteOptions) -> usize {
        let job = BatchJob {
            id: self.jobs.len(),
            name: name.to_string(),
            job_type: BatchJobType::Write,
            config: BatchJobConfig {
                input_file: Some(input_file.to_string()),
                write_options: Some(options),
                ..Default::default()
            },
            depends_on: Vec::new(),
        };
        self.add_job(job)
    }

    /// Add an analysis job
    pub fn add_analysis_job(
        &mut self,
        name: &str,
        depends_on: usize,
        options: AnalysisOptions,
    ) -> usize {
        let job = BatchJob {
            id: self.jobs.len(),
            name: name.to_string(),
            job_type: BatchJobType::Analyze,
            config: BatchJobConfig {
                analysis_options: Some(options),
                ..Default::default()
            },
            depends_on: vec![depends_on],
        };
        self.add_job(job)
    }

    /// Add a report export job
    pub fn add_report_job(
        &mut self,
        name: &str,
        output_file: &str,
        depends_on: usize,
        options: ReportOptions,
    ) -> usize {
        let job = BatchJob {
            id: self.jobs.len(),
            name: name.to_string(),
            job_type: BatchJobType::ExportReport,
            config: BatchJobConfig {
                output_file: Some(output_file.to_string()),
                report_options: Some(options),
                ..Default::default()
            },
            depends_on: vec![depends_on],
        };
        self.add_job(job)
    }

    /// Get job count
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Get completed job count
    pub fn completed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.status, BatchJobStatus::Completed))
            .count()
    }

    /// Get failed job count
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.status, BatchJobStatus::Failed(_)))
            .count()
    }

    /// Check if all jobs completed successfully
    pub fn all_succeeded(&self) -> bool {
        self.results.len() == self.jobs.len() && self.failed_count() == 0
    }

    /// Set stop on error behavior
    pub fn with_stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Enable parallel execution
    pub fn with_parallel(mut self, parallel: bool, max: usize) -> Self {
        self.parallel = parallel;
        self.max_parallel = max;
        self
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Plugin System API
// ============================================================================

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin author
    pub author: String,
    /// Plugin description
    pub description: String,
    /// Supported hooks
    pub hooks: Vec<PluginHook>,
    /// Required OpenFlash version
    pub min_openflash_version: String,
}

/// Plugin hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginHook {
    /// Called before read operation
    PreRead,
    /// Called after read operation
    PostRead,
    /// Called before write operation
    PreWrite,
    /// Called after write operation
    PostWrite,
    /// Called during analysis
    Analysis,
    /// Called for custom pattern detection
    PatternDetection,
    /// Called for custom filesystem detection
    FilesystemDetection,
    /// Called for report generation
    ReportGeneration,
}

/// Plugin context passed to plugin functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// Current operation
    pub operation: String,
    /// Data buffer (for read/write hooks)
    pub data: Option<Vec<u8>>,
    /// Chip information
    pub chip_info: Option<ChipDetectionResult>,
    /// Analysis result (for analysis hooks)
    pub analysis: Option<ScriptAnalysisResult>,
    /// Custom parameters
    pub params: HashMap<String, String>,
}

/// Plugin result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// Success flag
    pub success: bool,
    /// Modified data (if applicable)
    pub data: Option<Vec<u8>>,
    /// Additional patterns detected
    pub patterns: Vec<PatternInfo>,
    /// Additional filesystems detected
    pub filesystems: Vec<FilesystemInfo>,
    /// Messages/logs
    pub messages: Vec<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Plugin manager
#[derive(Debug, Clone)]
pub struct PluginManager {
    /// Loaded plugins
    pub plugins: Vec<PluginMetadata>,
    /// Plugin search paths
    pub search_paths: Vec<String>,
    /// Enabled plugins
    pub enabled: Vec<String>,
}

impl PluginManager {
    /// Create new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            search_paths: vec![
                "~/.openflash/plugins".to_string(),
                "/usr/share/openflash/plugins".to_string(),
            ],
            enabled: Vec::new(),
        }
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: &str) {
        self.search_paths.push(path.to_string());
    }

    /// Register a plugin
    pub fn register(&mut self, metadata: PluginMetadata) {
        self.plugins.push(metadata);
    }

    /// Enable a plugin
    pub fn enable(&mut self, name: &str) -> ScriptResult<()> {
        if self.plugins.iter().any(|p| p.name == name) {
            if !self.enabled.contains(&name.to_string()) {
                self.enabled.push(name.to_string());
            }
            Ok(())
        } else {
            Err(ScriptError::PluginError {
                plugin: name.to_string(),
                message: "Plugin not found".to_string(),
            })
        }
    }

    /// Disable a plugin
    pub fn disable(&mut self, name: &str) {
        self.enabled.retain(|n| n != name);
    }

    /// Get plugins for a specific hook
    pub fn get_plugins_for_hook(&self, hook: &PluginHook) -> Vec<&PluginMetadata> {
        self.plugins
            .iter()
            .filter(|p| self.enabled.contains(&p.name) && p.hooks.contains(hook))
            .collect()
    }

    /// List all plugins
    pub fn list_plugins(&self) -> &[PluginMetadata] {
        &self.plugins
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled.contains(&name.to_string())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CLI Support Types
// ============================================================================

/// CLI command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CliCommand {
    /// Scan for devices
    Scan,
    /// Connect to device
    Connect { port: Option<String> },
    /// Detect chip
    Detect,
    /// Read/dump chip
    Read {
        output: String,
        start: Option<u64>,
        length: Option<u64>,
        include_oob: bool,
    },
    /// Write/program chip
    Write {
        input: String,
        start: Option<u64>,
        verify: bool,
        erase: bool,
    },
    /// Erase chip
    Erase {
        start: Option<u64>,
        length: Option<u64>,
    },
    /// Verify chip contents
    Verify { file: String },
    /// AI analysis
    Analyze {
        input: Option<String>,
        output: Option<String>,
        deep: bool,
    },
    /// Compare two dumps
    Compare { file1: String, file2: String },
    /// Export report
    Report {
        input: String,
        output: String,
        format: String,
    },
    /// Clone chip-to-chip
    Clone { mode: String },
    /// Run batch file
    Batch { file: String },
    /// Run script
    Script { file: String },
    /// List supported chips
    ListChips { interface: Option<String> },
    /// Show device info
    Info,
    /// Show version
    Version,
}

/// CLI output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CliOutputFormat {
    Text,
    Json,
    Csv,
    Table,
}

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Output format
    pub output_format: CliOutputFormat,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode (minimal output)
    pub quiet: bool,
    /// Color output
    pub color: bool,
    /// Progress bar
    pub progress: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            output_format: CliOutputFormat::Text,
            verbose: false,
            quiet: false,
            color: true,
            progress: true,
        }
    }
}

// ============================================================================
// CI/CD Integration Types
// ============================================================================

/// CI/CD job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJobConfig {
    /// Job name
    pub name: String,
    /// Device port (or auto-detect)
    pub device: Option<String>,
    /// Expected chip (for verification)
    pub expected_chip: Option<String>,
    /// Operations to perform
    pub operations: Vec<CiOperation>,
    /// Artifacts to save
    pub artifacts: Vec<CiArtifact>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Retry count
    pub retries: u32,
}

/// CI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CiOperation {
    /// Verify device connection
    VerifyConnection,
    /// Verify chip type
    VerifyChip { expected: String },
    /// Read and save dump
    ReadDump { output: String },
    /// Write firmware
    WriteFirmware { input: String, verify: bool },
    /// Run analysis
    RunAnalysis { output: String },
    /// Compare with golden image
    CompareGolden { golden: String, tolerance: f32 },
    /// Custom command
    Custom { command: String, args: Vec<String> },
}

/// CI artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiArtifact {
    /// Artifact name
    pub name: String,
    /// File path
    pub path: String,
    /// Artifact type
    pub artifact_type: CiArtifactType,
}

/// CI artifact types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CiArtifactType {
    Dump,
    Report,
    Log,
    Analysis,
}

/// CI job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJobResult {
    /// Job name
    pub job_name: String,
    /// Success flag
    pub success: bool,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Operation results
    pub operation_results: Vec<CiOperationResult>,
    /// Artifacts produced
    pub artifacts: Vec<CiArtifact>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// CI operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiOperationResult {
    /// Operation name
    pub operation: String,
    /// Success flag
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Output message
    pub message: String,
}

// ============================================================================
// High-Level API (Python-like interface)
// ============================================================================

/// Where to look for a device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionTarget {
    /// Find the single connected USB device.
    #[default]
    AutoUsb,
    /// A USB device by serial number or `bus-address`.
    Usb(String),
    /// An SBC agent at `host:port`.
    Tcp(String),
    /// An SBC agent listening on a Unix socket.
    Unix(String),
    /// The in-process emulator, backed by a chip of the given size in bytes.
    ///
    /// Nothing real is touched. Every caller that offers this must label its
    /// output as emulated.
    Emulator {
        /// Emulated chip size in bytes; must be a multiple of 4096.
        size: usize,
    },
}

/// The high-level API, mirroring the Python bindings.
///
/// Every method here performs real I/O against a real device. There is no mode
/// in which a read returns invented bytes or a write reports success without
/// having written: when there is no device, `connect` fails.
pub struct OpenFlash {
    device: Option<Device<Box<dyn Transport>>>,
    info: Option<DeviceInfo>,
    plugins: PluginManager,
    last_dump: Option<DumpResult>,
    last_analysis: Option<ScriptAnalysisResult>,
}

impl std::fmt::Debug for OpenFlash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenFlash")
            .field("connected", &self.is_connected())
            .field("device", &self.info)
            .finish_non_exhaustive()
    }
}

fn to_script_error(error: DeviceError) -> ScriptError {
    match error {
        DeviceError::RangeOutOfBounds { start, .. } => ScriptError::ReadFailed {
            address: start,
            reason: "range does not fit on the chip".to_string(),
        },
        DeviceError::VerificationFailed { offset, .. } => ScriptError::WriteFailed {
            address: offset,
            reason: "verification failed".to_string(),
        },
        other => ScriptError::InvalidOperation(other.to_string()),
    }
}

impl OpenFlash {
    /// Create an unconnected instance.
    pub fn new() -> Self {
        Self {
            device: None,
            info: None,
            plugins: PluginManager::new(),
            last_dump: None,
            last_analysis: None,
        }
    }

    /// Connect to the single attached USB device.
    pub fn connect(&mut self) -> ScriptResult<&DeviceInfo> {
        self.connect_with_config(ConnectionConfig::default())
    }

    /// Connect to the device named by `config`.
    pub fn connect_with_config(&mut self, config: ConnectionConfig) -> ScriptResult<&DeviceInfo> {
        let transport = open_transport(&config)?;
        let device = Device::connect(transport).map_err(|e| {
            ScriptError::ConnectionFailed(format!("handshake with the device failed: {e}"))
        })?;

        let version = *device.version();
        let kind = device.kind();
        self.info = Some(DeviceInfo {
            port: kind.to_string(),
            firmware_version: format!(
                "{}.{}.{}",
                version.firmware.0, version.firmware.1, version.firmware.2
            ),
            platform: version
                .platform
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            serial_number: match &kind {
                TransportKind::Usb { address } => address.clone(),
                other => other.to_string(),
            },
            interfaces: FlashInterface::ALL
                .iter()
                .filter(|iface| version.supports(**iface))
                .map(|iface| iface.as_str().to_string())
                .collect(),
        });
        self.device = Some(device);

        Ok(self.info.as_ref().expect("just set"))
    }

    /// Close the connection.
    pub fn disconnect(&mut self) {
        self.device = None;
        self.info = None;
    }

    /// Whether a device is currently open.
    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    /// Information about the connected device.
    pub fn device_info(&self) -> Option<&DeviceInfo> {
        self.info.as_ref()
    }

    /// Refuse every operation that would modify the chip.
    pub fn set_read_only(&mut self, read_only: bool) -> ScriptResult<()> {
        self.device_mut()?.set_read_only(read_only);
        Ok(())
    }

    fn device_mut(&mut self) -> ScriptResult<&mut Device<Box<dyn Transport>>> {
        self.device.as_mut().ok_or(ScriptError::NotConnected)
    }

    /// Read the chip id off the bus and look it up.
    pub fn detect_chip(&mut self) -> ScriptResult<ChipDetectionResult> {
        let chip = self.device_mut()?.identify().map_err(to_script_error)?;

        let mut properties = HashMap::new();
        properties.insert("exact_match".to_string(), chip.exact_match.to_string());
        properties.insert("sector_size".to_string(), chip.sector_size.to_string());

        Ok(ChipDetectionResult {
            manufacturer: chip.manufacturer,
            model: chip.model,
            capacity: chip.capacity,
            page_size: chip.page_size,
            block_size: chip.sector_size,
            // SPI NOR has no spare area; a NAND interface will report one here
            // once the device layer covers NAND.
            oob_size: 0,
            id_bytes: chip.jedec_id.to_vec(),
            interface: FlashInterface::SpiNor.as_str().to_string(),
            properties,
        })
    }

    /// Dump the whole chip.
    pub fn read_full(&mut self) -> ScriptResult<&DumpResult> {
        self.read_with_options(ReadOptions::default())
    }

    /// Dump part of the chip.
    pub fn read_with_options(&mut self, options: ReadOptions) -> ScriptResult<&DumpResult> {
        let chip = self.detect_chip()?;
        let length = options.length.unwrap_or(chip.capacity);
        let device = self.device_mut()?;

        let mut data = Vec::new();
        let report = device
            .read_into(options.start_address, length, &mut data, None)
            .map_err(to_script_error)?;

        if options.include_oob {
            return Err(ScriptError::InvalidOperation(
                "the spare (OOB) area only exists on NAND, which this interface does not \
                 expose"
                    .to_string(),
            ));
        }

        let page_size = chip.page_size.max(1) as u64;
        let block_size = chip.block_size.max(1) as u64;
        self.last_dump = Some(DumpResult {
            data,
            oob_data: None,
            bad_blocks: Vec::new(),
            stats: ReadStats {
                bytes_read: report.bytes_read,
                pages_read: (report.bytes_read / page_size) as u32,
                blocks_read: (report.bytes_read / block_size) as u32,
                ecc_corrections: 0,
                duration_ms: report.duration.as_millis() as u64,
                speed_bps: report.bytes_per_second().unwrap_or(0),
            },
        });
        Ok(self.last_dump.as_ref().expect("just set"))
    }

    /// Write `data` at `options.start_address`, erasing first by default.
    pub fn write(&mut self, data: &[u8], options: WriteOptions) -> ScriptResult<ProgramReport> {
        let device = self.device_mut()?;
        device
            .program(
                options.start_address,
                data,
                DeviceProgramOptions {
                    erase_first: options.erase_before_write,
                    verify: options.verify,
                    skip_blank_pages: true,
                },
                None,
            )
            .map_err(to_script_error)
    }

    /// Erase whole sectors covering the range, returning the sector count.
    pub fn erase(&mut self, start: u64, length: u64) -> ScriptResult<u64> {
        self.device_mut()?
            .erase_range(start, length)
            .map_err(to_script_error)
    }

    /// Read the chip back and compare it with `expected`.
    pub fn verify(&mut self, start: u64, expected: &[u8]) -> ScriptResult<()> {
        self.device_mut()?
            .verify(start, expected, None)
            .map_err(to_script_error)
    }

    /// The most recent dump.
    pub fn last_dump(&self) -> Option<&DumpResult> {
        self.last_dump.as_ref()
    }

    /// The most recent analysis result.
    pub fn last_analysis(&self) -> Option<&ScriptAnalysisResult> {
        self.last_analysis.as_ref()
    }

    /// The plugin manager.
    pub fn plugins(&mut self) -> &mut PluginManager {
        &mut self.plugins
    }

    /// Create a batch processor.
    pub fn batch(&self) -> BatchProcessor {
        BatchProcessor::new()
    }
}

/// Open the transport described by `config`.
fn open_transport(config: &ConnectionConfig) -> ScriptResult<Box<dyn Transport>> {
    let timeout = std::time::Duration::from_millis(u64::from(config.timeout_ms).max(100));

    match &config.target {
        ConnectionTarget::Emulator { size } => Ok(Box::new(EmulatedDevice::new(
            "EMULATED",
            [0xEF, 0x40, 0x15],
            *size,
        ))),
        ConnectionTarget::Tcp(endpoint) => TcpTransport::connect(endpoint, timeout)
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| ScriptError::ConnectionFailed(e.to_string())),
        #[cfg(unix)]
        ConnectionTarget::Unix(path) => UnixTransport::connect(path)
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| ScriptError::ConnectionFailed(e.to_string())),
        #[cfg(not(unix))]
        ConnectionTarget::Unix(_) => Err(ScriptError::ConnectionFailed(
            "Unix sockets are not available on this platform".to_string(),
        )),
        #[cfg(feature = "usb")]
        ConnectionTarget::AutoUsb => UsbTransport::open_only()
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| ScriptError::ConnectionFailed(e.to_string())),
        #[cfg(feature = "usb")]
        ConnectionTarget::Usb(selector) => UsbTransport::open(selector)
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| ScriptError::ConnectionFailed(e.to_string())),
        #[cfg(not(feature = "usb"))]
        ConnectionTarget::AutoUsb | ConnectionTarget::Usb(_) => Err(ScriptError::ConnectionFailed(
            "this build was compiled without USB support".to_string(),
        )),
    }
}

impl Default for OpenFlash {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_config_defaults_to_the_single_usb_device() {
        let config = ConnectionConfig::default();
        assert_eq!(config.target, ConnectionTarget::AutoUsb);
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn connecting_to_the_emulator_reports_a_real_handshake() {
        let mut of = OpenFlash::new();
        assert!(!of.is_connected());

        let info = of
            .connect_with_config(ConnectionConfig::emulated(64 * 1024))
            .expect("the emulator always answers");
        assert!(info.port.starts_with("emulator:"));
        assert_eq!(info.interfaces, vec!["spi-nor".to_string()]);
        assert!(of.is_connected());

        of.disconnect();
        assert!(!of.is_connected());
    }

    /// The API must refuse to work rather than invent data when nothing is
    /// connected. This is the behaviour the mock implementation used to break.
    #[test]
    fn operations_without_a_connection_fail() {
        let mut of = OpenFlash::new();
        assert!(matches!(of.detect_chip(), Err(ScriptError::NotConnected)));
        assert!(matches!(of.read_full(), Err(ScriptError::NotConnected)));
        assert!(matches!(of.erase(0, 4096), Err(ScriptError::NotConnected)));
        assert!(matches!(
            of.write(&[0u8; 4], WriteOptions::default()),
            Err(ScriptError::NotConnected)
        ));
    }

    #[test]
    fn detect_read_write_and_verify_round_trip_against_the_emulator() {
        let mut of = OpenFlash::new();
        of.connect_with_config(ConnectionConfig::emulated(64 * 1024))
            .unwrap();

        let chip = of.detect_chip().unwrap();
        // 0x10 is the capacity byte for 2^16 = 64 KiB, the size asked for above:
        // the emulator derives it from its array so the id cannot disagree with
        // how much memory there is.
        assert_eq!(chip.id_bytes, vec![0xEF, 0x40, 0x10]);
        assert_eq!(chip.capacity, 64 * 1024);

        let payload: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();
        let report = of.write(&payload, WriteOptions::default()).unwrap();
        assert!(report.verified);

        of.verify(0, &payload).unwrap();

        let dump = of
            .read_with_options(ReadOptions {
                length: Some(payload.len() as u64),
                ..ReadOptions::default()
            })
            .unwrap();
        assert_eq!(dump.data, payload);
        assert_eq!(dump.stats.bytes_read, payload.len() as u64);
    }

    #[test]
    fn a_read_only_session_refuses_to_write() {
        let mut of = OpenFlash::new();
        of.connect_with_config(ConnectionConfig::emulated(64 * 1024))
            .unwrap();
        of.set_read_only(true).unwrap();

        assert!(of.write(&[0x00; 16], WriteOptions::default()).is_err());
        // Reading is still allowed.
        assert!(of
            .read_with_options(ReadOptions {
                length: Some(16),
                ..ReadOptions::default()
            })
            .is_ok());
    }

    #[test]
    fn test_batch_processor() {
        let mut batch = BatchProcessor::new();

        let id1 = batch.add_read_job("Read chip", "dump.bin", ReadOptions::default());
        let id2 = batch.add_analysis_job("Analyze", id1, AnalysisOptions::default());
        let _id3 = batch.add_report_job("Report", "report.md", id2, ReportOptions::default());

        assert_eq!(batch.job_count(), 3);
        assert_eq!(batch.jobs[1].depends_on, vec![id1]);
        assert_eq!(batch.jobs[2].depends_on, vec![id2]);
    }

    #[test]
    fn test_plugin_manager() {
        let mut pm = PluginManager::new();

        let plugin = PluginMetadata {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test plugin".to_string(),
            hooks: vec![PluginHook::PostRead, PluginHook::Analysis],
            min_openflash_version: "1.8.0".to_string(),
        };

        pm.register(plugin);
        assert_eq!(pm.list_plugins().len(), 1);

        assert!(pm.enable("test-plugin").is_ok());
        assert!(pm.is_enabled("test-plugin"));

        let hooks = pm.get_plugins_for_hook(&PluginHook::Analysis);
        assert_eq!(hooks.len(), 1);

        pm.disable("test-plugin");
        assert!(!pm.is_enabled("test-plugin"));
    }

    #[test]
    fn test_read_options_default() {
        let opts = ReadOptions::default();
        assert_eq!(opts.start_address, 0);
        assert!(opts.length.is_none());
        assert!(!opts.include_oob);
        assert!(opts.skip_bad_blocks);
    }

    #[test]
    fn test_write_options_default() {
        let opts = WriteOptions::default();
        assert!(opts.verify);
        assert!(opts.skip_bad_blocks);
        assert!(opts.erase_before_write);
    }

    #[test]
    fn test_analysis_options_default() {
        let opts = AnalysisOptions::default();
        assert!(!opts.deep_scan);
        assert!(opts.search_keys);
        assert!(opts.detect_filesystems);
    }

    #[test]
    fn test_report_format() {
        let opts = ReportOptions::default();
        assert_eq!(opts.format, ReportFormat::Markdown);
        assert!(opts.include_hex_samples);
    }

    #[test]
    fn test_cli_config_default() {
        let config = CliConfig::default();
        assert_eq!(config.output_format, CliOutputFormat::Text);
        assert!(!config.verbose);
        assert!(config.color);
        assert!(config.progress);
    }

    #[test]
    fn test_ci_job_config() {
        let config = CiJobConfig {
            name: "test-job".to_string(),
            device: None,
            expected_chip: Some("K9F1G08U0E".to_string()),
            operations: vec![
                CiOperation::VerifyConnection,
                CiOperation::ReadDump {
                    output: "dump.bin".to_string(),
                },
            ],
            artifacts: vec![],
            timeout_secs: 300,
            retries: 3,
        };

        assert_eq!(config.operations.len(), 2);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_chip_detection_not_connected() {
        let mut of = OpenFlash::new();
        let result = of.detect_chip();
        assert!(matches!(result, Err(ScriptError::NotConnected)));
    }

    #[test]
    fn test_read_not_connected() {
        let mut of = OpenFlash::new();
        let result = of.read_full();
        assert!(matches!(result, Err(ScriptError::NotConnected)));
    }
}
