//! Mock device for testing without real hardware

use crate::device::{ChipInfo, DeviceInfo, FlashInterface};
use openflash_core::protocol::Command;
use std::sync::atomic::{AtomicBool, Ordering};

static MOCK_ENABLED: AtomicBool = AtomicBool::new(false);
static MOCK_CONNECTED: AtomicBool = AtomicBool::new(false);

/// Enable mock device mode
pub fn enable_mock() {
    MOCK_ENABLED.store(true, Ordering::SeqCst);
}

/// Check if mock mode is enabled
pub fn is_mock_enabled() -> bool {
    MOCK_ENABLED.load(Ordering::SeqCst)
}

/// Get mock device list
pub fn get_mock_devices() -> Vec<DeviceInfo> {
    if !is_mock_enabled() {
        return vec![];
    }

    vec![DeviceInfo {
        id: "mock:0000:0001".to_string(),
        name: "OpenFlash Mock Device".to_string(),
        serial: Some("MOCK-001".to_string()),
        connected: MOCK_CONNECTED.load(Ordering::SeqCst),
    }]
}

/// Connect to mock device
pub fn mock_connect(device_id: &str) -> Result<(), String> {
    if device_id.starts_with("mock:") {
        MOCK_CONNECTED.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err("Not a mock device".to_string())
    }
}

/// Disconnect mock device
pub fn mock_disconnect() {
    MOCK_CONNECTED.store(false, Ordering::SeqCst);
}

/// Check if mock is connected
pub fn is_mock_connected() -> bool {
    MOCK_CONNECTED.load(Ordering::SeqCst)
}

/// Get mock chip info
pub fn get_mock_chip_info() -> ChipInfo {
    ChipInfo {
        manufacturer: "Samsung".to_string(),
        model: "K9F4G08U0D (Mock)".to_string(),
        chip_id: vec![0xEC, 0xDC, 0x10, 0x95, 0x54],
        size_mb: 512,
        page_size: 2048,
        block_size: 64,
        interface: FlashInterface::ParallelNand,
    }
}

/// Process mock command
pub fn process_mock_command(cmd: Command, args: &[u8]) -> Vec<u8> {
    match cmd {
        Command::Ping => vec![0x01, 0x00], // OK
        
        Command::NandReadId => {
            // Return Samsung K9F4G08U0D ID
            vec![0x14, 0x00, 0xEC, 0xDC, 0x10, 0x95, 0x54]
        }
        
        Command::SpiNandReadId => {
            // Return GigaDevice GD5F1GQ4 ID
            vec![0x20, 0x00, 0xC8, 0xD1, 0x00]
        }
        
        Command::Reset => vec![0x08, 0x00],
        
        Command::BusConfig => vec![0x02, 0x00],
        
        Command::NandCmd => vec![0x10, 0x00],
        
        Command::NandAddr => vec![0x11, 0x00],
        
        Command::NandReadPage | Command::SpiNandReadCache => {
            // Generate mock page data
            if args.len() >= 6 {
                let page_addr = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let page_size = u16::from_le_bytes([args[4], args[5]]) as usize;
                generate_mock_page(page_addr, page_size)
            } else {
                vec![0x12, 0x01] // Error
            }
        }
        
        Command::NandWritePage | Command::SpiNandProgramExec => vec![0x13, 0x00],
        
        Command::SpiNandReset => vec![0x21, 0x00],
        Command::SpiNandGetFeature => vec![0x22, 0x00, 0x00], // Feature value = 0
        Command::SpiNandSetFeature => vec![0x23, 0x00],
        Command::SpiNandPageRead => vec![0x24, 0x00],
        Command::SpiNandProgramLoad | Command::SpiNandProgramLoadX4 => vec![0x27, 0x00],
        Command::SpiNandBlockErase => vec![0x2A, 0x00],
        Command::SpiNandWriteEnable => vec![0x2B, 0x00],
        Command::SpiNandWriteDisable => vec![0x2C, 0x00],
        
        _ => vec![0x00, 0x01], // Unknown command error
    }
}

/// Generate mock page data with realistic patterns
fn generate_mock_page(page_addr: u32, size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // First few pages contain "bootloader" pattern
    if page_addr < 64 {
        // U-Boot header at page 0
        if page_addr == 0 {
            data.extend_from_slice(&[0x27, 0x05, 0x19, 0x56]); // U-Boot magic
            data.extend_from_slice(b"U-Boot 2024.01 for OpenFlash\0");
            while data.len() < size {
                data.push(0x00);
            }
        } else {
            // Bootloader code (random-ish but deterministic)
            for i in 0..size {
                let val = ((page_addr as usize * 256 + i) % 256) as u8;
                data.push(val ^ 0x5A);
            }
        }
    }
    // Pages 64-128 contain SquashFS
    else if page_addr >= 64 && page_addr < 128 {
        if page_addr == 64 {
            data.extend_from_slice(b"hsqs"); // SquashFS magic (little-endian)
            data.extend_from_slice(&[0x00; 28]); // Header padding
            while data.len() < size {
                data.push(((page_addr + data.len() as u32) % 256) as u8);
            }
        } else {
            // Compressed data pattern
            for i in 0..size {
                data.push(((page_addr as usize + i) % 256) as u8);
            }
        }
    }
    // Pages 128-256 contain JFFS2
    else if page_addr >= 128 && page_addr < 256 {
        if page_addr == 128 {
            data.extend_from_slice(&[0x85, 0x19]); // JFFS2 magic (LE)
            data.extend_from_slice(&[0x04, 0x00]); // Node type
            while data.len() < size {
                data.push(0xFF);
            }
        } else {
            // JFFS2 data
            for i in 0..size {
                if i % 64 == 0 {
                    data.extend_from_slice(&[0x85, 0x19]);
                } else {
                    data.push(((page_addr as usize + i) % 256) as u8);
                }
            }
            data.truncate(size);
        }
    }
    // Empty pages (erased)
    else if page_addr >= 256 && page_addr < 1024 {
        data.resize(size, 0xFF);
    }
    // Bad block simulation at block 16 (pages 1024-1087)
    else if page_addr >= 1024 && page_addr < 1088 {
        // Bad block marker
        data.push(0x00);
        data.push(0x00);
        data.resize(size, 0xFF);
    }
    // Rest is random data
    else {
        for i in 0..size {
            let seed = page_addr.wrapping_mul(31).wrapping_add(i as u32);
            data.push((seed % 256) as u8);
        }
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_page_generation() {
        let page = generate_mock_page(0, 2048);
        assert_eq!(page.len(), 2048);
        // Check U-Boot magic
        assert_eq!(&page[0..4], &[0x27, 0x05, 0x19, 0x56]);
    }

    #[test]
    fn test_mock_squashfs() {
        let page = generate_mock_page(64, 2048);
        assert_eq!(&page[0..4], b"hsqs");
    }

    #[test]
    fn test_mock_empty_page() {
        let page = generate_mock_page(300, 2048);
        assert!(page.iter().all(|&b| b == 0xFF));
    }
}
