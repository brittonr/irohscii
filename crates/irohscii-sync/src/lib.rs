//! P2P synchronization for irohscii using Iroh and Automerge.
//!
//! This crate provides:
//! - Real-time collaborative editing via iroh P2P networking
//! - Automerge CRDT-based conflict-free data synchronization
//! - Presence tracking for cursor/activity visibility

pub mod peer;
pub mod presence;
mod sync;

pub use peer::PeerId;
pub use presence::{
    CursorActivity, PEER_COLORS, PeerPresence, PresenceManager, PresenceMessage, ToolKind,
    peer_color,
};
pub use sync::{
    SyncCommand, SyncConfig, SyncEvent, SyncHandle, SyncMode, decode_ticket, encode_ticket,
    start_sync_thread,
};

// Re-export core types for convenience
pub use irohscii_core::{LayerId, Position, ShapeId};
