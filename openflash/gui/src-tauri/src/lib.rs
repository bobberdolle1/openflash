//! OpenFlash Tauri Backend

use std::sync::Mutex;

mod command;
mod config;
mod device;
mod flasher;
mod mock;

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
            command::enable_mock_mode,
            command::scan_devices,
            command::list_devices,
            command::connect_device,
            command::disconnect_device,
            command::ping,
            command::read_nand_id,
            command::get_chip_info,
            command::dump_nand,
            command::dump_nand_with_progress,
            command::analyze_dump,
            command::get_config,
            command::set_config,
            command::add_recent_file,
            // SPI NAND commands
            command::set_interface,
            command::get_interface,
            command::read_spi_nand_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
