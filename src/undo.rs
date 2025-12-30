//! Undo/redo manager using serialized automerge document snapshots.
//!
//! Automerge doesn't support true rollback (it tracks all changes ever made),
//! so we use serialized document snapshots for undo/redo. This is memory-efficient
//! because automerge's save format is compact.

use automerge::Automerge;

use crate::document::Document;

/// Manages undo/redo with document snapshots
pub struct UndoManager {
    /// Stack of document states (serialized for memory efficiency)
    undo_stack: Vec<Vec<u8>>,
    /// Redo stack
    redo_stack: Vec<Vec<u8>>,
    /// Maximum history size
    max_history: usize,
}

impl UndoManager {
    /// Create a new undo manager
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Save current state before mutation
    pub fn save_state(&mut self, doc: &Document) {
        let bytes = doc.automerge().save();
        self.undo_stack.push(bytes);
        self.redo_stack.clear();

        // Limit history size
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo to previous state, returns the previous automerge doc
    pub fn undo(&mut self, current_doc: &Document) -> Option<Automerge> {
        if let Some(prev_bytes) = self.undo_stack.pop() {
            // Save current for redo
            self.redo_stack.push(current_doc.automerge().save());

            // Load previous state
            Automerge::load(&prev_bytes).ok()
        } else {
            None
        }
    }

    /// Redo to next state, returns the next automerge doc
    pub fn redo(&mut self, current_doc: &Document) -> Option<Automerge> {
        if let Some(next_bytes) = self.redo_stack.pop() {
            // Save current for undo
            self.undo_stack.push(current_doc.automerge().save());

            // Load next state
            Automerge::load(&next_bytes).ok()
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get the number of undo states available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo states available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(100)
    }
}
