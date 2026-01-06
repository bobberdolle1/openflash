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
// AI Analysis commands (v1.4)
// ============================================================================

#[derive(Serialize, Deserialize)]
pub struct AiAnalysisResponse {
    pub patterns: Vec<PatternInfo>,
    pub anomalies: Vec<AnomalyInfo>,
    pub recovery_suggestions: Vec<RecoverySuggestionInfo>,
    pub chip_recommendations: Vec<ChipRecommendationInfo>,
    pub data_quality_score: f32,
    pub encryption_probability: f32,
    pub compression_probability: f32,
    pub summary: String,
    // v1.4 additions
    pub filesystems: Vec<FilesystemInfo>,
    pub oob_analysis: Option<OobAnalysisInfo>,
    pub key_candidates: Vec<KeyCandidateInfo>,
    pub wear_analysis: Option<WearAnalysisInfo>,
    pub memory_map: Option<MemoryMapInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct PatternInfo {
    pub pattern_type: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub confidence: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnomalyInfo {
    pub severity: String,
    pub location: Option<usize>,
    pub description: String,
    pub recommendation: String,
}

#[derive(Serialize, Deserialize)]
pub struct RecoverySuggestionInfo {
    pub priority: u8,
    pub action: String,
    pub description: String,
    pub estimated_success: f32,
}

#[derive(Serialize, Deserialize)]
pub struct ChipRecommendationInfo {
    pub category: String,
    pub title: String,
    pub description: String,
    pub importance: u8,
}

// v1.4 new structs
#[derive(Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub fs_type: String,
    pub offset: usize,
    pub size: Option<usize>,
    pub confidence: String,
}

#[derive(Serialize, Deserialize)]
pub struct OobAnalysisInfo {
    pub oob_size: usize,
    pub ecc_scheme: String,
    pub ecc_offset: usize,
    pub ecc_size: usize,
    pub bad_block_marker_offset: usize,
    pub confidence: String,
}

#[derive(Serialize, Deserialize)]
pub struct KeyCandidateInfo {
    pub offset: usize,
    pub key_type: String,
    pub key_length: usize,
    pub entropy: f64,
    pub confidence: String,
    pub context: String,
}

#[derive(Serialize, Deserialize)]
pub struct WearAnalysisInfo {
    pub hottest_blocks: Vec<usize>,
    pub coldest_blocks: Vec<usize>,
    pub min_erases: u32,
    pub max_erases: u32,
    pub avg_erases: f32,
    pub estimated_remaining_life_percent: f32,
    pub recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MemoryMapInfo {
    pub total_size: usize,
    pub regions: Vec<MemoryMapRegionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct MemoryMapRegionInfo {
    pub start: usize,
    pub end: usize,
    pub region_type: String,
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Deserialize)]
pub struct DumpDiffResponse {
    pub total_differences: usize,
    pub changed_pages: Vec<usize>,
    pub changed_blocks: Vec<usize>,
    pub similarity_percent: f32,
    pub modified_regions: Vec<DiffRegionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct DiffRegionInfo {
    pub offset: usize,
    pub size: usize,
    pub change_type: String,
    pub description: String,
}

/// AI-powered analysis of dump data (v1.4)
#[tauri::command]
pub fn ai_analyze_dump(
    data: Vec<u8>,
    page_size: u32,
    pages_per_block: u32,
) -> Result<AiAnalysisResponse, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(
        page_size as usize,
        pages_per_block as usize,
    );
    
    let result = analyzer.analyze(&data);
    
