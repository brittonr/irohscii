//! Generic P2P collaboration infrastructure using Iroh networking.
//!
//! This crate provides reusable building blocks for collaborative applications:
//!
//! - **Presence tracking**: Generic peer presence with customizable data
//! - **Document sync**: Trait-based document synchronization (with optional Automerge backend)
//! - **P2P networking**: Iroh-based connection management and protocol handling
//!
//! # Example
//!
//! ```ignore
//! use iroh_collab::{CollabConfig, CollabHandle, PresenceData, SyncableDocument};
//!
//! // Define your presence data
//! #[derive(Clone, Serialize, Deserialize)]
//! struct MyPresence {
//!     cursor_x: i32,
//!     cursor_y: i32,
//!     username: String,
//! }
//!
//! impl PresenceData for MyPresence {
//!     fn timestamp_ms(&self) -> u64 { ... }
//! }
//!
//! // Start collaboration
//! let handle = start_collab::<MyDoc, MyPresence>(config)?;
//! ```

mod document;
mod peer;
mod presence;
mod sync;

#[cfg(feature = "automerge")]
pub mod backends;

pub use document::SyncableDocument;
pub use peer::PeerId;
pub use presence::{PresenceData, PresenceManager, PresenceMessage};
pub use sync::{
    CollabCommand, CollabConfig, CollabEvent, CollabHandle, CollabMode, decode_ticket,
    encode_ticket, start_collab,
};

#[cfg(feature = "automerge")]
pub use backends::automerge::AutomergeDocument;
