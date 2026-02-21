//! Undo/redo manager using serialized automerge document snapshots.
//!
//! Automerge doesn't support true rollback (it tracks all changes ever made),
//! so we use serialized document snapshots for undo/redo. This module supports
//! both memory-only and disk-backed storage for unlimited undo history.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::PathBuf;

use automerge::Automerge;

use irohscii_core::Document;

/// How many snapshots to keep in memory cache for disk-backed mode
pub(crate) const MEMORY_CACHE_SIZE: u32 = 20;

/// Manages undo/redo with document snapshots (memory or disk-backed)
#[allow(dead_code)]
pub struct UndoManager {
    /// Storage backend
    storage: UndoStorage,
    /// Redo stack (always in memory, usually small)
    redo_stack: Vec<Vec<u8>>,
}

/// Storage backend for undo history
enum UndoStorage {
    /// Memory-only storage with max history limit
    Memory {
        stack: Vec<Vec<u8>>,
        max_history: u32,
    },
    /// Disk-backed storage with memory cache for recent items
    Disk {
        /// Directory for snapshot files
        dir: PathBuf,
        /// Current stack size (number of snapshots)
        count: u32,
        /// In-memory cache of recent snapshots (index, bytes)
        cache: VecDeque<(u32, Vec<u8>)>,
    },
}

#[allow(dead_code)]
impl UndoManager {
    /// Create a new memory-only undo manager
    pub fn new(max_history: u32) -> Self {
        debug_assert!(max_history > 0, "max_history must be positive");
        
        Self {
            storage: UndoStorage::Memory {
                stack: Vec::new(),
                max_history,
            },
            redo_stack: Vec::new(),
        }
    }

    /// Create a disk-backed undo manager for unlimited history
    pub fn new_disk_backed(session_id: &str) -> io::Result<Self> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("irohscii")
            .join("undo")
            .join(session_id);

        fs::create_dir_all(&dir)?;

        debug_assert!(dir.exists(), "Undo directory should exist after creation");

