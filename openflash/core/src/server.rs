//! Multi-device & Enterprise Server module for OpenFlash v2.0
//! Provides REST API, WebSocket, gRPC interfaces, device farm management,
//! parallel dumping, and production line integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Error Types
// ============================================================================

/// Server module errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerError {
    /// Device not found
    DeviceNotFound(String),
    /// Device busy
    DeviceBusy(String),
    /// Device offline
    DeviceOffline(String),
    /// Job not found
    JobNotFound(u64),
    /// Job failed
    JobFailed { job_id: u64, reason: String },
    /// Queue full
    QueueFull,
    /// Invalid configuration
    InvalidConfig(String),
    /// Authentication failed
    AuthFailed(String),
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Connection failed
    ConnectionFailed(String),
    /// Timeout
    Timeout(String),
    /// Internal server error
    InternalError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceNotFound(id) => write!(f, "Device not found: {}", id),
            Self::DeviceBusy(id) => write!(f, "Device busy: {}", id),
            Self::DeviceOffline(id) => write!(f, "Device offline: {}", id),
            Self::JobNotFound(id) => write!(f, "Job not found: {}", id),
            Self::JobFailed { job_id, reason } => write!(f, "Job {} failed: {}", job_id, reason),
            Self::QueueFull => write!(f, "Job queue is full"),
            Self::InvalidConfig(s) => write!(f, "Invalid configuration: {}", s),
            Self::AuthFailed(s) => write!(f, "Authentication failed: {}", s),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Self::ConnectionFailed(s) => write!(f, "Connection failed: {}", s),
            Self::Timeout(s) => write!(f, "Timeout: {}", s),
            Self::InternalError(s) => write!(f, "Internal error: {}", s),
        }
    }
}

impl std::error::Error for ServerError {}

pub type ServerResult<T> = Result<T, ServerError>;

// ============================================================================
// Device Pool Management
// ============================================================================

/// Device status in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    /// Device is online and available
    Available,
    /// Device is currently processing a job
    Busy,
    /// Device is offline
    Offline,
    /// Device has an error
    Error,
    /// Device is in maintenance mode
    Maintenance,
    /// Device is reserved for specific user/job
    Reserved,
}

/// Device platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevicePlatform {
    RP2040,
    STM32F1,
    STM32F4,
    ESP32,
    ESP32S3,
    Unknown,
}

impl DevicePlatform {
    // Not `std::str::FromStr`: parsing is infallible here, an unrecognised name
    // maps to `Unknown` rather than an error, and the name is public API.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "RP2040" => Self::RP2040,
            "STM32F1" | "STM32F103" => Self::STM32F1,
            "STM32F4" | "STM32F401" | "STM32F411" | "STM32F446" => Self::STM32F4,
            "ESP32" => Self::ESP32,
            "ESP32S3" | "ESP32-S3" => Self::ESP32S3,
            _ => Self::Unknown,
        }
    }
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Supported flash interfaces
    pub interfaces: Vec<String>,
    /// Maximum transfer speed (bytes/sec)
    pub max_speed: u64,
    /// Has WiFi connectivity
    pub has_wifi: bool,
    /// Has Bluetooth connectivity
    pub has_bluetooth: bool,
    /// Supports parallel operations
    pub parallel_ops: bool,
    /// Maximum concurrent operations
    pub max_concurrent: u8,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            interfaces: vec!["parallel_nand".to_string(), "spi_nand".to_string()],
            max_speed: 1_000_000,
            has_wifi: false,
            has_bluetooth: false,
            parallel_ops: false,
            max_concurrent: 1,
        }
    }
}

/// Device in the pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDevice {
    /// Unique device ID
    pub id: String,
    /// Device name/label
    pub name: String,
    /// Connection URI (serial://, tcp://, ws://)
    pub uri: String,
    /// Device platform
    pub platform: DevicePlatform,
    /// Firmware version
    pub firmware_version: String,
    /// Current status
    pub status: DeviceStatus,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// Current job ID (if busy)
    pub current_job: Option<u64>,
    /// Last seen timestamp (Unix epoch ms)
    pub last_seen: u64,
    /// Total jobs completed
    pub jobs_completed: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Error count
    pub error_count: u32,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl PoolDevice {
    /// Create a new pool device
    pub fn new(id: &str, name: &str, uri: &str, platform: DevicePlatform) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            uri: uri.to_string(),
            platform,
            firmware_version: "2.0.0".to_string(),
            status: DeviceStatus::Offline,
            capabilities: DeviceCapabilities::default(),
            current_job: None,
            last_seen: 0,
            jobs_completed: 0,
            bytes_processed: 0,
            error_count: 0,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Check if device is available for jobs
    pub fn is_available(&self) -> bool {
        self.status == DeviceStatus::Available
    }

    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Mark device as busy with a job
    pub fn assign_job(&mut self, job_id: u64) {
        self.status = DeviceStatus::Busy;
        self.current_job = Some(job_id);
    }

    /// Release device from job
    pub fn release(&mut self, success: bool, bytes: u64) {
        self.status = DeviceStatus::Available;
        self.current_job = None;
        if success {
            self.jobs_completed += 1;
            self.bytes_processed += bytes;
        } else {
            self.error_count += 1;
        }
    }
}

