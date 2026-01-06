//! ONFI NAND Flash chip database and detection
//! Contains known chip parameters and auto-detection logic

use serde::{Deserialize, Serialize};

/// NAND chip information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NandChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub size_mb: u32,
    pub page_size: u32,
    pub block_size: u32,     // pages per block
    pub oob_size: u32,       // spare/OOB bytes per page
    pub voltage: String,
    pub timing: NandTiming,
    pub bus_width: u8,       // 8 or 16 bit
    pub cell_type: CellType,
}

/// NAND cell type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CellType {
    SLC,
    MLC,
    TLC,
    QLC,
}

/// NAND timing parameters (in nanoseconds)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NandTiming {
    pub tRP: u8,   // RE# pulse width
    pub tWP: u8,   // WE# pulse width
    pub tCLS: u8,  // CLE setup time
    pub tALS: u8,  // ALE setup time
    pub tRR: u8,   // Ready to RE#
    pub tAR: u8,   // ALE to RE#
    pub tCLR: u8,  // Command latch to RE#
    pub tRHW: u8,  // RE# high to WE# low
    pub tWHR: u8,  // WE# high to RE#
    pub tR: u8,    // Page read time (in microseconds)
}

impl Default for NandTiming {
    fn default() -> Self {
        // Conservative ONFI Mode 0 timing
        Self {
            tRP: 50,
            tWP: 50,
            tCLS: 50,
            tALS: 50,
            tRR: 40,
            tAR: 25,
            tCLR: 20,
            tRHW: 200,
            tWHR: 120,
            tR: 200,
        }
    }
}

/// Fast timing for ONFI Mode 4/5
pub fn fast_timing() -> NandTiming {
    NandTiming {
        tRP: 12,
        tWP: 12,
        tCLS: 12,
        tALS: 12,
        tRR: 20,
        tAR: 10,
        tCLR: 10,
        tRHW: 100,
        tWHR: 60,
        tR: 25,
    }
}

/// Manufacturer IDs
pub mod manufacturers {
    pub const SAMSUNG: u8 = 0xEC;
    pub const TOSHIBA: u8 = 0x98;
    pub const HYNIX: u8 = 0xAD;
    pub const MICRON: u8 = 0x2C;
    pub const INTEL: u8 = 0x89;
    pub const SPANSION: u8 = 0x01;
    pub const MACRONIX: u8 = 0xC2;
    pub const WINBOND: u8 = 0xEF;
    pub const GIGADEVICE: u8 = 0xC8;
    pub const ESMT: u8 = 0x92;
}

/// Get manufacturer name from ID
pub fn get_manufacturer_name(id: u8) -> &'static str {
    match id {
        manufacturers::SAMSUNG => "Samsung",
        manufacturers::TOSHIBA => "Toshiba/Kioxia",
        manufacturers::HYNIX => "SK Hynix",
        manufacturers::MICRON => "Micron",
        manufacturers::INTEL => "Intel",
        manufacturers::SPANSION => "Spansion/Cypress",
        manufacturers::MACRONIX => "Macronix",
        manufacturers::WINBOND => "Winbond",
        manufacturers::GIGADEVICE => "GigaDevice",
        manufacturers::ESMT => "ESMT",
        _ => "Unknown",
    }
}

/// Database of known NAND flash chips
/// Returns chip info based on 5-byte chip ID
pub fn get_chip_info(chip_id: &[u8]) -> Option<NandChipInfo> {
    if chip_id.len() < 2 {
        return None;
    }

    let mfr = chip_id[0];
    let device = chip_id[1];
    
    // Try exact match first
    if chip_id.len() >= 5 {
        if let Some(info) = get_chip_info_exact(chip_id) {
            return Some(info);
        }
    }
    
    // Fall back to generic detection based on device ID
    get_chip_info_generic(mfr, device)
}

