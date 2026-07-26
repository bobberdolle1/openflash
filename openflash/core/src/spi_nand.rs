//! SPI NAND Flash chip database and protocol
//! Contains known SPI NAND chip parameters and command definitions

use serde::{Deserialize, Serialize};

/// SPI NAND chip information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiNandChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub size_mb: u32,
    pub page_size: u32,
    pub block_size: u32, // pages per block
    pub oob_size: u32,   // spare/OOB bytes per page
    pub voltage: String,
    pub max_clock_mhz: u8, // Maximum SPI clock frequency
    pub has_qspi: bool,    // Quad SPI support
    pub has_ecc: bool,     // Internal ECC
    pub cell_type: SpiNandCellType,
    pub planes: u8, // Number of planes (1, 2, or 4)
}

/// SPI NAND cell type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpiNandCellType {
    SLC,
    MLC,
    TLC,
}

/// SPI NAND standard commands
pub mod commands {
    // Reset
    pub const RESET: u8 = 0xFF;

    // Identification
    pub const READ_ID: u8 = 0x9F;

    // Feature operations
    pub const GET_FEATURE: u8 = 0x0F;
    pub const SET_FEATURE: u8 = 0x1F;

    // Read operations
    pub const PAGE_READ: u8 = 0x13; // Load page to cache
    pub const READ_FROM_CACHE: u8 = 0x03; // Read from cache (1-bit)
    pub const READ_FROM_CACHE_X2: u8 = 0x3B; // Read from cache (2-bit)
    pub const READ_FROM_CACHE_X4: u8 = 0x6B; // Read from cache (4-bit)
    pub const READ_FROM_CACHE_DUAL_IO: u8 = 0xBB;
    pub const READ_FROM_CACHE_QUAD_IO: u8 = 0xEB;

    // Write operations
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const WRITE_DISABLE: u8 = 0x04;
    pub const PROGRAM_LOAD: u8 = 0x02; // Load data to cache
    pub const PROGRAM_LOAD_X4: u8 = 0x32; // Load data to cache (4-bit)
    pub const PROGRAM_LOAD_RANDOM: u8 = 0x84; // Random data input
    pub const PROGRAM_EXECUTE: u8 = 0x10; // Program cache to array

    // Erase operations
    pub const BLOCK_ERASE: u8 = 0xD8;

    // Protection
    pub const READ_STATUS: u8 = 0x05; // Alias for GET_FEATURE(0xC0)
}

/// SPI NAND feature register addresses
pub mod features {
    pub const PROTECTION: u8 = 0xA0;
    pub const FEATURE: u8 = 0xB0;
    pub const STATUS: u8 = 0xC0;
    pub const DIE_SELECT: u8 = 0xD0;
}

/// Status register bits
pub mod status {
    pub const OIP: u8 = 0x01; // Operation In Progress
    pub const WEL: u8 = 0x02; // Write Enable Latch
    pub const E_FAIL: u8 = 0x04; // Erase Fail
    pub const P_FAIL: u8 = 0x08; // Program Fail
    pub const ECC_S0: u8 = 0x10; // ECC Status bit 0
    pub const ECC_S1: u8 = 0x20; // ECC Status bit 1
}

/// Feature register bits (0xB0)
pub mod feature_bits {
    pub const QE: u8 = 0x01; // Quad Enable
    pub const ECC_EN: u8 = 0x10; // ECC Enable
    pub const BUF: u8 = 0x08; // Buffer mode
}

/// Get manufacturer name from ID
pub fn get_spi_nand_manufacturer_name(id: u8) -> &'static str {
    match id {
        0xC8 => "GigaDevice",
        0xEF => "Winbond",
        0xC2 => "Macronix",
        0x2C => "Micron",
        0x98 => "Toshiba/Kioxia",
        0x01 => "Spansion/Cypress",
        0xA1 => "Fudan Micro",
        0x0B => "XTX",
        0xCD => "Foresee/Longsys",
        0xE5 => "Dosilicon",
        0xBA => "Zetta",
        0x85 => "Puya",
        0x68 => "Boya",
        _ => "Unknown",
    }
}

/// Database of known SPI NAND flash chips
/// Returns chip info based on manufacturer ID and device ID
pub fn get_spi_nand_chip_info(chip_id: &[u8]) -> Option<SpiNandChipInfo> {
    if chip_id.len() < 2 {
        return None;
    }

    let mfr = chip_id[0];
    let device = if chip_id.len() >= 3 {
        chip_id[1..3].to_vec()
    } else {
        vec![chip_id[1]]
    };

    // Try exact match first
    if let Some(info) = get_spi_nand_chip_info_exact(mfr, &device) {
        return Some(info);
    }

    // Fall back to generic detection
    get_spi_nand_chip_info_generic(mfr, &device)
}

