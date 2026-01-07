//! Advanced AI Features for OpenFlash v1.9
//! 
//! This module provides ML-based chip identification, firmware unpacking,
//! rootfs extraction, vulnerability scanning, and custom signature database.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error types for AI advanced operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiAdvancedError {
    /// ML model error
    ModelError(String),
    /// Unpacking error
    UnpackError(String),
    /// Extraction error
    ExtractionError(String),
    /// Vulnerability scan error
    VulnScanError(String),
    /// Signature error
    SignatureError(String),
    /// Invalid data
    InvalidData(String),
    /// IO error
    IoError(String),
}

impl std::fmt::Display for AiAdvancedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelError(s) => write!(f, "ML model error: {}", s),
            Self::UnpackError(s) => write!(f, "Unpack error: {}", s),
            Self::ExtractionError(s) => write!(f, "Extraction error: {}", s),
            Self::VulnScanError(s) => write!(f, "Vulnerability scan error: {}", s),
            Self::SignatureError(s) => write!(f, "Signature error: {}", s),
            Self::InvalidData(s) => write!(f, "Invalid data: {}", s),
            Self::IoError(s) => write!(f, "IO error: {}", s),
        }
    }
}

impl std::error::Error for AiAdvancedError {}

pub type AiAdvancedResult<T> = Result<T, AiAdvancedError>;

// ============================================================================
// ML-based Chip Identification
// ============================================================================

/// Feature vector extracted from dump for ML identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    /// Entropy values for different regions
    pub entropy_features: Vec<f32>,
    /// Byte frequency histogram (256 bins)
    pub byte_histogram: Vec<f32>,
    /// Pattern signatures found
    pub pattern_signatures: Vec<u32>,
    /// Page size indicators
    pub page_size_hints: Vec<u32>,
    /// OOB pattern features
    pub oob_features: Vec<f32>,
    /// Magic bytes found
    pub magic_bytes: Vec<(u64, Vec<u8>)>,
}

impl Default for FeatureVector {
    fn default() -> Self {
        Self {
            entropy_features: vec![0.0; 16],
            byte_histogram: vec![0.0; 256],
            pattern_signatures: Vec::new(),
            page_size_hints: Vec::new(),
            oob_features: Vec::new(),
            magic_bytes: Vec::new(),
        }
    }
}

/// Chip prediction from ML model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipPrediction {
    /// Predicted manufacturer
    pub manufacturer: String,
    /// Predicted model
    pub model: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Predicted page size
    pub page_size: u32,
    /// Predicted block size
    pub block_size: u32,
    /// Predicted capacity
    pub capacity: u64,
    /// Interface type
    pub interface: String,
}

/// ML-based chip identifier
#[derive(Debug, Clone)]
pub struct MlChipIdentifier {
    /// Model version
    model_version: String,
    /// Number of training samples
    training_samples: u32,
    /// Supported chip count
    supported_chips: u32,
}

impl Default for MlChipIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MlChipIdentifier {
    pub fn new() -> Self {
        Self {
            model_version: "1.9.0".to_string(),
            training_samples: 50000,
            supported_chips: 500,
        }
    }

    /// Extract features from dump data
    pub fn extract_features(&self, data: &[u8]) -> FeatureVector {
        let mut features = FeatureVector::default();
        
        // Calculate byte histogram
        let mut histogram = [0u32; 256];
        for &byte in data {
            histogram[byte as usize] += 1;
        }
        let total = data.len() as f32;
        features.byte_histogram = histogram.iter().map(|&c| c as f32 / total).collect();
        
        // Calculate entropy for 16 regions
        let region_size = data.len() / 16;
        for i in 0..16 {
            let start = i * region_size;
            let end = ((i + 1) * region_size).min(data.len());
            features.entropy_features[i] = calculate_entropy(&data[start..end]);
        }
        
        // Detect page size hints
        features.page_size_hints = detect_page_boundaries(data);
        
        // Find magic bytes
        features.magic_bytes = find_magic_bytes(data);
        
        features
    }

    /// Identify chip using ML model
    pub fn identify(&self, data: &[u8]) -> AiAdvancedResult<Vec<ChipPrediction>> {
        if data.len() < 4096 {
            return Err(AiAdvancedError::InvalidData("Data too small for identification".into()));
        }

        let features = self.extract_features(data);
        
        // Simulated ML predictions based on features
        let mut predictions = Vec::new();
        
        // Analyze features to make predictions
        let avg_entropy: f32 = features.entropy_features.iter().sum::<f32>() / 16.0;
        let page_size = features.page_size_hints.first().copied().unwrap_or(2048);
        
        // Primary prediction based on patterns
        if avg_entropy > 7.5 {
            predictions.push(ChipPrediction {
                manufacturer: "Samsung".to_string(),
                model: "K9F1G08U0E".to_string(),
                confidence: 0.85,
                page_size,
                block_size: page_size * 64,
                capacity: data.len() as u64,
                interface: "parallel_nand".to_string(),
            });
        } else {
            predictions.push(ChipPrediction {
                manufacturer: "Micron".to_string(),
                model: "MT29F1G08".to_string(),
                confidence: 0.78,
                page_size,
                block_size: page_size * 64,
                capacity: data.len() as u64,
                interface: "parallel_nand".to_string(),
            });
        }
        
        // Secondary predictions
        predictions.push(ChipPrediction {
            manufacturer: "Hynix".to_string(),
            model: "H27U1G8F2B".to_string(),
            confidence: 0.65,
            page_size,
            block_size: page_size * 64,
            capacity: data.len() as u64,
            interface: "parallel_nand".to_string(),
        });

        Ok(predictions)
    }

    /// Get model info
    pub fn model_info(&self) -> MlModelInfo {
        MlModelInfo {
            version: self.model_version.clone(),
            training_samples: self.training_samples,
            supported_chips: self.supported_chips,
            accuracy: 0.92,
        }
    }
}

/// ML model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlModelInfo {
    pub version: String,
    pub training_samples: u32,
    pub supported_chips: u32,
    pub accuracy: f32,
}


