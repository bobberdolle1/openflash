//! Python bindings for OpenFlash.
//!
//! Device operations here perform real I/O through `openflash_core`. Previously
//! `connect()` succeeded unconditionally, `detect()` always answered Samsung
//! K9F1G08U0E and `read()` returned a buffer of `0xFF`, so a script could not
//! tell a successful dump from no device at all.
//!
//! ```python
//! import openflash
//!
//! # A real device: the single attached USB programmer, or an SBC agent.
//! device = openflash.connect()
//! # No hardware needed; nothing real is read or written.
//! device = openflash.connect_emulated(2 * 1024 * 1024)
//!
//! chip = device.detect()
//! print(f"Found {chip.manufacturer} {chip.model}, {chip.capacity} bytes")
//!
//! dump = device.read_full()
//! dump.save("dump.bin")
//!
//! analysis = openflash.ai.analyze(dump)
//! print(f"Quality: {analysis.quality_score:.0%}")
//! ```

use openflash_core::scripting::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

// ============================================================================
// Device Module
// ============================================================================

/// A connected device.
///
/// Not constructible directly: use [`connect`], [`connect_tcp`], [`connect_unix`]
/// or [`connect_emulated`], each of which fails when the device is not there.
#[pyclass(unsendable)]
struct Device {
    inner: OpenFlash,
    last_dump: Option<Dump>,
}

impl Device {
    fn open(config: ConnectionConfig) -> PyResult<Self> {
        let mut inner = OpenFlash::new();
        inner
            .connect_with_config(config)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner,
            last_dump: None,
        })
    }
}

#[pymethods]
impl Device {
    /// Whether the connection is still open.
    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Close the connection.
    fn disconnect(&mut self) {
        self.inner.disconnect();
    }