fn get_spi_nand_chip_info_exact(mfr: u8, device: &[u8]) -> Option<SpiNandChipInfo> {
    match (mfr, device) {
        // ============ GigaDevice ============
        // GD5F1GQ4UBxIG - 128MB SLC
        (0xC8, [0xD1, ..]) | (0xC8, [0xB1, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F1GQ4UBxIG".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F2GQ4UBxIG - 256MB SLC
        (0xC8, [0xD2, ..]) | (0xC8, [0xB2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F2GQ4UBxIG".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F4GQ4UBxIG - 512MB SLC
        (0xC8, [0xD4, ..]) | (0xC8, [0xB4, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F4GQ4UBxIG".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F1GQ5UExxG - 128MB 1.8V
        (0xC8, [0x51, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F1GQ5UExxG".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Winbond ============
        // W25N01GV - 128MB SLC
        (0xEF, [0xAA, 0x21]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N01GV".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // W25N02KV - 256MB SLC
        (0xEF, [0xAA, 0x22]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N02KV".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 2,
        }),
        // W25N04KV - 512MB SLC
        (0xEF, [0xAA, 0x23]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N04KV".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 4,
        }),
        // W25N01JW - 128MB 1.8V
        (0xEF, [0xBC, 0x21]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N01JW".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Macronix ============
        // MX35LF1GE4AB - 128MB SLC
        (0xC2, [0x12, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35LF1GE4AB".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // MX35LF2GE4AB - 256MB SLC
        (0xC2, [0x22, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35LF2GE4AB".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // MX35LF4GE4AD - 512MB SLC
        (0xC2, [0x37, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35LF4GE4AD".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Micron ============
        // MT29F1G01ABAFD - 128MB SLC
        (0x2C, [0x14, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F1G01ABAFD".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // MT29F2G01ABAGD - 256MB SLC
        (0x2C, [0x24, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F2G01ABAGD".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 2,
        }),
        // MT29F4G01ABAFD - 512MB SLC
        (0x2C, [0x34, ..]) | (0x2C, [0x36, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F4G01ABAFD".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 256,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Toshiba/Kioxia ============
        // TC58CVG0S3HRAIG - 128MB SLC
        (0x98, [0xC2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58CVG0S3HRAIG".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // TC58CVG1S3HRAIG - 256MB SLC
        (0x98, [0xCB, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58CVG1S3HRAIG".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // TC58CVG2S0HRAIG - 512MB SLC
        (0x98, [0xCD, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58CVG2S0HRAIG".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ XTX ============
        // XT26G01A - 128MB SLC
        (0x0B, [0xE1, ..]) => Some(SpiNandChipInfo {
            manufacturer: "XTX".into(),
            model: "XT26G01A".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // XT26G02A - 256MB SLC
        (0x0B, [0xE2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "XTX".into(),
            model: "XT26G02A".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // XT26G04A - 512MB SLC (v2.2)
        (0x0B, [0xE4, ..]) => Some(SpiNandChipInfo {
            manufacturer: "XTX".into(),
            model: "XT26G04A".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // XT26G01C - 128MB SLC 1.8V (v2.2)
        (0x0B, [0x11, ..]) => Some(SpiNandChipInfo {
            manufacturer: "XTX".into(),
            model: "XT26G01C".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // XT26G02C - 256MB SLC 1.8V (v2.2)
        (0x0B, [0x12, ..]) => Some(SpiNandChipInfo {
            manufacturer: "XTX".into(),
            model: "XT26G02C".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Winbond W25N Series (v2.2) ============
        // W25N512GV - 64MB SLC
        (0xEF, [0xAA, 0x20]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N512GV".into(),
            size_mb: 64,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // W25N01KV - 128MB SLC (new series)
        (0xEF, [0xAE, 0x21]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N01KV".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // W25N02GV - 256MB SLC
        (0xEF, [0xAA, 0x24]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N02GV".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 2,
        }),
        // W25N01JW - 128MB 1.8V (already exists, adding W25N02JW)
        // W25N02JW - 256MB 1.8V
        (0xEF, [0xBC, 0x22]) => Some(SpiNandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25N02JW".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 2,
        }),

        // ============ GigaDevice GD5F1GM9 Series (v2.2) ============
        // GD5F1GM9UxxG - 128MB High-Speed QSPI NAND
        (0xC8, [0x91, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F1GM9UxxG".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 166,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F2GM9UxxG - 256MB High-Speed
        (0xC8, [0x92, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F2GM9UxxG".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 166,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F4GM9UxxG - 512MB High-Speed
        (0xC8, [0x94, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F4GM9UxxG".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 256,
            voltage: "3.3V".into(),
            max_clock_mhz: 166,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F1GQ5UExxH - 128MB 1.8V
        (0xC8, [0x31, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F1GQ5UExxH".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // GD5F2GQ5UExxH - 256MB 1.8V
        (0xC8, [0x32, ..]) => Some(SpiNandChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD5F2GQ5UExxH".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 128,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Foresee/Longsys (v2.2) ============
        // FS35ND01G - 128MB SLC
        (0xCD, [0xB1, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Foresee".into(),
            model: "FS35ND01G".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // FS35ND02G - 256MB SLC
        (0xCD, [0xB2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Foresee".into(),
            model: "FS35ND02G".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // FS35ND04G - 512MB SLC
        (0xCD, [0xB4, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Foresee".into(),
            model: "FS35ND04G".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Dosilicon (v2.2) ============
        // DS35Q1GA - 128MB SLC
        (0xE5, [0x71, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Dosilicon".into(),
            model: "DS35Q1GA".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // DS35Q2GA - 256MB SLC
        (0xE5, [0x72, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Dosilicon".into(),
            model: "DS35Q2GA".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Zetta (v2.2) ============
        // ZD35Q1GA - 128MB SLC
        (0xBA, [0x21, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Zetta".into(),
            model: "ZD35Q1GA".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // ZD35Q2GA - 256MB SLC
        (0xBA, [0x22, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Zetta".into(),
            model: "ZD35Q2GA".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Micron MT29F SPI NAND (v2.2) ============
        // MT29F8G01ADAFD - 1GB SLC
        (0x2C, [0x46, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F8G01ADAFD".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 256,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 2,
        }),

        // ============ Macronix MX35UF Series 1.8V (v2.2) ============
        // MX35UF1GE4AD - 128MB 1.8V
        (0xC2, [0x92, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35UF1GE4AD".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // MX35UF2GE4AD - 256MB 1.8V
        (0xC2, [0xA2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35UF2GE4AD".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // MX35UF4GE4AD - 512MB 1.8V
        (0xC2, [0xB7, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX35UF4GE4AD".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        // ============ Toshiba/Kioxia TC58 Series (v2.2) ============
        // TC58CVG2S0HRAIJ - 512MB SLC (new revision)
        (0x98, [0xED, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58CVG2S0HRAIJ".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 128,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // TC58CYG0S3HRAIJ - 128MB 1.8V
        (0x98, [0xD2, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58CYG0S3HRAIJ".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),
        // TC58CYG1S3HRAIJ - 256MB 1.8V
        (0x98, [0xDB, ..]) => Some(SpiNandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58CYG1S3HRAIJ".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_ecc: true,
            cell_type: SpiNandCellType::SLC,
            planes: 1,
        }),

        _ => None,
    }
}

/// Generic SPI NAND chip detection based on device ID patterns
fn get_spi_nand_chip_info_generic(mfr: u8, device: &[u8]) -> Option<SpiNandChipInfo> {
    let manufacturer = get_spi_nand_manufacturer_name(mfr).to_string();

    if device.is_empty() {
        return None;
    }

    // Try to decode size from common device ID patterns
    let first_byte = device[0];
    let (size_mb, page_size, oob_size) = match first_byte & 0x0F {
        0x01 | 0x11 | 0x21 => (128, 2048, 64),   // 1Gbit
        0x02 | 0x12 | 0x22 => (256, 2048, 64),   // 2Gbit
        0x04 | 0x14 | 0x24 => (512, 4096, 128),  // 4Gbit
        0x08 | 0x18 | 0x28 => (1024, 4096, 256), // 8Gbit
        _ => {
            // Alternative pattern based on high nibble
            match first_byte >> 4 {
                0xD | 0xB | 0xA => match first_byte & 0x0F {
                    1 => (128, 2048, 64),
                    2 => (256, 2048, 64),
                    4 => (512, 4096, 128),
                    _ => return None,
                },
                _ => return None,
            }
        }
    };

    Some(SpiNandChipInfo {
        manufacturer,
        model: format!("Generic SPI NAND 0x{:02X}", first_byte),
        size_mb,
        page_size,
        block_size: 64,
        oob_size,
        voltage: "3.3V".into(),
        max_clock_mhz: 80,
        has_qspi: true,
        has_ecc: true,
        cell_type: SpiNandCellType::SLC,
        planes: 1,
    })
}

/// SPI NAND operation result with ECC status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiNandReadResult {
    pub data: Vec<u8>,
    pub ecc_status: EccStatus,
}

/// ECC status from internal ECC engine
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EccStatus {
    /// No errors detected
    NoError,
    /// Errors corrected (1-4 bits typically)
    Corrected(u8),
    /// Uncorrectable errors
    Uncorrectable,
    /// ECC disabled or not available
    Disabled,
}

impl EccStatus {
    /// Parse ECC status from status register
    pub fn from_status_register(status: u8) -> Self {
        let ecc_bits = (status >> 4) & 0x03;
        match ecc_bits {
            0b00 => EccStatus::NoError,
            0b01 => EccStatus::Corrected(1), // 1-3 bits corrected
            0b10 => EccStatus::Corrected(4), // 4+ bits corrected
            0b11 => EccStatus::Uncorrectable,
            _ => EccStatus::Disabled,
        }
    }
}

/// Calculate page address for SPI NAND
/// SPI NAND uses row address (block + page within block)
pub fn calculate_row_address(block: u32, page_in_block: u32, pages_per_block: u32) -> u32 {
    block * pages_per_block + page_in_block
}

/// Column address for a byte within a SPI NAND page.
///
/// SPI NAND addresses the page buffer linearly: the spare (OOB) area continues
/// straight on from the main area, so the column address of any byte is just its
/// offset from the start of the page. No translation is needed — this function
/// exists to range-check the access, which is the part that is easy to get
/// wrong.
///
/// Returns `None` when `offset` falls outside the page: past `page_size` when
/// only the main area is addressable, or past `page_size + oob_size` when the
/// spare area is included.
pub fn calculate_column_address(
    offset: u16,
    include_oob: bool,
    page_size: u16,
    oob_size: u16,
) -> Option<u16> {
    let addressable = if include_oob {
        page_size.saturating_add(oob_size)
    } else {
        page_size
    };
    (offset < addressable).then_some(offset)
}

#[cfg(test)]
mod column_address_tests {
    use super::calculate_column_address;

    #[test]
    fn offsets_inside_the_main_area_map_to_themselves() {
        assert_eq!(calculate_column_address(0, false, 2048, 64), Some(0));
        assert_eq!(calculate_column_address(2047, false, 2048, 64), Some(2047));
    }

    #[test]
    fn the_spare_area_is_only_addressable_when_requested() {
        // The first spare byte sits immediately after the main area.
        assert_eq!(calculate_column_address(2048, true, 2048, 64), Some(2048));
        assert_eq!(calculate_column_address(2048, false, 2048, 64), None);
    }

    #[test]
    fn offsets_past_the_end_of_the_page_are_rejected() {
        assert_eq!(calculate_column_address(2112, true, 2048, 64), None);
        assert_eq!(calculate_column_address(u16::MAX, true, 2048, 64), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gigadevice_chip_recognition() {
        let chip_id = [0xC8, 0xD1, 0x00];
        let chip_info = get_spi_nand_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "GigaDevice");
        assert!(chip_info.model.contains("GD5F1GQ4"));
        assert_eq!(chip_info.size_mb, 128);
        assert!(chip_info.has_qspi);
    }

    #[test]
    fn test_winbond_chip_recognition() {
        let chip_id = [0xEF, 0xAA, 0x21];
        let chip_info = get_spi_nand_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Winbond");
        assert_eq!(chip_info.model, "W25N01GV");
        assert_eq!(chip_info.size_mb, 128);
    }

    #[test]
    fn test_micron_chip_recognition() {
        let chip_id = [0x2C, 0x24, 0x00];
        let chip_info = get_spi_nand_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Micron");
        assert_eq!(chip_info.size_mb, 256);
        assert_eq!(chip_info.planes, 2);
    }

    #[test]
    fn test_generic_detection() {
        let chip_id = [0xC8, 0xD2]; // Unknown GigaDevice 256MB
        let chip_info = get_spi_nand_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "GigaDevice");
        assert_eq!(chip_info.size_mb, 256);
    }

    #[test]
    fn test_manufacturer_names() {
        assert_eq!(get_spi_nand_manufacturer_name(0xC8), "GigaDevice");
        assert_eq!(get_spi_nand_manufacturer_name(0xEF), "Winbond");
        assert_eq!(get_spi_nand_manufacturer_name(0x2C), "Micron");
        assert_eq!(get_spi_nand_manufacturer_name(0xFF), "Unknown");
    }

    #[test]
    fn test_ecc_status_parsing() {
        assert_eq!(EccStatus::from_status_register(0x00), EccStatus::NoError);
        assert_eq!(
            EccStatus::from_status_register(0x10),
            EccStatus::Corrected(1)
        );
        assert_eq!(
            EccStatus::from_status_register(0x20),
            EccStatus::Corrected(4)
        );
        assert_eq!(
            EccStatus::from_status_register(0x30),
            EccStatus::Uncorrectable
        );
    }

    #[test]
    fn test_row_address_calculation() {
        // Block 10, page 5, 64 pages per block
        let addr = calculate_row_address(10, 5, 64);
        assert_eq!(addr, 10 * 64 + 5);
    }
}
