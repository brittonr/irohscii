//! Syncable document trait for collaborative editing.

use anyhow::Result;

/// A document that can be synchronized across peers.
///
/// This trait abstracts over different CRDT implementations (Automerge, Yrs, etc.)
/// to provide a common interface for document synchronization.
///
/// # Sync Protocol
///
/// The sync protocol works in rounds:
/// 1. Generate sync messages with [`generate_sync_message`]
/// 2. Send messages to peer
/// 3. Receive messages from peer
/// 4. Process with [`receive_sync_message`]
/// 5. Repeat until both sides return `None` from generate
///
/// # Example Implementation
///
/// ```ignore
/// impl SyncableDocument for MyDocument {
///     type SyncState = MySyncState;
///     type SyncMessage = MySyncMessage;
///
///     fn new_sync_state(&self) -> Self::SyncState {
///         MySyncState::new()
///     }
///
///     fn generate_sync_message(&self, state: &mut Self::SyncState) -> Option<Self::SyncMessage> {
///         // Generate next message based on sync state
///     }
///
///     fn receive_sync_message(&mut self, state: &mut Self::SyncState, msg: Self::SyncMessage) -> Result<()> {
///         // Process incoming message and update state
///     }
/// }
/// ```
pub trait SyncableDocument: Clone + Send + Sync + 'static {
    /// The sync state type used to track synchronization progress with a peer.
    type SyncState: Default + Send;

    /// The sync message type exchanged between peers.
    type SyncMessage: SyncMessage;

    /// Create a new sync state for a peer connection.
    fn new_sync_state(&self) -> Self::SyncState {
        Self::SyncState::default()
    }

    /// Generate the next sync message to send to a peer.
    ///
    /// Returns `None` when there are no more messages to send in this sync round.
    /// The sync state tracks what has already been sent/received.
    fn generate_sync_message(&self, state: &mut Self::SyncState) -> Option<Self::SyncMessage>;

    /// Process a sync message received from a peer.
    ///
    /// Updates both the document and sync state based on the message.
    fn receive_sync_message(
        &mut self,
        state: &mut Self::SyncState,
        msg: Self::SyncMessage,
    ) -> Result<()>;

    /// Merge another document into this one.
    ///
    /// Used for direct document merging (e.g., when receiving a full document snapshot).
    fn merge(&mut self, other: &mut Self) -> Result<()>;

    /// Serialize the document to bytes for persistence or transfer.
    fn save(&self) -> Vec<u8>;

    /// Load a document from serialized bytes.
    fn load(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

/// A sync message that can be encoded/decoded for network transmission.
pub trait SyncMessage: Send {
    /// Encode the message to bytes.
    fn encode(&self) -> Vec<u8>;

    /// Decode a message from bytes.
    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
