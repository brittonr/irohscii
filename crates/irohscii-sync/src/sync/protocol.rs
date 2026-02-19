//! Sync protocol - delegates to aspen-automerge's sync implementation.
//!
//! This module re-exports the aspen-automerge sync protocol types.

pub use aspen_automerge::AUTOMERGE_SYNC_ALPN;
pub use aspen_automerge::AutomergeSyncHandler;
pub use aspen_automerge::sync_with_peer;
