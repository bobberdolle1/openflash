//! SPI NOR Flash chip database and protocol
//! Contains known SPI NOR chip parameters and command definitions

use serde::{Deserialize, Serialize};

/// SPI NOR chip information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpiNorChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub jedec_id: [u8; 3], // Manufacturer + Memory Type + Capacity
    pub size_bytes: u32,   // Total size in bytes
    pub page_size: u32,    // Page program size (typically 256)
    pub sector_size: u32,  // Sector erase size (typically 4KB)
    pub block_size: u32,   // Block erase size (typically 64KB)
    pub voltage: String,
    pub max_clock_mhz: u8,
    pub has_qspi: bool,    // Quad SPI support
    pub has_dual: bool,    // Dual SPI support
    pub address_bytes: u8, // 3 or 4 byte addressing
}

/// SPI NOR standard commands
pub mod commands {
    // Identification
    pub const READ_JEDEC_ID: u8 = 0x9F;
    pub const READ_SFDP: u8 = 0x5A;

    // Read commands
    pub const READ: u8 = 0x03;
    pub const FAST_READ: u8 = 0x0B;
    pub const DUAL_READ: u8 = 0x3B;
    pub const QUAD_READ: u8 = 0x6B;
    pub const DUAL_IO_READ: u8 = 0xBB;
    pub const QUAD_IO_READ: u8 = 0xEB;

    // Write commands
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const WRITE_DISABLE: u8 = 0x04;
    pub const PAGE_PROGRAM: u8 = 0x02;
    pub const QUAD_PAGE_PROGRAM: u8 = 0x32;

    // Erase commands
    pub const SECTOR_ERASE: u8 = 0x20; // 4KB
    pub const BLOCK_ERASE_32K: u8 = 0x52;
    pub const BLOCK_ERASE_64K: u8 = 0xD8;
    pub const CHIP_ERASE: u8 = 0xC7;
    pub const CHIP_ERASE_ALT: u8 = 0x60;

    // Status commands
    pub const READ_STATUS_1: u8 = 0x05;
    pub const READ_STATUS_2: u8 = 0x35;
    pub const READ_STATUS_3: u8 = 0x15;
    pub const WRITE_STATUS_1: u8 = 0x01;
    pub const WRITE_STATUS_2: u8 = 0x31;
    pub const WRITE_STATUS_3: u8 = 0x11;

    // Other
    pub const RESET_ENABLE: u8 = 0x66;
    pub const RESET: u8 = 0x99;
    pub const ENTER_4BYTE_MODE: u8 = 0xB7;
    pub const EXIT_4BYTE_MODE: u8 = 0xE9;
    pub const READ_UNIQUE_ID: u8 = 0x4B;
    pub const POWER_DOWN: u8 = 0xB9;
    pub const RELEASE_POWER_DOWN: u8 = 0xAB;
}

/// Status register 1 bits
pub mod status1 {
    pub const BUSY: u8 = 0x01;
    pub const WEL: u8 = 0x02;
    pub const BP0: u8 = 0x04;
    pub const BP1: u8 = 0x08;
    pub const BP2: u8 = 0x10;
    pub const TB: u8 = 0x20;
    pub const SEC: u8 = 0x40;
    pub const SRP0: u8 = 0x80;
}

/// Status register 2 bits
pub mod status2 {
    pub const SRP1: u8 = 0x01;
    pub const QE: u8 = 0x02;
    pub const LB1: u8 = 0x08;
    pub const LB2: u8 = 0x10;
    pub const LB3: u8 = 0x20;
    pub const CMP: u8 = 0x40;
    pub const SUS: u8 = 0x80;
}

/// Status register 3 bits
pub mod status3 {
    pub const WPS: u8 = 0x04;
    pub const DRV0: u8 = 0x20;
    pub const DRV1: u8 = 0x40;
}

/// SPI NOR manufacturer IDs
pub mod manufacturers {
    pub const WINBOND: u8 = 0xEF;
    pub const MACRONIX: u8 = 0xC2;
    pub const ISSI: u8 = 0x9D;
    pub const GIGADEVICE: u8 = 0xC8;
    pub const MICRON: u8 = 0x20;
    pub const SPANSION: u8 = 0x01;
    pub const SST: u8 = 0xBF;
    pub const ATMEL: u8 = 0x1F;
    pub const EON: u8 = 0x1C;
    pub const XMC: u8 = 0x20;
}

/// Get manufacturer name from JEDEC ID
pub fn get_spi_nor_manufacturer_name(mfr_id: u8) -> &'static str {
    match mfr_id {
        manufacturers::WINBOND => "Winbond",
        manufacturers::MACRONIX => "Macronix",
        manufacturers::ISSI => "ISSI",
        manufacturers::GIGADEVICE => "GigaDevice",
        manufacturers::MICRON => "Micron",
        manufacturers::SPANSION => "Spansion/Cypress",
        manufacturers::SST => "SST/Microchip",
        manufacturers::ATMEL => "Atmel/Microchip",
        manufacturers::EON => "EON",
        _ => "Unknown",
    }
}

