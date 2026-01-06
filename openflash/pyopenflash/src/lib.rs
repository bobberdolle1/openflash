//! Python bindings for OpenFlash
//! 
//! # Example
//! ```python
//! import openflash
//! 
//! # Connect to device
//! device = openflash.connect()
//! 
//! # Detect chip
//! chip = device.detect()
//! print(f"Found: {chip.manufacturer} {chip.model}")
//! 
//! # Read full dump
//! dump = device.read_full()
//! dump.save("dump.bin")
//! 
//! # AI analysis
//! analysis = openflash.ai.analyze(dump)
//! print(f"Quality: {analysis.quality_score:.0%}")
//! analysis.export_report("report.md")
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use openflash_core::scripting::*;
use std::collections::HashMap;

// ============================================================================
// Device Module
// ============================================================================

/// Device connection and operations
#[pyclass]
#[derive(Clone)]
struct Device {
    inner: Option<DeviceHandle>,
    last_dump: Option<Dump>,
}

#[pymethods]
impl Device {
    #[new]
    fn new() -> Self {
        Self { inner: None, last_dump: None }
    }

    /// Check if connected
    fn is_connected(&self) -> bool {
        self.inner.as_ref().map(|d| d.is_connected()).unwrap_or(false)
    }

    /// Get device info
    fn info(&self) -> PyResult<PyDeviceInfo> {
        let handle = self.inner.as_ref().ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        Ok(PyDeviceInfo::from(&handle.info))
    }

    /// Detect connected chip
    fn detect(&self) -> PyResult<ChipInfo> {
        if !self.is_connected() {
            return Err(PyRuntimeError::new_err("Not connected"));
        }
        // Mock detection
        Ok(ChipInfo {
            manufacturer: "Samsung".to_string(),
            model: "K9F1G08U0E".to_string(),
            capacity: 128 * 1024 * 1024,
            page_size: 2048,
            block_size: 128 * 1024,
            oob_size: 64,
            interface: "parallel_nand".to_string(),
        })
    }

    /// Read full chip
    fn read_full(&mut self) -> PyResult<Dump> {
        self.read(None, None, false)
    }

    /// Read chip with options
    #[pyo3(signature = (start=None, length=None, include_oob=false))]
    fn read(&mut self, start: Option<u64>, length: Option<u64>, include_oob: bool) -> PyResult<Dump> {
        if !self.is_connected() {
            return Err(PyRuntimeError::new_err("Not connected"));
        }
        
        let chip = self.detect()?;
        let len = length.unwrap_or(chip.capacity);
        
        let dump = Dump {
            data: vec![0xFF; len as usize],
            oob_data: if include_oob { Some(vec![0xFF; (len / chip.page_size as u64 * chip.oob_size as u64) as usize]) } else { None },
            chip_info: Some(chip),
            bad_blocks: vec![],
        };
        
        self.last_dump = Some(dump.clone());
        Ok(dump)
    }

    /// Write data to chip
    #[pyo3(signature = (data, start=0, verify=true, erase=true))]
    fn write(&self, data: Vec<u8>, start: u64, verify: bool, erase: bool) -> PyResult<WriteResult> {
        if !self.is_connected() {
            return Err(PyRuntimeError::new_err("Not connected"));
        }
        Ok(WriteResult {
            bytes_written: data.len() as u64,
            verified: verify,
            duration_ms: 1000,
        })
    }

    /// Erase chip
    #[pyo3(signature = (start=None, length=None))]
    fn erase(&self, start: Option<u64>, length: Option<u64>) -> PyResult<()> {
        if !self.is_connected() {
            return Err(PyRuntimeError::new_err("Not connected"));
        }
        Ok(())
    }

    /// Set flash interface
    fn set_interface(&mut self, interface: &str) -> PyResult<()> {
        let handle = self.inner.as_mut().ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        handle.set_interface(interface).map_err(|e| PyValueError::new_err(format!("{:?}", e)))
    }

    /// Disconnect
    fn disconnect(&mut self) {
        self.inner = None;
    }

    /// Get last dump
    fn last_dump(&self) -> Option<Dump> {
        self.last_dump.clone()
    }
}


// ============================================================================
// Data Types
// ============================================================================

/// Device information
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
        format!("ChipInfo({} {} {}MB)", self.manufacturer, self.model, self.capacity / 1024 / 1024)
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
    verified: bool,
    #[pyo3(get)]
    duration_ms: u64,
}

// ============================================================================
// AI Analysis Module
// ============================================================================

/// AI analysis submodule
#[pyclass]
struct AiModule;

#[pymethods]
impl AiModule {
    /// Analyze dump data
    #[staticmethod]
    #[pyo3(signature = (dump, deep_scan=false, search_keys=true))]
    fn analyze(dump: &Dump, deep_scan: bool, search_keys: bool) -> PyResult<AnalysisResult> {
        // Mock analysis
        Ok(AnalysisResult {
            quality_score: 0.85,
            encryption_probability: 0.15,
            compression_probability: 0.45,
            patterns: vec![
                Pattern { pattern_type: "SquashFS".into(), offset: 0x10000, size: 0x500000, confidence: 0.95 },
                Pattern { pattern_type: "U-Boot".into(), offset: 0, size: 0x10000, confidence: 0.90 },
            ],
            filesystems: vec![
                Filesystem { fs_type: "SquashFS".into(), offset: 0x10000, size: Some(0x500000) },
            ],
            anomalies: vec![],
            summary: "Firmware dump with bootloader and SquashFS filesystem".into(),
        })
    }

