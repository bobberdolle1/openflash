//! ONFI NAND Flash chip database and detection
//! Contains known chip parameters and auto-detection logic
//! Supports ONFI 1.0 through 5.0 specifications

use serde::{Deserialize, Serialize};

// ============================================================================
// 16-bit NAND Bus Width Support
// ============================================================================

/// Bus width configuration for parallel NAND
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NandBusWidth {
    /// 8-bit data bus (x8)
    #[default]
    X8,
    /// 16-bit data bus (x16)
    X16,
}

impl NandBusWidth {
    /// Returns the bus width in bits
    pub fn bits(&self) -> u8 {
        match self {
            NandBusWidth::X8 => 8,
            NandBusWidth::X16 => 16,
        }
    }
}

/// Detect bus width by comparing chip ID read in x8 and x16 modes
///
/// In x16 mode, the chip ID bytes are interleaved with zeros on the upper byte
/// when read through an 8-bit interface. This function compares the two readings
/// to determine the actual bus width.
///
/// # Arguments
/// * `id_x8` - Chip ID read assuming 8-bit bus
/// * `id_x16` - Chip ID read assuming 16-bit bus (lower bytes of 16-bit words)
///
/// # Returns
/// * `NandBusWidth::X16` if the chip appears to be 16-bit
/// * `NandBusWidth::X8` otherwise
pub fn detect_bus_width(id_x8: &[u8], id_x16: &[u8]) -> NandBusWidth {
    if id_x8.is_empty() || id_x16.is_empty() {
        return NandBusWidth::X8;
    }

    // In x16 mode, when read through x8 interface, every other byte is 0x00
    // because the upper byte of each 16-bit word is not connected
    //
    // For a true x16 chip:
    // - x8 read: [MFR, 0x00, DEV, 0x00, ...]
    // - x16 read: [MFR, DEV, ...]
    //
    // For an x8 chip:
    // - x8 read: [MFR, DEV, ...]
    // - x16 read: [MFR, DEV, ...] (same)

    // Check if x8 reading has alternating zeros (indicating x16 chip read via x8)
    let has_alternating_zeros = id_x8.len() >= 4
        && id_x8[1] == 0x00
        && id_x8[3] == 0x00
        && id_x8[0] != 0x00
        && id_x8[2] != 0x00;

    // Also check if x16 reading matches the non-zero bytes of x8
    let x16_matches = if has_alternating_zeros && id_x16.len() >= 2 {
        id_x16[0] == id_x8[0] && id_x16[1] == id_x8[2]
    } else {
        false
    };

    if has_alternating_zeros && x16_matches {
        NandBusWidth::X16
    } else {
        NandBusWidth::X8
    }
}

/// Convert 16-bit words to bytes with proper endianness
///
/// # Arguments
/// * `data` - Slice of 16-bit words to convert
/// * `little_endian` - If true, use little-endian byte order (LSB first)
///
/// # Returns
/// * Vector of bytes representing the 16-bit data
///
/// # Example
/// ```
/// use openflash_core::onfi::x16_to_bytes;
///
/// let words = [0x1234u16, 0x5678u16];
/// let bytes_le = x16_to_bytes(&words, true);
/// assert_eq!(bytes_le, vec![0x34, 0x12, 0x78, 0x56]);
///
/// let bytes_be = x16_to_bytes(&words, false);
/// assert_eq!(bytes_be, vec![0x12, 0x34, 0x56, 0x78]);
/// ```
pub fn x16_to_bytes(data: &[u16], little_endian: bool) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() * 2);
    for &word in data {
        if little_endian {
            result.push((word & 0xFF) as u8);
            result.push(((word >> 8) & 0xFF) as u8);
        } else {
            result.push(((word >> 8) & 0xFF) as u8);
            result.push((word & 0xFF) as u8);
        }
    }
    result
}

/// Convert bytes to 16-bit words with proper endianness
///
/// # Arguments
/// * `data` - Slice of bytes to convert (must have even length)
/// * `little_endian` - If true, use little-endian byte order (LSB first)
///
/// # Returns
/// * Vector of 16-bit words
///
/// # Note
/// If the input has an odd number of bytes, the last byte is ignored.
///
/// # Example
/// ```
/// use openflash_core::onfi::bytes_to_x16;
///
/// let bytes = [0x34u8, 0x12, 0x78, 0x56];
/// let words_le = bytes_to_x16(&bytes, true);
/// assert_eq!(words_le, vec![0x1234u16, 0x5678u16]);
///
/// let words_be = bytes_to_x16(&bytes, false);
/// assert_eq!(words_be, vec![0x3412u16, 0x7856u16]);
/// ```
pub fn bytes_to_x16(data: &[u8], little_endian: bool) -> Vec<u16> {
    let mut result = Vec::with_capacity(data.len() / 2);
    for chunk in data.chunks_exact(2) {
        let word = if little_endian {
            (chunk[0] as u16) | ((chunk[1] as u16) << 8)
        } else {
            ((chunk[0] as u16) << 8) | (chunk[1] as u16)
        };
        result.push(word);
    }
    result
}

// ============================================================================
// ONFI 5.0 Structures
// ============================================================================

/// ONFI version enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnfiVersion {
    Onfi10,
    Onfi20,
    Onfi30,
    Onfi40,
    Onfi50,
    Unknown,
}

/// Extended ONFI timing for NV-DDR3 (ONFI 5.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnfiNvDdr3Timing {
    /// Data rate in MT/s (up to 1600)
    pub data_rate_mt_s: u16,
    /// Command/Address delay (tCAD) in picoseconds
    pub t_cad: u8,
    /// DQS output access time (tDQSCK) in picoseconds
    pub t_dqsck: u8,
    /// DQS-DQ skew (tDQSQ) in picoseconds
    pub t_dqsq: u8,
}

impl Default for OnfiNvDdr3Timing {
    fn default() -> Self {
        Self {
            data_rate_mt_s: 800,
            t_cad: 25,
            t_dqsck: 20,
            t_dqsq: 5,
        }
    }
}

/// Extended ECC information from ONFI 5.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtendedEccInfo {
    /// Number of ECC bits required per codeword
    pub ecc_bits: u8,
    /// Codeword size in bytes
    pub codeword_size: u16,
    /// Maximum correctable bits per codeword
    pub max_correctable_bits: u8,
}

/// ONFI 5.0 specific features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Onfi5Features {
    /// NV-DDR3 interface support
    pub nv_ddr3_support: bool,
    /// ZQ calibration support
    pub zq_calibration: bool,
    /// DCC (Duty Cycle Correction) training support
    pub dcc_training: bool,
    /// Multi-plane operations support
    pub multi_plane_ops: bool,
    /// Maximum number of planes
    pub max_planes: u8,
    /// Extended ECC information (if available)
    pub extended_ecc_info: Option<ExtendedEccInfo>,
    /// NV-DDR3 timing parameters (if supported)
    pub nv_ddr3_timing: Option<OnfiNvDdr3Timing>,
}

/// NAND chip information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NandChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub size_mb: u32,
    pub page_size: u32,
    pub block_size: u32, // pages per block
    pub oob_size: u32,   // spare/OOB bytes per page
    pub voltage: String,
    pub timing: NandTiming,
    pub bus_width: u8, // 8 or 16 bit
    pub cell_type: CellType,
}

