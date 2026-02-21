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

// Compile-time assertions for key invariants
const _: () = assert!(
    session::MAX_RECENT_SESSIONS > 0,
    "MAX_RECENT_SESSIONS must be positive"
);
const _: () = assert!(
    undo::MEMORY_CACHE_SIZE > 0,
    "MEMORY_CACHE_SIZE must be positive"
);
