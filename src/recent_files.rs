//! Recent files management for irohscii
//!
//! Tracks recently opened/saved files and persists them to disk.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 10;

/// A recently accessed file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: PathBuf,
    pub name: String,
}

/// Manager for recent files
#[derive(Debug, Default)]
pub struct RecentFiles {
    files: Vec<RecentFile>,
    config_path: PathBuf,
}

impl RecentFiles {
    /// Load recent files from config directory
    pub fn load() -> Self {
        let config_path = Self::config_path();
        let files = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self { files, config_path }
    }

    /// Get the config file path
    fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            });
        config_dir.join("irohscii").join("recent.json")
    }

    /// Save recent files to disk
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.files)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Add a file to recent files (moves to front if already exists)
    pub fn add(&mut self, path: PathBuf) {
        // Get display name from file name
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // Remove if already exists
        self.files.retain(|f| f.path != path);

        // Add to front
        self.files.insert(0, RecentFile { path, name });

        // Trim to max size
        self.files.truncate(MAX_RECENT_FILES);
    }

    /// Get a file by index
    pub fn get(&self, index: usize) -> Option<&RecentFile> {
        self.files.get(index)
    }

    /// Iterate over recent files
    pub fn iter(&self) -> impl Iterator<Item = &RecentFile> {
        self.files.iter()
    }

    /// Get the number of recent files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if there are no recent files
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