/// Device pool manager
#[derive(Debug, Clone)]
pub struct DevicePool {
    /// All devices in the pool
    pub devices: HashMap<String, PoolDevice>,
    /// Maximum devices allowed
    pub max_devices: usize,
    /// Health check interval (seconds)
    pub health_check_interval: u64,
    /// Device timeout (seconds)
    pub device_timeout: u64,
}

impl DevicePool {
    /// Create a new device pool
    pub fn new(max_devices: usize) -> Self {
        Self {
            devices: HashMap::new(),
            max_devices,
            health_check_interval: 30,
            device_timeout: 60,
        }
    }

    /// Add a device to the pool
    pub fn add_device(&mut self, device: PoolDevice) -> ServerResult<()> {
        if self.devices.len() >= self.max_devices {
            return Err(ServerError::InvalidConfig(format!(
                "Maximum devices ({}) reached",
                self.max_devices
            )));
        }
        self.devices.insert(device.id.clone(), device);
        Ok(())
    }

    /// Remove a device from the pool
    pub fn remove_device(&mut self, device_id: &str) -> ServerResult<PoolDevice> {
        self.devices
            .remove(device_id)
            .ok_or_else(|| ServerError::DeviceNotFound(device_id.to_string()))
    }

    /// Get a device by ID
    pub fn get_device(&self, device_id: &str) -> Option<&PoolDevice> {
        self.devices.get(device_id)
    }

    /// Get a mutable device by ID
    pub fn get_device_mut(&mut self, device_id: &str) -> Option<&mut PoolDevice> {
        self.devices.get_mut(device_id)
    }

    /// Get all available devices
    pub fn available_devices(&self) -> Vec<&PoolDevice> {
        self.devices.values().filter(|d| d.is_available()).collect()
    }

