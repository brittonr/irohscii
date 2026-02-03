//! Session management and undo history for irohscii.
//!
//! This crate provides:
//! - Named sessions with metadata and persistent storage
//! - Disk-backed undo/redo with unlimited history
//! - Session registry for quick access to recent sessions

mod session;
mod undo;

pub use session::{
    Collaborator, SessionId, SessionManager, SessionMeta, SessionRegistry, TicketInfo,
};
pub use undo::UndoManager;

// Re-export core types for convenience
pub use irohscii_core::Document;