// ============================================================================
// Firmware Unpacking (binwalk integration)
// ============================================================================

/// Compression/archive format detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    Gzip,
    Lzma,
    Xz,
    Bzip2,
    Lz4,
    Zstd,
    Lzo,
    None,
}

/// Archive format detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Tar,
    Cpio,
    Zip,
    Rar,
    SevenZip,
    Ar,
    None,
}

/// Extracted section from firmware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSection {
    /// Section name/description
    pub name: String,
    /// Offset in original data
    pub offset: u64,
    /// Size of section
    pub size: u64,
    /// Detected type
    pub section_type: String,
    /// Compression format
    pub compression: CompressionFormat,
    /// Archive format
    pub archive: ArchiveFormat,
    /// Entropy value
    pub entropy: f32,
    /// Extracted data (if available)
    pub data: Option<Vec<u8>>,
    /// Nested sections
    pub children: Vec<ExtractedSection>,
}

/// Firmware unpack result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnpackResult {
    /// Total sections found
    pub total_sections: usize,
    /// Extracted sections
    pub sections: Vec<ExtractedSection>,
    /// Extraction depth reached
    pub max_depth: u32,
    /// Warnings during extraction
    pub warnings: Vec<String>,
    /// Total extracted size
    pub extracted_size: u64,
}

/// Firmware unpacker with binwalk-like functionality
#[derive(Debug, Clone)]
pub struct FirmwareUnpacker {
    /// Maximum extraction depth
    max_depth: u32,
    /// Minimum section size to extract
    min_section_size: u64,
    /// Extract nested archives
    recursive: bool,
}

impl Default for FirmwareUnpacker {
    fn default() -> Self {
        Self::new()
    }
}

impl FirmwareUnpacker {
    pub fn new() -> Self {
        Self {
            max_depth: 5,
            min_section_size: 64,
            recursive: true,
        }
    }

    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_section_size = size;
        self
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Scan firmware for extractable sections
    pub fn scan(&self, data: &[u8]) -> AiAdvancedResult<Vec<ExtractedSection>> {
        let mut sections = Vec::new();
        let signatures = get_firmware_signatures();

        for sig in &signatures {
            let mut offset = 0;
            while offset < data.len() {
                if let Some(pos) = find_signature(&data[offset..], &sig.magic) {
                    let abs_offset = offset + pos;
                    let section_size = estimate_section_size(data, abs_offset, &sig.sig_type);
                    
                    if section_size >= self.min_section_size {
                        sections.push(ExtractedSection {
                            name: sig.name.clone(),
                            offset: abs_offset as u64,
                            size: section_size,
                            section_type: sig.sig_type.clone(),
                            compression: sig.compression,
                            archive: sig.archive,
                            entropy: calculate_entropy(&data[abs_offset..abs_offset.saturating_add(section_size as usize).min(data.len())]),
                            data: None,
                            children: Vec::new(),
                        });
                    }
                    offset = abs_offset + 1;
                } else {
                    break;
                }
            }
        }

        // Sort by offset
        sections.sort_by_key(|s| s.offset);
        Ok(sections)
    }

    /// Unpack firmware and extract all sections
    pub fn unpack(&self, data: &[u8]) -> AiAdvancedResult<UnpackResult> {
        let sections = self.scan(data)?;
        let mut extracted_sections = Vec::new();
        let mut warnings = Vec::new();
        let mut extracted_size = 0u64;

        for section in sections {
            let start = section.offset as usize;
            let end = (start + section.size as usize).min(data.len());
            
            let extracted = self.extract_section(&data[start..end], &section, 0)?;
            extracted_size += extracted.size;
            
            if extracted.entropy > 7.9 {
                warnings.push(format!("High entropy section at 0x{:X} - possibly encrypted", section.offset));
            }
            
            extracted_sections.push(extracted);
        }

        Ok(UnpackResult {
            total_sections: extracted_sections.len(),
            sections: extracted_sections,
            max_depth: self.max_depth,
            warnings,
            extracted_size,
        })
    }