    /// Get devices by tag
    pub fn devices_by_tag(&self, tag: &str) -> Vec<&PoolDevice> {
        self.devices
            .values()
            .filter(|d| d.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get devices by platform
    pub fn devices_by_platform(&self, platform: DevicePlatform) -> Vec<&PoolDevice> {
        self.devices
            .values()
            .filter(|d| d.platform == platform)
            .collect()
    }

    /// Find best available device for a job
    pub fn find_best_device(&self, required_interface: Option<&str>) -> Option<&PoolDevice> {
        self.devices
            .values()
            .filter(|d| d.is_available())
            .filter(|d| {
                required_interface
                    .map(|iface| d.capabilities.interfaces.contains(&iface.to_string()))
                    .unwrap_or(true)
            })
            .min_by_key(|d| d.error_count)
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let total = self.devices.len();
        let available = self.devices.values().filter(|d| d.is_available()).count();
        let busy = self
            .devices
            .values()
            .filter(|d| d.status == DeviceStatus::Busy)
            .count();
        let offline = self
            .devices
            .values()
            .filter(|d| d.status == DeviceStatus::Offline)
            .count();
        let error = self
            .devices
            .values()
            .filter(|d| d.status == DeviceStatus::Error)
            .count();
        let total_jobs: u64 = self.devices.values().map(|d| d.jobs_completed).sum();
        let total_bytes: u64 = self.devices.values().map(|d| d.bytes_processed).sum();

        PoolStats {
            total_devices: total,
            available_devices: available,
            busy_devices: busy,
            offline_devices: offline,
            error_devices: error,
            total_jobs_completed: total_jobs,
            total_bytes_processed: total_bytes,
        }
    }
}

impl Default for DevicePool {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_devices: usize,
    pub available_devices: usize,
    pub busy_devices: usize,
    pub offline_devices: usize,
    pub error_devices: usize,
    pub total_jobs_completed: u64,
    pub total_bytes_processed: u64,
}

// ============================================================================
// Job Queue System
// ============================================================================

static JOB_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate unique job ID
pub fn generate_job_id() -> u64 {
    JOB_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum JobPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Job status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued
    Queued,
    /// Job is assigned to a device
    Assigned { device_id: String },
    /// Job is running
    Running { device_id: String, progress: u8 },
    /// Job completed successfully
    Completed { device_id: String, duration_ms: u64 },
    /// Job failed
    Failed {
        device_id: Option<String>,
        error: String,
    },
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    TimedOut,
}

/// Job type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    /// Read/dump chip
    Read {
        output_path: String,
        start_address: u64,
        length: Option<u64>,
        include_oob: bool,
    },
    /// Write/program chip
    Write {
        input_path: String,
        start_address: u64,
        verify: bool,
    },
    /// Erase chip
    Erase {
        start_address: u64,
        length: Option<u64>,
    },
    /// Verify chip contents
    Verify { file_path: String },
    /// AI analysis
    Analyze {
        input_path: String,
        output_path: String,
        deep_scan: bool,
    },
    /// Clone chip-to-chip
    Clone {
        source_device: String,
        target_device: String,
    },
    /// Custom command
    Custom {
        command: String,
        params: HashMap<String, String>,
    },
}

/// Job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID
    pub id: u64,
    /// Job name/description
    pub name: String,
    /// Job type
    pub job_type: JobType,
    /// Job priority
    pub priority: JobPriority,
    /// Current status
    pub status: JobStatus,
    /// Required device ID (None = any available)
    pub device_id: Option<String>,
    /// Required interface type
    pub required_interface: Option<String>,
    /// Required tags (device must have all)
    pub required_tags: Vec<String>,
    /// Created timestamp (Unix epoch ms)
    pub created_at: u64,
    /// Started timestamp
    pub started_at: Option<u64>,
    /// Completed timestamp
    pub completed_at: Option<u64>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Retry count
    pub retries: u32,
    /// Max retries
    pub max_retries: u32,
    /// Result data (bytes processed, etc.)
    pub result: Option<JobResult>,
    /// User/client ID
    pub client_id: Option<String>,
    /// Callback URL for notifications
    pub callback_url: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Job {
    /// Create a new job
    pub fn new(name: &str, job_type: JobType) -> Self {
        Self {
            id: generate_job_id(),
            name: name.to_string(),
            job_type,
            priority: JobPriority::Normal,
            status: JobStatus::Queued,
            device_id: None,
            required_interface: None,
            required_tags: Vec::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            started_at: None,
            completed_at: None,
            timeout_secs: 3600, // 1 hour default
            retries: 0,
            max_retries: 3,
            result: None,
            client_id: None,
            callback_url: None,
            metadata: HashMap::new(),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set specific device
    pub fn with_device(mut self, device_id: &str) -> Self {
        self.device_id = Some(device_id.to_string());
        self
    }

    /// Set required interface
    pub fn with_interface(mut self, interface: &str) -> Self {
        self.required_interface = Some(interface.to_string());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set callback URL
    pub fn with_callback(mut self, url: &str) -> Self {
        self.callback_url = Some(url.to_string());
        self
    }

    /// Check if job is pending (queued or assigned)
    pub fn is_pending(&self) -> bool {
        matches!(self.status, JobStatus::Queued | JobStatus::Assigned { .. })
    }

    /// Check if job is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::Running { .. })
    }

    /// Check if job is finished
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed { .. }
                | JobStatus::Failed { .. }
                | JobStatus::Cancelled
                | JobStatus::TimedOut
        )
    }

    /// Mark job as started
    pub fn start(&mut self, device_id: &str) {
        self.status = JobStatus::Running {
            device_id: device_id.to_string(),
            progress: 0,
        };
        self.started_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
    }

    /// Update progress
    pub fn update_progress(&mut self, progress: u8) {
        if let JobStatus::Running { device_id, .. } = &self.status {
            self.status = JobStatus::Running {
                device_id: device_id.clone(),
                progress: progress.min(100),
            };
        }
    }

    /// Mark job as completed
    pub fn complete(&mut self, result: JobResult) {
        let device_id = match &self.status {
            JobStatus::Running { device_id, .. } => device_id.clone(),
            JobStatus::Assigned { device_id } => device_id.clone(),
            _ => "unknown".to_string(),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let duration = now - self.started_at.unwrap_or(now);
        self.status = JobStatus::Completed {
            device_id,
            duration_ms: duration,
        };
        self.completed_at = Some(now);
        self.result = Some(result);
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: &str) {
        let device_id = match &self.status {
            JobStatus::Running { device_id, .. } => Some(device_id.clone()),
            JobStatus::Assigned { device_id } => Some(device_id.clone()),
            _ => None,
        };
        self.status = JobStatus::Failed {
            device_id,
            error: error.to_string(),
        };
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
    }
}

/// Job result data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobResult {
    /// Bytes processed
    pub bytes_processed: u64,
    /// Pages processed
    pub pages_processed: u32,
    /// Blocks processed
    pub blocks_processed: u32,
    /// ECC corrections
    pub ecc_corrections: u32,
    /// Bad blocks found
    pub bad_blocks: Vec<u32>,
    /// Output file path (if applicable)
    pub output_path: Option<String>,
    /// Checksum of output
    pub checksum: Option<String>,
    /// Additional data
    pub data: HashMap<String, String>,
}

/// Job queue manager
#[derive(Debug)]
pub struct JobQueue {
    /// Pending jobs (sorted by priority)
    pub pending: Vec<Job>,
    /// Running jobs
    pub running: HashMap<u64, Job>,
    /// Completed jobs (recent history)
    pub completed: Vec<Job>,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Maximum history size
    pub max_history_size: usize,
}

impl JobQueue {
    /// Create a new job queue
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            pending: Vec::new(),
            running: HashMap::new(),
            completed: Vec::new(),
            max_queue_size,
            max_history_size: 1000,
        }
    }

    /// Submit a job to the queue
    pub fn submit(&mut self, job: Job) -> ServerResult<u64> {
        if self.pending.len() >= self.max_queue_size {
            return Err(ServerError::QueueFull);
        }
        let id = job.id;
        self.pending.push(job);
        // Sort by priority (highest first)
        self.pending.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(id)
    }

    /// Get next job for a device
    pub fn next_job_for_device(&mut self, device: &PoolDevice) -> Option<Job> {
        let idx = self.pending.iter().position(|job| {
            // Check device requirement
            if let Some(ref required_id) = job.device_id {
                if required_id != &device.id {
                    return false;
                }
            }
            // Check interface requirement
            if let Some(ref required_iface) = job.required_interface {
                if !device.capabilities.interfaces.contains(required_iface) {
                    return false;
                }
            }
            // Check tag requirements
            for tag in &job.required_tags {
                if !device.tags.contains(tag) {
                    return false;
                }
            }
            true
        })?;

        let mut job = self.pending.remove(idx);
        job.status = JobStatus::Assigned {
            device_id: device.id.clone(),
        };
        self.running.insert(job.id, job.clone());
        Some(job)
    }

    /// Get job by ID
    pub fn get_job(&self, job_id: u64) -> Option<&Job> {
        self.running
            .get(&job_id)
            .or_else(|| self.pending.iter().find(|j| j.id == job_id))
            .or_else(|| self.completed.iter().find(|j| j.id == job_id))
    }

    /// Update job status
    pub fn update_job(&mut self, job_id: u64, status: JobStatus) -> ServerResult<()> {
        if let Some(job) = self.running.get_mut(&job_id) {
            job.status = status;
            Ok(())
        } else {
            Err(ServerError::JobNotFound(job_id))
        }
    }

    /// Complete a job
    pub fn complete_job(&mut self, job_id: u64, result: JobResult) -> ServerResult<()> {
        if let Some(mut job) = self.running.remove(&job_id) {
            job.complete(result);
            self.add_to_history(job);
            Ok(())
        } else {
            Err(ServerError::JobNotFound(job_id))
        }
    }

    /// Fail a job
    pub fn fail_job(&mut self, job_id: u64, error: &str) -> ServerResult<()> {
        if let Some(mut job) = self.running.remove(&job_id) {
            job.fail(error);
            // Check if should retry
            if job.retries < job.max_retries {
                job.retries += 1;
                job.status = JobStatus::Queued;
                self.pending.push(job);
                self.pending.sort_by(|a, b| b.priority.cmp(&a.priority));
            } else {
                self.add_to_history(job);
            }
            Ok(())
        } else {
            Err(ServerError::JobNotFound(job_id))
        }
    }

    /// Cancel a job
    pub fn cancel_job(&mut self, job_id: u64) -> ServerResult<()> {
        // Check pending
        if let Some(idx) = self.pending.iter().position(|j| j.id == job_id) {
            let mut job = self.pending.remove(idx);
            job.status = JobStatus::Cancelled;
            self.add_to_history(job);
            return Ok(());
        }
        // Check running
        if let Some(mut job) = self.running.remove(&job_id) {
            job.status = JobStatus::Cancelled;
            self.add_to_history(job);
            return Ok(());
        }
        Err(ServerError::JobNotFound(job_id))
    }

    /// Add job to history
    fn add_to_history(&mut self, job: Job) {
        self.completed.push(job);
        // Trim history if needed
        while self.completed.len() > self.max_history_size {
            self.completed.remove(0);
        }
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending_count: self.pending.len(),
            running_count: self.running.len(),
            completed_count: self
                .completed
                .iter()
                .filter(|j| matches!(j.status, JobStatus::Completed { .. }))
                .count(),
            failed_count: self
                .completed
                .iter()
                .filter(|j| matches!(j.status, JobStatus::Failed { .. }))
                .count(),
            cancelled_count: self
                .completed
                .iter()
                .filter(|j| j.status == JobStatus::Cancelled)
                .count(),
        }
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending_count: usize,
    pub running_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub cancelled_count: usize,
}