    Ok(AiAnalysisResponse {
        patterns: result.patterns.into_iter().map(|p| PatternInfo {
            pattern_type: format!("{:?}", p.pattern_type),
            start_offset: p.start_offset,
            end_offset: p.end_offset,
            confidence: format!("{:?}", p.confidence),
            description: p.description,
        }).collect(),
        anomalies: result.anomalies.into_iter().map(|a| AnomalyInfo {
            severity: format!("{:?}", a.severity),
            location: a.location,
            description: a.description,
            recommendation: a.recommendation,
        }).collect(),
        recovery_suggestions: result.recovery_suggestions.into_iter().map(|s| RecoverySuggestionInfo {
            priority: s.priority,
            action: s.action,
            description: s.description,
            estimated_success: s.estimated_success,
        }).collect(),
        chip_recommendations: result.chip_recommendations.into_iter().map(|r| ChipRecommendationInfo {
            category: r.category,
            title: r.title,
            description: r.description,
            importance: r.importance,
        }).collect(),
        data_quality_score: result.data_quality_score,
        encryption_probability: result.encryption_probability,
        compression_probability: result.compression_probability,
        summary: result.summary,
        // v1.4 additions
        filesystems: result.filesystems.into_iter().map(|f| FilesystemInfo {
            fs_type: f.fs_type.name().to_string(),
            offset: f.offset,
            size: f.size,
            confidence: format!("{:?}", f.confidence),
        }).collect(),
        oob_analysis: result.oob_analysis.map(|o| OobAnalysisInfo {
            oob_size: o.oob_size,
            ecc_scheme: format!("{:?}", o.ecc_scheme),
            ecc_offset: o.ecc_offset,
            ecc_size: o.ecc_size,
            bad_block_marker_offset: o.bad_block_marker_offset,
            confidence: format!("{:?}", o.confidence),
        }),
        key_candidates: result.key_candidates.into_iter().map(|k| KeyCandidateInfo {
            offset: k.offset,
            key_type: k.key_type,
            key_length: k.key_length,
            entropy: k.entropy,
            confidence: format!("{:?}", k.confidence),
            context: k.context,
        }).collect(),
        wear_analysis: result.wear_analysis.map(|w| WearAnalysisInfo {
            hottest_blocks: w.hottest_blocks,
            coldest_blocks: w.coldest_blocks,
            min_erases: w.wear_distribution.min_erases,
            max_erases: w.wear_distribution.max_erases,
            avg_erases: w.wear_distribution.avg_erases,
            estimated_remaining_life_percent: w.estimated_remaining_life_percent,
            recommendations: w.recommendations,
        }),
        memory_map: result.memory_map.map(|m| MemoryMapInfo {
            total_size: m.total_size,
            regions: m.regions.into_iter().map(|r| MemoryMapRegionInfo {
                start: r.start,
                end: r.end,
                region_type: r.region_type,
                name: r.name,
                color: r.color,
            }).collect(),
        }),
    })
}

/// Quick AI pattern detection (lighter analysis)
#[tauri::command]
pub fn ai_detect_patterns(
    data: Vec<u8>,
    page_size: u32,
) -> Result<Vec<PatternInfo>, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(page_size as usize, 64);
    let patterns = analyzer.detect_patterns(&data);
    
    Ok(patterns.into_iter().map(|p| PatternInfo {
        pattern_type: format!("{:?}", p.pattern_type),
        start_offset: p.start_offset,
        end_offset: p.end_offset,
        confidence: format!("{:?}", p.confidence),
        description: p.description,
    }).collect())
}

/// Get AI-powered chip recommendations
#[tauri::command]
pub fn ai_get_recommendations(
    data: Vec<u8>,
    page_size: u32,
    pages_per_block: u32,
) -> Result<Vec<ChipRecommendationInfo>, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(
        page_size as usize,
        pages_per_block as usize,
    );
    
    let patterns = analyzer.detect_patterns(&data);
    let recommendations = analyzer.generate_chip_recommendations(&data, &patterns);
    
    Ok(recommendations.into_iter().map(|r| ChipRecommendationInfo {
        category: r.category,
        title: r.title,
        description: r.description,
        importance: r.importance,
    }).collect())
}

/// v1.4: Compare two dumps
#[tauri::command]
pub fn ai_compare_dumps(
    dump1: Vec<u8>,
    dump2: Vec<u8>,
    page_size: u32,
    pages_per_block: u32,
) -> Result<DumpDiffResponse, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(
        page_size as usize,
        pages_per_block as usize,
    );
    
    let diff = analyzer.compare_dumps(&dump1, &dump2);
    
    Ok(DumpDiffResponse {
        total_differences: diff.total_differences,
        changed_pages: diff.changed_pages,
        changed_blocks: diff.changed_blocks,
        similarity_percent: diff.similarity_percent,
        modified_regions: diff.modified_regions.into_iter().map(|r| DiffRegionInfo {
            offset: r.offset,
            size: r.size,
            change_type: format!("{:?}", r.change_type),
            description: r.description,
        }).collect(),
    })
}

/// v1.4: Deep scan for encryption keys
#[tauri::command]
pub fn ai_search_keys(
    data: Vec<u8>,
    page_size: u32,
) -> Result<Vec<KeyCandidateInfo>, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(page_size as usize, 64)
        .with_deep_scan(true);
    
    let keys = analyzer.search_encryption_keys(&data);
    
    Ok(keys.into_iter().map(|k| KeyCandidateInfo {
        offset: k.offset,
        key_type: k.key_type,
        key_length: k.key_length,
        entropy: k.entropy,
        confidence: format!("{:?}", k.confidence),
        context: k.context,
    }).collect())
}

/// v1.4: Generate AI analysis report
#[tauri::command]
pub fn ai_generate_report(
    data: Vec<u8>,
    page_size: u32,
    pages_per_block: u32,
) -> Result<String, String> {
    let analyzer = openflash_core::ai::AiAnalyzer::new(
        page_size as usize,
        pages_per_block as usize,
    );
    
    let result = analyzer.analyze(&data);
    let report = analyzer.generate_report(&result);
    
    Ok(report)
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