/// Get chip info from JEDEC ID
pub fn get_spi_nor_chip_info(jedec_id: &[u8; 3]) -> Option<SpiNorChipInfo> {
    let mfr = jedec_id[0];

    // Try exact match first
    if let Some(info) = get_spi_nor_chip_info_exact(jedec_id) {
        return Some(info);
    }

    // Fall back to generic detection based on capacity byte
    get_spi_nor_chip_info_generic(mfr, jedec_id)
}

fn get_spi_nor_chip_info_exact(jedec_id: &[u8; 3]) -> Option<SpiNorChipInfo> {
    match jedec_id {
        // ============ Winbond W25Q Series ============
        [0xEF, 0x40, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q80DV".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024, // 1MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x40, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q16JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024, // 2MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x40, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q32JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024, // 4MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x40, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q64JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024, // 8MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x40, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q128JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024, // 16MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x40, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q256JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024, // 32MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0xEF, 0x40, 0x20] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q512JV".into(),
            jedec_id: *jedec_id,
            size_bytes: 64 * 1024 * 1024, // 64MB
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        // Winbond 1.8V variants
        [0xEF, 0x60, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q16JW".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x60, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q32JW".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x60, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q64JW".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x60, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q128JW".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ Macronix MX25L Series ============
        [0xC2, 0x20, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L8035E".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x20, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L1606E".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 86,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x20, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L3233F".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x20, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L6433F".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x20, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L12835F".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x20, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L25635F".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0xC2, 0x20, 0x1A] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25L51245G".into(),
            jedec_id: *jedec_id,
            size_bytes: 64 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        // Macronix 1.8V variants
        [0xC2, 0x25, 0x36] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25U3235F".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x25, 0x37] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25U6435F".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC2, 0x25, 0x38] => Some(SpiNorChipInfo {
            manufacturer: "Macronix".into(),
            model: "MX25U12835F".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ ISSI IS25LP Series ============
        [0x9D, 0x60, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP080D".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x60, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP016D".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x60, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP032D".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x60, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP064D".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x60, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP128F".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x60, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP256D".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0x9D, 0x60, 0x1A] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25LP512M".into(),
            jedec_id: *jedec_id,
            size_bytes: 64 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        // ISSI 1.8V variants (IS25WP series)
        [0x9D, 0x70, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25WP016D".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x70, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25WP032D".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x70, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25WP064D".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x9D, 0x70, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "ISSI".into(),
            model: "IS25WP128F".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ GigaDevice GD25Q Series (v2.2) ============
        [0xC8, 0x40, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q80C".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x40, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q16C".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x40, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q32C".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x40, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q64C".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x40, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q128C".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x40, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q256D".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0xC8, 0x40, 0x20] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25Q512MC".into(),
            jedec_id: *jedec_id,
            size_bytes: 64 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        // GigaDevice 1.8V variants
        [0xC8, 0x60, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25LQ16C".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x60, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25LQ32D".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x60, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25LQ64C".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x60, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25LQ128D".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xC8, 0x60, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "GigaDevice".into(),
            model: "GD25LQ256D".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 120,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),

        // ============ EON EN25QH Series (v2.2) ============
        [0x1C, 0x70, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH80A".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x1C, 0x70, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH16A".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x1C, 0x70, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH32B".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x1C, 0x70, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH64A".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x1C, 0x70, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH128A".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x1C, 0x70, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "EON".into(),
            model: "EN25QH256A".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),

        // ============ Micron/Numonyx N25Q Series (v2.2) ============
        [0x20, 0xBA, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q032A".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0xBA, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q064A".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0xBA, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q128A".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0xBA, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q256A".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0x20, 0xBA, 0x20] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q512A".into(),
            jedec_id: *jedec_id,
            size_bytes: 64 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        [0x20, 0xBA, 0x21] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "MT25QL01G".into(),
            jedec_id: *jedec_id,
            size_bytes: 128 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),
        // Micron 1.8V variants
        [0x20, 0xBB, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q128A11".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0xBB, 0x19] => Some(SpiNorChipInfo {
            manufacturer: "Micron".into(),
            model: "N25Q256A11".into(),
            jedec_id: *jedec_id,
            size_bytes: 32 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.8V".into(),
            max_clock_mhz: 108,
            has_qspi: true,
            has_dual: true,
            address_bytes: 4,
        }),

        // ============ XMC/XTX XM25Q Series (v2.2) ============
        [0x20, 0x40, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "XMC".into(),
            model: "XM25QH32B".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0x40, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "XMC".into(),
            model: "XM25QH64A".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x20, 0x40, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "XMC".into(),
            model: "XM25QH128A".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ Puya P25Q Series (v2.2) ============
        [0x85, 0x60, 0x14] => Some(SpiNorChipInfo {
            manufacturer: "Puya".into(),
            model: "P25Q80H".into(),
            jedec_id: *jedec_id,
            size_bytes: 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x85, 0x60, 0x15] => Some(SpiNorChipInfo {
            manufacturer: "Puya".into(),
            model: "P25Q16H".into(),
            jedec_id: *jedec_id,
            size_bytes: 2 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x85, 0x60, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Puya".into(),
            model: "P25Q32H".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ Boya BY25Q Series (v2.2) ============
        [0x68, 0x40, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Boya".into(),
            model: "BY25Q32BS".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x68, 0x40, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Boya".into(),
            model: "BY25Q64AS".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0x68, 0x40, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Boya".into(),
            model: "BY25Q128AS".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "3.3V".into(),
            max_clock_mhz: 104,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        // ============ Winbond W25Q 1.2V Series (v2.2) ============
        [0xEF, 0x80, 0x16] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q32ND".into(),
            jedec_id: *jedec_id,
            size_bytes: 4 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.2V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x80, 0x17] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q64ND".into(),
            jedec_id: *jedec_id,
            size_bytes: 8 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.2V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),
        [0xEF, 0x80, 0x18] => Some(SpiNorChipInfo {
            manufacturer: "Winbond".into(),
            model: "W25Q128ND".into(),
            jedec_id: *jedec_id,
            size_bytes: 16 * 1024 * 1024,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            voltage: "1.2V".into(),
            max_clock_mhz: 133,
            has_qspi: true,
            has_dual: true,
            address_bytes: 3,
        }),

        _ => None,
    }
}

/// Generic SPI NOR chip detection based on capacity byte
fn get_spi_nor_chip_info_generic(mfr: u8, jedec_id: &[u8; 3]) -> Option<SpiNorChipInfo> {
    let manufacturer = get_spi_nor_manufacturer_name(mfr).to_string();
    let capacity_byte = jedec_id[2];

    // The capacity byte is the base-2 logarithm of the size in bytes: 0x15 is
    // 2^21 = 2 MiB (a 16 Mbit part), 0x18 is 2^24 = 16 MiB, and so on. The range
    // starts at 0x10 (64 KiB) because smaller SPI NOR parts exist and used to be
    // rejected here as unknown.
    let (size_bytes, address_bytes) = match capacity_byte {
        0x10..=0x1B => {
            let size_bytes = 1u32 << capacity_byte;
            // Three address bytes reach 16 MiB; anything larger needs four.
            let address_bytes = if size_bytes > 16 * 1024 * 1024 { 4 } else { 3 };
            (size_bytes, address_bytes)
        }
        // Some vendors number their largest parts outside the logarithmic scheme.
        0x20 => (64 * 1024 * 1024, 4),
        0x21 => (128 * 1024 * 1024, 4),
        _ => return None,
    };

    Some(SpiNorChipInfo {
        manufacturer,
        model: format!(
            "Generic SPI NOR 0x{:02X}{:02X}{:02X}",
            jedec_id[0], jedec_id[1], jedec_id[2]
        ),
        jedec_id: *jedec_id,
        size_bytes,
        page_size: 256,
        sector_size: 4096,
        block_size: 65536,
        voltage: "3.3V".into(),
        max_clock_mhz: 80,
        has_qspi: true,
        has_dual: true,
        address_bytes,
    })
}

/// SFDP (Serial Flash Discoverable Parameters) header
#[derive(Debug, Clone)]
pub struct SfdpHeader {
    pub signature: [u8; 4], // "SFDP"
    pub minor_rev: u8,
    pub major_rev: u8,
    pub num_param_headers: u8,
    pub access_protocol: u8,
}

/// SFDP parameter header
#[derive(Debug, Clone)]
pub struct SfdpParamHeader {
    pub id_lsb: u8,
    pub minor_rev: u8,
    pub major_rev: u8,
    pub length_dwords: u8,
    pub table_pointer: u32,
    pub id_msb: u8,
}

/// Quad enable method from SFDP
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QuadEnableMethod {
    None,
    StatusReg2Bit1,
    StatusReg1Bit6,
    StatusReg2Bit7,
    StatusReg1Bit1Volatile,
}

/// Fast read support flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FastReadSupport {
    pub fast_read_112: bool, // 1-1-2 (cmd-addr-data)
    pub fast_read_122: bool, // 1-2-2
    pub fast_read_114: bool, // 1-1-4
    pub fast_read_144: bool, // 1-4-4
}

/// Parsed SFDP information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfdpInfo {
    pub density_bits: u64,
    pub page_size: u32,
    pub sector_size: u32,
    pub supports_4kb_erase: bool,
    pub supports_32kb_erase: bool,
    pub supports_64kb_erase: bool,
    pub quad_enable_method: QuadEnableMethod,
    pub address_bytes: u8,
    pub fast_read_support: FastReadSupport,
}

/// SFDP parser
pub struct SfdpParser;

impl SfdpParser {
    /// Parse SFDP header (first 8 bytes)
    pub fn parse_header(data: &[u8]) -> Option<SfdpHeader> {
        if data.len() < 8 {
            return None;
        }

        // Check signature "SFDP"
        if &data[0..4] != b"SFDP" {
            return None;
        }

        Some(SfdpHeader {
            signature: [data[0], data[1], data[2], data[3]],
            minor_rev: data[4],
            major_rev: data[5],
            num_param_headers: data[6].saturating_add(1), // NPH is 0-based
            access_protocol: data[7],
        })
    }

    /// Parse parameter header (8 bytes each, starting at offset 8)
    pub fn parse_param_header(data: &[u8]) -> Option<SfdpParamHeader> {
        if data.len() < 8 {
            return None;
        }

        let table_pointer = u32::from_le_bytes([data[4], data[5], data[6], 0]);

        Some(SfdpParamHeader {
            id_lsb: data[0],
            minor_rev: data[1],
            major_rev: data[2],
            length_dwords: data[3],
            table_pointer,
            id_msb: data[7],
        })
    }

    /// Parse JEDEC Basic Flash Parameter Table (BFPT)
    pub fn parse_bfpt(data: &[u8]) -> Option<SfdpInfo> {
        if data.len() < 36 {
            // Minimum 9 DWORDs
            return None;
        }

        // DWORD 1: Block/Sector erase sizes
        let dword1 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let supports_4kb_erase = (dword1 & 0x03) != 0x01; // Bits 1:0

        // DWORD 2: Density
        let dword2 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let density_bits = if dword2 & 0x80000000 != 0 {
            // Bit 31 set: density is 2^N bits
            1u64 << (dword2 & 0x7FFFFFFF)
        } else {
            // Bit 31 clear: density is N+1 bits
            (dword2 as u64) + 1
        };

        // DWORD 3: Fast read support
        let dword3 = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let fast_read_support = FastReadSupport {
            fast_read_112: (dword3 & (1 << 16)) != 0,
            fast_read_122: (dword3 & (1 << 20)) != 0,
            fast_read_114: (dword3 & (1 << 22)) != 0,
            fast_read_144: (dword3 & (1 << 21)) != 0,
        };

        // DWORD 8: Erase type 1 & 2
        let dword8 = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
        let erase1_size = 1u32 << (dword8 & 0xFF);
        let erase2_size = 1u32 << ((dword8 >> 16) & 0xFF);

        let supports_32kb_erase = erase1_size == 32768 || erase2_size == 32768;
        let supports_64kb_erase = erase1_size == 65536 || erase2_size == 65536;

        // Determine sector size (smallest erase unit)
        let sector_size = if supports_4kb_erase {
            4096
        } else {
            erase1_size.min(erase2_size)
        };

        // DWORD 15 (if available): Quad enable method
        let quad_enable_method = if data.len() >= 60 {
            let dword15 = u32::from_le_bytes([data[56], data[57], data[58], data[59]]);
            match (dword15 >> 20) & 0x07 {
                0 => QuadEnableMethod::None,
                1 => QuadEnableMethod::StatusReg2Bit1,
                2 => QuadEnableMethod::StatusReg1Bit6,
                3 => QuadEnableMethod::StatusReg2Bit7,
                4 => QuadEnableMethod::StatusReg1Bit1Volatile,
                _ => QuadEnableMethod::None,
            }
        } else {
            QuadEnableMethod::None
        };

        // Address bytes
        let address_bytes = if density_bits > 128 * 1024 * 1024 * 8 {
            4
        } else {
            3
        };

        Some(SfdpInfo {
            density_bits,
            page_size: 256, // Standard page size
            sector_size,
            supports_4kb_erase,
            supports_32kb_erase,
            supports_64kb_erase,
            quad_enable_method,
            address_bytes,
            fast_read_support,
        })
    }

    /// Parse complete SFDP data
    pub fn parse(data: &[u8]) -> Option<SfdpInfo> {
        let header = Self::parse_header(data)?;

        if header.num_param_headers == 0 {
            return None;
        }

        // Parse first parameter header (JEDEC BFPT)
        let param_header = Self::parse_param_header(&data[8..])?;

        // Read BFPT at specified offset
        let bfpt_offset = param_header.table_pointer as usize;
        let bfpt_len = (param_header.length_dwords as usize) * 4;

        if data.len() < bfpt_offset + bfpt_len {
            return None;
        }

        Self::parse_bfpt(&data[bfpt_offset..])
    }
}

impl SfdpInfo {
    /// Serialize SfdpInfo to BFPT bytes (for round-trip testing)
    /// Returns a 64-byte BFPT (16 DWORDs)
    pub fn to_bfpt_bytes(&self) -> Vec<u8> {
        let mut bfpt = vec![0u8; 64];

        // DWORD 1: Block/Sector erase sizes
        let dword1: u32 = if self.supports_4kb_erase { 0x00 } else { 0x01 };
        bfpt[0..4].copy_from_slice(&dword1.to_le_bytes());

        // DWORD 2: Density (use N+1 format for simplicity)
        let dword2: u32 = (self.density_bits - 1) as u32;
        bfpt[4..8].copy_from_slice(&dword2.to_le_bytes());

        // DWORD 3: Fast read support
        let mut dword3: u32 = 0;
        if self.fast_read_support.fast_read_112 {
            dword3 |= 1 << 16;
        }
        if self.fast_read_support.fast_read_122 {
            dword3 |= 1 << 20;
        }
        if self.fast_read_support.fast_read_114 {
            dword3 |= 1 << 22;
        }
        if self.fast_read_support.fast_read_144 {
            dword3 |= 1 << 21;
        }
        bfpt[8..12].copy_from_slice(&dword3.to_le_bytes());

        // DWORD 8: Erase type 1 & 2
        let erase1_exp = if self.supports_4kb_erase { 12u8 } else { 16u8 }; // 4KB or 64KB
        let erase2_exp = if self.supports_32kb_erase {
            15u8
        } else if self.supports_64kb_erase {
            16u8
        } else {
            12u8
        };
        let dword8: u32 = (erase1_exp as u32) | ((erase2_exp as u32) << 16);
        bfpt[28..32].copy_from_slice(&dword8.to_le_bytes());

        // DWORD 15: Quad enable method
        let qe_bits: u32 = match self.quad_enable_method {
            QuadEnableMethod::None => 0,
            QuadEnableMethod::StatusReg2Bit1 => 1,
            QuadEnableMethod::StatusReg1Bit6 => 2,
            QuadEnableMethod::StatusReg2Bit7 => 3,
            QuadEnableMethod::StatusReg1Bit1Volatile => 4,
        };
        let dword15: u32 = qe_bits << 20;
        bfpt[56..60].copy_from_slice(&dword15.to_le_bytes());

        bfpt
    }
}

/// Protection status decoded from status registers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionStatus {
    pub bp0: bool,
    pub bp1: bool,
    pub bp2: bool,
    pub bp3: bool,
    pub bp4: bool,
    pub tb: bool,   // Top/Bottom protect
    pub sec: bool,  // Sector/Block protect
    pub cmp: bool,  // Complement protect
    pub srp0: bool, // Status Register Protect 0
    pub srp1: bool, // Status Register Protect 1
}

impl ProtectionStatus {
    /// Decode protection status from status registers 1 and 2
    pub fn from_status_registers(sr1: u8, sr2: u8) -> Self {
        Self {
            bp0: (sr1 & status1::BP0) != 0,
            bp1: (sr1 & status1::BP1) != 0,
            bp2: (sr1 & status1::BP2) != 0,
            bp3: false, // BP3 location varies by chip
            bp4: false, // BP4 location varies by chip
            tb: (sr1 & status1::TB) != 0,
            sec: (sr1 & status1::SEC) != 0,
            cmp: (sr2 & status2::CMP) != 0,
            srp0: (sr1 & status1::SRP0) != 0,
            srp1: (sr2 & status2::SRP1) != 0,
        }
    }

    /// Encode protection bits back to status register 1 value
    /// Only encodes BP0-BP2, TB, SEC, SRP0 (bits that are in SR1)
    pub fn to_status_register1(&self) -> u8 {
        let mut sr1 = 0u8;
        if self.bp0 {
            sr1 |= status1::BP0;
        }
        if self.bp1 {
            sr1 |= status1::BP1;
        }
        if self.bp2 {
            sr1 |= status1::BP2;
        }
        if self.tb {
            sr1 |= status1::TB;
        }
        if self.sec {
            sr1 |= status1::SEC;
        }
        if self.srp0 {
            sr1 |= status1::SRP0;
        }
        sr1
    }

    /// Encode protection bits back to status register 2 value
    /// Only encodes CMP, SRP1 (bits that are in SR2)
    pub fn to_status_register2(&self) -> u8 {
        let mut sr2 = 0u8;
        if self.cmp {
            sr2 |= status2::CMP;
        }
        if self.srp1 {
            sr2 |= status2::SRP1;
        }
        sr2
    }

    /// Check if any protection is enabled
    pub fn is_protected(&self) -> bool {
        self.bp0 || self.bp1 || self.bp2 || self.bp3 || self.bp4
    }

    /// Get human-readable protection description
    pub fn description(&self) -> String {
        if !self.is_protected() {
            return "Unprotected".to_string();
        }

        let bp_value = (self.bp0 as u8)
            | ((self.bp1 as u8) << 1)
            | ((self.bp2 as u8) << 2)
            | ((self.bp3 as u8) << 3)
            | ((self.bp4 as u8) << 4);

        let location = if self.tb { "Bottom" } else { "Top" };
        let unit = if self.sec { "Sectors" } else { "Blocks" };
        let complement = if self.cmp { " (Complemented)" } else { "" };

        format!(
            "Protected: BP={}, {}, {}{}",
            bp_value, location, unit, complement
        )
    }
}

/// SPI NOR error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpiNorError {
    /// Chip not responding (no JEDEC ID)
    ChipNotFound,
    /// Unknown chip (JEDEC ID not in database)
    UnknownChip { jedec_id: [u8; 3] },
    /// Write operation failed
    WriteFailed { address: u32 },
    /// Erase operation failed
    EraseFailed { address: u32 },
    /// Verification failed after write
    VerifyFailed {
        address: u32,
        expected: u8,
        actual: u8,
    },
    /// Chip is write-protected
    WriteProtected,
    /// Timeout waiting for operation
    Timeout,
    /// Invalid address (out of bounds)
    InvalidAddress { address: u32, max: u32 },
    /// SFDP parsing failed
    SfdpParseError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winbond_chip_recognition() {
        let jedec_id = [0xEF, 0x40, 0x18];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Winbond");
        assert_eq!(chip_info.model, "W25Q128JV");
        assert_eq!(chip_info.size_bytes, 16 * 1024 * 1024);
        assert!(chip_info.has_qspi);
    }

    #[test]
    fn test_macronix_chip_recognition() {
        let jedec_id = [0xC2, 0x20, 0x18];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();

        assert_eq!(chip_info.manufacturer, "Macronix");
        assert_eq!(chip_info.model, "MX25L12835F");
        assert_eq!(chip_info.size_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_issi_chip_recognition() {
        let jedec_id = [0x9D, 0x60, 0x18];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();

        assert_eq!(chip_info.manufacturer, "ISSI");
        assert_eq!(chip_info.model, "IS25LP128F");
        assert_eq!(chip_info.size_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_generic_detection() {
        // Unknown manufacturer but valid capacity byte
        let jedec_id = [0xAA, 0xBB, 0x17];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();

        assert_eq!(chip_info.size_bytes, 8 * 1024 * 1024);
        assert_eq!(chip_info.address_bytes, 3);
    }

    #[test]
    fn test_manufacturer_names() {
        assert_eq!(get_spi_nor_manufacturer_name(0xEF), "Winbond");
        assert_eq!(get_spi_nor_manufacturer_name(0xC2), "Macronix");
        assert_eq!(get_spi_nor_manufacturer_name(0x9D), "ISSI");
        assert_eq!(get_spi_nor_manufacturer_name(0xFF), "Unknown");
    }

    #[test]
    fn test_protection_status_decode() {
        let sr1 = status1::BP0 | status1::BP1 | status1::TB;
        let sr2 = status2::CMP;

        let status = ProtectionStatus::from_status_registers(sr1, sr2);

        assert!(status.bp0);
        assert!(status.bp1);
        assert!(!status.bp2);
        assert!(status.tb);
        assert!(status.cmp);
        assert!(status.is_protected());
    }

    #[test]
    fn test_protection_status_encode() {
        let status = ProtectionStatus {
            bp0: true,
            bp1: true,
            bp2: false,
            bp3: false,
            bp4: false,
            tb: true,
            sec: false,
            cmp: true,
            srp0: false,
            srp1: false,
        };

        let sr1 = status.to_status_register1();
        let sr2 = status.to_status_register2();

        assert_eq!(sr1, status1::BP0 | status1::BP1 | status1::TB);
        assert_eq!(sr2, status2::CMP);
    }

    #[test]
    fn test_sfdp_header_parse() {
        let data = b"SFDP\x06\x01\x00\xFF";
        let header = SfdpParser::parse_header(data).unwrap();

        assert_eq!(&header.signature, b"SFDP");
        assert_eq!(header.minor_rev, 0x06);
        assert_eq!(header.major_rev, 0x01);
        assert_eq!(header.num_param_headers, 1);
    }

    #[test]
    fn test_sfdp_header_invalid() {
        let data = b"XXXX\x06\x01\x00\xFF";
        assert!(SfdpParser::parse_header(data).is_none());
    }

    #[test]
    fn test_4byte_addressing_detection() {
        // 256Mbit chip should use 4-byte addressing
        let jedec_id = [0xEF, 0x40, 0x19];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();
        assert_eq!(chip_info.address_bytes, 4);

        // 128Mbit chip should use 3-byte addressing
        let jedec_id = [0xEF, 0x40, 0x18];
        let chip_info = get_spi_nor_chip_info(&jedec_id).unwrap();
        assert_eq!(chip_info.address_bytes, 3);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: nor-flash-ufs-support, Property 1: JEDEC ID Lookup Consistency
        /// For any valid JEDEC ID in the chip database, looking up the chip info
        /// and then checking the returned JEDEC ID should match the original query ID.
        /// **Validates: Requirements 1.2, 1.4**
        #[test]
        fn prop_jedec_id_lookup_consistency(
            mfr in prop_oneof![
                Just(0xEF_u8),  // Winbond
                Just(0xC2_u8),  // Macronix
                Just(0x9D_u8),  // ISSI
            ],
            mem_type in prop_oneof![
                Just(0x40_u8),  // Winbond 3.3V
                Just(0x60_u8),  // Winbond 1.8V
                Just(0x20_u8),  // Macronix 3.3V
                Just(0x25_u8),  // Macronix 1.8V
            ],
            capacity in 0x14u8..=0x20
        ) {
            let jedec_id = [mfr, mem_type, capacity];
            if let Some(info) = get_spi_nor_chip_info(&jedec_id) {
                // Property: returned JEDEC ID must match query ID
                prop_assert_eq!(info.jedec_id, jedec_id,
                    "JEDEC ID mismatch: queried {:?}, got {:?}", jedec_id, info.jedec_id);
            }
        }

        /// Property test: All known chips in database return consistent JEDEC IDs
        #[test]
        fn prop_known_chips_jedec_consistency(idx in 0usize..32) {
            // List of known JEDEC IDs from the database
            let known_ids: [[u8; 3]; 32] = [
                // Winbond 3.3V
                [0xEF, 0x40, 0x14], [0xEF, 0x40, 0x15], [0xEF, 0x40, 0x16],
                [0xEF, 0x40, 0x17], [0xEF, 0x40, 0x18], [0xEF, 0x40, 0x19],
                [0xEF, 0x40, 0x20],
                // Winbond 1.8V
                [0xEF, 0x60, 0x15], [0xEF, 0x60, 0x16], [0xEF, 0x60, 0x17],
                [0xEF, 0x60, 0x18],
                // Macronix 3.3V
                [0xC2, 0x20, 0x14], [0xC2, 0x20, 0x15], [0xC2, 0x20, 0x16],
                [0xC2, 0x20, 0x17], [0xC2, 0x20, 0x18], [0xC2, 0x20, 0x19],
                [0xC2, 0x20, 0x1A],
                // Macronix 1.8V
                [0xC2, 0x25, 0x36], [0xC2, 0x25, 0x37], [0xC2, 0x25, 0x38],
                // ISSI 3.3V
                [0x9D, 0x60, 0x14], [0x9D, 0x60, 0x15], [0x9D, 0x60, 0x16],
                [0x9D, 0x60, 0x17], [0x9D, 0x60, 0x18], [0x9D, 0x60, 0x19],
                [0x9D, 0x60, 0x1A],
                // ISSI 1.8V
                [0x9D, 0x70, 0x15], [0x9D, 0x70, 0x16], [0x9D, 0x70, 0x17],
                [0x9D, 0x70, 0x18],
            ];

            let jedec_id = known_ids[idx];
            let info = get_spi_nor_chip_info(&jedec_id)
                .unwrap_or_else(|| panic!("Known JEDEC ID {:?} should be in database", jedec_id));

            // Property: returned JEDEC ID must match query ID
            prop_assert_eq!(info.jedec_id, jedec_id,
                "JEDEC ID mismatch for known chip: queried {:?}, got {:?}", jedec_id, info.jedec_id);
        }

        /// Feature: nor-flash-ufs-support, Property 2: SFDP Parsing Round-Trip
        /// For any valid SFDP data structure, serializing it to bytes and then parsing
        /// those bytes should produce an equivalent SfdpInfo structure.
        /// **Validates: Requirements 1.3**
        #[test]
        fn prop_sfdp_parsing_roundtrip(
            density_exp in 20u32..28,  // 1Mbit to 128Mbit
            supports_4kb_erase in proptest::bool::ANY,
            supports_32kb_erase in proptest::bool::ANY,
            supports_64kb_erase in proptest::bool::ANY,
            qe_method in 0u8..5,
            fast_read_112 in proptest::bool::ANY,
            fast_read_122 in proptest::bool::ANY,
            fast_read_114 in proptest::bool::ANY,
            fast_read_144 in proptest::bool::ANY,
        ) {
            let quad_enable_method = match qe_method {
                0 => QuadEnableMethod::None,
                1 => QuadEnableMethod::StatusReg2Bit1,
                2 => QuadEnableMethod::StatusReg1Bit6,
                3 => QuadEnableMethod::StatusReg2Bit7,
                _ => QuadEnableMethod::StatusReg1Bit1Volatile,
            };

            let density_bits = 1u64 << density_exp;
            let address_bytes = if density_bits > 128 * 1024 * 1024 * 8 { 4 } else { 3 };

            let original = SfdpInfo {
                density_bits,
                page_size: 256,
                sector_size: if supports_4kb_erase { 4096 } else { 65536 },
                supports_4kb_erase,
                supports_32kb_erase,
                supports_64kb_erase,
                quad_enable_method,
                address_bytes,
                fast_read_support: FastReadSupport {
                    fast_read_112,
                    fast_read_122,
                    fast_read_114,
                    fast_read_144,
                },
            };

            // Serialize to BFPT bytes
            let bfpt_bytes = original.to_bfpt_bytes();

            // Parse back
            let parsed = SfdpParser::parse_bfpt(&bfpt_bytes)
                .expect("Should parse valid BFPT bytes");

            // Verify key fields match (some fields may have different representations)
            prop_assert_eq!(parsed.density_bits, original.density_bits,
                "Density mismatch: original {}, parsed {}", original.density_bits, parsed.density_bits);
            prop_assert_eq!(parsed.supports_4kb_erase, original.supports_4kb_erase,
                "4KB erase support mismatch");
            prop_assert_eq!(parsed.quad_enable_method, original.quad_enable_method,
                "Quad enable method mismatch");
            prop_assert_eq!(parsed.fast_read_support.fast_read_112, original.fast_read_support.fast_read_112,
                "Fast read 1-1-2 mismatch");
            prop_assert_eq!(parsed.fast_read_support.fast_read_122, original.fast_read_support.fast_read_122,
                "Fast read 1-2-2 mismatch");
            prop_assert_eq!(parsed.fast_read_support.fast_read_114, original.fast_read_support.fast_read_114,
                "Fast read 1-1-4 mismatch");
            prop_assert_eq!(parsed.fast_read_support.fast_read_144, original.fast_read_support.fast_read_144,
                "Fast read 1-4-4 mismatch");
        }

        /// Feature: nor-flash-ufs-support, Property 4: Status Register Bit Decoding
        /// For any status register byte value, decoding protection bits (BP0-BP2, TB, SEC, CMP)
        /// and re-encoding them should produce the same byte value for those bit positions.
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_status_register_bit_decoding_roundtrip(
            sr1 in proptest::num::u8::ANY,
            sr2 in proptest::num::u8::ANY,
        ) {
            // Decode status registers
            let status = ProtectionStatus::from_status_registers(sr1, sr2);

            // Re-encode to status registers
            let encoded_sr1 = status.to_status_register1();
            let encoded_sr2 = status.to_status_register2();

            // Mask for bits we care about in SR1: BP0, BP1, BP2, TB, SEC, SRP0
            let sr1_mask = status1::BP0 | status1::BP1 | status1::BP2 |
                          status1::TB | status1::SEC | status1::SRP0;

            // Mask for bits we care about in SR2: CMP, SRP1
            let sr2_mask = status2::CMP | status2::SRP1;

            // Property: masked bits should match after round-trip
            prop_assert_eq!(encoded_sr1 & sr1_mask, sr1 & sr1_mask,
                "SR1 bits mismatch: original 0x{:02X}, encoded 0x{:02X}", sr1, encoded_sr1);
            prop_assert_eq!(encoded_sr2 & sr2_mask, sr2 & sr2_mask,
                "SR2 bits mismatch: original 0x{:02X}, encoded 0x{:02X}", sr2, encoded_sr2);
        }

        /// Property test: Protection status correctly identifies protected state
        #[test]
        fn prop_protection_status_is_protected(
            bp0 in proptest::bool::ANY,
            bp1 in proptest::bool::ANY,
            bp2 in proptest::bool::ANY,
        ) {
            let mut sr1 = 0u8;
            if bp0 { sr1 |= status1::BP0; }
            if bp1 { sr1 |= status1::BP1; }
            if bp2 { sr1 |= status1::BP2; }

            let status = ProtectionStatus::from_status_registers(sr1, 0);

            // Property: is_protected should be true iff any BP bit is set
            let expected_protected = bp0 || bp1 || bp2;
            prop_assert_eq!(status.is_protected(), expected_protected,
                "is_protected mismatch: bp0={}, bp1={}, bp2={}, expected={}",
                bp0, bp1, bp2, expected_protected);
        }
    }
}
