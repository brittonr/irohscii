//! Recent files management for irohscii
//!
//! Tracks recently opened/saved files and persists them to disk.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Maximum number of recent files to track
const MAX_RECENT_FILES: u32 = 10;

// Compile-time assertion for MAX_RECENT_FILES
const _: () = assert!(MAX_RECENT_FILES > 0, "must track at least one recent file");
const _: () = assert!(MAX_RECENT_FILES <= 100, "recent files limit should be reasonable");

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
                .map(|content| {
                    serde_json::from_str(&content)
                        .unwrap_or_else(|e| {
                            eprintln!("Warning: Failed to parse recent files: {}", e);
                            Vec::new()
                        })
                })
                .unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to read recent files: {}", e);
                    Vec::new()
                })
        } else {
            Vec::new()
        };

        debug_assert!(files.len() <= MAX_RECENT_FILES as usize, 
                      "postcondition: recent files count is within limit");

        Self { files, config_path }
    }

    /// Get the config file path
    fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("home directory should be available")
                    .join(".config")
            });
        let path = config_dir.join("irohscii").join("recent.json");
        
        debug_assert!(path.as_os_str().len() > 0, "postcondition: config path is non-empty");
        
        path
    }

    /// Save recent files to disk
    pub fn save(&self) -> Result<()> {
        debug_assert!(self.files.len() <= MAX_RECENT_FILES as usize, 
                      "precondition: recent files count is within limit");
        
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.files)?;
        fs::write(&self.config_path, content)?;
        
        debug_assert!(self.config_path.exists(), "postcondition: config file was created");
        
        Ok(())
    }

    /// Add a file to recent files (moves to front if already exists)
    pub fn add(&mut self, path: PathBuf) {
        debug_assert!(path.as_os_str().len() > 0, "precondition: path is non-empty");
        
        // Get display name from file name
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        debug_assert!(!name.is_empty(), "postcondition: display name is non-empty");

        // Remove if already exists
        self.files.retain(|f| f.path != path);

        // Add to front
        self.files.insert(0, RecentFile { path, name });

        // Trim to max size
        self.files.truncate(MAX_RECENT_FILES as usize);
        
        debug_assert!(self.files.len() <= MAX_RECENT_FILES as usize, 
                      "postcondition: recent files count is within limit");
        debug_assert!(!self.files.is_empty(), "postcondition: at least one file in recent list");
    }

    /// Get a file by index
    pub fn get(&self, index: usize) -> Option<&RecentFile> {
        debug_assert!(index < MAX_RECENT_FILES as usize, 
                      "precondition: index is within reasonable bounds");
        
        let result = self.files.get(index);
        
        debug_assert!(result.is_none() || index < self.files.len(), 
                      "postcondition: result validity matches index bounds");
        
        result
    }

    /// Iterate over recent files
    pub fn iter(&self) -> impl Iterator<Item = &RecentFile> {
        self.files.iter()
    }

    /// Get the number of recent files
    pub fn len(&self) -> usize {
        let len = self.files.len();
        
        debug_assert!(len <= MAX_RECENT_FILES as usize, 
                      "postcondition: recent files count is within limit");
        
        len
    }

    /// Check if there are no recent files
    pub fn is_empty(&self) -> bool {
        let is_empty = self.files.is_empty();
        
        debug_assert!(is_empty == (self.files.len() == 0), 
                      "postcondition: is_empty matches len() == 0");
        
        is_empty
    }
}
