//! Turning a raw NAND dump into usable data.
//!
//! A raw dump interleaves each page with its spare (OOB) area. Stripping the
//! spare area and applying the ECC bytes stored in it is what turns the raw
//! capture into the image that was written.
//!
//! The results are reported per page. The previous version discarded them: an
//! uncorrectable page hit `Err(_) => { // Mark page as potentially corrupted }`,
//! a comment where the action should have been, and the damaged page went into
//! the output looking like every other page. For a dump used to recover
//! firmware, or as evidence, silently including unreadable data is the failure
//! that matters most.

use openflash_core::ecc::{decode_with_ecc, EccAlgorithm, EccError};
use serde::{Deserialize, Serialize};

/// Geometry and ECC scheme of the chip a dump came from.
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

/// A page whose ECC could not repair it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UncorrectablePage {
    /// Page index within the dump.
    pub page: u32,
    /// Byte offset of the page in the raw dump.
    pub offset: u64,
    /// Why it could not be corrected.
    pub reason: String,
}

/// What ECC processing produced, and what it could not fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EccProcessResult {
    /// Page data with the spare areas removed.
    pub data: Vec<u8>,
    /// Pages processed.
    pub pages: u32,
    /// Data bits repaired using the stored ECC.
    pub corrected_bits: u32,
    /// Pages where ECC reported damage it could not repair.
    ///
    /// Their data is still present in `data` — dropping it would be worse — but
    /// it is known to be wrong, and the caller must say so.
    pub uncorrectable: Vec<UncorrectablePage>,
    /// Bytes at the end of the dump that do not make up a whole page.
    pub trailing_bytes: u32,
}

impl EccProcessResult {
    /// Whether every page came through intact or repaired.
    pub fn is_clean(&self) -> bool {
        self.uncorrectable.is_empty()
    }
}

/// Strip the spare areas from a raw dump, correcting each page with its ECC.
///
/// Never fails on damaged data: a dump with unreadable pages is exactly the case
/// this is used for. The damage is reported in
/// [`EccProcessResult::uncorrectable`] instead.
pub fn process_dump_with_ecc(
    raw_data: &[u8],
    config: &FlashConfig,
) -> Result<EccProcessResult, String> {
    let page_size = config.page_size as usize;
    let oob_size = config.oob_size as usize;
    if page_size == 0 {
        return Err("page size must not be zero".to_string());
    }
    let stride = page_size + oob_size;

    let mut result = EccProcessResult {
        data: Vec::with_capacity(raw_data.len()),
        pages: 0,
        corrected_bits: 0,
        uncorrectable: Vec::new(),
        trailing_bytes: 0,
    };

    for (page, chunk) in raw_data.chunks(stride).enumerate() {
        if chunk.len() < page_size {
            // A partial page at the end of the dump: reported rather than
            // silently dropped, because it usually means the read was cut short.
            result.trailing_bytes = chunk.len() as u32;
            break;
        }

        let mut data = chunk[..page_size].to_vec();
        let oob = &chunk[page_size..];

        if !oob.is_empty() && config.ecc_algorithm != EccAlgorithm::None {
            match decode_with_ecc(&mut data, oob, &config.ecc_algorithm) {
                Ok(corrected) => result.corrected_bits += corrected,
                Err(error) => result.uncorrectable.push(UncorrectablePage {
                    page: page as u32,
                    offset: (page * stride) as u64,
                    reason: describe(&error),
                }),
            }
        }

        result.data.extend_from_slice(&data);
        result.pages += 1;
    }

    Ok(result)
}

fn describe(error: &EccError) -> String {
    match error {
        EccError::UncorrectableError => "more bit errors than the ECC can repair".to_string(),
        EccError::InvalidInput => {
            "page or ECC size does not match the configured geometry".to_string()
        }
        EccError::InvalidEccData => "the page's ECC bytes are missing or malformed".to_string(),
        EccError::NotImplemented(what) => what.to_string(),
    }
}

/// Strip the spare areas without applying ECC.
pub fn extract_data_only(raw_data: &[u8], config: &FlashConfig) -> Vec<u8> {
    let stride = config.page_size as usize + config.oob_size as usize;
    if stride == 0 {
        return Vec::new();
    }

    let mut data_only = Vec::with_capacity(raw_data.len());
    for chunk in raw_data.chunks(stride) {
        let take = chunk.len().min(config.page_size as usize);
        data_only.extend_from_slice(&chunk[..take]);
    }
    data_only
}

/// Page counts for a dump whose spare areas have already been removed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DumpStats {
    pub total_pages: u32,
    pub empty_pages: u32,
    pub data_pages: u32,
    /// Blocks whose first page is entirely zero.
    ///
    /// A heuristic, not a bad block table: manufacturers mark bad blocks in the
    /// spare area, which this function does not see. Named for what it measures
    /// rather than presented as a bad block count, which the previous version
    /// reported as a hardcoded 0.
    pub all_zero_blocks: u32,
}