// ============================================================================
// REST API Types
// ============================================================================

/// API authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// No authentication
    None,
    /// API key in header
    ApiKey { header_name: String },
    /// Bearer token (JWT)
    BearerToken,
    /// Basic auth
    BasicAuth,
}

/// API rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            enabled: true,
        }
    }
}

/// REST API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    /// Listen address
    pub host: String,
    /// Listen port
    pub port: u16,
    /// Enable HTTPS
    pub https: bool,
    /// TLS certificate path
    pub cert_path: Option<String>,
    /// TLS key path
    pub key_path: Option<String>,
    /// Authentication method
    pub auth: AuthMethod,
    /// Rate limiting
    pub rate_limit: RateLimitConfig,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// API prefix (e.g., "/api/v1")
    pub prefix: String,
}

impl Default for RestApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            https: false,
            cert_path: None,
            key_path: None,
            auth: AuthMethod::None,
            rate_limit: RateLimitConfig::default(),
            cors_origins: vec!["*".to_string()],
            prefix: "/api/v1".to_string(),
        }
    }
}

/// REST API request for submitting a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobRequest {
    /// Job name
    pub name: String,
    /// Job type
    pub job_type: String,
    /// Job parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Priority (low, normal, high, critical)
    pub priority: Option<String>,
    /// Specific device ID
    pub device_id: Option<String>,
    /// Required interface
    pub interface: Option<String>,
    /// Required tags
    pub tags: Option<Vec<String>>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Callback URL
    pub callback_url: Option<String>,
}

/// REST API response for job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobResponse {
    /// Job ID
    pub job_id: u64,
    /// Status
    pub status: String,
    /// Message
    pub message: String,
    /// Estimated wait time (seconds)
    pub estimated_wait: Option<u64>,
}

/// REST API response for job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    /// Job ID
    pub job_id: u64,
    /// Job name
    pub name: String,
    /// Status
    pub status: String,
    /// Progress (0-100)
    pub progress: Option<u8>,
    /// Device ID
    pub device_id: Option<String>,
    /// Created timestamp
    pub created_at: u64,
    /// Started timestamp
    pub started_at: Option<u64>,
    /// Completed timestamp
    pub completed_at: Option<u64>,
    /// Result (if completed)
    pub result: Option<JobResult>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// REST API response for device list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    /// Total devices
    pub total: usize,
    /// Devices
    pub devices: Vec<DeviceInfo>,
}