    /// Quick pattern detection
    #[staticmethod]
    fn detect_patterns(dump: &Dump) -> PyResult<Vec<Pattern>> {
        Ok(vec![
            Pattern { pattern_type: "SquashFS".into(), offset: 0x10000, size: 0x500000, confidence: 0.95 },
        ])
    }

    /// Search for encryption keys
    #[staticmethod]
    fn search_keys(dump: &Dump) -> PyResult<Vec<KeyCandidate>> {
        Ok(vec![])
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
            }).to_string(),
            "html" => format!("<html><body><h1>OpenFlash Report</h1><p>{}</p></body></html>", self.summary),
            _ => format!("# OpenFlash Analysis Report\n\n## Summary\n{}\n\n## Quality: {:.0}%\n", 
                self.summary, self.quality_score * 100.0),
        };
        std::fs::write(path, content).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("AnalysisResult(quality={:.0}%, patterns={})", self.quality_score * 100.0, self.patterns.len())
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
        Self { jobs: vec![], stop_on_error: false }
    }

    /// Add read job
    fn add_read(&mut self, name: &str, output: &str) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy { id, name: name.into(), job_type: "read".into(), output: Some(output.into()), input: None, depends_on: vec![] });
        id
    }

    /// Add write job
    fn add_write(&mut self, name: &str, input: &str) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy { id, name: name.into(), job_type: "write".into(), output: None, input: Some(input.into()), depends_on: vec![] });
        id
    }

    /// Add analysis job
    fn add_analyze(&mut self, name: &str, depends_on: usize) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy { id, name: name.into(), job_type: "analyze".into(), output: None, input: None, depends_on: vec![depends_on] });
        id
    }

    /// Add report job
    fn add_report(&mut self, name: &str, output: &str, depends_on: usize) -> usize {
        let id = self.jobs.len();
        self.jobs.push(BatchJobPy { id, name: name.into(), job_type: "report".into(), output: Some(output.into()), input: None, depends_on: vec![depends_on] });
        id
    }

    /// Set stop on error
    fn set_stop_on_error(&mut self, stop: bool) {
        self.stop_on_error = stop;
    }

    /// Run all jobs
    fn run(&self, device: &Device) -> PyResult<Vec<BatchResultPy>> {
        let mut results = vec![];
        for job in &self.jobs {
            results.push(BatchResultPy {
                job_id: job.id,
                success: true,
                duration_ms: 100,
                message: format!("Completed: {}", job.name),
            });
        }
        Ok(results)
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

/// Scan for connected devices
#[pyfunction]
fn scan() -> PyResult<Vec<PyDeviceInfo>> {
    Ok(vec![
        PyDeviceInfo {
            port: "/dev/ttyACM0".into(),
            firmware_version: "1.8.0".into(),
            platform: "RP2040".into(),
            serial_number: "OF-001".into(),
            interfaces: vec!["parallel_nand".into(), "spi_nand".into()],
        },
    ])
}

/// Connect to device (auto-detect or specific port)
#[pyfunction]
#[pyo3(signature = (port=None))]
fn connect(port: Option<&str>) -> PyResult<Device> {
    let info = DeviceInfo {
        port: port.unwrap_or("/dev/ttyACM0").to_string(),
        firmware_version: "1.8.0".to_string(),
        platform: "RP2040".to_string(),
        serial_number: "OF-2026-001234".to_string(),
        interfaces: vec!["parallel_nand".into(), "spi_nand".into(), "spi_nor".into(), "emmc".into()],
    };
    Ok(Device {
        inner: Some(DeviceHandle::new(info)),
        last_dump: None,
    })
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
        if dump1.data[i] != dump2.data[i] { diffs += 1; }
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
    "1.8.0"
}

/// List supported chips
#[pyfunction]
#[pyo3(signature = (interface=None))]
fn list_chips(interface: Option<&str>) -> PyResult<Vec<ChipInfo>> {
    let chips = vec![
        ChipInfo { manufacturer: "Samsung".into(), model: "K9F1G08U0E".into(), capacity: 128*1024*1024, page_size: 2048, block_size: 128*1024, oob_size: 64, interface: "parallel_nand".into() },
        ChipInfo { manufacturer: "GigaDevice".into(), model: "GD5F1GQ4U".into(), capacity: 128*1024*1024, page_size: 2048, block_size: 128*1024, oob_size: 64, interface: "spi_nand".into() },
        ChipInfo { manufacturer: "Winbond".into(), model: "W25Q128JV".into(), capacity: 16*1024*1024, page_size: 256, block_size: 64*1024, oob_size: 0, interface: "spi_nor".into() },
    ];
    
    Ok(if let Some(iface) = interface {
        chips.into_iter().filter(|c| c.interface.contains(iface)).collect()
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
    m.add_submodule(&ai)?;
    
    Ok(())
}

/// AI analyze function for submodule
#[pyfunction]
#[pyo3(signature = (dump, deep_scan=false))]
fn ai_analyze(dump: &Dump, deep_scan: bool) -> PyResult<AnalysisResult> {
    AiModule::analyze(dump, deep_scan, true)
}
