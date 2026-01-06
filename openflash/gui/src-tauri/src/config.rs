//! Configuration persistence for OpenFlash

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Last used device ID
    pub last_device: Option<String>,
    /// Default dump directory
    pub dump_directory: Option<String>,
    /// ECC algorithm preference
    pub ecc_algorithm: String,
    /// Auto-analyze after dump
    pub auto_analyze: bool,
    /// Page size override (0 = auto-detect)
    pub page_size_override: u32,
    /// Include OOB in dumps
    pub include_oob: bool,
    /// Recent files
    pub recent_files: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_device: None,
            dump_directory: None,
            ecc_algorithm: "none".to_string(),
            auto_analyze: true,
            page_size_override: 0,
            include_oob: false,
            recent_files: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Get config file path
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("openflash").join("config.json"))
    }

    /// Load config from disk
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config path")?;
        
        // Create directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Add recent file
    pub fn add_recent_file(&mut self, path: &str) {
        // Remove if already exists
        self.recent_files.retain(|p| p != path);
        // Add to front
        self.recent_files.insert(0, path.to_string());
        // Keep only last 10
        self.recent_files.truncate(10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(config.auto_analyze);
        assert_eq!(config.ecc_algorithm, "none");
    }

    #[test]
    fn test_recent_files() {
        let mut config = AppConfig::default();
        config.add_recent_file("/path/to/file1.bin");
        config.add_recent_file("/path/to/file2.bin");
        config.add_recent_file("/path/to/file1.bin"); // Duplicate
        
        assert_eq!(config.recent_files.len(), 2);
        assert_eq!(config.recent_files[0], "/path/to/file1.bin");
    }
}
