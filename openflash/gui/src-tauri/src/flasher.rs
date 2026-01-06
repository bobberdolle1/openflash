//! High-level NAND flash operations

use openflash_core::ecc::{decode_with_ecc, EccAlgorithm};
use serde::{Deserialize, Serialize};

/// Flash operation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashConfig {
    pub page_size: u32,
    pub oob_size: u32,
    pub pages_per_block: u32,
    pub total_blocks: u32,
    pub ecc_algorithm: EccAlgorithm,
}

impl Default for FlashConfig {
    fn default() -> Self {
        Self {
            page_size: 2048,
            oob_size: 64,
            pages_per_block: 64,
            total_blocks: 1024,
            ecc_algorithm: EccAlgorithm::None,
        }
    }
}

/// Process raw dump with ECC
pub fn process_dump_with_ecc(
    raw_data: &[u8],
    config: &FlashConfig,
) -> Result<Vec<u8>, String> {
    let page_with_oob = config.page_size as usize + config.oob_size as usize;
    let mut processed = Vec::new();

    for chunk in raw_data.chunks(page_with_oob) {
        if chunk.len() < config.page_size as usize {
            break;
        }

        let page_data = &chunk[..config.page_size as usize];
        let oob_data = if chunk.len() > config.page_size as usize {
            &chunk[config.page_size as usize..]
        } else {
            &[]
        };

        let mut data = page_data.to_vec();
        
        // Apply ECC correction if we have OOB data
        if !oob_data.is_empty() && config.ecc_algorithm != EccAlgorithm::None {
            match decode_with_ecc(&mut data, oob_data, &config.ecc_algorithm) {
                Ok(corrected) => {
                    if corrected > 0 {
                        // Log corrected bits
                    }
                }
                Err(_) => {
                    // Mark page as potentially corrupted
                }
            }
        }

        processed.extend(data);
    }

    Ok(processed)
}

/// Extract only data pages (skip OOB)
pub fn extract_data_only(raw_data: &[u8], config: &FlashConfig) -> Vec<u8> {
    let page_with_oob = config.page_size as usize + config.oob_size as usize;
    let mut data_only = Vec::new();

    for chunk in raw_data.chunks(page_with_oob) {
        let data_size = chunk.len().min(config.page_size as usize);
        data_only.extend_from_slice(&chunk[..data_size]);
    }

    data_only
}

/// Calculate dump statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpStats {
    pub total_pages: u32,
    pub empty_pages: u32,
    pub data_pages: u32,
    pub bad_blocks: u32,
}

pub fn calculate_stats(data: &[u8], config: &FlashConfig) -> DumpStats {
    let page_size = config.page_size as usize;
    let mut empty = 0u32;
    let mut with_data = 0u32;

    for chunk in data.chunks(page_size) {
        if chunk.iter().all(|&b| b == 0xFF) {
            empty += 1;
        } else {
            with_data += 1;
        }
    }

    DumpStats {
        total_pages: empty + with_data,
        empty_pages: empty,
        data_pages: with_data,
        bad_blocks: 0, // TODO: Implement bad block detection
    }
}