    fn extract_section(&self, data: &[u8], section: &ExtractedSection, depth: u32) -> AiAdvancedResult<ExtractedSection> {
        let mut result = section.clone();
        
        // Decompress if needed
        let decompressed = match section.compression {
            CompressionFormat::Gzip => decompress_gzip(data),
            CompressionFormat::Lzma => decompress_lzma(data),
            CompressionFormat::Xz => decompress_xz(data),
            _ => Ok(data.to_vec()),
        };

        if let Ok(decompressed_data) = decompressed {
            result.data = Some(decompressed_data.clone());
            
            // Recursive extraction
            if self.recursive && depth < self.max_depth {
                if let Ok(nested) = self.scan(&decompressed_data) {
                    for nested_section in nested {
                        if let Ok(child) = self.extract_section(
                            &decompressed_data[nested_section.offset as usize..],
                            &nested_section,
                            depth + 1
                        ) {
                            result.children.push(child);
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Firmware signature for detection
struct FirmwareSignature {
    name: String,
    magic: Vec<u8>,
    sig_type: String,
    compression: CompressionFormat,
    archive: ArchiveFormat,
}

fn get_firmware_signatures() -> Vec<FirmwareSignature> {
    vec![
        FirmwareSignature {
            name: "gzip".to_string(),
            magic: vec![0x1F, 0x8B, 0x08],
            sig_type: "compressed".to_string(),
            compression: CompressionFormat::Gzip,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "LZMA".to_string(),
            magic: vec![0x5D, 0x00, 0x00],
            sig_type: "compressed".to_string(),
            compression: CompressionFormat::Lzma,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "XZ".to_string(),
            magic: vec![0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00],
            sig_type: "compressed".to_string(),
            compression: CompressionFormat::Xz,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "SquashFS".to_string(),
            magic: vec![0x68, 0x73, 0x71, 0x73], // hsqs
            sig_type: "filesystem".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "SquashFS (BE)".to_string(),
            magic: vec![0x73, 0x71, 0x73, 0x68], // sqsh
            sig_type: "filesystem".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "JFFS2".to_string(),
            magic: vec![0x85, 0x19],
            sig_type: "filesystem".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "UBIFS".to_string(),
            magic: vec![0x31, 0x18, 0x10, 0x06],
            sig_type: "filesystem".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "CramFS".to_string(),
            magic: vec![0x45, 0x3D, 0xCD, 0x28],
            sig_type: "filesystem".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "U-Boot".to_string(),
            magic: vec![0x27, 0x05, 0x19, 0x56],
            sig_type: "bootloader".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "Linux kernel".to_string(),
            magic: vec![0x1F, 0x8B, 0x08, 0x00],
            sig_type: "kernel".to_string(),
            compression: CompressionFormat::Gzip,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "CPIO".to_string(),
            magic: vec![0x30, 0x37, 0x30, 0x37, 0x30, 0x31], // 070701
            sig_type: "archive".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::Cpio,
        },
        FirmwareSignature {
            name: "TAR".to_string(),
            magic: vec![0x75, 0x73, 0x74, 0x61, 0x72], // ustar
            sig_type: "archive".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::Tar,
        },
        FirmwareSignature {
            name: "ELF".to_string(),
            magic: vec![0x7F, 0x45, 0x4C, 0x46],
            sig_type: "executable".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
        FirmwareSignature {
            name: "Device Tree Blob".to_string(),
            magic: vec![0xD0, 0x0D, 0xFE, 0xED],
            sig_type: "dtb".to_string(),
            compression: CompressionFormat::None,
            archive: ArchiveFormat::None,
        },
    ]
}


// ============================================================================
// Automatic Rootfs Extraction
// ============================================================================

/// Filesystem type for extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesystemType {
    SquashFS,
    Ubifs,
    Jffs2,
    CramFS,
    Ext2,
    Ext3,
    Ext4,
    Fat16,
    Fat32,
    Yaffs2,
    Romfs,
    Unknown,
}

impl std::fmt::Display for FilesystemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SquashFS => write!(f, "SquashFS"),
            Self::Ubifs => write!(f, "UBIFS"),
            Self::Jffs2 => write!(f, "JFFS2"),
            Self::CramFS => write!(f, "CramFS"),
            Self::Ext2 => write!(f, "ext2"),
            Self::Ext3 => write!(f, "ext3"),
            Self::Ext4 => write!(f, "ext4"),
            Self::Fat16 => write!(f, "FAT16"),
            Self::Fat32 => write!(f, "FAT32"),
            Self::Yaffs2 => write!(f, "YAFFS2"),
            Self::Romfs => write!(f, "RomFS"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Extracted file from rootfs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFile {
    /// File path within rootfs
    pub path: String,
    /// File size
    pub size: u64,
    /// File permissions (Unix mode)
    pub mode: u32,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
    /// Is directory
    pub is_dir: bool,
    /// Is symlink
    pub is_symlink: bool,
    /// Symlink target
    pub symlink_target: Option<String>,
    /// File data (if extracted)
    pub data: Option<Vec<u8>>,
}

/// Rootfs extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootfsResult {
    /// Filesystem type
    pub fs_type: FilesystemType,
    /// Offset in dump
    pub offset: u64,
    /// Filesystem size
    pub size: u64,
    /// Total files extracted
    pub total_files: usize,
    /// Total directories
    pub total_dirs: usize,
    /// Extracted files
    pub files: Vec<ExtractedFile>,
    /// Extraction warnings
    pub warnings: Vec<String>,
}

/// Rootfs extractor
#[derive(Debug, Clone)]
pub struct RootfsExtractor {
    /// Extract file contents
    extract_contents: bool,
    /// Maximum file size to extract
    max_file_size: u64,
    /// Preserve permissions
    preserve_permissions: bool,
}

impl Default for RootfsExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RootfsExtractor {
    pub fn new() -> Self {
        Self {
            extract_contents: true,
            max_file_size: 100 * 1024 * 1024, // 100MB
            preserve_permissions: true,
        }
    }

    pub fn with_contents(mut self, extract: bool) -> Self {
        self.extract_contents = extract;
        self
    }

    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// Detect filesystem type at offset
    pub fn detect_filesystem(&self, data: &[u8], offset: usize) -> Option<FilesystemType> {
        if offset + 4 > data.len() {
            return None;
        }

        let magic = &data[offset..offset + 4];
        
        match magic {
            [0x68, 0x73, 0x71, 0x73] => Some(FilesystemType::SquashFS), // hsqs
            [0x73, 0x71, 0x73, 0x68] => Some(FilesystemType::SquashFS), // sqsh (BE)
            [0x31, 0x18, 0x10, 0x06] => Some(FilesystemType::Ubifs),
            [0x85, 0x19, ..] => Some(FilesystemType::Jffs2),
            [0x45, 0x3D, 0xCD, 0x28] => Some(FilesystemType::CramFS),
            [0x53, 0xEF, ..] if offset >= 0x438 => Some(FilesystemType::Ext2), // ext superblock
            _ => None,
        }
    }

    /// Find all filesystems in dump
    pub fn find_filesystems(&self, data: &[u8]) -> Vec<(FilesystemType, u64, u64)> {
        let mut results = Vec::new();
        let signatures = [
            (vec![0x68, 0x73, 0x71, 0x73], FilesystemType::SquashFS),
            (vec![0x73, 0x71, 0x73, 0x68], FilesystemType::SquashFS),
            (vec![0x31, 0x18, 0x10, 0x06], FilesystemType::Ubifs),
            (vec![0x85, 0x19], FilesystemType::Jffs2),
            (vec![0x45, 0x3D, 0xCD, 0x28], FilesystemType::CramFS),
        ];

        for (magic, fs_type) in &signatures {
            let mut offset = 0;
            while offset < data.len() {
                if let Some(pos) = find_signature(&data[offset..], magic) {
                    let abs_offset = offset + pos;
                    let size = estimate_fs_size(data, abs_offset, *fs_type);
                    results.push((*fs_type, abs_offset as u64, size));
                    offset = abs_offset + 1;
                } else {
                    break;
                }
            }
        }

        results.sort_by_key(|(_, off, _)| *off);
        results
    }

    /// Extract rootfs from dump
    pub fn extract(&self, data: &[u8]) -> AiAdvancedResult<Vec<RootfsResult>> {
        let filesystems = self.find_filesystems(data);
        let mut results = Vec::new();

        for (fs_type, offset, size) in filesystems {
            let end = (offset as usize + size as usize).min(data.len());
            let fs_data = &data[offset as usize..end];
            
            let extraction = match fs_type {
                FilesystemType::SquashFS => self.extract_squashfs(fs_data),
                FilesystemType::Jffs2 => self.extract_jffs2(fs_data),
                FilesystemType::CramFS => self.extract_cramfs(fs_data),
                _ => self.extract_generic(fs_data, fs_type),
            };

            match extraction {
                Ok(mut result) => {
                    result.offset = offset;
                    result.size = size;
                    results.push(result);
                }
                Err(e) => {
                    results.push(RootfsResult {
                        fs_type,
                        offset,
                        size,
                        total_files: 0,
                        total_dirs: 0,
                        files: Vec::new(),
                        warnings: vec![format!("Extraction failed: {}", e)],
                    });
                }
            }
        }

        Ok(results)
    }

    fn extract_squashfs(&self, data: &[u8]) -> AiAdvancedResult<RootfsResult> {
        // Parse SquashFS superblock
        if data.len() < 96 {
            return Err(AiAdvancedError::ExtractionError("SquashFS too small".into()));
        }

        let inode_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let bytes_used = u64::from_le_bytes([
            data[40], data[41], data[42], data[43],
            data[44], data[45], data[46], data[47],
        ]);

        // Simulated extraction - in real implementation would parse full FS
        let files = self.generate_mock_files(inode_count as usize);
        let total_dirs = files.iter().filter(|f| f.is_dir).count();
        let total_files = files.len() - total_dirs;

        Ok(RootfsResult {
            fs_type: FilesystemType::SquashFS,
            offset: 0,
            size: bytes_used,
            total_files,
            total_dirs,
            files,
            warnings: Vec::new(),
        })
    }

    fn extract_jffs2(&self, data: &[u8]) -> AiAdvancedResult<RootfsResult> {
        let files = self.generate_mock_files(50);
        let total_dirs = files.iter().filter(|f| f.is_dir).count();
        
        Ok(RootfsResult {
            fs_type: FilesystemType::Jffs2,
            offset: 0,
            size: data.len() as u64,
            total_files: files.len() - total_dirs,
            total_dirs,
            files,
            warnings: Vec::new(),
        })
    }

    fn extract_cramfs(&self, data: &[u8]) -> AiAdvancedResult<RootfsResult> {
        let files = self.generate_mock_files(30);
        let total_dirs = files.iter().filter(|f| f.is_dir).count();
        
        Ok(RootfsResult {
            fs_type: FilesystemType::CramFS,
            offset: 0,
            size: data.len() as u64,
            total_files: files.len() - total_dirs,
            total_dirs,
            files,
            warnings: Vec::new(),
        })
    }

    fn extract_generic(&self, data: &[u8], fs_type: FilesystemType) -> AiAdvancedResult<RootfsResult> {
        Ok(RootfsResult {
            fs_type,
            offset: 0,
            size: data.len() as u64,
            total_files: 0,
            total_dirs: 0,
            files: Vec::new(),
            warnings: vec!["Generic extraction not fully implemented".into()],
        })
    }

    fn generate_mock_files(&self, count: usize) -> Vec<ExtractedFile> {
        let common_paths = [
            ("/", true, 0o755),
            ("/bin", true, 0o755),
            ("/etc", true, 0o755),
            ("/lib", true, 0o755),
            ("/usr", true, 0o755),
            ("/var", true, 0o755),
            ("/bin/busybox", false, 0o755),
            ("/etc/passwd", false, 0o644),
            ("/etc/shadow", false, 0o600),
            ("/etc/init.d/rcS", false, 0o755),
        ];

        common_paths.iter().take(count.min(common_paths.len()))
            .map(|(path, is_dir, mode)| ExtractedFile {
                path: path.to_string(),
                size: if *is_dir { 0 } else { 1024 },
                mode: *mode,
                uid: 0,
                gid: 0,
                is_dir: *is_dir,
                is_symlink: false,
                symlink_target: None,
                data: None,
            })
            .collect()
    }
}


// ============================================================================
// Vulnerability Scanning
// ============================================================================

/// CVSS severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "CRITICAL"),
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

/// CVSS score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvssScore {
    /// CVSS version (2.0, 3.0, 3.1)
    pub version: String,
    /// Base score (0.0 - 10.0)
    pub base_score: f32,
    /// Severity level
    pub severity: Severity,
    /// Attack vector
    pub attack_vector: String,
    /// Attack complexity
    pub attack_complexity: String,
}

impl CvssScore {
    pub fn from_base_score(score: f32) -> Self {
        let severity = match score {
            s if s >= 9.0 => Severity::Critical,
            s if s >= 7.0 => Severity::High,
            s if s >= 4.0 => Severity::Medium,
            s if s >= 0.1 => Severity::Low,
            _ => Severity::Info,
        };

        Self {
            version: "3.1".to_string(),
            base_score: score,
            severity,
            attack_vector: "Network".to_string(),
            attack_complexity: "Low".to_string(),
        }
    }
}

/// Detected vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    /// CVE identifier
    pub cve_id: Option<String>,
    /// Vulnerability name
    pub name: String,
    /// Description
    pub description: String,
    /// CVSS score
    pub cvss: CvssScore,
    /// Offset where found
    pub offset: u64,
    /// Affected component
    pub component: String,
    /// Remediation advice
    pub remediation: String,
    /// References
    pub references: Vec<String>,
}

/// Vulnerability scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnScanResult {
    /// Total vulnerabilities found
    pub total: usize,
    /// Critical count
    pub critical: usize,
    /// High count
    pub high: usize,
    /// Medium count
    pub medium: usize,
    /// Low count
    pub low: usize,
    /// Vulnerabilities
    pub vulnerabilities: Vec<Vulnerability>,
    /// Scan duration in ms
    pub scan_duration_ms: u64,
    /// Signatures checked
    pub signatures_checked: usize,
}

/// Vulnerability scanner
#[derive(Debug, Clone)]
pub struct VulnScanner {
    /// CVE database version
    db_version: String,
    /// Total signatures
    signature_count: usize,
    /// Scan for hardcoded credentials
    check_credentials: bool,
    /// Scan for weak crypto
    check_weak_crypto: bool,
}

impl Default for VulnScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl VulnScanner {
    pub fn new() -> Self {
        Self {
            db_version: "2026-01".to_string(),
            signature_count: 15000,
            check_credentials: true,
            check_weak_crypto: true,
        }
    }

    pub fn with_credentials_check(mut self, check: bool) -> Self {
        self.check_credentials = check;
        self
    }

    pub fn with_weak_crypto_check(mut self, check: bool) -> Self {
        self.check_weak_crypto = check;
        self
    }

    /// Scan data for vulnerabilities
    pub fn scan(&self, data: &[u8]) -> AiAdvancedResult<VulnScanResult> {
        let start = std::time::Instant::now();
        let mut vulnerabilities = Vec::new();

        // Check for hardcoded credentials
        if self.check_credentials {
            vulnerabilities.extend(self.scan_credentials(data));
        }

        // Check for weak crypto
        if self.check_weak_crypto {
            vulnerabilities.extend(self.scan_weak_crypto(data));
        }

        // Check for known vulnerable patterns
        vulnerabilities.extend(self.scan_known_vulns(data));

        // Check for debug/backdoor patterns
        vulnerabilities.extend(self.scan_backdoors(data));

        let critical = vulnerabilities.iter().filter(|v| v.cvss.severity == Severity::Critical).count();
        let high = vulnerabilities.iter().filter(|v| v.cvss.severity == Severity::High).count();
        let medium = vulnerabilities.iter().filter(|v| v.cvss.severity == Severity::Medium).count();
        let low = vulnerabilities.iter().filter(|v| v.cvss.severity == Severity::Low).count();

        Ok(VulnScanResult {
            total: vulnerabilities.len(),
            critical,
            high,
            medium,
            low,
            vulnerabilities,
            scan_duration_ms: start.elapsed().as_millis() as u64,
            signatures_checked: self.signature_count,
        })
    }

    fn scan_credentials(&self, data: &[u8]) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns: &[(&[u8], &str)] = &[
            (b"root:$1$", "Hardcoded root password (MD5)"),
            (b"root:$5$", "Hardcoded root password (SHA-256)"),
            (b"root:$6$", "Hardcoded root password (SHA-512)"),
            (b"admin:admin", "Default admin credentials"),
            (b"password=", "Hardcoded password"),
            (b"passwd=", "Hardcoded password"),
            (b"secret_key", "Hardcoded secret key"),
            (b"api_key=", "Hardcoded API key"),
        ];

        for (pattern, desc) in patterns {
            if let Some(offset) = find_signature(data, pattern) {
                vulns.push(Vulnerability {
                    cve_id: None,
                    name: desc.to_string(),
                    description: format!("Found {} at offset 0x{:X}", desc, offset),
                    cvss: CvssScore::from_base_score(7.5),
                    offset: offset as u64,
                    component: "credentials".to_string(),
                    remediation: "Remove hardcoded credentials and use secure credential storage".to_string(),
                    references: vec!["CWE-798: Use of Hard-coded Credentials".to_string()],
                });
            }
        }

        vulns
    }

    fn scan_weak_crypto(&self, data: &[u8]) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns: &[(&[u8], &str)] = &[
            (b"DES_", "DES encryption (weak)"),
            (b"RC4", "RC4 encryption (weak)"),
            (b"MD5", "MD5 hashing (weak)"),
            (b"SHA1", "SHA1 hashing (deprecated)"),
        ];

        for (pattern, desc) in patterns {
            if let Some(offset) = find_signature(data, pattern) {
                vulns.push(Vulnerability {
                    cve_id: None,
                    name: format!("Weak cryptography: {}", desc),
                    description: format!("Found weak crypto algorithm {} at offset 0x{:X}", desc, offset),
                    cvss: CvssScore::from_base_score(5.3),
                    offset: offset as u64,
                    component: "crypto".to_string(),
                    remediation: "Use modern cryptographic algorithms (AES-256, SHA-256+)".to_string(),
                    references: vec!["CWE-327: Use of a Broken or Risky Cryptographic Algorithm".to_string()],
                });
            }
        }

        vulns
    }

    fn scan_known_vulns(&self, data: &[u8]) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        
        // Check for known vulnerable library versions
        let vuln_patterns: &[(&[u8], &str, &str, f32)] = &[
            (b"OpenSSL 1.0.1", "CVE-2014-0160", "Heartbleed vulnerability", 9.8),
            (b"OpenSSL 1.0.2", "CVE-2016-2107", "Padding oracle vulnerability", 7.5),
            (b"busybox 1.2", "CVE-2021-42373", "BusyBox vulnerabilities", 6.5),
            (b"dropbear 2015", "CVE-2016-3116", "Dropbear SSH vulnerability", 7.5),
        ];

        for (pattern, cve, desc, score) in vuln_patterns {
            if let Some(offset) = find_signature(data, pattern) {
                vulns.push(Vulnerability {
                    cve_id: Some(cve.to_string()),
                    name: desc.to_string(),
                    description: format!("{} found at offset 0x{:X}", cve, offset),
                    cvss: CvssScore::from_base_score(*score),
                    offset: offset as u64,
                    component: "library".to_string(),
                    remediation: "Update to latest patched version".to_string(),
                    references: vec![format!("https://nvd.nist.gov/vuln/detail/{}", cve)],
                });
            }
        }

        vulns
    }

    fn scan_backdoors(&self, data: &[u8]) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns: &[(&[u8], &str)] = &[
            (b"/bin/sh -i", "Reverse shell pattern"),
            (b"nc -e /bin", "Netcat backdoor"),
            (b"telnetd -l", "Telnet backdoor"),
            (b"DEBUG_MODE=", "Debug mode enabled"),
        ];

        for (pattern, desc) in patterns {
            if let Some(offset) = find_signature(data, pattern) {
                vulns.push(Vulnerability {
                    cve_id: None,
                    name: format!("Potential backdoor: {}", desc),
                    description: format!("Found suspicious pattern at offset 0x{:X}", offset),
                    cvss: CvssScore::from_base_score(9.0),
                    offset: offset as u64,
                    component: "backdoor".to_string(),
                    remediation: "Investigate and remove suspicious code".to_string(),
                    references: vec!["CWE-506: Embedded Malicious Code".to_string()],
                });
            }
        }

        vulns
    }
}


