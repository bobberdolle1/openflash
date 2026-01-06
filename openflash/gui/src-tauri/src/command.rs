//! Tauri IPC commands for OpenFlash GUI

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::config::AppConfig;
use crate::device::{ChipInfo, DeviceInfo, DeviceManager, FlashInterface};
use crate::mock;

#[tauri::command]
pub fn enable_mock_mode() -> Result<(), String> {
    mock::enable_mock();
    Ok(())
}

#[tauri::command]
pub fn scan_devices(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<DeviceInfo>, String> {
    let mut manager = device_manager.lock().map_err(|e| e.to_string())?;
    let mut devices = manager.scan_devices();
    
    // Add mock devices if enabled
    devices.extend(mock::get_mock_devices());
    
    Ok(devices)
}

#[tauri::command]
pub fn list_devices(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<DeviceInfo>, String> {
    let manager = device_manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.list_devices())
}

#[tauri::command]
pub fn connect_device(
    device_id: String,
    device_manager: State<'_, Mutex<DeviceManager>>,
    config: State<'_, Mutex<AppConfig>>,
) -> Result<(), String> {
    // Save last device
    if let Ok(mut cfg) = config.lock() {
        cfg.last_device = Some(device_id.clone());
        let _ = cfg.save();
    }
    
    // Check if it's a mock device
    if device_id.starts_with("mock:") {
        return mock::mock_connect(&device_id);
    }
    
    let mut manager = device_manager.lock().map_err(|e| e.to_string())?;
    manager.connect(&device_id)
}

#[tauri::command]
pub fn disconnect_device(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<(), String> {
    if mock::is_mock_connected() {
        mock::mock_disconnect();
        return Ok(());
    }
    
    let mut manager = device_manager.lock().map_err(|e| e.to_string())?;
    manager.disconnect();
    Ok(())
}

#[tauri::command]
pub async fn ping(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<bool, String> {
    if mock::is_mock_connected() {
        let response = mock::process_mock_command(openflash_core::protocol::Command::Ping, &[]);
        return Ok(response.len() >= 2 && response[0] == 0x01 && response[1] == 0x00);
    }
    
    let device = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_active_device().ok_or("No device connected")?
    };

    let dev = device.lock().await;
    let response = dev.send_command(openflash_core::protocol::Command::Ping, &[]).await?;
    Ok(response.len() >= 2 && response[0] == 0x01 && response[1] == 0x00)
}

#[tauri::command]
pub async fn read_nand_id(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<u8>, String> {
    if mock::is_mock_connected() {
        let response = mock::process_mock_command(openflash_core::protocol::Command::NandReadId, &[]);
        if response.len() >= 7 && response[1] == 0x00 {
            return Ok(response[2..7].to_vec());
        }
        return Err("Mock read ID failed".to_string());
    }
    
    let device = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_active_device().ok_or("No device connected")?
    };

    let dev = device.lock().await;
    let response = dev.send_command(openflash_core::protocol::Command::NandReadId, &[]).await?;

    if response.len() >= 7 && response[1] == 0x00 {
        Ok(response[2..7].to_vec())
    } else {
        Err("Failed to read chip ID".to_string())
    }
}

#[tauri::command]
pub async fn get_chip_info(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<ChipInfo, String> {
    if mock::is_mock_connected() {
        return Ok(mock::get_mock_chip_info());
    }
    
    let chip_id = read_nand_id(device_manager.clone()).await?;
    
    // Check current interface mode
    let interface = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_interface()
    };
    
    match interface {
        FlashInterface::ParallelNand => {
            if let Some(info) = openflash_core::onfi::get_chip_info(&chip_id) {
                Ok(ChipInfo {
                    manufacturer: info.manufacturer,
                    model: info.model,
                    chip_id,
                    size_mb: info.size_mb,
                    page_size: info.page_size,
                    block_size: info.block_size,
                    interface: FlashInterface::ParallelNand,
                })
            } else {
                Ok(ChipInfo {
                    manufacturer: format!("Unknown (0x{:02X})", chip_id.first().unwrap_or(&0)),
                    model: "Unknown".to_string(),
                    chip_id,
                    size_mb: 0,
                    page_size: 2048,
                    block_size: 64,
                    interface: FlashInterface::ParallelNand,
                })
            }
        }
        FlashInterface::SpiNand => {
            if let Some(info) = openflash_core::spi_nand::get_spi_nand_chip_info(&chip_id) {
                Ok(ChipInfo {
                    manufacturer: info.manufacturer,
                    model: info.model,
                    chip_id,
                    size_mb: info.size_mb,
                    page_size: info.page_size,
                    block_size: info.block_size,
                    interface: FlashInterface::SpiNand,
                })
            } else {
                Ok(ChipInfo {
                    manufacturer: format!("Unknown (0x{:02X})", chip_id.first().unwrap_or(&0)),
                    model: "Unknown SPI NAND".to_string(),
                    chip_id,
                    size_mb: 0,
                    page_size: 2048,
                    block_size: 64,
                    interface: FlashInterface::SpiNand,
                })
            }
        }
    }
}

/// Set the flash interface mode (Parallel NAND or SPI NAND)
#[tauri::command]
pub fn set_interface(
    interface: FlashInterface,
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<(), String> {
    let mut manager = device_manager.lock().map_err(|e| e.to_string())?;
    manager.set_interface(interface);
    Ok(())
}

/// Get the current flash interface mode
#[tauri::command]
pub fn get_interface(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<FlashInterface, String> {
    let manager = device_manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_interface())
}

/// Read SPI NAND chip ID
#[tauri::command]
pub async fn read_spi_nand_id(
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<u8>, String> {
    if mock::is_mock_connected() {
        // Return mock SPI NAND ID (GigaDevice GD5F1GQ4)
        return Ok(vec![0xC8, 0xD1, 0x00]);
    }
    
    let device = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_active_device().ok_or("No device connected")?
    };

    let dev = device.lock().await;
    let response = dev.send_command(openflash_core::protocol::Command::SpiNandReadId, &[]).await?;

    if response.len() >= 5 && response[1] == 0x00 {
        Ok(response[2..5].to_vec())
    } else {
        Err("Failed to read SPI NAND chip ID".to_string())
    }
}

#[tauri::command]
pub async fn dump_nand(
    start_page: u32,
    num_pages: u32,
    page_size: u16,
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<u8>, String> {
    if mock::is_mock_connected() {
        let mut data = Vec::with_capacity((num_pages as usize) * (page_size as usize));
        for page in start_page..(start_page + num_pages) {
            let mut args = [0u8; 6];
            args[0..4].copy_from_slice(&page.to_le_bytes());
            args[4..6].copy_from_slice(&page_size.to_le_bytes());
            let page_data = mock::process_mock_command(
                openflash_core::protocol::Command::NandReadPage,
                &args,
            );
            data.extend(page_data);
        }
        return Ok(data);
    }
    
    let device = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_active_device().ok_or("No device connected")?
    };

    let dev = device.lock().await;
    let mut data = Vec::with_capacity((num_pages as usize) * (page_size as usize));

    for page in start_page..(start_page + num_pages) {
        let page_data = dev.read_page(page, page_size).await?;
        data.extend(page_data);
    }

    Ok(data)
}

#[tauri::command]
pub fn analyze_dump(
    data: Vec<u8>,
    page_size: u32,
    pages_per_block: u32,
) -> Result<AnalysisResult, String> {
    let analyzer = openflash_core::analysis::Analyzer::new(
        page_size as usize,
        pages_per_block as usize,
    );
    
    let result = analyzer.analyze_dump(&data);
    
    Ok(AnalysisResult {
        filesystem_type: result.filesystem_type,
        signatures: result.signatures_found.into_iter().map(|s| SignatureInfo {
            name: s.name,
            offset: s.offset,
            confidence: s.confidence,
        }).collect(),
        bad_blocks: result.bad_blocks,
        empty_pages: result.empty_pages,
        data_pages: result.data_pages,
    })
}

#[derive(Serialize, Deserialize)]
pub struct AnalysisResult {
    pub filesystem_type: Option<String>,
    pub signatures: Vec<SignatureInfo>,
    pub bad_blocks: Vec<u32>,
    pub empty_pages: u32,
    pub data_pages: u32,
}

#[derive(Serialize, Deserialize)]
pub struct SignatureInfo {
    pub name: String,
    pub offset: usize,
    pub confidence: f32,
}

// ============================================================================
// Progress-enabled dump command
// ============================================================================

#[derive(Clone, Serialize)]
pub struct DumpProgress {
    pub current_page: u32,
    pub total_pages: u32,
    pub percent: u8,
    pub bytes_read: usize,
}

#[tauri::command]
pub async fn dump_nand_with_progress(
    app: AppHandle,
    start_page: u32,
    num_pages: u32,
    page_size: u16,
    device_manager: State<'_, Mutex<DeviceManager>>,
) -> Result<Vec<u8>, String> {
    let chunk_size = 64u32; // Pages per progress update
    let mut data = Vec::with_capacity((num_pages as usize) * (page_size as usize));
    
    if mock::is_mock_connected() {
        for page in start_page..(start_page + num_pages) {
            let mut args = [0u8; 6];
            args[0..4].copy_from_slice(&page.to_le_bytes());
            args[4..6].copy_from_slice(&page_size.to_le_bytes());
            let page_data = mock::process_mock_command(
                openflash_core::protocol::Command::NandReadPage,
                &args,
            );
            data.extend(page_data);
            
            // Emit progress every chunk_size pages
            if (page - start_page) % chunk_size == 0 || page == start_page + num_pages - 1 {
                let progress = DumpProgress {
                    current_page: page - start_page + 1,
                    total_pages: num_pages,
                    percent: (((page - start_page + 1) as f32 / num_pages as f32) * 100.0) as u8,
                    bytes_read: data.len(),
                };
                let _ = app.emit("dump-progress", progress);
            }
        }
        return Ok(data);
    }
    
    let device = {
        let manager = device_manager.lock().map_err(|e| e.to_string())?;
        manager.get_active_device().ok_or("No device connected")?
    };

    let dev = device.lock().await;

    for page in start_page..(start_page + num_pages) {
        let page_data = dev.read_page(page, page_size).await?;
        data.extend(page_data);
        
        // Emit progress
        if (page - start_page) % chunk_size == 0 || page == start_page + num_pages - 1 {
            let progress = DumpProgress {
                current_page: page - start_page + 1,
                total_pages: num_pages,
                percent: (((page - start_page + 1) as f32 / num_pages as f32) * 100.0) as u8,
                bytes_read: data.len(),
            };
            let _ = app.emit("dump-progress", progress);
        }
    }

    Ok(data)
}

// ============================================================================
// Configuration commands
// ============================================================================

#[tauri::command]
pub fn get_config(config: State<'_, Mutex<AppConfig>>) -> Result<AppConfig, String> {
    let cfg = config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

#[tauri::command]
pub fn set_config(
    new_config: AppConfig,
    config: State<'_, Mutex<AppConfig>>,
) -> Result<(), String> {
    let mut cfg = config.lock().map_err(|e| e.to_string())?;
    *cfg = new_config;
    cfg.save()
}

#[tauri::command]
pub fn add_recent_file(
    path: String,
    config: State<'_, Mutex<AppConfig>>,
) -> Result<(), String> {
    let mut cfg = config.lock().map_err(|e| e.to_string())?;
    cfg.add_recent_file(&path);
    cfg.save()
}