fn get_chip_info_exact(chip_id: &[u8]) -> Option<NandChipInfo> {
    match chip_id {
        // ============ Samsung ============
        // K9F1G08U0B - 128MB SLC
        [0xEC, 0xF1, 0x00, 0x95, 0x40] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F1G08U0B".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9F2G08U0C - 256MB SLC
        [0xEC, 0xDA, 0x10, 0x95, 0x44] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F2G08U0C".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9F4G08U0D - 512MB SLC
        [0xEC, 0xDC, 0x10, 0x95, 0x54] | [0xEC, 0xDC, 0x10, 0x95, 0x50] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F4G08U0D".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9F8G08U0M - 1GB SLC (4KB page)
        [0xEC, 0xD3, 0x51, 0x95, 0x58] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F8G08U0M".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9K8G08U0M - 1GB SLC
        [0xEC, 0xD7, 0x10, 0x95, 0x44] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9K8G08U0M".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9GAG08U0E - 2GB MLC
        [0xEC, 0xD5, 0x84, 0x72, 0x50] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9GAG08U0E".into(),
            size_mb: 2048,
            page_size: 8192,
            block_size: 128,
            oob_size: 436,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // K9LBG08U0M - 4GB MLC
        [0xEC, 0xD7, 0xD5, 0x29, 0x38] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9LBG08U0M".into(),
            size_mb: 4096,
            page_size: 4096,
            block_size: 128,
            oob_size: 128,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),

        // ============ Hynix ============
        // HY27UF081G2A - 128MB SLC
        [0xAD, 0xF1, 0x80, 0x1D, ..] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "HY27UF081G2A".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // HY27UF082G2A - 256MB SLC
        [0xAD, 0xDA, 0x10, 0x95, 0x44] | [0xAD, 0xDC, 0x10, 0x95, 0x50] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "HY27UF082G2A".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // H27U4G8F2DTR - 512MB SLC
        [0xAD, 0xDC, 0x90, 0x95, 0x54] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27U4G8F2DTR".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // H27UAG8T2BTR - 2GB MLC
        [0xAD, 0xD5, 0x94, 0x25, 0x44] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27UAG8T2BTR".into(),
            size_mb: 2048,
            page_size: 4096,
            block_size: 128,
            oob_size: 224,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),

        // ============ Micron ============
        // MT29F1G08ABADAWP - 128MB SLC
        [0x2C, 0xF1, 0x80, 0x95, 0x04] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F1G08ABADAWP".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MT29F2G08ABAEAWP - 256MB SLC
        [0x2C, 0xDA, 0x90, 0x95, 0x06] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F2G08ABAEAWP".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MT29F4G08ABADAWP - 512MB SLC
        [0x2C, 0xDC, 0x90, 0x95, 0x56] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F4G08ABADAWP".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MT29F8G08ADBDAWP - 1GB SLC
        [0x2C, 0xD3, 0xD1, 0x95, 0xA6] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F8G08ADBDAWP".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 224,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MT29F16G08CBACAWP - 2GB MLC
        [0x2C, 0x48, 0x04, 0x46, 0x85] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F16G08CBACAWP".into(),
            size_mb: 2048,
            page_size: 4096,
            block_size: 256,
            oob_size: 224,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),

        // ============ Toshiba/Kioxia ============
        // TC58NVG0S3ETA00 - 128MB SLC
        [0x98, 0xF1, 0x80, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG0S3ETA00".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // TC58NVG1S3ETA00 - 256MB SLC
        [0x98, 0xDA, 0x90, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG1S3ETA00".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // TC58NVG2S3ETA00 - 512MB SLC
        [0x98, 0xDC, 0x90, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG2S3ETA00".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        // ============ Macronix ============
        // MX30LF1G08AA - 128MB SLC
        [0xC2, 0xF1, 0x80, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30LF1G08AA".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MX30LF2G18AC - 256MB SLC
        [0xC2, 0xDA, 0x90, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30LF2G18AC".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        // ============ Winbond ============
        // W29N01GVSIAA - 128MB SLC
        [0xEF, 0xF1, 0x00, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W29N01GVSIAA".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        // ============ GigaDevice ============
        // GD9FU1G8F2A - 128MB SLC
        [0xC8, 0xF1, 0x80, 0x1D, ..] => Some(NandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD9FU1G8F2A".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        _ => None,
    }
}

/// Generic chip detection based on device ID byte
fn get_chip_info_generic(mfr: u8, device: u8) -> Option<NandChipInfo> {
    let manufacturer = get_manufacturer_name(mfr).to_string();
    
    // Decode device ID according to ONFI conventions
    let (size_mb, page_size, block_size, cell_type) = match device {
        // 128MB class
        0xF1 => (128, 2048, 64, CellType::SLC),
        // 256MB class
        0xDA => (256, 2048, 64, CellType::SLC),
        // 512MB class
        0xDC => (512, 2048, 64, CellType::SLC),
        // 1GB class
        0xD3 => (1024, 4096, 64, CellType::SLC),
        // 2GB class (often MLC)
        0xD5 => (2048, 4096, 128, CellType::MLC),
        // 4GB class
        0xD7 => (4096, 4096, 128, CellType::MLC),
        // 8GB class
        0xDE => (8192, 8192, 256, CellType::MLC),
        // 16GB+ class
        0x48 => (2048, 4096, 256, CellType::MLC),
        0x68 => (4096, 8192, 256, CellType::MLC),
        0x88 => (8192, 8192, 256, CellType::TLC),
        _ => return None,
    };

    Some(NandChipInfo {
        manufacturer,
        model: format!("Generic 0x{:02X}", device),
        size_mb,
        page_size,
        block_size,
        oob_size: page_size / 32, // Typical OOB ratio
        voltage: "3.3V".into(),
        timing: NandTiming::default(),
        bus_width: 8,
        cell_type,
    })
}

/// Parse ONFI parameter page (256 bytes)
pub fn parse_onfi_parameter_page(data: &[u8]) -> Option<NandChipInfo> {
    if data.len() < 256 {
        return None;
    }

    // Check ONFI signature "ONFI"
    if &data[0..4] != b"ONFI" {
        return None;
    }

    // Parse manufacturer (bytes 32-43)
    let manufacturer = String::from_utf8_lossy(&data[32..44])
        .trim()
        .to_string();

    // Parse model (bytes 44-63)
    let model = String::from_utf8_lossy(&data[44..64])
        .trim()
        .to_string();

    // Parse geometry
    let page_size = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
    let oob_size = u16::from_le_bytes([data[84], data[85]]) as u32;
    let pages_per_block = u32::from_le_bytes([data[92], data[93], data[94], data[95]]);
    let blocks_per_lun = u32::from_le_bytes([data[96], data[97], data[98], data[99]]);
    let luns = data[100];

    let total_blocks = blocks_per_lun * luns as u32;
    let size_mb = (total_blocks as u64 * pages_per_block as u64 * page_size as u64 / 1024 / 1024) as u32;

    // Parse timing (simplified)
    let t_prog = u16::from_le_bytes([data[133], data[134]]);
    let t_r = u16::from_le_bytes([data[139], data[140]]);

    Some(NandChipInfo {
        manufacturer,
        model,
        size_mb,
        page_size,
        block_size: pages_per_block,
        oob_size,
        voltage: "3.3V".into(),
        timing: NandTiming {
            tR: (t_r / 1000).min(255) as u8,
            ..NandTiming::default()
        },
        bus_width: 8,
        cell_type: CellType::SLC, // Would need to parse features byte
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samsung_chip_recognition() {
        let chip_id = [0xEC, 0xD7, 0x10, 0x95, 0x44];
        let chip_info = get_chip_info(&chip_id).unwrap();
        
        assert_eq!(chip_info.manufacturer, "Samsung");
        assert_eq!(chip_info.model, "K9K8G08U0M");
        assert_eq!(chip_info.page_size, 4096);
    }

    #[test]
    fn test_generic_detection() {
        let chip_id = [0x2C, 0xDA]; // Micron 256MB
        let chip_info = get_chip_info(&chip_id).unwrap();
        
        assert_eq!(chip_info.manufacturer, "Micron");
        assert_eq!(chip_info.size_mb, 256);
    }

    #[test]
    fn test_manufacturer_names() {
        assert_eq!(get_manufacturer_name(0xEC), "Samsung");
        assert_eq!(get_manufacturer_name(0x2C), "Micron");
        assert_eq!(get_manufacturer_name(0xAD), "SK Hynix");
        assert_eq!(get_manufacturer_name(0x98), "Toshiba/Kioxia");
        assert_eq!(get_manufacturer_name(0xFF), "Unknown");
    }

    #[test]
    fn test_hynix_chip() {
        let chip_id = [0xAD, 0xD5, 0x94, 0x25, 0x44];
        let chip_info = get_chip_info(&chip_id).unwrap();
        
        assert_eq!(chip_info.manufacturer, "SK Hynix");
        assert_eq!(chip_info.cell_type, CellType::MLC);
    }
}