/// Device info for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Platform
    pub platform: String,
    /// Status
    pub status: String,
    /// Current job ID
    pub current_job: Option<u64>,
    /// Interfaces
    pub interfaces: Vec<String>,
    /// Tags
    pub tags: Vec<String>,
}

impl From<&PoolDevice> for DeviceInfo {
    fn from(device: &PoolDevice) -> Self {
        Self {
            id: device.id.clone(),
            name: device.name.clone(),
            platform: format!("{:?}", device.platform),
            status: format!("{:?}", device.status),
            current_job: device.current_job,
            interfaces: device.capabilities.interfaces.clone(),
            tags: device.tags.clone(),
        }
    }
}

// ============================================================================
// WebSocket Types
// ============================================================================

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Subscribe to job updates
    Subscribe {
        job_ids: Vec<u64>,
    },
    /// Unsubscribe from job updates
    Unsubscribe {
        job_ids: Vec<u64>,
    },
    /// Subscribe to device updates
    SubscribeDevices {
        device_ids: Vec<String>,
    },
    /// Job status update
    JobUpdate {
        job_id: u64,
        status: String,
        progress: Option<u8>,
    },
    /// Job completed
    JobCompleted {
        job_id: u64,
        result: JobResult,
    },
    /// Job failed
    JobFailed {
        job_id: u64,
        error: String,
    },
    /// Device status update
    DeviceUpdate {
        device_id: String,
        status: String,
    },
    /// Error message
    Error {
        code: String,
        message: String,
    },
    /// Ping/pong for keepalive
    Ping,
    Pong,
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Enable WebSocket
    pub enabled: bool,
    /// WebSocket path
    pub path: String,
    /// Ping interval (seconds)
    pub ping_interval: u64,
    /// Connection timeout (seconds)
    pub timeout: u64,
    /// Max connections
    pub max_connections: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/ws".to_string(),
            ping_interval: 30,
            timeout: 60,
            max_connections: 1000,
        }
    }
}

// ============================================================================
// gRPC Types
// ============================================================================

/// gRPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Enable gRPC
    pub enabled: bool,
    /// Listen address
    pub host: String,
    /// Listen port
    pub port: u16,
    /// Enable TLS
    pub tls: bool,
    /// TLS certificate path
    pub cert_path: Option<String>,
    /// TLS key path
    pub key_path: Option<String>,
    /// Max message size (bytes)
    pub max_message_size: usize,
    /// Enable reflection
    pub reflection: bool,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "0.0.0.0".to_string(),
            port: 50051,
            tls: false,
            cert_path: None,
            key_path: None,
            max_message_size: 16 * 1024 * 1024, // 16MB
            reflection: true,
        }
    }
}

// ============================================================================
// Server Configuration
// ============================================================================

/// OpenFlash Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// REST API configuration
    pub rest: RestApiConfig,
    /// WebSocket configuration
    pub websocket: WebSocketConfig,
    /// gRPC configuration
    pub grpc: GrpcConfig,
    /// Maximum devices in pool
    pub max_devices: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Job timeout (seconds)
    pub default_job_timeout: u64,
    /// Enable metrics endpoint
    pub metrics_enabled: bool,
    /// Metrics port
    pub metrics_port: u16,
    /// Log level
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "OpenFlash Server".to_string(),
            rest: RestApiConfig::default(),
            websocket: WebSocketConfig::default(),
            grpc: GrpcConfig::default(),
            max_devices: 100,
            max_queue_size: 10000,
            default_job_timeout: 3600,
            metrics_enabled: true,
            metrics_port: 9090,
            log_level: "info".to_string(),
        }
    }
}

// ============================================================================
// OpenFlash Server
// ============================================================================

/// OpenFlash Server - Main server instance
#[derive(Debug)]
pub struct OpenFlashServer {
    /// Server configuration
    pub config: ServerConfig,
    /// Device pool
    pub device_pool: DevicePool,
    /// Job queue
    pub job_queue: JobQueue,
    /// Server start time
    pub started_at: u64,
    /// Server version
    pub version: String,
}

