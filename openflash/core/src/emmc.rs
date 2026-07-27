//! eMMC Flash chip database and protocol
//! Contains known eMMC chip parameters and command definitions

use serde::{Deserialize, Serialize};

/// eMMC chip information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmmcChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub size_gb: u32,
    pub sector_size: u32,      // Typically 512 bytes
    pub erase_group_size: u32, // Sectors per erase group
    pub voltage: String,
    pub max_clock_mhz: u8,
    pub ddr_support: bool,    // DDR mode support
    pub hs200_support: bool,  // HS200 mode support
    pub hs400_support: bool,  // HS400 mode support
    pub boot_partition: bool, // Has boot partitions
    pub rpmb_support: bool,   // Replay Protected Memory Block
}

/// eMMC standard commands (SD/MMC protocol)
pub mod commands {
    // Basic commands (class 0)
    pub const GO_IDLE_STATE: u8 = 0; // CMD0 - Reset
    pub const SEND_OP_COND: u8 = 1; // CMD1 - Send operating conditions
    pub const ALL_SEND_CID: u8 = 2; // CMD2 - Get CID
    pub const SET_RELATIVE_ADDR: u8 = 3; // CMD3 - Set RCA
    pub const SET_DSR: u8 = 4; // CMD4 - Set DSR
    pub const SWITCH: u8 = 6; // CMD6 - Switch function
    pub const SELECT_CARD: u8 = 7; // CMD7 - Select/deselect card
    pub const SEND_EXT_CSD: u8 = 8; // CMD8 - Send extended CSD
    pub const SEND_CSD: u8 = 9; // CMD9 - Send CSD
    pub const SEND_CID: u8 = 10; // CMD10 - Send CID
    pub const STOP_TRANSMISSION: u8 = 12; // CMD12 - Stop transmission
    pub const SEND_STATUS: u8 = 13; // CMD13 - Send status
    pub const GO_INACTIVE_STATE: u8 = 15; // CMD15 - Go inactive

    // Block read commands (class 2)
    pub const SET_BLOCKLEN: u8 = 16; // CMD16 - Set block length
    pub const READ_SINGLE_BLOCK: u8 = 17; // CMD17 - Read single block
    pub const READ_MULTIPLE_BLOCK: u8 = 18; // CMD18 - Read multiple blocks

    // Block write commands (class 4)
    pub const WRITE_BLOCK: u8 = 24; // CMD24 - Write single block
    pub const WRITE_MULTIPLE_BLOCK: u8 = 25; // CMD25 - Write multiple blocks
    pub const PROGRAM_CSD: u8 = 27; // CMD27 - Program CSD

    // Erase commands (class 5)
    pub const ERASE_GROUP_START: u8 = 35; // CMD35 - Set erase start
    pub const ERASE_GROUP_END: u8 = 36; // CMD36 - Set erase end
    pub const ERASE: u8 = 38; // CMD38 - Erase

    // Application specific commands
    pub const APP_CMD: u8 = 55; // CMD55 - App command prefix
    pub const GEN_CMD: u8 = 56; // CMD56 - General command
}

/// SPI mode commands (for SPI interface)
pub mod spi_commands {
    pub const READ_OCR: u8 = 58; // CMD58 - Read OCR
    pub const CRC_ON_OFF: u8 = 59; // CMD59 - CRC on/off
}

/// Response types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseType {
    R1,  // Normal response
    R1b, // Normal response with busy
    R2,  // CID/CSD response (136 bits)
    R3,  // OCR response
    R4,  // Fast I/O response
    R5,  // Interrupt request response
    R6,  // Published RCA response
    R7,  // Card interface condition
}

/// Card status bits (R1 response)
pub mod status {
    pub const READY_FOR_DATA: u32 = 1 << 8;
    pub const CURRENT_STATE_MASK: u32 = 0xF << 9;
    pub const ERASE_RESET: u32 = 1 << 13;
    pub const CARD_ECC_DISABLED: u32 = 1 << 14;
    pub const WP_ERASE_SKIP: u32 = 1 << 15;
    pub const CSD_OVERWRITE: u32 = 1 << 16;
    pub const ERROR: u32 = 1 << 19;
    pub const CC_ERROR: u32 = 1 << 20;
    pub const CARD_ECC_FAILED: u32 = 1 << 21;
    pub const ILLEGAL_COMMAND: u32 = 1 << 22;
    pub const COM_CRC_ERROR: u32 = 1 << 23;
    pub const LOCK_UNLOCK_FAILED: u32 = 1 << 24;
    pub const CARD_IS_LOCKED: u32 = 1 << 25;
    pub const WP_VIOLATION: u32 = 1 << 26;
    pub const ERASE_PARAM: u32 = 1 << 27;
    pub const ERASE_SEQ_ERROR: u32 = 1 << 28;
    pub const BLOCK_LEN_ERROR: u32 = 1 << 29;
    pub const ADDRESS_ERROR: u32 = 1 << 30;
    pub const OUT_OF_RANGE: u32 = 1 << 31;
}