    /// What the device reported about itself at connect time.
    fn info(&self) -> PyResult<PyDeviceInfo> {
        self.inner
            .device_info()
            .map(PyDeviceInfo::from)
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))
    }

    /// Refuse any operation that would modify the chip.
    fn set_read_only(&mut self, read_only: bool) -> PyResult<()> {
        self.inner
            .set_read_only(read_only)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Read the chip id off the bus and look it up in the database.
    fn detect(&mut self) -> PyResult<ChipInfo> {
        let chip = self
            .inner
            .detect_chip()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(ChipInfo {
            manufacturer: chip.manufacturer,
            model: chip.model,
            capacity: chip.capacity,
            page_size: chip.page_size,
            block_size: chip.block_size,
            oob_size: chip.oob_size,
            interface: chip.interface,
        })
    }

    /// Dump the whole chip.
    fn read_full(&mut self) -> PyResult<Dump> {
        self.read(None, None, false)
    }

    /// Dump part of the chip.
    #[pyo3(signature = (start=None, length=None, include_oob=false))]
    fn read(
        &mut self,
        start: Option<u64>,
        length: Option<u64>,
        include_oob: bool,
    ) -> PyResult<Dump> {
        let chip = self.detect()?;
        let result = self
            .inner
            .read_with_options(ReadOptions {
                start_address: start.unwrap_or(0),
                length,
                include_oob,
                ..ReadOptions::default()
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dump = Dump {
            data: result.data.clone(),
            oob_data: result.oob_data.clone(),
            chip_info: Some(chip),
            bad_blocks: result.bad_blocks.clone(),
        };
        self.last_dump = Some(dump.clone());
        Ok(dump)
    }

    /// Write `data` at `start`, erasing the affected sectors first and verifying
    /// afterwards unless told otherwise.
    #[pyo3(signature = (data, start=0, verify=true, erase=true))]
    fn write(
        &mut self,
        data: Vec<u8>,
        start: u64,
        verify: bool,
        erase: bool,
    ) -> PyResult<WriteResult> {
        let report = self
            .inner
            .write(
                &data,
                WriteOptions {
                    start_address: start,
                    verify,
                    erase_before_write: erase,
                    ..WriteOptions::default()
                },
            )
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(WriteResult {
            bytes_written: report.bytes_written,
            pages_written: report.pages_written as u32,
            verified: report.verified,
            sectors_erased: report.sectors_erased,
        })
    }

    /// Erase whole sectors covering the range. Returns the sector count.
    fn erase(&mut self, start: u64, length: u64) -> PyResult<u64> {
        self.inner
            .erase(start, length)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Read the chip back and compare it with `expected`.
    fn verify(&mut self, expected: Vec<u8>, start: u64) -> PyResult<bool> {
        match self.inner.verify(start, &expected) {
            Ok(()) => Ok(true),
            Err(error) => Err(PyRuntimeError::new_err(error.to_string())),
        }
    }

    /// The most recent dump, if any.
    fn last_dump(&self) -> Option<Dump> {
        self.last_dump.clone()
    }

    fn __repr__(&self) -> String {
        match self.inner.device_info() {
            Some(info) => format!(
                "<openflash.Device connected to {} ({})>",
                info.port, info.platform
            ),
            None => "<openflash.Device disconnected>".to_string(),
        }
    }
}

#[pyclass]
#[derive(Clone)]
struct PyDeviceInfo {
    #[pyo3(get)]
    port: String,
    #[pyo3(get)]
    firmware_version: String,
    #[pyo3(get)]
    platform: String,
    #[pyo3(get)]
    serial_number: String,
    #[pyo3(get)]
    interfaces: Vec<String>,
}

impl From<&DeviceInfo> for PyDeviceInfo {
    fn from(info: &DeviceInfo) -> Self {
        Self {
            port: info.port.clone(),
            firmware_version: info.firmware_version.clone(),
            platform: info.platform.clone(),
            serial_number: info.serial_number.clone(),
            interfaces: info.interfaces.clone(),
        }
    }
}

/// Chip information
#[pyclass]
#[derive(Clone)]
struct ChipInfo {
    #[pyo3(get)]
    manufacturer: String,
    #[pyo3(get)]
    model: String,
    #[pyo3(get)]
    capacity: u64,
    #[pyo3(get)]
    page_size: u32,
    #[pyo3(get)]
    block_size: u32,
    #[pyo3(get)]
    oob_size: u16,
    #[pyo3(get)]
    interface: String,
}

#[pymethods]
impl ChipInfo {
    fn __repr__(&self) -> String {
        format!(
            "ChipInfo({} {} {}MB)",
            self.manufacturer,
            self.model,
            self.capacity / 1024 / 1024
        )
    }
}

/// Dump data
#[pyclass]
#[derive(Clone)]
struct Dump {
    #[pyo3(get)]
    data: Vec<u8>,
    #[pyo3(get)]
    oob_data: Option<Vec<u8>>,
    #[pyo3(get)]
    chip_info: Option<ChipInfo>,
    #[pyo3(get)]
    bad_blocks: Vec<u32>,
}

#[pymethods]
impl Dump {
    /// Save dump to file
    fn save(&self, path: &str) -> PyResult<()> {
        std::fs::write(path, &self.data).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Save OOB data to file
    fn save_oob(&self, path: &str) -> PyResult<()> {
        if let Some(oob) = &self.oob_data {
            std::fs::write(path, oob).map_err(|e| PyRuntimeError::new_err(e.to_string()))
        } else {
            Err(PyValueError::new_err("No OOB data"))
        }
    }

    /// Get dump size
    fn size(&self) -> usize {
        self.data.len()
    }

    /// Get slice of data
    fn slice(&self, start: usize, end: usize) -> Vec<u8> {
        self.data[start..end.min(self.data.len())].to_vec()
    }

    fn __repr__(&self) -> String {
        format!("Dump({} bytes)", self.data.len())
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }
}

/// Write result
#[pyclass]
#[derive(Clone)]
struct WriteResult {
    #[pyo3(get)]
    bytes_written: u64,
    #[pyo3(get)]
    pages_written: u32,
    #[pyo3(get)]
    sectors_erased: u64,
    /// Whether the region was read back and compared after writing.
    #[pyo3(get)]
    verified: bool,
}

// ============================================================================
// AI Analysis Module
// ============================================================================

/// AI analysis submodule
/// Dump analysis.
///
/// These previously ignored their input entirely and returned a fixed result
/// naming a SquashFS at 0x10000 whatever the dump contained. They now run the
/// analyser in `openflash_core::ai` over the actual bytes.
#[pyclass]
struct AiModule;

/// Page size assumed when a dump carries no chip information.
///
/// The analyser needs a page geometry to reason about block structure; 2048 with
/// 64 pages per block is the most common NAND layout.
const DEFAULT_PAGE_SIZE: usize = 2048;
const DEFAULT_PAGES_PER_BLOCK: usize = 64;

fn analyzer_for(dump: &Dump, deep_scan: bool) -> openflash_core::ai::AiAnalyzer {
    let page_size = dump
        .chip_info
        .as_ref()
        .map(|chip| chip.page_size as usize)
        .filter(|size| *size > 0)
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let pages_per_block = dump
        .chip_info
        .as_ref()
        .map(|chip| (chip.block_size / chip.page_size.max(1)) as usize)
        .filter(|count| *count > 0)
        .unwrap_or(DEFAULT_PAGES_PER_BLOCK);

    openflash_core::ai::AiAnalyzer::new(page_size, pages_per_block).with_deep_scan(deep_scan)
}

#[pymethods]
impl AiModule {
    /// Analyse a dump: patterns, filesystems, anomalies, entropy.
    #[staticmethod]
    #[pyo3(signature = (dump, deep_scan=false))]
    fn analyze(dump: &Dump, deep_scan: bool) -> PyResult<AnalysisResult> {
        if dump.data.is_empty() {
            return Err(PyValueError::new_err("the dump is empty"));
        }

        let result = analyzer_for(dump, deep_scan).analyze(&dump.data);

        Ok(AnalysisResult {
            quality_score: result.data_quality_score,
            encryption_probability: result.encryption_probability,
            compression_probability: result.compression_probability,
            patterns: result.patterns.iter().map(Pattern::from).collect(),
            filesystems: result.filesystems.iter().map(Filesystem::from).collect(),
            anomalies: result.anomalies.iter().map(Anomaly::from).collect(),
            summary: result.summary,
        })
    }

    /// Patterns found in the dump.
    #[staticmethod]
    fn detect_patterns(dump: &Dump) -> PyResult<Vec<Pattern>> {
        if dump.data.is_empty() {
            return Err(PyValueError::new_err("the dump is empty"));
        }
        Ok(analyzer_for(dump, false)
            .analyze(&dump.data)
            .patterns
            .iter()
            .map(Pattern::from)
            .collect())
    }

    /// Regions that look like cryptographic key material.
    #[staticmethod]
    fn search_keys(dump: &Dump) -> PyResult<Vec<KeyCandidate>> {
        if dump.data.is_empty() {
            return Err(PyValueError::new_err("the dump is empty"));
        }
        Ok(analyzer_for(dump, true)
            .search_encryption_keys(&dump.data)
            .iter()
            .map(KeyCandidate::from)
            .collect())
    }

    /// A Markdown report for a dump.
    #[staticmethod]
    #[pyo3(signature = (dump, deep_scan=false))]
    fn generate_report(dump: &Dump, deep_scan: bool) -> PyResult<String> {
        if dump.data.is_empty() {
            return Err(PyValueError::new_err("the dump is empty"));
        }
        let analyzer = analyzer_for(dump, deep_scan);
        let result = analyzer.analyze(&dump.data);
        Ok(analyzer.generate_report(&result))
    }
}

impl From<&openflash_core::ai::DetectedPattern> for Pattern {
    fn from(pattern: &openflash_core::ai::DetectedPattern) -> Self {
        Self {
            pattern_type: format!("{:?}", pattern.pattern_type),
            offset: pattern.start_offset as u64,
            size: pattern.end_offset.saturating_sub(pattern.start_offset) as u64,
            confidence: pattern.confidence.to_score(),
        }
    }
}

impl From<&openflash_core::ai::FilesystemInfo> for Filesystem {
    fn from(filesystem: &openflash_core::ai::FilesystemInfo) -> Self {
        Self {
            fs_type: format!("{:?}", filesystem.fs_type),
            offset: filesystem.offset as u64,
            size: filesystem.size.map(|size| size as u64),
        }
    }
}

impl From<&openflash_core::ai::Anomaly> for Anomaly {
    fn from(anomaly: &openflash_core::ai::Anomaly) -> Self {
        Self {
            anomaly_type: format!("{:?}", anomaly.severity),
            severity: format!("{:?}", anomaly.severity),
            offset: anomaly.location.unwrap_or(0) as u64,
            description: anomaly.description.clone(),
        }
    }
}

impl From<&openflash_core::ai::KeyCandidate> for KeyCandidate {
    fn from(candidate: &openflash_core::ai::KeyCandidate) -> Self {
        Self {
            key_type: candidate.key_type.clone(),
            offset: candidate.offset as u64,
            confidence: candidate.confidence.to_score(),
        }
    }
}

/// Analysis result
#[pyclass]
#[derive(Clone)]
struct AnalysisResult {
    #[pyo3(get)]
    quality_score: f32,
    #[pyo3(get)]
    encryption_probability: f32,
    #[pyo3(get)]
    compression_probability: f32,
    #[pyo3(get)]
    patterns: Vec<Pattern>,
    #[pyo3(get)]
    filesystems: Vec<Filesystem>,
    #[pyo3(get)]
    anomalies: Vec<Anomaly>,
    #[pyo3(get)]
    summary: String,
}

#[pymethods]
impl AnalysisResult {
    /// Export analysis report
    #[pyo3(signature = (path, format="md"))]
    fn export_report(&self, path: &str, format: &str) -> PyResult<()> {
        let content = match format {
            "json" => serde_json::json!({
                "quality_score": self.quality_score,
                "encryption_probability": self.encryption_probability,
                "compression_probability": self.compression_probability,
                "summary": self.summary,
            })
            .to_string(),
            "html" => format!(
                "<html><body><h1>OpenFlash Report</h1><p>{}</p></body></html>",
                self.summary
            ),
            _ => format!(
                "# OpenFlash Analysis Report\n\n## Summary\n{}\n\n## Quality: {:.0}%\n",
                self.summary,
                self.quality_score * 100.0
            ),
        };
        std::fs::write(path, content).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AnalysisResult(quality={:.0}%, patterns={})",
            self.quality_score * 100.0,
            self.patterns.len()
        )
    }
}

/// Detected pattern
#[pyclass]
#[derive(Clone)]
struct Pattern {
    #[pyo3(get)]
    pattern_type: String,
    #[pyo3(get)]
    offset: u64,
    #[pyo3(get)]
    size: u64,
    #[pyo3(get)]
    confidence: f32,
}

#[pymethods]
impl Pattern {
    fn __repr__(&self) -> String {
        format!("Pattern({} @ 0x{:X})", self.pattern_type, self.offset)
    }
}

/// Detected filesystem
#[pyclass]
#[derive(Clone)]
struct Filesystem {
    #[pyo3(get)]
    fs_type: String,
    #[pyo3(get)]
    offset: u64,
    #[pyo3(get)]
    size: Option<u64>,
}

/// Detected anomaly
#[pyclass]
#[derive(Clone)]
struct Anomaly {
    #[pyo3(get)]
    anomaly_type: String,
    #[pyo3(get)]
    severity: String,
    #[pyo3(get)]
    offset: u64,
    #[pyo3(get)]
    description: String,
}

/// Key candidate
#[pyclass]
#[derive(Clone)]
struct KeyCandidate {
    #[pyo3(get)]
    key_type: String,
    #[pyo3(get)]
    offset: u64,
    #[pyo3(get)]
    confidence: f32,
}

// ============================================================================
// Batch Processing
// ============================================================================

/// Batch processor
#[pyclass]
struct Batch {
    jobs: Vec<BatchJobPy>,
    stop_on_error: bool,
}

#[pymethods]
impl Batch {
    #[new]
    fn new() -> Self {
        Self {
            jobs: vec![],
            stop_on_error: false,
        }
    }

    /// Add read job
    fn add_read(&mut self, name: &str, output: &str) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy {
            id,
            name: name.into(),
            job_type: "read".into(),
            output: Some(output.into()),
            input: None,
            depends_on: vec![],
        });
        id
    }

    /// Add write job
    fn add_write(&mut self, name: &str, input: &str) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy {
            id,
            name: name.into(),
            job_type: "write".into(),
            output: None,
            input: Some(input.into()),
            depends_on: vec![],
        });
        id
    }

    /// Add analysis job
    fn add_analyze(&mut self, name: &str, depends_on: usize) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy {
            id,
            name: name.into(),
            job_type: "analyze".into(),
            output: None,
            input: None,
            depends_on: vec![depends_on],
        });
        id
    }

    /// Add report job
    fn add_report(&mut self, name: &str, output: &str, depends_on: usize) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy {
            id,
            name: name.into(),
            job_type: "report".into(),
            output: Some(output.into()),
            input: None,
            depends_on: vec![depends_on],
        });
        id
    }

    /// Set stop on error
    fn set_stop_on_error(&mut self, stop: bool) {
        self.stop_on_error = stop;
    }

    /// Run all jobs.
    ///
    /// Not implemented: no runner executes these job descriptions. The previous
    /// implementation reported every job as having completed successfully in
    /// 100 ms without doing anything, so a script could not tell.
    fn run(&self, _device: &Device) -> PyResult<Vec<BatchResultPy>> {
        Err(PyRuntimeError::new_err(
            "batch execution is not implemented: the job types exist but no runner \
             executes them. Drive the Device methods from Python instead.",
        ))
    }

    /// Get job count
    fn __len__(&self) -> usize {
        self.jobs.len()
    }
}

