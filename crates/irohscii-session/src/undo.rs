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
const MEMORY_CACHE_SIZE: usize = 20;

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
        max_history: usize,
    },
    /// Disk-backed storage with memory cache for recent items
    Disk {
        /// Directory for snapshot files
        dir: PathBuf,
        /// Current stack size (number of snapshots)
        count: usize,
        /// In-memory cache of recent snapshots (index, bytes)
        cache: VecDeque<(usize, Vec<u8>)>,
    },
}

#[allow(dead_code)]
impl UndoManager {
    /// Create a new memory-only undo manager
    pub fn new(max_history: usize) -> Self {
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
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("irohscii")
            .join("undo")
            .join(session_id);

        fs::create_dir_all(&dir)?;

        // Count existing snapshots
        let count = fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "snap")
            })
            .count();

        Ok(Self {
            storage: UndoStorage::Disk {
                dir,
                count,
                cache: VecDeque::with_capacity(MEMORY_CACHE_SIZE),
            },
            redo_stack: Vec::new(),
        })
    }

    /// Save current state before mutation
    pub fn save_state(&mut self, doc: &Document) {
        let bytes = doc.automerge().save();

        match &mut self.storage {
            UndoStorage::Memory { stack, max_history } => {
                stack.push(bytes);
                // Limit history size
                while stack.len() > *max_history {
                    stack.remove(0);
                }
            }
            UndoStorage::Disk { dir, count, cache } => {
                // Write to disk
                let path = dir.join(format!("{:08}.snap", count));
                if let Err(e) = fs::write(&path, &bytes) {
                    eprintln!("Warning: Failed to write undo snapshot: {}", e);
                    return;
                }

                // Add to cache
                if cache.len() >= MEMORY_CACHE_SIZE {
                    cache.pop_front();
                }
                cache.push_back((*count, bytes));
                *count += 1;
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
                    let (_, b) = cache.remove(pos)?;
                    b
                } else {
                    // Load from disk
                    let path = dir.join(format!("{:08}.snap", target_idx));
                    match fs::read(&path) {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("Warning: Failed to read undo snapshot: {}", e);
                            return None;
                        }
                    }
                };

                // Delete the file
                let path = dir.join(format!("{:08}.snap", target_idx));
                let _ = fs::remove_file(&path);
                *count -= 1;

                Some(bytes)
            }
        }?;

        // Save current for redo
        self.redo_stack.push(current_doc.automerge().save());

        // Load previous state
        Automerge::load(&prev_bytes).ok()
    }

    /// Redo to next state, returns the next automerge doc
    pub fn redo(&mut self, current_doc: &Document) -> Option<Automerge> {
        let next_bytes = self.redo_stack.pop()?;

        // Save current for undo (don't clear redo stack here)
        let current_bytes = current_doc.automerge().save();
        match &mut self.storage {
            UndoStorage::Memory { stack, max_history } => {
                stack.push(current_bytes);
                while stack.len() > *max_history {
                    stack.remove(0);
                }
            }
            UndoStorage::Disk { dir, count, cache } => {
                let path = dir.join(format!("{:08}.snap", count));
                if let Err(e) = fs::write(&path, &current_bytes) {
                    eprintln!("Warning: Failed to write undo snapshot: {}", e);
                } else {
                    if cache.len() >= MEMORY_CACHE_SIZE {
                        cache.pop_front();
                    }
                    cache.push_back((*count, current_bytes));
                    *count += 1;
                }
            }
        }

        // Load next state
        Automerge::load(&next_bytes).ok()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        match &self.storage {
            UndoStorage::Memory { stack, .. } => !stack.is_empty(),
            UndoStorage::Disk { count, .. } => *count > 0,
        }
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        match &mut self.storage {
            UndoStorage::Memory { stack, .. } => stack.clear(),
            UndoStorage::Disk { dir, count, cache } => {
                // Delete all snapshot files
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if entry
                            .path()
                            .extension()
                            .is_some_and(|ext| ext == "snap")
                        {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
                *count = 0;
                cache.clear();
            }
        }
        self.redo_stack.clear();
    }

    /// Get the number of undo states available
    pub fn undo_count(&self) -> usize {
        match &self.storage {
            UndoStorage::Memory { stack, .. } => stack.len(),
            UndoStorage::Disk { count, .. } => *count,
        }
    }

    /// Get the number of redo states available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Check if this is a disk-backed manager
    pub fn is_disk_backed(&self) -> bool {
        matches!(self.storage, UndoStorage::Disk { .. })
    }

    /// Cleanup disk storage (call on session close if not saving)
    pub fn cleanup_disk_storage(&mut self) {
        if let UndoStorage::Disk { dir, count, cache } = &mut self.storage {
            // Delete all snapshot files
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let _ = fs::remove_file(entry.path());
                }
            }
            // Try to remove the directory itself
            let _ = fs::remove_dir(&dir);
            *count = 0;
            cache.clear();
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