impl OpenFlashServer {
    /// Create a new server instance
    pub fn new(config: ServerConfig) -> Self {
        Self {
            device_pool: DevicePool::new(config.max_devices),
            job_queue: JobQueue::new(config.max_queue_size),
            config,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            version: "2.0.0".to_string(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ServerConfig::default())
    }

    /// Register a device
    pub fn register_device(&mut self, device: PoolDevice) -> ServerResult<()> {
        self.device_pool.add_device(device)
    }

    /// Submit a job
    pub fn submit_job(&mut self, job: Job) -> ServerResult<u64> {
        self.job_queue.submit(job)
    }

    /// Get job status
    pub fn get_job_status(&self, job_id: u64) -> Option<JobStatusResponse> {
        self.job_queue.get_job(job_id).map(|job| {
            let (status, progress, device_id, error) = match &job.status {
                JobStatus::Queued => ("queued".to_string(), None, None, None),
                JobStatus::Assigned { device_id } => {
                    ("assigned".to_string(), None, Some(device_id.clone()), None)
                }
                JobStatus::Running {
                    device_id,
                    progress,
                } => (
                    "running".to_string(),
                    Some(*progress),
                    Some(device_id.clone()),
                    None,
                ),
                JobStatus::Completed { device_id, .. } => (
                    "completed".to_string(),
                    Some(100),
                    Some(device_id.clone()),
                    None,
                ),
                JobStatus::Failed { device_id, error } => (
                    "failed".to_string(),
                    None,
                    device_id.clone(),
                    Some(error.clone()),
                ),
                JobStatus::Cancelled => ("cancelled".to_string(), None, None, None),
                JobStatus::TimedOut => (
                    "timed_out".to_string(),
                    None,
                    None,
                    Some("Job timed out".to_string()),
                ),
            };

            JobStatusResponse {
                job_id: job.id,
                name: job.name.clone(),
                status,
                progress,
                device_id,
                created_at: job.created_at,
                started_at: job.started_at,
                completed_at: job.completed_at,
                result: job.result.clone(),
                error,
            }
        })
    }

    /// Cancel a job
    pub fn cancel_job(&mut self, job_id: u64) -> ServerResult<()> {
        self.job_queue.cancel_job(job_id)
    }

    /// List devices
    pub fn list_devices(&self) -> DeviceListResponse {
        let devices: Vec<DeviceInfo> = self
            .device_pool
            .devices
            .values()
            .map(DeviceInfo::from)
            .collect();
        DeviceListResponse {
            total: devices.len(),
            devices,
        }
    }

    /// Get server info
    pub fn server_info(&self) -> ServerInfo {
        let uptime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - self.started_at;

        ServerInfo {
            name: self.config.name.clone(),
            version: self.version.clone(),
            uptime_ms: uptime,
            pool_stats: self.device_pool.stats(),
            queue_stats: self.job_queue.stats(),
        }
    }

    /// Process job queue - assign jobs to available devices
    pub fn process_queue(&mut self) -> Vec<(u64, String)> {
        let mut assignments = Vec::new();

        // Get available devices
        let available: Vec<String> = self
            .device_pool
            .devices
            .values()
            .filter(|d| d.is_available())
            .map(|d| d.id.clone())
            .collect();

        for device_id in available {
            if let Some(device) = self.device_pool.get_device(&device_id) {
                if let Some(mut job) = self.job_queue.next_job_for_device(device) {
                    job.start(&device_id);
                    if let Some(d) = self.device_pool.get_device_mut(&device_id) {
                        d.assign_job(job.id);
                    }
                    assignments.push((job.id, device_id.clone()));
                    self.job_queue.running.insert(job.id, job);
                }
            }
        }

        assignments
    }
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Uptime in milliseconds
    pub uptime_ms: u64,
    /// Pool statistics
    pub pool_stats: PoolStats,
    /// Queue statistics
    pub queue_stats: QueueStats,
}

// ============================================================================
// Parallel Dumping
// ============================================================================

/// Parallel dump configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelDumpConfig {
    /// Number of devices to use
    pub device_count: usize,
    /// Chunk size per device (bytes)
    pub chunk_size: u64,
    /// Output directory
    pub output_dir: String,
    /// Merge output files
    pub merge_output: bool,
    /// Verify after dump
    pub verify: bool,
}

impl Default for ParallelDumpConfig {
    fn default() -> Self {
        Self {
            device_count: 4,
            chunk_size: 64 * 1024 * 1024, // 64MB chunks
            output_dir: "./dumps".to_string(),
            merge_output: true,
            verify: true,
        }
    }
}

/// Parallel dump job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelDumpJob {
    /// Master job ID
    pub id: u64,
    /// Total size to dump
    pub total_size: u64,
    /// Chunk jobs
    pub chunks: Vec<ChunkJob>,
    /// Status
    pub status: ParallelJobStatus,
    /// Configuration
    pub config: ParallelDumpConfig,
    /// Start time
    pub started_at: Option<u64>,
    /// End time
    pub completed_at: Option<u64>,
}

/// Chunk job for parallel operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkJob {
    /// Chunk index
    pub index: usize,
    /// Start address
    pub start_address: u64,
    /// Length
    pub length: u64,
    /// Assigned device ID
    pub device_id: Option<String>,
    /// Job ID
    pub job_id: Option<u64>,
    /// Status
    pub status: ChunkStatus,
    /// Output file
    pub output_file: String,
    /// Checksum
    pub checksum: Option<String>,
}

/// Chunk status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed(String),
}

/// Parallel job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParallelJobStatus {
    /// Job is being prepared
    Preparing,
    /// Job is running
    Running {
        completed_chunks: usize,
        total_chunks: usize,
        bytes_completed: u64,
    },
    /// Merging output files
    Merging,
    /// Verifying output
    Verifying,
    /// Job completed
    Completed {
        duration_ms: u64,
        output_file: String,
        checksum: String,
    },
    /// Job failed
    Failed(String),
}

impl ParallelDumpJob {
    /// Create a new parallel dump job
    pub fn new(total_size: u64, config: ParallelDumpConfig) -> Self {
        let chunk_count = ((total_size + config.chunk_size - 1) / config.chunk_size) as usize;
        let mut chunks = Vec::with_capacity(chunk_count);

        for i in 0..chunk_count {
            let start = i as u64 * config.chunk_size;
            let length = (total_size - start).min(config.chunk_size);
            chunks.push(ChunkJob {
                index: i,
                start_address: start,
                length,
                device_id: None,
                job_id: None,
                status: ChunkStatus::Pending,
                output_file: format!("{}/chunk_{:04}.bin", config.output_dir, i),
                checksum: None,
            });
        }

        Self {
            id: generate_job_id(),
            total_size,
            chunks,
            status: ParallelJobStatus::Preparing,
            config,
            started_at: None,
            completed_at: None,
        }
    }

