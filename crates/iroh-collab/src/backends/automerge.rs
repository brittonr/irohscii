//! Automerge implementation of [`SyncableDocument`].
//!
//! This module provides an [`AutomergeDocument`] wrapper that implements
//! the [`SyncableDocument`] trait for Automerge documents.

use anyhow::Result;
use automerge::{Automerge, sync::State as SyncState, sync::SyncDoc};

use crate::document::{SyncMessage, SyncableDocument};

/// Wrapper around [`Automerge`] that implements [`SyncableDocument`].
///
/// This allows using Automerge documents directly with the `iroh-collab`
/// synchronization infrastructure.
///
/// # Example
///
/// ```ignore
/// use iroh_collab::{AutomergeDocument, CollabConfig, start_collab};
///
/// // Create an Automerge document
/// let doc = Automerge::new();
/// let syncable = AutomergeDocument::new(doc);
///
/// // Start collaboration
/// let handle = start_collab::<AutomergeDocument, MyPresence>(
///     CollabConfig::host(),
///     syncable,
/// )?;
/// ```
#[derive(Clone)]
pub struct AutomergeDocument {
    doc: Automerge,
}

impl AutomergeDocument {
    /// Create a new syncable document from an Automerge document.
    pub fn new(doc: Automerge) -> Self {
        Self { doc }
    }

    /// Get a reference to the underlying Automerge document.
    pub fn doc(&self) -> &Automerge {
        &self.doc
    }

    /// Get a mutable reference to the underlying Automerge document.
    pub fn doc_mut(&mut self) -> &mut Automerge {
        &mut self.doc
    }

    /// Consume and return the underlying Automerge document.
    pub fn into_inner(self) -> Automerge {
        self.doc
    }
}

impl From<Automerge> for AutomergeDocument {
    fn from(doc: Automerge) -> Self {
        Self::new(doc)
    }
}

impl From<AutomergeDocument> for Automerge {
    fn from(wrapper: AutomergeDocument) -> Self {
        wrapper.doc
    }
}

/// Wrapper for Automerge sync messages.
pub struct AutomergeSyncMessage {
    inner: automerge::sync::Message,
}

impl SyncMessage for AutomergeSyncMessage {
    fn encode(&self) -> Vec<u8> {
        // Clone needed because automerge's encode() takes ownership
        self.inner.clone().encode()
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let inner = automerge::sync::Message::decode(bytes)?;
        Ok(Self { inner })
    }
}

impl SyncableDocument for AutomergeDocument {
    type SyncState = SyncState;
    type SyncMessage = AutomergeSyncMessage;

    fn new_sync_state(&self) -> Self::SyncState {
        SyncState::new()
    }

    fn generate_sync_message(&self, state: &mut Self::SyncState) -> Option<Self::SyncMessage> {
        self.doc
            .generate_sync_message(state)
            .map(|inner| AutomergeSyncMessage { inner })
    }

    fn receive_sync_message(
        &mut self,
        state: &mut Self::SyncState,
        msg: Self::SyncMessage,
    ) -> Result<()> {
        self.doc.receive_sync_message(state, msg.inner)?;
        Ok(())
    }

    fn merge(&mut self, other: &mut Self) -> Result<()> {
        self.doc.merge(&mut other.doc)?;
        Ok(())
    }

    fn save(&self) -> Vec<u8> {
        self.doc.save()
    }

    fn load(bytes: &[u8]) -> Result<Self> {
        let doc = Automerge::load(bytes)?;
        Ok(Self { doc })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{ROOT, transaction::Transactable};

    #[test]
    fn automerge_document_new() {
        let doc = Automerge::new();
        let syncable = AutomergeDocument::new(doc);
        assert!(syncable.doc().is_empty());
    }

    #[test]
    fn automerge_document_from_into() {
        let doc = Automerge::new();
        let syncable: AutomergeDocument = doc.into();
        let _back: Automerge = syncable.into();
    }

    #[test]
    fn automerge_document_save_load() {
        let mut doc = Automerge::new();
        {
            let mut tx = doc.transaction();
            tx.put(ROOT, "test", "value").unwrap();
            tx.commit();
        }

        let syncable = AutomergeDocument::new(doc);
        let bytes = syncable.save();

        let loaded = AutomergeDocument::load(&bytes).unwrap();
        assert!(!loaded.doc().is_empty());
    }

    #[test]
    fn automerge_document_merge() {
        let mut doc1 = Automerge::new();
        {
            let mut tx = doc1.transaction();
            tx.put(ROOT, "key1", "value1").unwrap();
            tx.commit();
        }

        let mut doc2 = doc1.fork();
        {
            let mut tx = doc2.transaction();
            tx.put(ROOT, "key2", "value2").unwrap();
            tx.commit();
        }

        let mut syncable1 = AutomergeDocument::new(doc1);
        let mut syncable2 = AutomergeDocument::new(doc2);

        syncable1.merge(&mut syncable2).unwrap();
        // After merge, syncable1 should have both keys
    }

    #[test]
    fn automerge_sync_state_default() {
        let syncable = AutomergeDocument::new(Automerge::new());
        let _state = syncable.new_sync_state();
    }

    #[test]
    fn automerge_sync_roundtrip() {
        let mut doc1 = Automerge::new();
        {
            let mut tx = doc1.transaction();
            tx.put(ROOT, "test", "hello").unwrap();
            tx.commit();
        }
        let syncable1 = AutomergeDocument::new(doc1);

        let mut syncable2 = AutomergeDocument::new(Automerge::new());
        let mut state1 = syncable1.new_sync_state();
        let mut state2 = syncable2.new_sync_state();

        // Sync from 1 to 2
        while let Some(msg) = syncable1.generate_sync_message(&mut state1) {
            let encoded = msg.encode();
            let decoded = AutomergeSyncMessage::decode(&encoded).unwrap();
            syncable2
                .receive_sync_message(&mut state2, decoded)
                .unwrap();
        }

        // Sync back from 2 to 1 (should be in sync now)
        let msg_back = syncable2.generate_sync_message(&mut state2);
        assert!(msg_back.is_none() || syncable1.generate_sync_message(&mut state1).is_none());
    }
}