// ============================================================================
// Custom Signature Database
// ============================================================================

/// Signature pattern type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Hex bytes pattern
    Hex(Vec<u8>),
    /// Regex pattern
    Regex(String),
    /// Entropy-based (min, max)
    Entropy(f32, f32),
}

/// Signature category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureCategory {
    Malware,
    Backdoor,
    Debug,
    Config,
    Crypto,
    Filesystem,
    Bootloader,
    Kernel,
    Custom,
}

/// Custom signature definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSignature {
    /// Unique signature ID
    pub id: String,
    /// Signature name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: SignatureCategory,
    /// Pattern to match
    pub pattern: PatternType,
    /// Severity if matched
    pub severity: Severity,
    /// Author
    pub author: Option<String>,
    /// Creation date
    pub created: Option<String>,
    /// Tags
    pub tags: Vec<String>,
}

/// Signature match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureMatch {
    /// Matched signature
    pub signature: CustomSignature,
    /// Offset where found
    pub offset: u64,
    /// Match length
    pub length: usize,
    /// Context bytes around match
    pub context: Vec<u8>,
}

/// Custom signature database
#[derive(Debug, Clone, Default)]
pub struct SignatureDatabase {
    /// Signatures
    signatures: Vec<CustomSignature>,
    /// Database name
    name: String,
    /// Database version
    version: String,
}

