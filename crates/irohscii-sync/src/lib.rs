//! P2P synchronization for irohscii using Iroh and Automerge.
//!
//! This crate provides:
//! - Real-time collaborative editing via iroh P2P networking
//! - Automerge CRDT-based conflict-free data synchronization
//! - Presence tracking for cursor/activity visibility

pub mod presence;
mod sync;

pub use presence::{
    peer_color, CursorActivity, PeerId, PeerPresence, PresenceManager, PresenceMessage, ToolKind,
    PEER_COLORS,
};
pub use sync::{
    decode_ticket, encode_ticket, start_sync_thread, SyncCommand, SyncConfig, SyncEvent,
    SyncHandle, SyncMode,
};

// Re-export core types for convenience
pub use irohscii_core::{LayerId, Position, ShapeId};