/// Card states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardState {
    Idle = 0,
    Ready = 1,
    Ident = 2,
    Stby = 3,
    Tran = 4,
    Data = 5,
    Rcv = 6,
    Prg = 7,
    Dis = 8,
    Btst = 9,
    Slp = 10,
}

impl CardState {
    pub fn from_status(status: u32) -> Self {
        match (status >> 9) & 0xF {
            0 => CardState::Idle,
            1 => CardState::Ready,
            2 => CardState::Ident,
            3 => CardState::Stby,
            4 => CardState::Tran,
            5 => CardState::Data,
            6 => CardState::Rcv,
            7 => CardState::Prg,
            8 => CardState::Dis,
            9 => CardState::Btst,
            10 => CardState::Slp,
            _ => CardState::Idle,
        }
    }
}

/// Extended CSD register fields (important ones)
pub mod ext_csd {
    // Modes
    pub const PARTITION_CONFIG: usize = 179;
    pub const BUS_WIDTH: usize = 183;
    pub const HS_TIMING: usize = 185;
    pub const POWER_CLASS: usize = 187;

    // Properties
    pub const SEC_COUNT: usize = 212; // 4 bytes, sector count
    pub const DEVICE_TYPE: usize = 196;
    pub const CSD_STRUCTURE: usize = 194;
    pub const EXT_CSD_REV: usize = 192;
    pub const BOOT_SIZE_MULT: usize = 226;
    pub const RPMB_SIZE_MULT: usize = 168;
}

/// Get manufacturer name from CID manufacturer ID
pub fn get_emmc_manufacturer_name(mid: u8) -> &'static str {
    match mid {
        0x02 => "SanDisk",
        0x11 => "Toshiba",
        0x13 => "Micron",
        0x15 => "Samsung",
        0x45 => "SanDisk",
        0x70 => "Kingston",
        0x88 => "Foresee",
        0x90 => "Hynix",
        0xFE => "Micron",
        _ => "Unknown",
    }
}

/// Database of known eMMC chips
pub fn get_emmc_chip_info(cid: &[u8]) -> Option<EmmcChipInfo> {
    if cid.len() < 16 {
        return None;
    }

    let mid = cid[0];
    let manufacturer = get_emmc_manufacturer_name(mid).to_string();

    // Extract product name (6 bytes, ASCII)
    let pnm: String = cid[3..9]
        .iter()
        .filter(|&&b| (0x20..=0x7E).contains(&b))
        .map(|&b| b as char)
        .collect();

    // Try to match known chips
    match (mid, pnm.as_str()) {
        // Samsung
        (0x15, "BJTD4R") => Some(EmmcChipInfo {
            manufacturer,
            model: "KLMBG4JETD-B041".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x15, "AJTD4R") => Some(EmmcChipInfo {
            manufacturer,
            model: "KLMAG1JETD-B041".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // Micron
        (0x13, "Q2J54A") | (0xFE, "Q2J54A") => Some(EmmcChipInfo {
            manufacturer,
            model: "MTFC4GACAJCN".into(),
            size_gb: 4,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: false,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x13, "Q3J55A") | (0xFE, "Q3J55A") => Some(EmmcChipInfo {
            manufacturer,
            model: "MTFC8GACAAAM".into(),
            size_gb: 8,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // SanDisk
        (0x02, "DA4032") | (0x45, "DA4032") => Some(EmmcChipInfo {
            manufacturer,
            model: "SDINBDG4-32G".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),

        // Toshiba
        (0x11, "064G30") => Some(EmmcChipInfo {
            manufacturer,
            model: "THGBMJG6C1LBAIL".into(),
            size_gb: 8,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // Kingston
        (0x70, "EMMC04G") => Some(EmmcChipInfo {
            manufacturer,
            model: "EMMC04G-M627".into(),
            size_gb: 4,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: false,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: false,
        }),
        (0x70, "EMMC08G") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "EMMC08G-M627".into(),
            size_gb: 8,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: false,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: false,
        }),
        (0x70, "EMMC16G") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "EMMC16G-M627".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x70, "EMMC32G") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "EMMC32G-M627".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // ============ Samsung eMMC 5.1 (v2.2) ============
        (0x15, "CJNB4R") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "KLMCG2JETD-B041".into(),
            size_gb: 64,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x15, "DJNB4R") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "KLMDG4UCTA-B041".into(),
            size_gb: 128,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x15, "8GTF4R") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "KLMAG2GEND-B031".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // ============ Micron eMMC 5.1 (v2.2) ============
        (0x13, "Q4J55A") | (0xFE, "Q4J55A") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "MTFC16GACAANA".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x13, "Q5J56A") | (0xFE, "Q5J56A") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "MTFC32GACAANA".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x13, "Q6J57A") | (0xFE, "Q6J57A") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "MTFC64GACAANA".into(),
            size_gb: 64,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x13, "Q7J58A") | (0xFE, "Q7J58A") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "MTFC128GACAANA".into(),
            size_gb: 128,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),

        // ============ SK Hynix eMMC (v2.2) ============
        (0x90, "hB8aP>") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "H26M41208HPR".into(),
            size_gb: 8,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x90, "hC8aP>") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "H26M52208FPR".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x90, "hD8aP>") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "H26M64208EMR".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x90, "hE8aP>") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "H26M78208CMR".into(),
            size_gb: 64,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),

        // ============ SanDisk/WD eMMC (v2.2) ============
        (0x02, "DA4064") | (0x45, "DA4064") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "SDINBDG4-64G".into(),
            size_gb: 64,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x02, "DG4016") | (0x45, "DG4016") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "SDINBDG4-16G".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),

        // ============ Foresee eMMC (v2.2) ============
        (0x88, "NCEMAM") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "NCEMAM8G-08".into(),
            size_gb: 8,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: true,
            hs200_support: false,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: false,
        }),
        (0x88, "NCEMBM") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "NCEMBM8G-16".into(),
            size_gb: 16,
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: true,
        }),
        (0x88, "NCEMCM") => Some(EmmcChipInfo {
            manufacturer: manufacturer.clone(),
            model: "NCEMCM8G-32".into(),
            size_gb: 32,
            sector_size: 512,
            erase_group_size: 1024,
            voltage: "3.3V".into(),
            max_clock_mhz: 200,
            ddr_support: true,
            hs200_support: true,
            hs400_support: true,
            boot_partition: true,
            rpmb_support: true,
        }),

        // Generic fallback
        _ => Some(EmmcChipInfo {
            manufacturer,
            model: format!("Generic eMMC ({})", pnm),
            size_gb: 0, // Will be determined from EXT_CSD
            sector_size: 512,
            erase_group_size: 512,
            voltage: "3.3V".into(),
            max_clock_mhz: 52,
            ddr_support: false,
            hs200_support: false,
            hs400_support: false,
            boot_partition: true,
            rpmb_support: false,
        }),
    }
}