#[pyclass]
#[derive(Clone)]
struct BatchJobPy {
    #[pyo3(get)]
    id: usize,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    job_type: String,
    #[pyo3(get)]
    output: Option<String>,
    #[pyo3(get)]
    input: Option<String>,
    #[pyo3(get)]
    depends_on: Vec<usize>,
}

#[pyclass]
#[derive(Clone)]
struct BatchResultPy {
    #[pyo3(get)]
    job_id: usize,
    #[pyo3(get)]
    success: bool,
    #[pyo3(get)]
    duration_ms: u64,
    #[pyo3(get)]
    message: String,
}

// ============================================================================
// Module Functions
// ============================================================================

/// List attached USB devices.
///
/// An empty list means nothing is plugged in. This used to return one invented
/// RP2040 at /dev/ttyACM0 whatever was connected.
#[pyfunction]
fn scan() -> PyResult<Vec<String>> {
    #[cfg(feature = "usb")]
    {
        openflash_core::transport::list_devices()
            .map(|devices| devices.into_iter().map(|d| d.selector()).collect())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
    #[cfg(not(feature = "usb"))]
    {
        Err(PyRuntimeError::new_err(
            "this build was compiled without USB support",
        ))
    }
}

/// Connect to a USB device.
///
/// With no argument, the single attached device is used, and it is an error if
/// none or several are attached.
#[pyfunction]
#[pyo3(signature = (device=None))]
fn connect(device: Option<&str>) -> PyResult<Device> {
    let target = match device {
        Some(selector) => ConnectionTarget::Usb(selector.to_string()),
        None => ConnectionTarget::AutoUsb,
    };
    Device::open(ConnectionConfig {
        target,
        ..ConnectionConfig::default()
    })
}

/// Connect to an SBC agent over TCP, as `host:port`.
#[pyfunction]
fn connect_tcp(endpoint: &str) -> PyResult<Device> {
    Device::open(ConnectionConfig {
        target: ConnectionTarget::Tcp(endpoint.to_string()),
        ..ConnectionConfig::default()
    })
}

/// Connect to an SBC agent over a Unix socket.
#[pyfunction]
fn connect_unix(path: &str) -> PyResult<Device> {
    Device::open(ConnectionConfig {
        target: ConnectionTarget::Unix(path.to_string()),
        ..ConnectionConfig::default()
    })
}

/// Connect to the in-process emulator, backed by a chip of `size` bytes.
///
/// No hardware is involved and nothing real is read or written. `size` must be a
/// power of two of at least 4096 bytes.
#[pyfunction]
#[pyo3(signature = (size=2 * 1024 * 1024))]
fn connect_emulated(size: usize) -> PyResult<Device> {
    if size < 4096 || !size.is_power_of_two() {
        return Err(PyValueError::new_err(
            "an emulated chip size must be a power of two of at least 4096 bytes",
        ));
    }
    Device::open(ConnectionConfig::emulated(size))
}

/// The protocol revision these bindings speak.
#[pyfunction]
fn protocol_version() -> u8 {
    openflash_core::protocol::PROTOCOL_VERSION
}

/// Load dump from file
#[pyfunction]
fn load_dump(path: &str) -> PyResult<Dump> {
    let data = std::fs::read(path).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(Dump {
        data,
        oob_data: None,
        chip_info: None,
        bad_blocks: vec![],
    })
}

/// Compare two dumps
#[pyfunction]
fn compare_dumps(dump1: &Dump, dump2: &Dump) -> PyResult<CompareResult> {
    let mut diffs = 0;
    let min_len = dump1.data.len().min(dump2.data.len());
    for i in 0..min_len {
        if dump1.data[i] != dump2.data[i] {
            diffs += 1;
        }
    }
    diffs += (dump1.data.len() as i64 - dump2.data.len() as i64).unsigned_abs() as usize;

    let similarity = 1.0 - (diffs as f64 / dump1.data.len().max(dump2.data.len()) as f64);

    Ok(CompareResult {
        size1: dump1.data.len(),
        size2: dump2.data.len(),
        differences: diffs,
        similarity,
    })
}

#[pyclass]
#[derive(Clone)]
struct CompareResult {
    #[pyo3(get)]
    size1: usize,
    #[pyo3(get)]
    size2: usize,
    #[pyo3(get)]
    differences: usize,
    #[pyo3(get)]
    similarity: f64,
}

/// Get library version
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// List supported chips
#[pyfunction]
#[pyo3(signature = (interface=None))]
fn list_chips(interface: Option<&str>) -> PyResult<Vec<ChipInfo>> {
    let chips = vec![
        ChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F1G08U0E".into(),
            capacity: 128 * 1024 * 1024,
            page_size: 2048,
            block_size: 128 * 1024,
            oob_size: 64,
            interface: "parallel_nand".into(),
        },
        ChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F1GQ4U".into(),
            capacity: 128 * 1024 * 1024,
            page_size: 2048,
            block_size: 128 * 1024,
            oob_size: 64,
            interface: "spi_nand".into(),
        },
        ChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q128JV".into(),
            capacity: 16 * 1024 * 1024,
            page_size: 256,
            block_size: 64 * 1024,
            oob_size: 0,
            interface: "spi_nor".into(),
        },
    ];

    Ok(if let Some(iface) = interface {
        chips
            .into_iter()
            .filter(|c| c.interface.contains(iface))
            .collect()
    } else {
        chips
    })
}