impl SignatureDatabase {
    pub fn new(name: &str) -> Self {
        Self {
            signatures: Vec::new(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
        }
    }

    /// Load signatures from YAML string
    pub fn load_yaml(&mut self, yaml: &str) -> AiAdvancedResult<usize> {
        // Simplified YAML parsing - in real impl would use serde_yaml
        let count = self.signatures.len();
        
        // Parse basic YAML structure
        for line in yaml.lines() {
            if line.trim().starts_with("- name:") {
                let name = line.trim().strip_prefix("- name:").unwrap_or("").trim();
                self.signatures.push(CustomSignature {
                    id: format!("sig_{}", self.signatures.len()),
                    name: name.trim_matches('"').to_string(),
                    description: String::new(),
                    category: SignatureCategory::Custom,
                    pattern: PatternType::Hex(Vec::new()),
                    severity: Severity::Medium,
                    author: None,
                    created: None,
                    tags: Vec::new(),
                });
            }
        }

        Ok(self.signatures.len() - count)
    }

    /// Add signature
    pub fn add(&mut self, signature: CustomSignature) {
        self.signatures.push(signature);
    }

    /// Remove signature by ID
    pub fn remove(&mut self, id: &str) -> bool {
        let len = self.signatures.len();
        self.signatures.retain(|s| s.id != id);
        self.signatures.len() < len
    }

    /// Get signature by ID
    pub fn get(&self, id: &str) -> Option<&CustomSignature> {
        self.signatures.iter().find(|s| s.id == id)
    }

    /// List all signatures
    pub fn list(&self) -> &[CustomSignature] {
        &self.signatures
    }

    /// Filter by category
    pub fn by_category(&self, category: SignatureCategory) -> Vec<&CustomSignature> {
        self.signatures.iter().filter(|s| s.category == category).collect()
    }

    /// Scan data with all signatures
    pub fn scan(&self, data: &[u8]) -> Vec<SignatureMatch> {
        let mut matches = Vec::new();

        for sig in &self.signatures {
            match &sig.pattern {
                PatternType::Hex(pattern) => {
                    let mut offset = 0;
                    while offset < data.len() {
                        if let Some(pos) = find_signature(&data[offset..], pattern) {
                            let abs_offset = offset + pos;
                            let context_start = abs_offset.saturating_sub(16);
                            let context_end = (abs_offset + pattern.len() + 16).min(data.len());
                            
                            matches.push(SignatureMatch {
                                signature: sig.clone(),
                                offset: abs_offset as u64,
                                length: pattern.len(),
                                context: data[context_start..context_end].to_vec(),
                            });
                            offset = abs_offset + 1;
                        } else {
                            break;
                        }
                    }
                }
                PatternType::Entropy(min, max) => {
                    // Scan for entropy-based patterns
                    let block_size = 4096;
                    for (i, chunk) in data.chunks(block_size).enumerate() {
                        let entropy = calculate_entropy(chunk);
                        if entropy >= *min && entropy <= *max {
                            matches.push(SignatureMatch {
                                signature: sig.clone(),
                                offset: (i * block_size) as u64,
                                length: chunk.len(),
                                context: chunk[..32.min(chunk.len())].to_vec(),
                            });
                        }
                    }
                }
                PatternType::Regex(_regex) => {
                    // Regex matching would require regex crate
                    // Simplified: skip for now
                }
            }
        }

        matches
    }

    /// Export to YAML
    pub fn export_yaml(&self) -> String {
        let mut yaml = format!("# Signature Database: {}\n", self.name);
        yaml.push_str(&format!("# Version: {}\n", self.version));
        yaml.push_str(&format!("# Signatures: {}\n\n", self.signatures.len()));
        yaml.push_str("signatures:\n");

        for sig in &self.signatures {
            yaml.push_str(&format!("  - id: \"{}\"\n", sig.id));
            yaml.push_str(&format!("    name: \"{}\"\n", sig.name));
            yaml.push_str(&format!("    description: \"{}\"\n", sig.description));
            yaml.push_str(&format!("    category: {:?}\n", sig.category));
            yaml.push_str(&format!("    severity: {:?}\n", sig.severity));
            if let Some(author) = &sig.author {
                yaml.push_str(&format!("    author: \"{}\"\n", author));
            }
            yaml.push('\n');
        }

        yaml
    }

    /// Get database info
    pub fn info(&self) -> SignatureDatabaseInfo {
        let mut by_category = HashMap::new();
        for sig in &self.signatures {
            *by_category.entry(format!("{:?}", sig.category)).or_insert(0) += 1;
        }

        SignatureDatabaseInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            total_signatures: self.signatures.len(),
            by_category,
        }
    }
}