        // Count existing snapshots
        let count = fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "snap"))
            .count();

        debug_assert!(count <= u32::MAX as usize, "Snapshot count should fit in u32");
        let count = count as u32;

        Ok(Self {
            storage: UndoStorage::Disk {
                dir,
                count,
                cache: VecDeque::with_capacity(MEMORY_CACHE_SIZE as usize),
            },
            redo_stack: Vec::new(),
        })
    }

    /// Save current state before mutation
    pub fn save_state(&mut self, doc: &Document) {
        let bytes = doc.automerge().save();
        debug_assert!(!bytes.is_empty(), "Serialized document should not be empty");

        match &mut self.storage {
            UndoStorage::Memory { stack, max_history } => {
                let prev_len = stack.len();
                stack.push(bytes);
                
                debug_assert_eq!(stack.len(), prev_len + 1, "Push should increase stack size by 1");
                
                // Limit history size
                while stack.len() > *max_history as usize {
                    stack.remove(0);
                }
                
                debug_assert!(
                    stack.len() <= *max_history as usize,
                    "Stack size should not exceed max_history"
                );
            }
            UndoStorage::Disk { dir, count, cache } => {
                // Write to disk
                let path = dir.join(format!("{:08}.snap", count));
                if let Err(e) = fs::write(&path, &bytes) {
                    eprintln!("ERROR: Failed to write undo snapshot to {:?}: {}", path, e);
                    eprintln!("       Undo history may be incomplete.");
                    return;
                }

                debug_assert!(path.exists(), "Snapshot file should exist after writing");

                // Add to cache
                if cache.len() >= MEMORY_CACHE_SIZE as usize {
                    cache.pop_front();
                }
                cache.push_back((*count, bytes));
                *count += 1;
                
                debug_assert!(
                    cache.len() <= MEMORY_CACHE_SIZE as usize,
                    "Cache size should not exceed MEMORY_CACHE_SIZE"
                );
            }
        }

        self.redo_stack.clear();
    }

    /// Undo to previous state, returns the previous automerge doc
    pub fn undo(&mut self, current_doc: &Document) -> Option<Automerge> {
        let prev_bytes = match &mut self.storage {
            UndoStorage::Memory { stack, .. } => stack.pop(),
            UndoStorage::Disk { dir, count, cache } => {
                if *count == 0 {
                    return None;
                }

                let target_idx = *count - 1;

                // Try cache first
                let bytes = if let Some(pos) = cache.iter().position(|(idx, _)| *idx == target_idx)
                {
                    let removed = cache.remove(pos);
                    debug_assert!(removed.is_some(), "Position exists but remove returned None");
                    let (_, b) = removed?;
                    b
                } else {
                    // Load from disk
                    let path = dir.join(format!("{:08}.snap", target_idx));
                    match fs::read(&path) {
                        Ok(b) => {
                            debug_assert!(!b.is_empty(), "Snapshot file should not be empty");
                            b
                        }
                        Err(e) => {
                            eprintln!("ERROR: Failed to read undo snapshot from {:?}: {}", path, e);
                            eprintln!("       Cannot undo to previous state.");
                            return None;
                        }
                    }
                };

                // Delete the file
                let path = dir.join(format!("{:08}.snap", target_idx));
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("WARNING: Failed to delete undo snapshot {:?}: {}", path, e);
                    // Continue anyway - snapshot was read successfully
                }
                *count -= 1;

                Some(bytes)
            }
        }?;

        debug_assert!(!prev_bytes.is_empty(), "Previous state bytes should not be empty");

        // Save current for redo
        let current_bytes = current_doc.automerge().save();
        debug_assert!(!current_bytes.is_empty(), "Current state bytes should not be empty");
        self.redo_stack.push(current_bytes);

        // Load previous state
        match Automerge::load(&prev_bytes) {
            Ok(doc) => Some(doc),
            Err(e) => {
                eprintln!("ERROR: Failed to deserialize undo snapshot: {}", e);
                eprintln!("       Undo snapshot may be corrupted.");
                None
            }
        }
    }

    /// Redo to next state, returns the next automerge doc
    pub fn redo(&mut self, current_doc: &Document) -> Option<Automerge> {
        let next_bytes = self.redo_stack.pop()?;
        debug_assert!(!next_bytes.is_empty(), "Redo stack entry should not be empty");

        // Save current for undo (don't clear redo stack here)
        let current_bytes = current_doc.automerge().save();
        debug_assert!(!current_bytes.is_empty(), "Current state bytes should not be empty");
        
        match &mut self.storage {
            UndoStorage::Memory { stack, max_history } => {
                stack.push(current_bytes);
                while stack.len() > *max_history as usize {
                    stack.remove(0);
                }
                debug_assert!(
                    stack.len() <= *max_history as usize,
                    "Stack size should not exceed max_history after redo"
                );
            }
            UndoStorage::Disk { dir, count, cache } => {
                let path = dir.join(format!("{:08}.snap", count));
                if let Err(e) = fs::write(&path, &current_bytes) {
                    eprintln!("ERROR: Failed to write undo snapshot to {:?}: {}", path, e);
                    eprintln!("       Redo may not be undoable.");
                    // Don't return early - we can still complete the redo operation
                } else {
                    debug_assert!(path.exists(), "Snapshot file should exist after writing");
                    
                    if cache.len() >= MEMORY_CACHE_SIZE as usize {
                        cache.pop_front();
                    }
                    cache.push_back((*count, current_bytes));
                    *count += 1;
                    
                    debug_assert!(
                        cache.len() <= MEMORY_CACHE_SIZE as usize,
                        "Cache size should not exceed MEMORY_CACHE_SIZE after redo"
                    );
                }
            }
        }

        // Load next state
        match Automerge::load(&next_bytes) {
            Ok(doc) => Some(doc),
            Err(e) => {
                eprintln!("ERROR: Failed to deserialize redo snapshot: {}", e);
                eprintln!("       Redo snapshot may be corrupted.");
                None
            }
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        let result = match &self.storage {
            UndoStorage::Memory { stack, .. } => !stack.is_empty(),
            UndoStorage::Disk { count, .. } => *count > 0,
        };
        debug_assert_eq!(
            result,
            self.undo_count() > 0,
            "can_undo should match undo_count > 0"
        );
        result
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        let result = !self.redo_stack.is_empty();
        debug_assert_eq!(
            result,
            self.redo_count() > 0,
            "can_redo should match redo_count > 0"
        );
        result
    }

    /// Clear all history
    pub fn clear(&mut self) {
        match &mut self.storage {
            UndoStorage::Memory { stack, .. } => {
                stack.clear();
                debug_assert!(stack.is_empty(), "Stack should be empty after clear");
            }
            UndoStorage::Disk { dir, count, cache } => {
                // Delete all snapshot files
                let dir_path = dir.clone();
                match fs::read_dir(&dir_path) {
                    Ok(entries) => {
                        for entry in entries {
                            match entry {
                                Ok(e) => {
                                    if e.path().extension().is_some_and(|ext| ext == "snap") {
                                        if let Err(err) = fs::remove_file(e.path()) {
                                            eprintln!(
                                                "WARNING: Failed to delete snapshot {:?}: {}",
                                                e.path(),
                                                err
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("WARNING: Failed to read directory entry: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("WARNING: Failed to read undo directory {:?}: {}", dir_path, e);
                    }
                }
                *count = 0;
                cache.clear();
                debug_assert_eq!(*count, 0, "Count should be 0 after clear");
                debug_assert!(cache.is_empty(), "Cache should be empty after clear");
            }
        }
        self.redo_stack.clear();
        debug_assert!(self.redo_stack.is_empty(), "Redo stack should be empty after clear");
        debug_assert!(!self.can_undo(), "Should not be able to undo after clear");
        debug_assert!(!self.can_redo(), "Should not be able to redo after clear");
    }

    /// Get the number of undo states available
    pub fn undo_count(&self) -> u32 {
        match &self.storage {
            UndoStorage::Memory { stack, .. } => {
                debug_assert!(stack.len() <= u32::MAX as usize, "Stack size should fit in u32");
                stack.len() as u32
            }
            UndoStorage::Disk { count, .. } => *count,
        }
    }

    /// Get the number of redo states available
    pub fn redo_count(&self) -> u32 {
        debug_assert!(
            self.redo_stack.len() <= u32::MAX as usize,
            "Redo stack size should fit in u32"
        );
        self.redo_stack.len() as u32
    }

    /// Check if this is a disk-backed manager
    pub fn is_disk_backed(&self) -> bool {
        matches!(self.storage, UndoStorage::Disk { .. })
    }

    /// Cleanup disk storage (call on session close if not saving)
    pub fn cleanup_disk_storage(&mut self) {
        if let UndoStorage::Disk { dir, count, cache } = &mut self.storage {
            let dir_path = dir.clone();
            
            // Delete all snapshot files
            match fs::read_dir(&dir_path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(e) => {
                                if let Err(err) = fs::remove_file(e.path()) {
                                    eprintln!(
                                        "WARNING: Failed to delete snapshot {:?} during cleanup: {}",
                                        e.path(),
                                        err
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("WARNING: Failed to read directory entry during cleanup: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to read undo directory {:?} during cleanup: {}", dir_path, e);
                }
            }
            
            // Try to remove the directory itself
            if let Err(e) = fs::remove_dir(&dir_path) {
                // This is expected to fail if directory is not empty or doesn't exist
                // Only log if it's an unexpected error
                if e.kind() != io::ErrorKind::NotFound {
                    eprintln!("INFO: Could not remove undo directory {:?}: {}", dir_path, e);
                }
            }
            
            *count = 0;
            cache.clear();
            
            debug_assert_eq!(*count, 0, "Count should be 0 after cleanup");
            debug_assert!(cache.is_empty(), "Cache should be empty after cleanup");
        }
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_manager_new() {
        let mgr = UndoManager::new(50);
        assert!(!mgr.can_undo());
        assert!(!mgr.can_redo());
        assert_eq!(mgr.undo_count(), 0);
        assert_eq!(mgr.redo_count(), 0);
    }

    #[test]
    fn undo_manager_default() {
        let mgr = UndoManager::default();
        assert_eq!(mgr.undo_count(), 0);
        assert_eq!(mgr.redo_count(), 0);
    }

    #[test]
    fn undo_manager_save_state() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        mgr.save_state(&doc);
        assert!(mgr.can_undo());
        assert!(!mgr.can_redo());
        assert_eq!(mgr.undo_count(), 1);
    }

    #[test]
    fn undo_manager_save_clears_redo() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        mgr.save_state(&doc);
        let _ = mgr.undo(&doc); // Creates redo entry
        assert!(mgr.can_redo());

        mgr.save_state(&doc); // Should clear redo
        assert!(!mgr.can_redo());
    }

    #[test]
    fn undo_manager_max_history() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(3);

        for _ in 0..5 {
            mgr.save_state(&doc);
        }

        assert_eq!(mgr.undo_count(), 3); // Limited to max
    }

    #[test]
    fn undo_manager_clear() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        mgr.save_state(&doc);
        mgr.clear();

        assert!(!mgr.can_undo());
        assert!(!mgr.can_redo());
        assert_eq!(mgr.undo_count(), 0);
        assert_eq!(mgr.redo_count(), 0);
    }

    #[test]
    fn undo_manager_undo_redo_cycle() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        // Save initial state
        mgr.save_state(&doc);
        assert_eq!(mgr.undo_count(), 1);

        // Undo
        let prev = mgr.undo(&doc);
        assert!(prev.is_some());
        assert_eq!(mgr.undo_count(), 0);
        assert_eq!(mgr.redo_count(), 1);

        // Redo
        let next = mgr.redo(&doc);
        assert!(next.is_some());
        assert_eq!(mgr.undo_count(), 1);
        assert_eq!(mgr.redo_count(), 0);
    }

    #[test]
    fn undo_manager_undo_empty() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        let result = mgr.undo(&doc);
        assert!(result.is_none());
    }

    #[test]
    fn undo_manager_redo_empty() {
        let doc = Document::new();
        let mut mgr = UndoManager::new(50);

        let result = mgr.redo(&doc);
        assert!(result.is_none());
    }

    #[test]
    fn undo_manager_disk_backed_create() {
        let session_id = format!("test_disk_{}", std::process::id());
        let mut mgr = UndoManager::new_disk_backed(&session_id).unwrap();

        assert!(mgr.is_disk_backed());
        assert!(!mgr.can_undo());
        assert_eq!(mgr.undo_count(), 0);

        // Cleanup
        mgr.cleanup_disk_storage();
    }

    #[test]
    fn undo_manager_disk_backed_save_and_undo() {
        let session_id = format!("test_disk_undo_{}", std::process::id());
        let mut mgr = UndoManager::new_disk_backed(&session_id).unwrap();
        let doc = Document::new();

        // Save states
        mgr.save_state(&doc);
        mgr.save_state(&doc);
        assert_eq!(mgr.undo_count(), 2);
        assert!(mgr.can_undo());

        // Undo
        let prev = mgr.undo(&doc);
        assert!(prev.is_some());
        assert_eq!(mgr.undo_count(), 1);
        assert!(mgr.can_redo());

        // Redo
        let next = mgr.redo(&doc);
        assert!(next.is_some());
        assert_eq!(mgr.undo_count(), 2);
        assert!(!mgr.can_redo());

        // Cleanup
        mgr.cleanup_disk_storage();
    }

    #[test]
    fn undo_manager_disk_backed_unlimited_history() {
        let session_id = format!("test_disk_unlimited_{}", std::process::id());
        let mut mgr = UndoManager::new_disk_backed(&session_id).unwrap();
        let doc = Document::new();

        // Save many more states than memory limit would allow
        for _ in 0..150 {
            mgr.save_state(&doc);
        }

        // All 150 states should be preserved (no limit)
        assert_eq!(mgr.undo_count(), 150);

        // Cleanup
        mgr.cleanup_disk_storage();
    }

    #[test]
    fn undo_manager_disk_backed_clear() {
        let session_id = format!("test_disk_clear_{}", std::process::id());
        let mut mgr = UndoManager::new_disk_backed(&session_id).unwrap();
        let doc = Document::new();

        mgr.save_state(&doc);
        mgr.save_state(&doc);
        assert_eq!(mgr.undo_count(), 2);

        mgr.clear();
        assert_eq!(mgr.undo_count(), 0);
        assert!(!mgr.can_undo());

        // Cleanup
        mgr.cleanup_disk_storage();
    }
}