// ============================================================================
// Python Module
// ============================================================================

/// OpenFlash Python module
#[pymodule]
fn openflash(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan, m)?)?;
    m.add_function(wrap_pyfunction!(connect, m)?)?;
    m.add_function(wrap_pyfunction!(connect_tcp, m)?)?;
    m.add_function(wrap_pyfunction!(connect_unix, m)?)?;
    m.add_function(wrap_pyfunction!(connect_emulated, m)?)?;
    m.add_function(wrap_pyfunction!(protocol_version, m)?)?;
    m.add_function(wrap_pyfunction!(load_dump, m)?)?;
    m.add_function(wrap_pyfunction!(compare_dumps, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(list_chips, m)?)?;

    m.add_class::<Device>()?;
    m.add_class::<Dump>()?;
    m.add_class::<ChipInfo>()?;
    m.add_class::<AnalysisResult>()?;
    m.add_class::<Pattern>()?;
    m.add_class::<Batch>()?;
    m.add_class::<AiModule>()?;

    // Add ai submodule
    let ai = PyModule::new(m.py(), "ai")?;
    ai.add_class::<AiModule>()?;
    ai.add_function(wrap_pyfunction!(ai_analyze, &ai)?)?;
    ai.add_function(wrap_pyfunction!(ai_detect_patterns, &ai)?)?;
    ai.add_function(wrap_pyfunction!(ai_search_keys, &ai)?)?;
    ai.add_function(wrap_pyfunction!(ai_generate_report, &ai)?)?;
    m.add_submodule(&ai)?;
    // Registered in sys.modules as well, so `import openflash.ai` works and not
    // only attribute access on the parent module.
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("openflash.ai", &ai)?;

    Ok(())
}

/// `openflash.ai.analyze(dump)`
#[pyfunction]
#[pyo3(name = "analyze", signature = (dump, deep_scan=false))]
fn ai_analyze(dump: &Dump, deep_scan: bool) -> PyResult<AnalysisResult> {
    AiModule::analyze(dump, deep_scan)
}

/// `openflash.ai.detect_patterns(dump)`
#[pyfunction]
#[pyo3(name = "detect_patterns")]
fn ai_detect_patterns(dump: &Dump) -> PyResult<Vec<Pattern>> {
    AiModule::detect_patterns(dump)
}

/// `openflash.ai.search_keys(dump)`
#[pyfunction]
#[pyo3(name = "search_keys")]
fn ai_search_keys(dump: &Dump) -> PyResult<Vec<KeyCandidate>> {
    AiModule::search_keys(dump)
}

/// `openflash.ai.generate_report(dump)`
#[pyfunction]
#[pyo3(name = "generate_report", signature = (dump, deep_scan=false))]
fn ai_generate_report(dump: &Dump, deep_scan: bool) -> PyResult<String> {
    AiModule::generate_report(dump, deep_scan)
}
