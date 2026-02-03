//! Backend implementations for [`SyncableDocument`].
//!
//! This module contains implementations of the [`SyncableDocument`] trait
//! for various CRDT libraries.

#[cfg(feature = "automerge")]
pub mod automerge;
