//! Data analysis module for OpenFlash
//! Detects filesystem signatures and analyzes NAND dumps

use serde::{Deserialize, Serialize};

/// Known filesystem signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSignature {
    pub name: String,
    pub magic: Vec<u8>,
    pub offset: usize,
    pub confidence: f32,
}

/// Analysis result for a NAND dump
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub filesystem_type: Option<String>,
    pub signatures_found: Vec<FileSystemSignature>,
    pub bad_blocks: Vec<u32>,
    pub empty_pages: u32,
    pub data_pages: u32,
}

/// Signature definition for detection
struct SignatureDef {
    name: &'static str,
    magic: &'static [u8],
    typical_offsets: &'static [usize],
}

const SIGNATURES: &[SignatureDef] = &[
    SignatureDef {
        name: "SquashFS",
        magic: b"hsqs",
        typical_offsets: &[0, 0x10000, 0x20000, 0x40000],
    },
    SignatureDef {
        name: "SquashFS (BE)",
        magic: b"sqsh",
        typical_offsets: &[0, 0x10000, 0x20000, 0x40000],
    },
    SignatureDef {
        name: "UBIFS",
        magic: &[0x31, 0x18, 0x10, 0x06], // UBI EC header
        typical_offsets: &[0],
    },
    SignatureDef {
        name: "JFFS2 (LE)",
        magic: &[0x85, 0x19],
        typical_offsets: &[0],
    },
    SignatureDef {
        name: "JFFS2 (BE)",
        magic: &[0x19, 0x85],
        typical_offsets: &[0],
    },
    SignatureDef {
        name: "CramFS",
        magic: &[0x45, 0x3D, 0xCD, 0x28],
        typical_offsets: &[0],
    },
    SignatureDef {
        name: "U-Boot",
        magic: &[0x27, 0x05, 0x19, 0x56],
        typical_offsets: &[0, 0x40, 0x100],
    },
    SignatureDef {
        name: "gzip",
        magic: &[0x1F, 0x8B, 0x08],
        typical_offsets: &[0, 0x40, 0x100],
    },
    SignatureDef {
        name: "LZMA",
        magic: &[0x5D, 0x00, 0x00],
        typical_offsets: &[0, 0x40],
    },
    SignatureDef {
        name: "XZ",
        magic: &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00],
        typical_offsets: &[0],
    },
];

/// NAND dump analyzer
pub struct Analyzer {
    page_size: usize,
    block_size: usize,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new(2048, 64)
    }
}

impl Analyzer {
    /// Create analyzer with page and block size
    pub fn new(page_size: usize, pages_per_block: usize) -> Self {
        Self {
            page_size,
            block_size: pages_per_block,
        }
    }

    /// Analyze a NAND dump
    pub fn analyze_dump(&self, data: &[u8]) -> AnalysisResult {
        let signatures_found = self.find_all_signatures(data);
        let (empty_pages, data_pages) = self.count_pages(data);
        let bad_blocks = self.detect_bad_blocks(data);

        let filesystem_type = self.determine_filesystem(&signatures_found);

        AnalysisResult {
            filesystem_type,
            signatures_found,
            bad_blocks,
            empty_pages,
            data_pages,
        }
    }

    /// Find all known signatures in the dump
    fn find_all_signatures(&self, data: &[u8]) -> Vec<FileSystemSignature> {
        let mut found = Vec::new();

        for sig_def in SIGNATURES {
            // Check typical offsets first
            for &offset in sig_def.typical_offsets {
                if let Some(sig) = self.check_signature_at(data, sig_def, offset) {
                    found.push(sig);
                }
            }

            // Scan through data at page boundaries
            let mut offset = 0;
            while offset + sig_def.magic.len() <= data.len() {
                if let Some(sig) = self.check_signature_at(data, sig_def, offset) {
                    // Avoid duplicates
                    if !found.iter().any(|s| s.offset == offset && s.name == sig_def.name) {
                        found.push(sig);
                    }
                }
                offset += self.page_size;
            }
        }

        // Sort by offset
        found.sort_by_key(|s| s.offset);
        found
    }

    fn check_signature_at(&self, data: &[u8], sig_def: &SignatureDef, offset: usize) -> Option<FileSystemSignature> {
        if offset + sig_def.magic.len() > data.len() {
            return None;
        }

        if &data[offset..offset + sig_def.magic.len()] == sig_def.magic {
            let confidence = if sig_def.typical_offsets.contains(&offset) {
                0.95
            } else {
                0.75
            };

            Some(FileSystemSignature {
                name: sig_def.name.to_string(),
                magic: sig_def.magic.to_vec(),
                offset,
                confidence,
            })
        } else {
            None
        }
    }

    /// Count empty vs data pages
    fn count_pages(&self, data: &[u8]) -> (u32, u32) {
        let mut empty = 0u32;
        let mut with_data = 0u32;

        for chunk in data.chunks(self.page_size) {
            if chunk.iter().all(|&b| b == 0xFF) {
                empty += 1;
            } else {
                with_data += 1;
            }
        }

        (empty, with_data)
    }

    /// Detect bad blocks by checking spare area markers
    fn detect_bad_blocks(&self, data: &[u8]) -> Vec<u32> {
        let mut bad = Vec::new();
        let block_bytes = self.page_size * self.block_size;
        
        for (block_num, chunk) in data.chunks(block_bytes).enumerate() {
            // Check first page's spare area (typically byte 0 of spare)
            // Bad block marker is usually != 0xFF
            if chunk.len() >= self.page_size {
                // Simplified check - real implementation would check OOB area
                let first_byte = chunk[0];
                let second_byte = if chunk.len() > 1 { chunk[1] } else { 0xFF };
                
                // Heuristic: if first two bytes are 0x00, might be bad block marker
                if first_byte == 0x00 && second_byte == 0x00 {
                    bad.push(block_num as u32);
                }
            }
        }

        bad
    }

    /// Determine most likely filesystem from signatures
    fn determine_filesystem(&self, signatures: &[FileSystemSignature]) -> Option<String> {
        if signatures.is_empty() {
            return None;
        }

        // Find highest confidence signature
        signatures
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|s| s.name.clone())
    }

    /// Calculate entropy of data (0.0 = uniform, 8.0 = random)
    pub fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut counts = [0u64; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = Analyzer::new(2048, 64);
        assert_eq!(analyzer.page_size, 2048);
    }

    #[test]
    fn test_squashfs_detection() {
        let analyzer = Analyzer::default();
        let mut data = vec![0xFFu8; 4096];
        // SquashFS magic "hsqs"
        data[0] = b'h';
        data[1] = b's';
        data[2] = b'q';
        data[3] = b's';

        let result = analyzer.analyze_dump(&data);
        assert!(result.signatures_found.iter().any(|s| s.name == "SquashFS"));
    }

    #[test]
    fn test_empty_page_detection() {
        let analyzer = Analyzer::new(512, 32);
        let data = vec![0xFFu8; 2048]; // 4 empty pages

        let result = analyzer.analyze_dump(&data);
        assert_eq!(result.empty_pages, 4);
        assert_eq!(result.data_pages, 0);
    }

    #[test]
    fn test_entropy_calculation() {
        let analyzer = Analyzer::default();
        
        // Uniform data has low entropy
        let uniform = vec![0xAAu8; 1000];
        let entropy = analyzer.calculate_entropy(&uniform);
        assert!(entropy < 0.1);

        // Random-ish data has higher entropy
        let varied: Vec<u8> = (0..=255).collect();
        let entropy = analyzer.calculate_entropy(&varied);
        assert!(entropy > 7.0);
    }
}