impl NandChipInfo {
    /// Check if chip supports 16-bit mode
    pub fn supports_x16(&self) -> bool {
        self.bus_width == 16
    }

    /// Get the bus width as NandBusWidth enum
    pub fn get_bus_width(&self) -> NandBusWidth {
        if self.bus_width == 16 {
            NandBusWidth::X16
        } else {
            NandBusWidth::X8
        }
    }
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
    pub tRP: u8,  // RE# pulse width
    pub tWP: u8,  // WE# pulse width
    pub tCLS: u8, // CLE setup time
    pub tALS: u8, // ALE setup time
    pub tRR: u8,  // Ready to RE#
    pub tAR: u8,  // ALE to RE#
    pub tCLR: u8, // Command latch to RE#
    pub tRHW: u8, // RE# high to WE# low
    pub tWHR: u8, // WE# high to RE#
    pub tR: u8,   // Page read time (in microseconds)
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

/// An exact catalogue entry, or `None`. See [`get_spi_nor_chip_info_exact`]
/// in `spi_nor` for why this is separate from the generic fallback.
///
/// [`get_spi_nor_chip_info_exact`]: crate::spi_nor::get_spi_nor_chip_info_exact
pub fn get_chip_info_exact(chip_id: &[u8]) -> Option<NandChipInfo> {
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
        // MX30UF2G28AD - 256MB SLC 1.8V
        [0xC2, 0xDA, 0x90, 0x95, 0x46] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30UF2G28AD".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
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

        // ============ 16-bit (x16) Chip Variants ============

        // Samsung x16 variants
        // K9F1G16U0B - 128MB SLC x16
        [0xEC, 0xA1, 0x00, 0x95, 0x40] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F1G16U0B".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // K9F2G16U0C - 256MB SLC x16
        [0xEC, 0xCA, 0x10, 0x95, 0x44] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F2G16U0C".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // K9F4G16U0D - 512MB SLC x16
        [0xEC, 0xCC, 0x10, 0x95, 0x54] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F4G16U0D".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // K9GAG16U0E - 2GB MLC x16
        [0xEC, 0xC5, 0x84, 0x72, 0x50] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9GAG16U0E".into(),
            size_mb: 2048,
            page_size: 8192,
            block_size: 128,
            oob_size: 436,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::MLC,
        }),

        // Hynix x16 variants
        // HY27UF161G2A - 128MB SLC x16
        [0xAD, 0xA1, 0x80, 0x1D, ..] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "HY27UF161G2A".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // HY27UF162G2A - 256MB SLC x16
        [0xAD, 0xCA, 0x10, 0x95, 0x44] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "HY27UF162G2A".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // H27U4G16F2DTR - 512MB SLC x16
        [0xAD, 0xCC, 0x90, 0x95, 0x54] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27U4G16F2DTR".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),

        // Micron x16 variants
        // MT29F1G16ABADAWP - 128MB SLC x16
        [0x2C, 0xA1, 0x80, 0x95, 0x04] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F1G16ABADAWP".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // MT29F2G16ABAEAWP - 256MB SLC x16
        [0x2C, 0xCA, 0x90, 0x95, 0x06] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F2G16ABAEAWP".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // MT29F4G16ABADAWP - 512MB SLC x16
        [0x2C, 0xCC, 0x90, 0x95, 0x56] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F4G16ABADAWP".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // MT29F8G16ADBDAWP - 1GB SLC x16
        [0x2C, 0xB3, 0xD1, 0x95, 0xA6] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F8G16ADBDAWP".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 224,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),

        // Toshiba x16 variants
        // TC58NVG0S3HTA00 - 128MB SLC x16
        [0x98, 0xA1, 0x80, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG0S3HTA00".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // TC58NVG1S3HTA00 - 256MB SLC x16
        [0x98, 0xCA, 0x90, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG1S3HTA00".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // TC58NVG2S3HTA00 - 512MB SLC x16
        [0x98, 0xCC, 0x90, 0x15, ..] => Some(NandChipInfo {
            manufacturer: "Toshiba".into(),
            model: "TC58NVG2S3HTA00".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),

        // Macronix x16 variants
        // MX30LF1G18AC - 128MB SLC x16
        [0xC2, 0xA1, 0x80, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30LF1G18AC".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),
        // MX30LF2G18AC-TI - 256MB SLC x16
        [0xC2, 0xCA, 0x90, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30LF2G18AC-TI".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 16,
            cell_type: CellType::SLC,
        }),

        // ============ New Chips v2.2 ============

        // Samsung K9F Series (new models)
        // K9F1G08U0E - 128MB SLC (new revision)
        [0xEC, 0xF1, 0x00, 0x95, 0x42] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F1G08U0E".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9F2G08U0E - 256MB SLC (new revision)
        [0xEC, 0xDA, 0x10, 0x95, 0x46] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9F2G08U0E".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // K9GBG08U0A - 4GB MLC
        [0xEC, 0xD7, 0x94, 0x7A, 0x54] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9GBG08U0A".into(),
            size_mb: 4096,
            page_size: 8192,
            block_size: 128,
            oob_size: 640,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // K9GBG08U0B - 4GB MLC (new revision)
        [0xEC, 0xD7, 0x94, 0x7E, 0x64] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9GBG08U0B".into(),
            size_mb: 4096,
            page_size: 8192,
            block_size: 128,
            oob_size: 1024,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // K9LCG08U0A - 8GB MLC
        [0xEC, 0xDE, 0xD5, 0x7A, 0x58] => Some(NandChipInfo {
            manufacturer: "Samsung".into(),
            model: "K9LCG08U0A".into(),
            size_mb: 8192,
            page_size: 8192,
            block_size: 128,
            oob_size: 640,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),

        // Micron MT29F Series (new models)
        // MT29F32G08CBADAWP - 4GB MLC
        [0x2C, 0x44, 0x44, 0x4B, 0xA9] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F32G08CBADAWP".into(),
            size_mb: 4096,
            page_size: 8192,
            block_size: 256,
            oob_size: 744,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // MT29F64G08CBABAWP - 8GB MLC
        [0x2C, 0x64, 0x44, 0x4B, 0xA9] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F64G08CBABAWP".into(),
            size_mb: 8192,
            page_size: 8192,
            block_size: 256,
            oob_size: 744,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // MT29F128G08CFABAWP - 16GB MLC
        [0x2C, 0x88, 0x04, 0x4B, 0xA9] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F128G08CFABAWP".into(),
            size_mb: 16384,
            page_size: 8192,
            block_size: 256,
            oob_size: 744,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // MT29F256G08CJAABWP - 32GB TLC
        [0x2C, 0xA8, 0x05, 0xCB, 0xA9] => Some(NandChipInfo {
            manufacturer: "Micron".into(),
            model: "MT29F256G08CJAABWP".into(),
            size_mb: 32768,
            page_size: 16384,
            block_size: 512,
            oob_size: 1872,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::TLC,
        }),

        // SK Hynix H27 Series (new models)
        // H27U8G8T2BTR - 1GB SLC
        [0xAD, 0xD3, 0x90, 0x2D, 0x64] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27U8G8T2BTR".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 224,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // H27UBG8T2ATR - 4GB MLC
        [0xAD, 0xD7, 0x94, 0x91, 0x60] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27UBG8T2ATR".into(),
            size_mb: 4096,
            page_size: 8192,
            block_size: 256,
            oob_size: 640,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // H27UCG8T2ETR - 8GB MLC
        [0xAD, 0xDE, 0x94, 0xEB, 0x74] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27UCG8T2ETR".into(),
            size_mb: 8192,
            page_size: 16384,
            block_size: 256,
            oob_size: 1664,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // H27QDG8VEBIR - 16GB TLC
        [0xAD, 0x3A, 0x14, 0xAB, 0x42] => Some(NandChipInfo {
            manufacturer: "SK Hynix".into(),
            model: "H27QDG8VEBIR".into(),
            size_mb: 16384,
            page_size: 16384,
            block_size: 512,
            oob_size: 1872,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::TLC,
        }),

        // Kioxia/Toshiba TC58 Series (new models)
        // TC58NVG3S0FTA00 - 1GB SLC
        [0x98, 0xD3, 0x90, 0x26, 0x76] => Some(NandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58NVG3S0FTA00".into(),
            size_mb: 1024,
            page_size: 4096,
            block_size: 64,
            oob_size: 232,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // TC58NVG4D2FTA00 - 2GB MLC
        [0x98, 0xD5, 0x94, 0x32, 0x76] => Some(NandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58NVG4D2FTA00".into(),
            size_mb: 2048,
            page_size: 8192,
            block_size: 128,
            oob_size: 448,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // TC58NVG5D2HTA00 - 4GB MLC
        [0x98, 0xD7, 0x94, 0x32, 0x76] => Some(NandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58NVG5D2HTA00".into(),
            size_mb: 4096,
            page_size: 8192,
            block_size: 128,
            oob_size: 640,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // TC58TEG6DDKTA00 - 8GB MLC
        [0x98, 0xDE, 0x94, 0x93, 0x76] => Some(NandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58TEG6DDKTA00".into(),
            size_mb: 8192,
            page_size: 16384,
            block_size: 256,
            oob_size: 1280,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::MLC,
        }),
        // TC58TFG7DDLTA0D - 16GB TLC (BiCS)
        [0x98, 0x3A, 0x94, 0x93, 0x76] => Some(NandChipInfo {
            manufacturer: "Kioxia".into(),
            model: "TC58TFG7DDLTA0D".into(),
            size_mb: 16384,
            page_size: 16384,
            block_size: 384,
            oob_size: 1872,
            voltage: "3.3V".into(),
            timing: fast_timing(),
            bus_width: 8,
            cell_type: CellType::TLC,
        }),

        // Macronix MX30LF Series (new models)
        // MX30LF4G28AD - 512MB SLC
        [0xC2, 0xDC, 0x90, 0xA6, 0x54] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30LF4G28AD".into(),
            size_mb: 512,
            page_size: 4096,
            block_size: 64,
            oob_size: 256,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // MX30UF1G28AD - 128MB SLC 1.8V
        [0xC2, 0xF1, 0x80, 0x1D, 0x42] => Some(NandChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX30UF1G28AD".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "1.8V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        // Winbond W29N Series (new models)
        // W29N02GVSIAA - 256MB SLC
        [0xEF, 0xDA, 0x10, 0x95, 0x44] => Some(NandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W29N02GVSIAA".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // W29N04GVSIAA - 512MB SLC
        [0xEF, 0xDC, 0x10, 0x95, 0x54] => Some(NandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W29N04GVSIAA".into(),
            size_mb: 512,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // W29N08GVSIAA - 1GB SLC
        [0xEF, 0xD3, 0x10, 0x95, 0x58] => Some(NandChipInfo {
            manufacturer: "Winbond".into(),
            model: "W29N08GVSIAA".into(),
            size_mb: 1024,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),

        // ESMT F59L Series
        // F59L1G81LA - 128MB SLC
        [0x92, 0xF1, 0x80, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "ESMT".into(),
            model: "F59L1G81LA".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // F59L2G81A - 256MB SLC
        [0x92, 0xDA, 0x90, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "ESMT".into(),
            model: "F59L2G81A".into(),
            size_mb: 256,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        }),
        // F59L4G81A - 512MB SLC
        [0x92, 0xDC, 0x90, 0x95, ..] => Some(NandChipInfo {
            manufacturer: "ESMT".into(),
            model: "F59L4G81A".into(),
            size_mb: 512,
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
    // x8 devices use 0xFx, 0xDx series
    // x16 devices use 0xAx, 0xBx, 0xCx series (same capacity, different bus width)
    let (size_mb, page_size, block_size, cell_type, bus_width) = match device {
        // ============ x8 devices ============
        // 128MB class x8
        0xF1 => (128, 2048, 64, CellType::SLC, 8),
        // 256MB class x8
        0xDA => (256, 2048, 64, CellType::SLC, 8),
        // 512MB class x8
        0xDC => (512, 2048, 64, CellType::SLC, 8),
        // 1GB class x8
        0xD3 => (1024, 4096, 64, CellType::SLC, 8),
        // 2GB class x8 (often MLC)
        0xD5 => (2048, 4096, 128, CellType::MLC, 8),
        // 4GB class x8
        0xD7 => (4096, 4096, 128, CellType::MLC, 8),
        // 8GB class x8
        0xDE => (8192, 8192, 256, CellType::MLC, 8),
        // 16GB+ class x8
        0x48 => (2048, 4096, 256, CellType::MLC, 8),
        0x68 => (4096, 8192, 256, CellType::MLC, 8),
        0x88 => (8192, 8192, 256, CellType::TLC, 8),

        // ============ x16 devices ============
        // 128MB class x16
        0xA1 => (128, 2048, 64, CellType::SLC, 16),
        // 256MB class x16
        0xCA => (256, 2048, 64, CellType::SLC, 16),
        // 512MB class x16
        0xCC => (512, 2048, 64, CellType::SLC, 16),
        // 1GB class x16
        0xB3 => (1024, 4096, 64, CellType::SLC, 16),
        // 2GB class x16 (often MLC)
        0xC5 => (2048, 4096, 128, CellType::MLC, 16),
        // 4GB class x16
        0xB7 => (4096, 4096, 128, CellType::MLC, 16),
        // 8GB class x16
        0xBE => (8192, 8192, 256, CellType::MLC, 16),

        _ => return None,
    };

    Some(NandChipInfo {
        manufacturer,
        model: format!("Generic 0x{:02X} x{}", device, bus_width),
        size_mb,
        page_size,
        block_size,
        oob_size: page_size / 32, // Typical OOB ratio
        voltage: "3.3V".into(),
        timing: NandTiming::default(),
        bus_width,
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
    let manufacturer = String::from_utf8_lossy(&data[32..44]).trim().to_string();

    // Parse model (bytes 44-63)
    let model = String::from_utf8_lossy(&data[44..64]).trim().to_string();

    // Parse geometry
    let page_size = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
    let oob_size = u16::from_le_bytes([data[84], data[85]]) as u32;
    let pages_per_block = u32::from_le_bytes([data[92], data[93], data[94], data[95]]);
    let blocks_per_lun = u32::from_le_bytes([data[96], data[97], data[98], data[99]]);
    let luns = data[100];

    let total_blocks = blocks_per_lun * luns as u32;
    let size_mb =
        (total_blocks as u64 * pages_per_block as u64 * page_size as u64 / 1024 / 1024) as u32;

    // Parse timing (simplified)
    let _t_prog = u16::from_le_bytes([data[133], data[134]]);
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

// ============================================================================
// ONFI Version Detection
// ============================================================================

/// ONFI revision bit masks (from parameter page bytes 4-5)
mod revision_bits {
    pub const ONFI_1_0: u16 = 1 << 1;
    pub const ONFI_2_0: u16 = 1 << 2;
    pub const ONFI_2_1: u16 = 1 << 3;
    pub const ONFI_2_2: u16 = 1 << 4;
    pub const ONFI_2_3: u16 = 1 << 5;
    pub const ONFI_3_0: u16 = 1 << 6;
    pub const ONFI_3_1: u16 = 1 << 7;
    pub const ONFI_3_2: u16 = 1 << 8;
    pub const ONFI_4_0: u16 = 1 << 9;
    pub const ONFI_4_1: u16 = 1 << 10;
    pub const ONFI_4_2: u16 = 1 << 11;
    pub const ONFI_5_0: u16 = 1 << 12;
}

/// Detect ONFI version from parameter page
///
/// The revision field is at bytes 4-5 of the parameter page.
/// Each bit indicates support for a specific ONFI version.
/// Returns the highest supported version.
pub fn detect_onfi_version(param_page: &[u8]) -> OnfiVersion {
    if param_page.len() < 6 {
        return OnfiVersion::Unknown;
    }

    // Check ONFI signature
    if &param_page[0..4] != b"ONFI" {
        return OnfiVersion::Unknown;
    }

    // Parse revision field (bytes 4-5, little-endian)
    let revision = u16::from_le_bytes([param_page[4], param_page[5]]);

    // Return highest supported version
    if revision & revision_bits::ONFI_5_0 != 0 {
        OnfiVersion::Onfi50
    } else if revision
        & (revision_bits::ONFI_4_0 | revision_bits::ONFI_4_1 | revision_bits::ONFI_4_2)
        != 0
    {
        OnfiVersion::Onfi40
    } else if revision
        & (revision_bits::ONFI_3_0 | revision_bits::ONFI_3_1 | revision_bits::ONFI_3_2)
        != 0
    {
        OnfiVersion::Onfi30
    } else if revision
        & (revision_bits::ONFI_2_0
            | revision_bits::ONFI_2_1
            | revision_bits::ONFI_2_2
            | revision_bits::ONFI_2_3)
        != 0
    {
        OnfiVersion::Onfi20
    } else if revision & revision_bits::ONFI_1_0 != 0 {
        OnfiVersion::Onfi10
    } else {
        OnfiVersion::Unknown
    }
}

// ============================================================================
// ONFI 5.0 Extended Parameter Page Parser
// ============================================================================

/// Extended parameter page section types
mod extended_section_types {
    pub const SECTION_TYPE_2: u8 = 2; // Extended ECC information
}

/// Parse ONFI 5.0 extended parameter page
///
/// The extended parameter page contains additional information not present
/// in the standard parameter page, including:
/// - Extended ECC requirements
/// - NV-DDR3 timing parameters
/// - Advanced feature support flags
///
/// # Arguments
/// * `data` - Extended parameter page data (typically 32+ bytes)
///
/// # Returns
/// * `Some(Onfi5Features)` if parsing succeeds
/// * `None` if data is invalid or too short
pub fn parse_onfi5_extended_params(data: &[u8]) -> Option<Onfi5Features> {
    // Minimum size for extended parameter page header
    if data.len() < 32 {
        return None;
    }

    // Check extended parameter page signature (bytes 0-1)
    // ONFI 5.0 uses 0x0FF0 as signature
    let signature = u16::from_le_bytes([data[0], data[1]]);
    if signature != 0x0FF0 {
        return None;
    }

    // Parse features support byte (byte 2)
    let features_byte = data[2];
    let nv_ddr3_support = (features_byte & 0x01) != 0;
    let zq_calibration = (features_byte & 0x02) != 0;
    let dcc_training = (features_byte & 0x04) != 0;
    let multi_plane_ops = (features_byte & 0x08) != 0;

    // Parse max planes (byte 3, bits 0-3)
    let max_planes = (data[3] & 0x0F) + 1; // 0 = 1 plane, 1 = 2 planes, etc.

    // Parse extended ECC info if present (section type 2)
    let extended_ecc_info = parse_extended_ecc_section(data);

    // Parse NV-DDR3 timing if supported
    let nv_ddr3_timing = if nv_ddr3_support {
        parse_nv_ddr3_timing(data)
    } else {
        None
    };

    Some(Onfi5Features {
        nv_ddr3_support,
        zq_calibration,
        dcc_training,
        multi_plane_ops,
        max_planes,
        extended_ecc_info,
        nv_ddr3_timing,
    })
}

/// Parse extended ECC section from extended parameter page
fn parse_extended_ecc_section(data: &[u8]) -> Option<ExtendedEccInfo> {
    // Extended ECC info starts at byte 16 in the extended parameter page
    if data.len() < 22 {
        return None;
    }

    // Check section type (byte 16)
    let section_type = data[16];
    if section_type != extended_section_types::SECTION_TYPE_2 {
        // Section type 2 is extended ECC information
        // If not present, try to parse from standard location
        if data.len() >= 20 {
            // Fallback: parse from bytes 4-7 if section header not found
            let ecc_bits = data[4];
            let codeword_size = u16::from_le_bytes([data[5], data[6]]);
            let max_correctable_bits = data[7];

            if ecc_bits > 0 && codeword_size > 0 {
                return Some(ExtendedEccInfo {
                    ecc_bits,
                    codeword_size,
                    max_correctable_bits,
                });
            }
        }
        return None;
    }

    // Parse ECC info from section
    // Section length at byte 17
    let section_length = data[17];
    if section_length < 4 || data.len() < 22 {
        return None;
    }

    // ECC bits required (byte 18)
    let ecc_bits = data[18];
    // Codeword size (bytes 19-20)
    let codeword_size = u16::from_le_bytes([data[19], data[20]]);
    // Max correctable bits (byte 21)
    let max_correctable_bits = data[21];

    Some(ExtendedEccInfo {
        ecc_bits,
        codeword_size,
        max_correctable_bits,
    })
}

/// Parse NV-DDR3 timing parameters from extended parameter page
fn parse_nv_ddr3_timing(data: &[u8]) -> Option<OnfiNvDdr3Timing> {
    // NV-DDR3 timing starts at byte 24 in the extended parameter page
    if data.len() < 30 {
        return None;
    }

    // Data rate in MT/s (bytes 24-25)
    let data_rate_mt_s = u16::from_le_bytes([data[24], data[25]]);

    // Validate data rate (ONFI 5.0 supports up to 1600 MT/s)
    if data_rate_mt_s == 0 || data_rate_mt_s > 1600 {
        return None;
    }

    // Timing parameters (bytes 26-28)
    let t_cad = data[26];
    let t_dqsck = data[27];
    let t_dqsq = data[28];

    Some(OnfiNvDdr3Timing {
        data_rate_mt_s,
        t_cad,
        t_dqsck,
        t_dqsq,
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

    // ========================================================================
    // ONFI 5.0 Tests
    // ========================================================================

    #[test]
    fn test_onfi_version_detection_5_0() {
        // Create a parameter page with ONFI 5.0 revision bit set
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // Set ONFI 5.0 bit (bit 12)
        param_page[4] = 0x00;
        param_page[5] = 0x10; // 0x1000 = bit 12

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Onfi50);
    }

    #[test]
    fn test_onfi_version_detection_4_0() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // Set ONFI 4.0 bit (bit 9)
        param_page[4] = 0x00;
        param_page[5] = 0x02; // 0x0200 = bit 9

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Onfi40);
    }

    #[test]
    fn test_onfi_version_detection_3_0() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // Set ONFI 3.0 bit (bit 6)
        param_page[4] = 0x40; // 0x0040 = bit 6
        param_page[5] = 0x00;

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Onfi30);
    }

    #[test]
    fn test_onfi_version_detection_2_0() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // Set ONFI 2.0 bit (bit 2)
        param_page[4] = 0x04; // 0x0004 = bit 2
        param_page[5] = 0x00;

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Onfi20);
    }

    #[test]
    fn test_onfi_version_detection_1_0() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // Set ONFI 1.0 bit (bit 1)
        param_page[4] = 0x02; // 0x0002 = bit 1
        param_page[5] = 0x00;

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Onfi10);
    }

    #[test]
    fn test_onfi_version_detection_unknown() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"ONFI");
        // No version bits set
        param_page[4] = 0x00;
        param_page[5] = 0x00;

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Unknown);
    }

    #[test]
    fn test_onfi_version_detection_invalid_signature() {
        let mut param_page = vec![0u8; 256];
        param_page[0..4].copy_from_slice(b"XXXX");
        param_page[4] = 0x00;
        param_page[5] = 0x10;

        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Unknown);
    }

    #[test]
    fn test_onfi_version_detection_short_data() {
        let param_page = vec![0u8; 4]; // Too short
        assert_eq!(detect_onfi_version(&param_page), OnfiVersion::Unknown);
    }

    #[test]
    fn test_onfi5_extended_params_parsing() {
        let mut ext_page = vec![0u8; 32];
        // Signature 0x0FF0
        ext_page[0] = 0xF0;
        ext_page[1] = 0x0F;
        // Features: NV-DDR3, ZQ cal, DCC training, multi-plane
        ext_page[2] = 0x0F;
        // Max planes: 4 (encoded as 3)
        ext_page[3] = 0x03;
        // ECC info (fallback location)
        ext_page[4] = 40; // ECC bits
        ext_page[5] = 0x00;
        ext_page[6] = 0x02; // Codeword size = 512
        ext_page[7] = 40; // Max correctable bits
                          // NV-DDR3 timing
        ext_page[24] = 0x40;
        ext_page[25] = 0x06; // 1600 MT/s
        ext_page[26] = 25; // t_cad
        ext_page[27] = 20; // t_dqsck
        ext_page[28] = 5; // t_dqsq

        let features = parse_onfi5_extended_params(&ext_page).unwrap();

        assert!(features.nv_ddr3_support);
        assert!(features.zq_calibration);
        assert!(features.dcc_training);
        assert!(features.multi_plane_ops);
        assert_eq!(features.max_planes, 4);

        let ecc = features.extended_ecc_info.unwrap();
        assert_eq!(ecc.ecc_bits, 40);
        assert_eq!(ecc.codeword_size, 512);
        assert_eq!(ecc.max_correctable_bits, 40);

        let timing = features.nv_ddr3_timing.unwrap();
        assert_eq!(timing.data_rate_mt_s, 1600);
        assert_eq!(timing.t_cad, 25);
        assert_eq!(timing.t_dqsck, 20);
        assert_eq!(timing.t_dqsq, 5);
    }

    #[test]
    fn test_onfi5_extended_params_invalid_signature() {
        let mut ext_page = vec![0u8; 32];
        ext_page[0] = 0x00;
        ext_page[1] = 0x00;

        assert!(parse_onfi5_extended_params(&ext_page).is_none());
    }

    #[test]
    fn test_onfi5_extended_params_short_data() {
        let ext_page = vec![0u8; 16]; // Too short
        assert!(parse_onfi5_extended_params(&ext_page).is_none());
    }

    #[test]
    fn test_onfi5_features_no_nv_ddr3() {
        let mut ext_page = vec![0u8; 32];
        ext_page[0] = 0xF0;
        ext_page[1] = 0x0F;
        // No NV-DDR3 support
        ext_page[2] = 0x0E; // ZQ, DCC, multi-plane but not NV-DDR3
        ext_page[3] = 0x01; // 2 planes

        let features = parse_onfi5_extended_params(&ext_page).unwrap();

        assert!(!features.nv_ddr3_support);
        assert!(features.zq_calibration);
        assert!(features.dcc_training);
        assert!(features.multi_plane_ops);
        assert_eq!(features.max_planes, 2);
        assert!(features.nv_ddr3_timing.is_none());
    }

    #[test]
    fn test_extended_ecc_info_struct() {
        let ecc = ExtendedEccInfo {
            ecc_bits: 72,
            codeword_size: 1024,
            max_correctable_bits: 72,
        };

        assert_eq!(ecc.ecc_bits, 72);
        assert_eq!(ecc.codeword_size, 1024);
        assert_eq!(ecc.max_correctable_bits, 72);
    }

    #[test]
    fn test_nv_ddr3_timing_default() {
        let timing = OnfiNvDdr3Timing::default();

        assert_eq!(timing.data_rate_mt_s, 800);
        assert_eq!(timing.t_cad, 25);
        assert_eq!(timing.t_dqsck, 20);
        assert_eq!(timing.t_dqsq, 5);
    }

    // ========================================================================
    // 16-bit Bus Width Tests
    // ========================================================================

    #[test]
    fn test_nand_bus_width_default() {
        let bus_width = NandBusWidth::default();
        assert_eq!(bus_width, NandBusWidth::X8);
    }

    #[test]
    fn test_nand_bus_width_bits() {
        assert_eq!(NandBusWidth::X8.bits(), 8);
        assert_eq!(NandBusWidth::X16.bits(), 16);
    }

    #[test]
    fn test_nand_chip_info_supports_x16() {
        let mut chip = NandChipInfo {
            manufacturer: "Test".into(),
            model: "Test".into(),
            size_mb: 128,
            page_size: 2048,
            block_size: 64,
            oob_size: 64,
            voltage: "3.3V".into(),
            timing: NandTiming::default(),
            bus_width: 8,
            cell_type: CellType::SLC,
        };

        assert!(!chip.supports_x16());
        assert_eq!(chip.get_bus_width(), NandBusWidth::X8);

        chip.bus_width = 16;
        assert!(chip.supports_x16());
        assert_eq!(chip.get_bus_width(), NandBusWidth::X16);
    }

    #[test]
    fn test_detect_bus_width_x8() {
        // Normal x8 chip - same ID in both modes
        let id_x8 = [0xEC, 0xF1, 0x00, 0x95, 0x40];
        let id_x16 = [0xEC, 0xF1, 0x00, 0x95, 0x40];

        assert_eq!(detect_bus_width(&id_x8, &id_x16), NandBusWidth::X8);
    }

    #[test]
    fn test_detect_bus_width_x16() {
        // x16 chip read via x8 interface has alternating zeros
        let id_x8 = [0xEC, 0x00, 0xF1, 0x00, 0x95, 0x00];
        let id_x16 = [0xEC, 0xF1, 0x95];

        assert_eq!(detect_bus_width(&id_x8, &id_x16), NandBusWidth::X16);
    }

    #[test]
    fn test_detect_bus_width_empty() {
        assert_eq!(detect_bus_width(&[], &[]), NandBusWidth::X8);
        assert_eq!(detect_bus_width(&[0xEC], &[]), NandBusWidth::X8);
        assert_eq!(detect_bus_width(&[], &[0xEC]), NandBusWidth::X8);
    }

    #[test]
    fn test_x16_to_bytes_little_endian() {
        let words = [0x1234u16, 0x5678u16];
        let bytes = x16_to_bytes(&words, true);
        assert_eq!(bytes, vec![0x34, 0x12, 0x78, 0x56]);
    }

    #[test]
    fn test_x16_to_bytes_big_endian() {
        let words = [0x1234u16, 0x5678u16];
        let bytes = x16_to_bytes(&words, false);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_x16_to_bytes_empty() {
        let words: [u16; 0] = [];
        let bytes = x16_to_bytes(&words, true);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_bytes_to_x16_little_endian() {
        let bytes = [0x34u8, 0x12, 0x78, 0x56];
        let words = bytes_to_x16(&bytes, true);
        assert_eq!(words, vec![0x1234u16, 0x5678u16]);
    }

    #[test]
    fn test_bytes_to_x16_big_endian() {
        let bytes = [0x12u8, 0x34, 0x56, 0x78];
        let words = bytes_to_x16(&bytes, false);
        assert_eq!(words, vec![0x1234u16, 0x5678u16]);
    }

    #[test]
    fn test_bytes_to_x16_empty() {
        let bytes: [u8; 0] = [];
        let words = bytes_to_x16(&bytes, true);
        assert!(words.is_empty());
    }

    #[test]
    fn test_bytes_to_x16_odd_length() {
        // Odd number of bytes - last byte should be ignored
        let bytes = [0x34u8, 0x12, 0x78];
        let words = bytes_to_x16(&bytes, true);
        assert_eq!(words, vec![0x1234u16]);
    }

    #[test]
    fn test_x16_roundtrip() {
        let original = [0xABCDu16, 0x1234u16, 0xFFFFu16, 0x0000u16];

        // Little-endian round-trip
        let bytes_le = x16_to_bytes(&original, true);
        let recovered_le = bytes_to_x16(&bytes_le, true);
        assert_eq!(original.to_vec(), recovered_le);

        // Big-endian round-trip
        let bytes_be = x16_to_bytes(&original, false);
        let recovered_be = bytes_to_x16(&bytes_be, false);
        assert_eq!(original.to_vec(), recovered_be);
    }

    // ========================================================================
    // 16-bit Chip Database Tests
    // ========================================================================

    #[test]
    fn test_samsung_x16_chip_recognition() {
        // K9F1G16U0B - 128MB SLC x16
        let chip_id = [0xEC, 0xA1, 0x00, 0x95, 0x40];
        let chip_info = get_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Samsung");
        assert_eq!(chip_info.model, "K9F1G16U0B");
        assert_eq!(chip_info.size_mb, 128);
        assert_eq!(chip_info.bus_width, 16);
        assert!(chip_info.supports_x16());
    }

    #[test]
    fn test_micron_x16_chip_recognition() {
        // MT29F2G16ABAEAWP - 256MB SLC x16
        let chip_id = [0x2C, 0xCA, 0x90, 0x95, 0x06];
        let chip_info = get_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Micron");
        assert_eq!(chip_info.model, "MT29F2G16ABAEAWP");
        assert_eq!(chip_info.size_mb, 256);
        assert_eq!(chip_info.bus_width, 16);
        assert!(chip_info.supports_x16());
    }

    #[test]
    fn test_generic_x16_detection() {
        // Generic x16 128MB chip
        let chip_id = [0x2C, 0xA1]; // Micron 128MB x16
        let chip_info = get_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Micron");
        assert_eq!(chip_info.size_mb, 128);
        assert_eq!(chip_info.bus_width, 16);
        assert!(chip_info.supports_x16());
        assert!(chip_info.model.contains("x16"));
    }

    #[test]
    fn test_generic_x16_256mb_detection() {
        // Generic x16 256MB chip
        let chip_id = [0xEC, 0xCA]; // Samsung 256MB x16
        let chip_info = get_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Samsung");
        assert_eq!(chip_info.size_mb, 256);
        assert_eq!(chip_info.bus_width, 16);
        assert!(chip_info.supports_x16());
    }

    #[test]
    fn test_generic_x16_512mb_detection() {
        // Generic x16 512MB chip
        let chip_id = [0xAD, 0xCC]; // Hynix 512MB x16
        let chip_info = get_chip_info(&chip_id).unwrap();

        assert_eq!(chip_info.manufacturer, "SK Hynix");
        assert_eq!(chip_info.size_mb, 512);
        assert_eq!(chip_info.bus_width, 16);
        assert!(chip_info.supports_x16());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: nor-flash-ufs-support, Property 8: ONFI Version Detection
        /// For any ONFI parameter page with a valid revision field, detecting the ONFI version
        /// should return the correct OnfiVersion enum value.
        /// **Validates: Requirements 7.1**
        #[test]
        fn prop_onfi_version_detection(
            version_bits in prop_oneof![
                Just(0x0002u16),  // ONFI 1.0 (bit 1)
                Just(0x0004u16),  // ONFI 2.0 (bit 2)
                Just(0x0008u16),  // ONFI 2.1 (bit 3)
                Just(0x0010u16),  // ONFI 2.2 (bit 4)
                Just(0x0020u16),  // ONFI 2.3 (bit 5)
                Just(0x0040u16),  // ONFI 3.0 (bit 6)
                Just(0x0080u16),  // ONFI 3.1 (bit 7)
                Just(0x0100u16),  // ONFI 3.2 (bit 8)
                Just(0x0200u16),  // ONFI 4.0 (bit 9)
                Just(0x0400u16),  // ONFI 4.1 (bit 10)
                Just(0x0800u16),  // ONFI 4.2 (bit 11)
                Just(0x1000u16),  // ONFI 5.0 (bit 12)
            ],
            // Additional random bits that shouldn't affect version detection
            extra_bits in 0u16..0x0002,  // Only bit 0 can be set without affecting version
        ) {
            // Create a valid ONFI parameter page
            let mut param_page = vec![0u8; 256];
            param_page[0..4].copy_from_slice(b"ONFI");

            // Set the revision field with version bits and extra bits
            let revision = version_bits | extra_bits;
            param_page[4] = (revision & 0xFF) as u8;
            param_page[5] = ((revision >> 8) & 0xFF) as u8;

            let detected = detect_onfi_version(&param_page);

            // Determine expected version based on highest bit set
            let expected = if version_bits & 0x1000 != 0 {
                OnfiVersion::Onfi50
            } else if version_bits & 0x0E00 != 0 {  // bits 9-11
                OnfiVersion::Onfi40
            } else if version_bits & 0x01C0 != 0 {  // bits 6-8
                OnfiVersion::Onfi30
            } else if version_bits & 0x003C != 0 {  // bits 2-5
                OnfiVersion::Onfi20
            } else if version_bits & 0x0002 != 0 {  // bit 1
                OnfiVersion::Onfi10
            } else {
                OnfiVersion::Unknown
            };

            prop_assert_eq!(detected, expected,
                "Version detection mismatch for revision bits 0x{:04X}: expected {:?}, got {:?}",
                revision, expected, detected);
        }

        /// Property test: Multiple version bits set should return highest version
        #[test]
        fn prop_onfi_version_highest_wins(
            include_1_0 in proptest::bool::ANY,
            include_2_0 in proptest::bool::ANY,
            include_3_0 in proptest::bool::ANY,
            include_4_0 in proptest::bool::ANY,
            include_5_0 in proptest::bool::ANY,
        ) {
            let mut revision: u16 = 0;

            if include_1_0 { revision |= 0x0002; }
            if include_2_0 { revision |= 0x0004; }
            if include_3_0 { revision |= 0x0040; }
            if include_4_0 { revision |= 0x0200; }
            if include_5_0 { revision |= 0x1000; }

            // Skip if no version bits set
            prop_assume!(revision != 0);

            let mut param_page = vec![0u8; 256];
            param_page[0..4].copy_from_slice(b"ONFI");
            param_page[4] = (revision & 0xFF) as u8;
            param_page[5] = ((revision >> 8) & 0xFF) as u8;

            let detected = detect_onfi_version(&param_page);

            // Should return highest version
            let expected = if include_5_0 {
                OnfiVersion::Onfi50
            } else if include_4_0 {
                OnfiVersion::Onfi40
            } else if include_3_0 {
                OnfiVersion::Onfi30
            } else if include_2_0 {
                OnfiVersion::Onfi20
            } else {
                OnfiVersion::Onfi10
            };

            prop_assert_eq!(detected, expected,
                "Highest version should win: revision 0x{:04X}, expected {:?}, got {:?}",
                revision, expected, detected);
        }

        /// Property test: Invalid signature should always return Unknown
        #[test]
        fn prop_onfi_version_invalid_signature(
            sig0 in proptest::num::u8::ANY,
            sig1 in proptest::num::u8::ANY,
            sig2 in proptest::num::u8::ANY,
            sig3 in proptest::num::u8::ANY,
            revision in proptest::num::u16::ANY,
        ) {
            // Skip if signature happens to be "ONFI"
            prop_assume!(!(sig0 == b'O' && sig1 == b'N' && sig2 == b'F' && sig3 == b'I'));

            let mut param_page = vec![0u8; 256];
            param_page[0] = sig0;
            param_page[1] = sig1;
            param_page[2] = sig2;
            param_page[3] = sig3;
            param_page[4] = (revision & 0xFF) as u8;
            param_page[5] = ((revision >> 8) & 0xFF) as u8;

            let detected = detect_onfi_version(&param_page);

            prop_assert_eq!(detected, OnfiVersion::Unknown,
                "Invalid signature should return Unknown, got {:?}", detected);
        }

        /// Feature: nor-flash-ufs-support, Property 9: ONFI 5.0 Extended ECC Parsing
        /// For any valid ONFI 5.0 extended parameter page, parsing should extract ECC bits,
        /// codeword size, and max correctable bits consistently.
        /// **Validates: Requirements 7.3**
        #[test]
        fn prop_onfi5_ecc_parsing(
            ecc_bits in 1u8..=128,
            codeword_size in prop_oneof![
                Just(512u16),
                Just(1024u16),
                Just(2048u16),
            ],
            max_correctable_bits in 1u8..=128,
            nv_ddr3_support in proptest::bool::ANY,
            zq_calibration in proptest::bool::ANY,
            dcc_training in proptest::bool::ANY,
            multi_plane_ops in proptest::bool::ANY,
            max_planes in 1u8..=8,
            data_rate in prop_oneof![
                Just(400u16),
                Just(533u16),
                Just(667u16),
                Just(800u16),
                Just(1066u16),
                Just(1200u16),
                Just(1333u16),
                Just(1600u16),
            ],
        ) {
            // Build extended parameter page with the given values
            let mut ext_page = vec![0u8; 32];

            // Signature 0x0FF0
            ext_page[0] = 0xF0;
            ext_page[1] = 0x0F;

            // Features byte
            let mut features: u8 = 0;
            if nv_ddr3_support { features |= 0x01; }
            if zq_calibration { features |= 0x02; }
            if dcc_training { features |= 0x04; }
            if multi_plane_ops { features |= 0x08; }
            ext_page[2] = features;

            // Max planes (encoded as max_planes - 1)
            ext_page[3] = max_planes - 1;

            // ECC info (fallback location at bytes 4-7)
            ext_page[4] = ecc_bits;
            ext_page[5] = (codeword_size & 0xFF) as u8;
            ext_page[6] = ((codeword_size >> 8) & 0xFF) as u8;
            ext_page[7] = max_correctable_bits;

            // NV-DDR3 timing (bytes 24-28)
            if nv_ddr3_support {
                ext_page[24] = (data_rate & 0xFF) as u8;
                ext_page[25] = ((data_rate >> 8) & 0xFF) as u8;
                ext_page[26] = 25;  // t_cad
                ext_page[27] = 20;  // t_dqsck
                ext_page[28] = 5;   // t_dqsq
            }

            let features_result = parse_onfi5_extended_params(&ext_page);
            prop_assert!(features_result.is_some(), "Should parse valid extended params");

            let parsed = features_result.unwrap();

            // Verify feature flags
            prop_assert_eq!(parsed.nv_ddr3_support, nv_ddr3_support,
                "NV-DDR3 support mismatch");
            prop_assert_eq!(parsed.zq_calibration, zq_calibration,
                "ZQ calibration mismatch");
            prop_assert_eq!(parsed.dcc_training, dcc_training,
                "DCC training mismatch");
            prop_assert_eq!(parsed.multi_plane_ops, multi_plane_ops,
                "Multi-plane ops mismatch");
            prop_assert_eq!(parsed.max_planes, max_planes,
                "Max planes mismatch: expected {}, got {}", max_planes, parsed.max_planes);

            // Verify ECC info
            if let Some(ecc) = &parsed.extended_ecc_info {
                prop_assert_eq!(ecc.ecc_bits, ecc_bits,
                    "ECC bits mismatch: expected {}, got {}", ecc_bits, ecc.ecc_bits);
                prop_assert_eq!(ecc.codeword_size, codeword_size,
                    "Codeword size mismatch: expected {}, got {}", codeword_size, ecc.codeword_size);
                prop_assert_eq!(ecc.max_correctable_bits, max_correctable_bits,
                    "Max correctable bits mismatch: expected {}, got {}",
                    max_correctable_bits, ecc.max_correctable_bits);
            }

            // Verify NV-DDR3 timing if supported
            if nv_ddr3_support {
                prop_assert!(parsed.nv_ddr3_timing.is_some(),
                    "NV-DDR3 timing should be present when supported");
                let timing = parsed.nv_ddr3_timing.unwrap();
                prop_assert_eq!(timing.data_rate_mt_s, data_rate,
                    "Data rate mismatch: expected {}, got {}", data_rate, timing.data_rate_mt_s);
            } else {
                prop_assert!(parsed.nv_ddr3_timing.is_none(),
                    "NV-DDR3 timing should not be present when not supported");
            }
        }

        /// Property test: Invalid extended parameter page signature should return None
        #[test]
        fn prop_onfi5_invalid_signature(
            sig0 in proptest::num::u8::ANY,
            sig1 in proptest::num::u8::ANY,
        ) {
            // Skip if signature happens to be 0x0FF0
            prop_assume!(!(sig0 == 0xF0 && sig1 == 0x0F));

            let mut ext_page = vec![0u8; 32];
            ext_page[0] = sig0;
            ext_page[1] = sig1;

            let result = parse_onfi5_extended_params(&ext_page);
            prop_assert!(result.is_none(),
                "Invalid signature 0x{:02X}{:02X} should return None", sig1, sig0);
        }

        /// Property test: Extended ECC info consistency
        #[test]
        fn prop_extended_ecc_info_consistency(
            ecc_bits in 1u8..=128,
            codeword_size in 256u16..=4096,
            max_correctable in 1u8..=128,
        ) {
            let ecc = ExtendedEccInfo {
                ecc_bits,
                codeword_size,
                max_correctable_bits: max_correctable,
            };

            // Verify struct fields are stored correctly
            prop_assert_eq!(ecc.ecc_bits, ecc_bits);
            prop_assert_eq!(ecc.codeword_size, codeword_size);
            prop_assert_eq!(ecc.max_correctable_bits, max_correctable);

            // Clone should produce identical values
            let cloned = ecc.clone();
            prop_assert_eq!(cloned.ecc_bits, ecc.ecc_bits);
            prop_assert_eq!(cloned.codeword_size, ecc.codeword_size);
            prop_assert_eq!(cloned.max_correctable_bits, ecc.max_correctable_bits);
        }

        /// Property test: NV-DDR3 timing data rate bounds
        #[test]
        fn prop_nv_ddr3_timing_bounds(
            data_rate in 1u16..=1600,
            t_cad in proptest::num::u8::ANY,
            t_dqsck in proptest::num::u8::ANY,
            t_dqsq in proptest::num::u8::ANY,
        ) {
            let timing = OnfiNvDdr3Timing {
                data_rate_mt_s: data_rate,
                t_cad,
                t_dqsck,
                t_dqsq,
            };

            // Verify struct fields are stored correctly
            prop_assert_eq!(timing.data_rate_mt_s, data_rate);
            prop_assert_eq!(timing.t_cad, t_cad);
            prop_assert_eq!(timing.t_dqsck, t_dqsck);
            prop_assert_eq!(timing.t_dqsq, t_dqsq);

            // Data rate should be within ONFI 5.0 spec (up to 1600 MT/s)
            prop_assert!(timing.data_rate_mt_s <= 1600,
                "Data rate {} exceeds ONFI 5.0 max of 1600 MT/s", timing.data_rate_mt_s);
        }

        /// Feature: nor-flash-ufs-support, Property 10: 16-bit Byte Swapping Correctness
        /// For any sequence of 16-bit words, converting to bytes and back (with consistent
        /// endianness) should produce the original sequence.
        /// **Validates: Requirements 8.4**
        #[test]
        fn prop_x16_byte_swap_roundtrip(words in proptest::collection::vec(proptest::num::u16::ANY, 0..100)) {
            // Test little-endian round-trip
            let bytes_le = x16_to_bytes(&words, true);
            let recovered_le = bytes_to_x16(&bytes_le, true);
            prop_assert_eq!(&words, &recovered_le,
                "Little-endian round-trip failed: {:?} -> {:?} -> {:?}",
                words, bytes_le, recovered_le);

            // Test big-endian round-trip
            let bytes_be = x16_to_bytes(&words, false);
            let recovered_be = bytes_to_x16(&bytes_be, false);
            prop_assert_eq!(&words, &recovered_be,
                "Big-endian round-trip failed: {:?} -> {:?} -> {:?}",
                words, bytes_be, recovered_be);
        }

        /// Property test: x16_to_bytes produces correct byte count
        #[test]
        fn prop_x16_to_bytes_length(words in proptest::collection::vec(proptest::num::u16::ANY, 0..100)) {
            let bytes_le = x16_to_bytes(&words, true);
            let bytes_be = x16_to_bytes(&words, false);

            // Output should always be exactly 2x the input length
            prop_assert_eq!(bytes_le.len(), words.len() * 2,
                "Little-endian byte count mismatch: {} words -> {} bytes (expected {})",
                words.len(), bytes_le.len(), words.len() * 2);
            prop_assert_eq!(bytes_be.len(), words.len() * 2,
                "Big-endian byte count mismatch: {} words -> {} bytes (expected {})",
                words.len(), bytes_be.len(), words.len() * 2);
        }

        /// Property test: bytes_to_x16 produces correct word count
        #[test]
        fn prop_bytes_to_x16_length(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..200)) {
            let words_le = bytes_to_x16(&bytes, true);
            let words_be = bytes_to_x16(&bytes, false);

            // Output should be floor(input_len / 2)
            let expected_len = bytes.len() / 2;
            prop_assert_eq!(words_le.len(), expected_len,
                "Little-endian word count mismatch: {} bytes -> {} words (expected {})",
                bytes.len(), words_le.len(), expected_len);
            prop_assert_eq!(words_be.len(), expected_len,
                "Big-endian word count mismatch: {} bytes -> {} words (expected {})",
                bytes.len(), words_be.len(), expected_len);
        }

        /// Property test: Endianness affects byte order correctly
        #[test]
        fn prop_x16_endianness_difference(word in proptest::num::u16::ANY) {
            // Skip symmetric values where endianness doesn't matter
            prop_assume!((word & 0xFF) != ((word >> 8) & 0xFF));

            let bytes_le = x16_to_bytes(&[word], true);
            let bytes_be = x16_to_bytes(&[word], false);

            // Little-endian: LSB first
            prop_assert_eq!(bytes_le[0], (word & 0xFF) as u8,
                "Little-endian LSB mismatch for 0x{:04X}", word);
            prop_assert_eq!(bytes_le[1], ((word >> 8) & 0xFF) as u8,
                "Little-endian MSB mismatch for 0x{:04X}", word);

            // Big-endian: MSB first
            prop_assert_eq!(bytes_be[0], ((word >> 8) & 0xFF) as u8,
                "Big-endian MSB mismatch for 0x{:04X}", word);
            prop_assert_eq!(bytes_be[1], (word & 0xFF) as u8,
                "Big-endian LSB mismatch for 0x{:04X}", word);

            // The two representations should be byte-swapped versions of each other
            prop_assert_eq!(bytes_le[0], bytes_be[1],
                "Byte swap mismatch for 0x{:04X}", word);
            prop_assert_eq!(bytes_le[1], bytes_be[0],
                "Byte swap mismatch for 0x{:04X}", word);
        }
    }
}