    /// Get progress percentage
    pub fn progress(&self) -> u8 {
        let completed = self
            .chunks
            .iter()
            .filter(|c| c.status == ChunkStatus::Completed)
            .count();
        ((completed * 100) / self.chunks.len().max(1)) as u8
    }

    /// Get bytes completed
    pub fn bytes_completed(&self) -> u64 {
        self.chunks
            .iter()
            .filter(|c| c.status == ChunkStatus::Completed)
            .map(|c| c.length)
            .sum()
    }

    /// Check if all chunks are completed
    pub fn is_complete(&self) -> bool {
        self.chunks
            .iter()
            .all(|c| c.status == ChunkStatus::Completed)
    }

    /// Get next pending chunk
    pub fn next_pending_chunk(&mut self) -> Option<&mut ChunkJob> {
        self.chunks
            .iter_mut()
            .find(|c| c.status == ChunkStatus::Pending)
    }
}

// ============================================================================
// Production Line Integration
// ============================================================================

/// Production line configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionLineConfig {
    /// Line ID
    pub line_id: String,
    /// Line name
    pub name: String,
    /// Stations in the line
    pub stations: Vec<StationConfig>,
    /// Default firmware to flash
    pub default_firmware: Option<String>,
    /// Expected chip type
    pub expected_chip: Option<String>,
    /// Auto-start on device connect
    pub auto_start: bool,
    /// Verification mode
    pub verification: VerificationMode,
    /// Logging configuration
    pub logging: ProductionLogging,
}

/// Station configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationConfig {
    /// Station ID
    pub id: String,
    /// Station name
    pub name: String,
    /// Device ID assigned to this station
    pub device_id: Option<String>,
    /// Operations to perform
    pub operations: Vec<StationOperation>,
    /// Pass criteria
    pub pass_criteria: PassCriteria,
}

/// Station operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StationOperation {
    /// Detect and verify chip
    DetectChip { expected: Option<String> },
    /// Erase chip
    Erase { full: bool },
    /// Program firmware
    Program { firmware_path: String, verify: bool },
    /// Verify contents
    Verify { golden_path: String, tolerance: f32 },
    /// Read and save dump
    Dump { output_path: String },
    /// Run custom test
    CustomTest { script_path: String },
    /// Mark serial number
    MarkSerial { format: String },
}

/// Pass criteria for station
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassCriteria {
    /// Maximum allowed bad blocks
    pub max_bad_blocks: u32,
    /// Maximum ECC corrections
    pub max_ecc_corrections: u32,
    /// Minimum data match percentage
    pub min_match_percent: f32,
    /// Maximum operation time (seconds)
    pub max_time_secs: u64,
}

impl Default for PassCriteria {
    fn default() -> Self {
        Self {
            max_bad_blocks: 20,
            max_ecc_corrections: 100,
            min_match_percent: 99.9,
            max_time_secs: 300,
        }
    }
}

/// Verification mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMode {
    /// No verification
    None,
    /// Quick verification (sample pages)
    Quick,
    /// Full verification
    Full,
    /// Checksum only
    Checksum,
}

/// Production logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionLogging {
    /// Enable logging
    pub enabled: bool,
    /// Log directory
    pub log_dir: String,
    /// Log format (json, csv, text)
    pub format: String,
    /// Include detailed timing
    pub detailed_timing: bool,
    /// Upload to server
    pub upload_url: Option<String>,
}

impl Default for ProductionLogging {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: "./production_logs".to_string(),
            format: "json".to_string(),
            detailed_timing: true,
            upload_url: None,
        }
    }
}

/// Production unit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionUnitResult {
    /// Unit serial number
    pub serial_number: String,
    /// Line ID
    pub line_id: String,
    /// Station ID
    pub station_id: String,
    /// Pass/fail status
    pub passed: bool,
    /// Timestamp
    pub timestamp: u64,
    /// Duration (ms)
    pub duration_ms: u64,
    /// Chip info
    pub chip_info: Option<String>,
    /// Bad blocks found
    pub bad_blocks: u32,
    /// ECC corrections
    pub ecc_corrections: u32,
    /// Failure reason (if failed)
    pub failure_reason: Option<String>,
    /// Additional data
    pub data: HashMap<String, String>,
}