pub fn calculate_stats(data: &[u8], config: &FlashConfig) -> DumpStats {
    let page_size = (config.page_size as usize).max(1);
    let pages_per_block = (config.pages_per_block as usize).max(1);

    let mut empty = 0u32;
    let mut with_data = 0u32;
    let mut all_zero_blocks = 0u32;

    for (index, chunk) in data.chunks(page_size).enumerate() {
        if chunk.iter().all(|&byte| byte == 0xFF) {
            empty += 1;
        } else {
            with_data += 1;
        }

        // An all-zero first page of a block is the pattern a worn or failed block
        // usually leaves behind.
        if index % pages_per_block == 0 && chunk.iter().all(|&byte| byte == 0x00) {
            all_zero_blocks += 1;
        }
    }

    DumpStats {
        total_pages: empty + with_data,
        empty_pages: empty,
        data_pages: with_data,
        all_zero_blocks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openflash_core::ecc::encode_with_ecc;

    fn config(algorithm: EccAlgorithm) -> FlashConfig {
        FlashConfig {
            page_size: 512,
            oob_size: 4,
            pages_per_block: 4,
            total_blocks: 8,
            ecc_algorithm: algorithm,
        }
    }

    /// Build a raw dump: each page followed by its own ECC in the spare area.
    fn raw_dump(pages: &[Vec<u8>]) -> Vec<u8> {
        let mut raw = Vec::new();
        for page in pages {
            let (_, ecc) = encode_with_ecc(page, &EccAlgorithm::Hamming).unwrap();
            raw.extend_from_slice(page);
            raw.extend_from_slice(&ecc);
        }
        raw
    }

    fn page(seed: u32) -> Vec<u8> {
        (0..512u32).map(|i| ((i * 31 + seed) % 251) as u8).collect()
    }

    #[test]
    fn the_spare_area_is_stripped_and_intact_pages_pass_through() {
        let pages = vec![page(1), page(2)];
        let raw = raw_dump(&pages);

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::Hamming)).unwrap();

        assert_eq!(result.pages, 2);
        assert_eq!(result.corrected_bits, 0);
        assert!(result.is_clean());
        assert_eq!(result.data, [pages[0].clone(), pages[1].clone()].concat());
    }

    #[test]
    fn a_single_bit_error_is_repaired_and_counted() {
        let pages = vec![page(3)];
        let mut raw = raw_dump(&pages);
        raw[100] ^= 0x10;

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::Hamming)).unwrap();

        assert_eq!(result.corrected_bits, 1);
        assert!(result.is_clean());
        assert_eq!(result.data, pages[0], "the page must come out as written");
    }

    /// The case the old code hid: two bit errors in a page cannot be repaired,
    /// and the caller has to be told which page.
    #[test]
    fn an_uncorrectable_page_is_named_rather_than_passed_off_as_clean() {
        let pages = vec![page(4), page(5)];
        let mut raw = raw_dump(&pages);

        // Two flipped bits in the second page.
        let second_page_start = 516;
        raw[second_page_start + 10] ^= 0x01;
        raw[second_page_start + 20] ^= 0x80;

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::Hamming)).unwrap();

        assert!(!result.is_clean());
        assert_eq!(result.uncorrectable.len(), 1);
        let bad = &result.uncorrectable[0];
        assert_eq!(bad.page, 1);
        assert_eq!(bad.offset, second_page_start as u64);
        assert!(bad.reason.contains("more bit errors"), "{}", bad.reason);

        // Both pages are still present: dropping data would be worse than
        // reporting it as damaged.
        assert_eq!(result.pages, 2);
        assert_eq!(result.data.len(), 1024);
    }

    #[test]
    fn a_truncated_final_page_is_reported() {
        let pages = vec![page(6)];
        let mut raw = raw_dump(&pages);
        raw.extend_from_slice(&[0xAB; 200]);

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::Hamming)).unwrap();

        assert_eq!(result.pages, 1);
        assert_eq!(result.trailing_bytes, 200);
    }

    #[test]
    fn bch_is_reported_per_page_rather_than_silently_ignored() {
        let pages = vec![page(7)];
        let raw = raw_dump(&pages);

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::Bch { t: 4 })).unwrap();

        assert_eq!(result.uncorrectable.len(), 1);
        assert!(
            result.uncorrectable[0].reason.contains("BCH"),
            "{}",
            result.uncorrectable[0].reason
        );
    }

    #[test]
    fn without_ecc_the_pages_are_only_stripped() {
        let pages = vec![page(8)];
        let raw = raw_dump(&pages);

        let result = process_dump_with_ecc(&raw, &config(EccAlgorithm::None)).unwrap();
        assert!(result.is_clean());
        assert_eq!(result.data, pages[0]);
    }

    #[test]
    fn extract_data_only_drops_the_spare_areas() {
        let pages = vec![page(9), page(10)];
        let raw = raw_dump(&pages);

        let stripped = extract_data_only(&raw, &config(EccAlgorithm::None));
        assert_eq!(stripped, [pages[0].clone(), pages[1].clone()].concat());
    }

    #[test]
    fn a_zero_page_size_is_refused_rather_than_dividing_by_zero() {
        let mut cfg = config(EccAlgorithm::None);
        cfg.page_size = 0;
        assert!(process_dump_with_ecc(&[0u8; 16], &cfg).is_err());
    }

    #[test]
    fn statistics_separate_erased_pages_from_written_ones() {
        let mut data = vec![0xFFu8; 512 * 3];
        data.extend_from_slice(&[0x42u8; 512]);

        let stats = calculate_stats(&data, &config(EccAlgorithm::None));
        assert_eq!(stats.total_pages, 4);
        assert_eq!(stats.empty_pages, 3);
        assert_eq!(stats.data_pages, 1);
    }

    #[test]
    fn statistics_count_all_zero_block_starts() {
        // Four pages per block; the first block starts all-zero, the second does
        // not.
        let mut data = vec![0x00u8; 512];
        data.extend_from_slice(&[0xFFu8; 512 * 3]);
        data.extend_from_slice(&[0x11u8; 512]);
        data.extend_from_slice(&[0xFFu8; 512 * 3]);

        let stats = calculate_stats(&data, &config(EccAlgorithm::None));
        assert_eq!(stats.all_zero_blocks, 1);
    }
}