/// Parse capacity from Extended CSD
pub fn parse_capacity_from_ext_csd(ext_csd: &[u8]) -> u64 {
    if ext_csd.len() < 216 {
        return 0;
    }

    let sec_count = u32::from_le_bytes([
        ext_csd[ext_csd::SEC_COUNT],
        ext_csd[ext_csd::SEC_COUNT + 1],
        ext_csd[ext_csd::SEC_COUNT + 2],
        ext_csd[ext_csd::SEC_COUNT + 3],
    ]);

    (sec_count as u64) * 512
}

/// Parse boot partition size from Extended CSD
pub fn parse_boot_size_from_ext_csd(ext_csd: &[u8]) -> u32 {
    if ext_csd.len() <= ext_csd::BOOT_SIZE_MULT {
        return 0;
    }

    // Boot partition size = BOOT_SIZE_MULT * 128KB
    (ext_csd[ext_csd::BOOT_SIZE_MULT] as u32) * 128 * 1024
}

/// eMMC operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmmcReadResult {
    pub data: Vec<u8>,
    pub status: u32,
}

/// Calculate CRC7 for command
pub fn crc7(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        for i in (0..8).rev() {
            crc <<= 1;
            if ((byte >> i) & 1) ^ ((crc >> 7) & 1) != 0 {
                crc ^= 0x09;
            }
        }
    }
    (crc << 1) | 1
}

/// Calculate CRC16 for data
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manufacturer_names() {
        assert_eq!(get_emmc_manufacturer_name(0x15), "Samsung");
        assert_eq!(get_emmc_manufacturer_name(0x13), "Micron");
        assert_eq!(get_emmc_manufacturer_name(0x11), "Toshiba");
        assert_eq!(get_emmc_manufacturer_name(0xFF), "Unknown");
    }

    #[test]
    fn test_card_state_parsing() {
        let status = 0x00000900; // State = 4 (Tran)
        assert_eq!(CardState::from_status(status), CardState::Tran);
    }

    #[test]
    fn test_crc7() {
        // CMD0 with no argument
        let cmd = [0x40, 0x00, 0x00, 0x00, 0x00];
        let crc = crc7(&cmd);
        assert_eq!(crc, 0x95);
    }

    #[test]
    fn test_capacity_parsing() {
        let mut ext_csd = [0u8; 512];
        // Set SEC_COUNT to 0x00E90000 (15,269,888 sectors = ~7.8GB)
        ext_csd[ext_csd::SEC_COUNT] = 0x00;
        ext_csd[ext_csd::SEC_COUNT + 1] = 0x00;
        ext_csd[ext_csd::SEC_COUNT + 2] = 0xE9;
        ext_csd[ext_csd::SEC_COUNT + 3] = 0x00;

        let capacity = parse_capacity_from_ext_csd(&ext_csd);
        assert_eq!(capacity, 15_269_888 * 512);
    }
}