/// Production statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionStats {
    /// Line ID
    pub line_id: String,
    /// Total units processed
    pub total_units: u64,
    /// Passed units
    pub passed_units: u64,
    /// Failed units
    pub failed_units: u64,
    /// Pass rate (percentage)
    pub pass_rate: f32,
    /// Average cycle time (ms)
    pub avg_cycle_time_ms: u64,
    /// Units per hour
    pub units_per_hour: f32,
    /// Common failure reasons
    pub failure_reasons: HashMap<String, u32>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_pool_creation() {
        let pool = DevicePool::new(10);
        assert_eq!(pool.max_devices, 10);
        assert!(pool.devices.is_empty());
    }

    #[test]
    fn test_add_device_to_pool() {
        let mut pool = DevicePool::new(10);
        let device = PoolDevice::new(
            "dev1",
            "Test Device",
            "serial:///dev/ttyUSB0",
            DevicePlatform::RP2040,
        );
        assert!(pool.add_device(device).is_ok());
        assert_eq!(pool.devices.len(), 1);
    }

    #[test]
    fn test_pool_max_devices() {
        let mut pool = DevicePool::new(2);
        let d1 = PoolDevice::new(
            "dev1",
            "Device 1",
            "serial:///dev/ttyUSB0",
            DevicePlatform::RP2040,
        );
        let d2 = PoolDevice::new(
            "dev2",
            "Device 2",
            "serial:///dev/ttyUSB1",
            DevicePlatform::STM32F4,
        );
        let d3 = PoolDevice::new(
            "dev3",
            "Device 3",
            "serial:///dev/ttyUSB2",
            DevicePlatform::ESP32,
        );

        assert!(pool.add_device(d1).is_ok());
        assert!(pool.add_device(d2).is_ok());
        assert!(pool.add_device(d3).is_err());
    }

    #[test]
    fn test_device_assignment() {
        let mut device = PoolDevice::new(
            "dev1",
            "Test",
            "serial:///dev/ttyUSB0",
            DevicePlatform::RP2040,
        );
        device.status = DeviceStatus::Available;
        assert!(device.is_available());

        device.assign_job(123);
        assert!(!device.is_available());
        assert_eq!(device.current_job, Some(123));

        device.release(true, 1024);
        assert!(device.is_available());
        assert_eq!(device.jobs_completed, 1);
        assert_eq!(device.bytes_processed, 1024);
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new(
            "Test Job",
            JobType::Read {
                output_path: "/tmp/dump.bin".to_string(),
                start_address: 0,
                length: Some(1024),
                include_oob: false,
            },
        );

        assert!(job.id > 0);
        assert_eq!(job.name, "Test Job");
        assert!(matches!(job.status, JobStatus::Queued));
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new(
            "Test",
            JobType::Erase {
                start_address: 0,
                length: None,
            },
        );

        assert!(job.is_pending());

        job.start("dev1");
        assert!(job.is_running());

        job.update_progress(50);
        if let JobStatus::Running { progress, .. } = job.status {
            assert_eq!(progress, 50);
        }

        job.complete(JobResult::default());
        assert!(job.is_finished());
    }

    #[test]
    fn test_job_queue() {
        let mut queue = JobQueue::new(100);

        let job1 = Job::new(
            "Job 1",
            JobType::Erase {
                start_address: 0,
                length: None,
            },
        );
        let job2 = Job::new(
            "Job 2",
            JobType::Erase {
                start_address: 0,
                length: None,
            },
        )
        .with_priority(JobPriority::High);

        queue.submit(job1).unwrap();
        queue.submit(job2).unwrap();

        // High priority job should be first
        assert_eq!(queue.pending[0].priority, JobPriority::High);
    }

    #[test]
    fn test_server_creation() {
        let server = OpenFlashServer::with_defaults();
        assert_eq!(server.version, "2.0.0");
        assert!(server.device_pool.devices.is_empty());
    }

    #[test]
    fn test_parallel_dump_job() {
        let config = ParallelDumpConfig {
            chunk_size: 1024,
            ..Default::default()
        };
        let job = ParallelDumpJob::new(4096, config);

        assert_eq!(job.chunks.len(), 4);
        assert_eq!(job.chunks[0].start_address, 0);
        assert_eq!(job.chunks[0].length, 1024);
        assert_eq!(job.chunks[3].start_address, 3072);
    }

    #[test]
    fn test_pool_stats() {
        let mut pool = DevicePool::new(10);
        let mut d1 = PoolDevice::new(
            "dev1",
            "Device 1",
            "serial:///dev/ttyUSB0",
            DevicePlatform::RP2040,
        );
        d1.status = DeviceStatus::Available;
        d1.jobs_completed = 10;
        d1.bytes_processed = 1024 * 1024;

        let mut d2 = PoolDevice::new(
            "dev2",
            "Device 2",
            "serial:///dev/ttyUSB1",
            DevicePlatform::STM32F4,
        );
        d2.status = DeviceStatus::Busy;
        d2.jobs_completed = 5;

        pool.add_device(d1).unwrap();
        pool.add_device(d2).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total_devices, 2);
        assert_eq!(stats.available_devices, 1);
        assert_eq!(stats.busy_devices, 1);
        assert_eq!(stats.total_jobs_completed, 15);
    }

    #[test]
    fn test_queue_stats() {
        let mut queue = JobQueue::new(100);

        for i in 0..5 {
            let job = Job::new(
                &format!("Job {}", i),
                JobType::Erase {
                    start_address: 0,
                    length: None,
                },
            );
            queue.submit(job).unwrap();
        }

        let stats = queue.stats();
        assert_eq!(stats.pending_count, 5);
        assert_eq!(stats.running_count, 0);
    }

    #[test]
    fn test_device_platform_from_str() {
        assert_eq!(DevicePlatform::from_str("RP2040"), DevicePlatform::RP2040);
        assert_eq!(DevicePlatform::from_str("stm32f4"), DevicePlatform::STM32F4);
        assert_eq!(
            DevicePlatform::from_str("ESP32-S3"),
            DevicePlatform::ESP32S3
        );
        assert_eq!(DevicePlatform::from_str("unknown"), DevicePlatform::Unknown);
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Critical > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::JobUpdate {
            job_id: 123,
            status: "running".to_string(),
            progress: Some(50),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("JobUpdate"));
        assert!(json.contains("123"));
    }
}