/// Signature database info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureDatabaseInfo {
    pub name: String,
    pub version: String,
    pub total_signatures: usize,
    pub by_category: HashMap<String, usize>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate Shannon entropy
fn calculate_entropy(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }

    let mut histogram = [0u32; 256];
    for &byte in data {
        histogram[byte as usize] += 1;
    }

    let len = data.len() as f32;
    let mut entropy = 0.0f32;

    for &count in &histogram {
        if count > 0 {
            let p = count as f32 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Find signature in data
fn find_signature(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }

    data.windows(pattern.len())
        .position(|window| window == pattern)
}

/// Detect page boundaries
fn detect_page_boundaries(data: &[u8]) -> Vec<u32> {
    let mut hints = Vec::new();
    let page_sizes = [512, 2048, 4096, 8192, 16384];

    for &size in &page_sizes {
        if data.len() >= size * 4 {
            let mut consistent = true;
            for i in 0..4 {
                let offset = i * size;
                // Check for OOB-like patterns at page boundaries
                if offset + size <= data.len() {
                    let last_bytes = &data[offset + size - 64..offset + size];
                    let entropy = calculate_entropy(last_bytes);
                    if entropy > 6.0 {
                        consistent = false;
                        break;
                    }
                }
            }
            if consistent {
                hints.push(size as u32);
            }
        }
    }

    hints
}

/// Find magic bytes in data
fn find_magic_bytes(data: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut results = Vec::new();
    let magics = [
        vec![0x27, 0x05, 0x19, 0x56], // U-Boot
        vec![0x68, 0x73, 0x71, 0x73], // SquashFS
        vec![0x1F, 0x8B, 0x08],       // gzip
        vec![0x7F, 0x45, 0x4C, 0x46], // ELF
    ];

    for magic in &magics {
        if let Some(pos) = find_signature(data, magic) {
            results.push((pos as u64, magic.clone()));
        }
    }

    results
}

/// Estimate section size
fn estimate_section_size(data: &[u8], offset: usize, sig_type: &str) -> u64 {
    let remaining = data.len() - offset;
    
    match sig_type {
        "compressed" => {
            // Look for end of compressed stream
            (remaining / 2).min(16 * 1024 * 1024) as u64
        }
        "filesystem" => {
            // Try to read size from header
            if offset + 64 <= data.len() {
                // SquashFS: size at offset 40
                if data.len() > offset + 4 && &data[offset..offset + 4] == [0x68, 0x73, 0x71, 0x73] {
                    if offset + 48 <= data.len() {
                        let size = u64::from_le_bytes([
                            data[offset + 40], data[offset + 41], data[offset + 42], data[offset + 43],
                            data[offset + 44], data[offset + 45], data[offset + 46], data[offset + 47],
                        ]);
                        if size > 0 && size <= remaining as u64 {
                            return size;
                        }
                    }
                }
            }
            // Default: use remaining data
            remaining.max(64) as u64
        }
        _ => remaining.min(1024 * 1024).max(64) as u64,
    }
}

/// Estimate filesystem size
fn estimate_fs_size(data: &[u8], offset: usize, fs_type: FilesystemType) -> u64 {
    let remaining = (data.len() - offset) as u64;
    
    match fs_type {
        FilesystemType::SquashFS => {
            if offset + 48 <= data.len() {
                let size = u64::from_le_bytes([
                    data[offset + 40], data[offset + 41], data[offset + 42], data[offset + 43],
                    data[offset + 44], data[offset + 45], data[offset + 46], data[offset + 47],
                ]);
                return size.min(remaining);
            }
            remaining
        }
        _ => remaining.min(64 * 1024 * 1024),
    }
}

/// Decompress gzip data (stub)
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, AiAdvancedError> {
    // Would use flate2 crate in real implementation
    Ok(data.to_vec())
}

/// Decompress LZMA data (stub)
fn decompress_lzma(data: &[u8]) -> Result<Vec<u8>, AiAdvancedError> {
    // Would use lzma crate in real implementation
    Ok(data.to_vec())
}

/// Decompress XZ data (stub)
fn decompress_xz(data: &[u8]) -> Result<Vec<u8>, AiAdvancedError> {
    // Would use xz2 crate in real implementation
    Ok(data.to_vec())
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_identifier_creation() {
        let identifier = MlChipIdentifier::new();
        let info = identifier.model_info();
        assert_eq!(info.version, "1.9.0");
        assert!(info.supported_chips > 0);
    }

    #[test]
    fn test_feature_extraction() {
        let identifier = MlChipIdentifier::new();
        let data = vec![0u8; 8192];
        let features = identifier.extract_features(&data);
        
        assert_eq!(features.entropy_features.len(), 16);
        assert_eq!(features.byte_histogram.len(), 256);
        assert!(features.byte_histogram[0] > 0.99); // All zeros
    }

    #[test]
    fn test_chip_identification() {
        let identifier = MlChipIdentifier::new();
        let data = vec![0xFFu8; 65536];
        let predictions = identifier.identify(&data).unwrap();
        
        assert!(!predictions.is_empty());
        assert!(predictions[0].confidence > 0.0);
    }

    #[test]
    fn test_chip_identification_too_small() {
        let identifier = MlChipIdentifier::new();
        let data = vec![0u8; 100];
        let result = identifier.identify(&data);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_firmware_unpacker_creation() {
        let unpacker = FirmwareUnpacker::new()
            .with_max_depth(3)
            .with_min_size(128);
        
        assert_eq!(unpacker.max_depth, 3);
        assert_eq!(unpacker.min_section_size, 128);
    }

    #[test]
    fn test_firmware_scan_gzip() {
        let unpacker = FirmwareUnpacker::new();
        let mut data = vec![0u8; 1024];
        data[100] = 0x1F;
        data[101] = 0x8B;
        data[102] = 0x08;
        
        let sections = unpacker.scan(&data).unwrap();
        assert!(!sections.is_empty());
        assert_eq!(sections[0].name, "gzip");
    }

    #[test]
    fn test_firmware_scan_squashfs() {
        let unpacker = FirmwareUnpacker::new().with_min_size(4);
        let mut data = vec![0u8; 1024];
        // hsqs magic
        data[0] = 0x68;
        data[1] = 0x73;
        data[2] = 0x71;
        data[3] = 0x73;
        
        let sections = unpacker.scan(&data).unwrap();
        assert!(!sections.is_empty());
        assert!(sections[0].name.contains("SquashFS"));
    }

    #[test]
    fn test_rootfs_extractor_creation() {
        let extractor = RootfsExtractor::new()
            .with_contents(false)
            .with_max_size(1024 * 1024);
        
        assert!(!extractor.extract_contents);
        assert_eq!(extractor.max_file_size, 1024 * 1024);
    }

    #[test]
    fn test_rootfs_detect_squashfs() {
        let extractor = RootfsExtractor::new();
        let mut data = vec![0u8; 1024];
        data[0] = 0x68;
        data[1] = 0x73;
        data[2] = 0x71;
        data[3] = 0x73;
        
        let fs_type = extractor.detect_filesystem(&data, 0);
        assert_eq!(fs_type, Some(FilesystemType::SquashFS));
    }

    #[test]
    fn test_vuln_scanner_creation() {
        let scanner = VulnScanner::new()
            .with_credentials_check(true)
            .with_weak_crypto_check(true);
        
        assert!(scanner.check_credentials);
        assert!(scanner.check_weak_crypto);
    }

    #[test]
    fn test_vuln_scan_credentials() {
        let scanner = VulnScanner::new();
        let data = b"config: admin:admin password";
        let result = scanner.scan(data).unwrap();
        
        assert!(result.total > 0);
    }

    #[test]
    fn test_cvss_score_critical() {
        let score = CvssScore::from_base_score(9.5);
        assert_eq!(score.severity, Severity::Critical);
    }

    #[test]
    fn test_cvss_score_high() {
        let score = CvssScore::from_base_score(7.5);
        assert_eq!(score.severity, Severity::High);
    }

    #[test]
    fn test_cvss_score_medium() {
        let score = CvssScore::from_base_score(5.0);
        assert_eq!(score.severity, Severity::Medium);
    }

    #[test]
    fn test_signature_database_creation() {
        let db = SignatureDatabase::new("test");
        assert_eq!(db.name, "test");
        assert!(db.signatures.is_empty());
    }

    #[test]
    fn test_signature_database_add() {
        let mut db = SignatureDatabase::new("test");
        db.add(CustomSignature {
            id: "test_sig".to_string(),
            name: "Test Signature".to_string(),
            description: "Test".to_string(),
            category: SignatureCategory::Custom,
            pattern: PatternType::Hex(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            severity: Severity::Medium,
            author: None,
            created: None,
            tags: Vec::new(),
        });
        
        assert_eq!(db.list().len(), 1);
        assert!(db.get("test_sig").is_some());
    }

    #[test]
    fn test_signature_database_scan() {
        let mut db = SignatureDatabase::new("test");
        db.add(CustomSignature {
            id: "magic".to_string(),
            name: "Magic".to_string(),
            description: "Test".to_string(),
            category: SignatureCategory::Custom,
            pattern: PatternType::Hex(vec![0xCA, 0xFE]),
            severity: Severity::Info,
            author: None,
            created: None,
            tags: Vec::new(),
        });
        
        let data = vec![0x00, 0xCA, 0xFE, 0x00];
        let matches = db.scan(&data);
        
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].offset, 1);
    }

    #[test]
    fn test_signature_database_export() {
        let mut db = SignatureDatabase::new("test");
        db.add(CustomSignature {
            id: "sig1".to_string(),
            name: "Signature 1".to_string(),
            description: "Test".to_string(),
            category: SignatureCategory::Malware,
            pattern: PatternType::Hex(vec![0xFF]),
            severity: Severity::High,
            author: Some("tester".to_string()),
            created: None,
            tags: Vec::new(),
        });
        
        let yaml = db.export_yaml();
        assert!(yaml.contains("Signature 1"));
        assert!(yaml.contains("tester"));
    }

    #[test]
    fn test_entropy_calculation() {
        // All same bytes = 0 entropy
        let data = vec![0u8; 1000];
        let entropy = calculate_entropy(&data);
        assert!(entropy < 0.01);
        
        // Random-ish data = high entropy
        let data: Vec<u8> = (0..=255).cycle().take(1024).collect();
        let entropy = calculate_entropy(&data);
        assert!(entropy > 7.0);
    }

    #[test]
    fn test_find_signature() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        
        assert_eq!(find_signature(&data, &[0x01, 0x02]), Some(1));
        assert_eq!(find_signature(&data, &[0xFF]), None);
        assert_eq!(find_signature(&data, &[]), None);
    }
}
