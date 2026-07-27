//! OpenFlash Tauri Backend

use std::sync::Mutex;

mod command;
mod config;
mod device;
mod flasher;

use config::AppConfig;
use device::DeviceManager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(DeviceManager::new()))
        .manage(Mutex::new(AppConfig::load()))
        .setup(|_app| {
            #[cfg(debug_assertions)]
            {
                // DevTools disabled for now
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command::scan_devices,
            command::list_devices,
            command::connect_device,
            command::disconnect_device,
            command::ping,
            command::read_nand_id,
            command::get_chip_info,
            command::is_connected,
            command::identify_chip,
            command::chip_capacity,
            command::program_chip,
            command::erase_chip_range,
            command::verify_chip,
            command::process_dump_with_ecc,
            command::dump_is_clean,
            command::extract_data_only,
            command::dump_statistics,
            command::dump_range,
            command::dump_range_with_progress,
            command::analyze_dump,
            command::get_config,
            command::set_config,
            command::add_recent_file,
            // SPI NAND commands
            command::set_interface,
            command::get_interface,
            command::read_spi_nand_id,
            // SPI NOR commands (v1.6)
            command::read_spi_nor_jedec_id,
            command::spi_nor_sector_erase,
            command::spi_nor_block_erase,
            command::spi_nor_chip_erase,
            command::spi_nor_unlock_all,
            // UFS commands (v1.6)
            command::read_ufs_device_info,
            command::ufs_select_lun,
            // AI commands (v1.3)
            command::ai_analyze_dump,
            command::ai_detect_patterns,
            command::ai_get_recommendations,
            command::ai_compare_dumps,
            command::ai_search_keys,
            command::ai_generate_report,
            // Platform commands (v2.3)
            command::get_device_info,
            command::get_platform_info,
            command::connect_network_device,
            #[cfg(unix)]
            command::connect_unix_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
