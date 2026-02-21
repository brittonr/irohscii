//! Local in-memory document store for irohscii.
//!
//! Implements the aspen-automerge DocumentStore trait using in-memory storage,
//! suitable for a single-user or small collaborative editing session.
//! Documents are stored as serialized bytes and loaded on demand.

use std::collections::HashMap;

use async_trait::async_trait;
use automerge::AutoCommit;
use tokio::sync::{RwLock, mpsc};

use aspen_automerge::{
    ApplyResult, AutomergeError, AutomergeResult, DocumentChange, DocumentId,
    DocumentMetadata, DocumentStore, ListOptions, ListResult,
};

/// Local in-memory document store.
///
/// Stores Automerge documents as serialized bytes in memory.
/// When the sync protocol saves a document (after merging remote changes),
/// it notifies the application via a channel.
pub struct LocalDocumentStore {
    /// Document storage: content_key -> automerge document bytes
    docs: RwLock<HashMap<String, Vec<u8>>>,
    /// Metadata storage: doc_id string -> metadata
    metas: RwLock<HashMap<String, DocumentMetadata>>,
    /// Channel to notify the application of document changes from sync.
    /// Sends the document ID when a document is saved (e.g. after sync).
    change_tx: mpsc::Sender<String>,
}

impl LocalDocumentStore {
    /// Create a new empty local document store.
    pub fn new(change_tx: mpsc::Sender<String>) -> Self {
        Self {
            docs: RwLock::new(HashMap::new()),
            metas: RwLock::new(HashMap::new()),
            change_tx,
        }
    }

    /// Update a document from external source (e.g., the application's local changes).
    /// This does NOT trigger the change notification (to avoid feedback loops).
    pub async fn update_from_app(&self, id: &DocumentId, doc_bytes: &[u8]) -> AutomergeResult<()> {
        let key = id.content_key();
        self.docs.write().await.insert(key, doc_bytes.to_vec());
        Ok(())
    }

    /// Get raw document bytes (for sending to the application).
    pub async fn get_bytes(&self, id: &DocumentId) -> Option<Vec<u8>> {
        let key = id.content_key();
        self.docs.read().await.get(&key).cloned()
    }
}

#[async_trait]
impl DocumentStore for LocalDocumentStore {
    async fn create(&self, id: Option<DocumentId>, metadata: Option<DocumentMetadata>) -> AutomergeResult<DocumentId> {
        let doc_id = id.unwrap_or_default();
        let key = doc_id.content_key();

        // Check if already exists
        if self.docs.read().await.contains_key(&key) {
            return Err(AutomergeError::DocumentAlreadyExists {
                document_id: doc_id.to_string(),
            });
        }

        // Create empty document
        let mut doc = AutoCommit::new();
        let bytes = doc.save();

        self.docs.write().await.insert(key, bytes);

        // Store metadata
        let meta = metadata.unwrap_or_else(|| DocumentMetadata::new(doc_id.clone()));
        self.metas.write().await.insert(doc_id.to_string(), meta);

        Ok(doc_id)
    }

    async fn get(&self, id: &DocumentId) -> AutomergeResult<Option<AutoCommit>> {
        let key = id.content_key();
        let docs = self.docs.read().await;
        match docs.get(&key) {
            Some(bytes) => {
                let doc = AutoCommit::load(bytes).map_err(|e| AutomergeError::AutomergeLib {
                    message: e.to_string(),
                })?;
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    async fn get_metadata(&self, id: &DocumentId) -> AutomergeResult<Option<DocumentMetadata>> {
        Ok(self.metas.read().await.get(&id.to_string()).cloned())
    }

    async fn save(&self, id: &DocumentId, doc: &mut AutoCommit) -> AutomergeResult<()> {
        let key = id.content_key();
        let bytes = doc.save();

        // Only notify when the document actually changed (avoids spurious events
        // from sync rounds that exchange no data, e.g. initial sync of two empty docs).
        let changed = {
            let mut docs = self.docs.write().await;
            let changed = match docs.get(&key) {
                Some(existing) => *existing != bytes,
                None => true,
            };
            docs.insert(key, bytes);
            changed
        };

        // Update metadata (decomposed compound condition)
        if changed {
            if let Some(meta) = self.metas.write().await.get_mut(&id.to_string()) {
                meta.touch();
                let doc_len = doc.save().len();
                debug_assert!(doc_len <= u64::MAX as usize, "document too large for u64");
                meta.size_bytes = doc_len as u64;
            }
        }

        // Notify application only when document content actually changed
        if changed {
            let _ = self.change_tx.send(id.to_string()).await;
        }

        Ok(())
    }

    async fn delete(&self, id: &DocumentId) -> AutomergeResult<bool> {
        let key = id.content_key();
        let existed = self.docs.write().await.remove(&key).is_some();
        self.metas.write().await.remove(&id.to_string());
        Ok(existed)
    }

    async fn exists(&self, id: &DocumentId) -> AutomergeResult<bool> {
        let key = id.content_key();
        Ok(self.docs.read().await.contains_key(&key))
    }

    async fn apply_changes(&self, id: &DocumentId, changes: Vec<DocumentChange>) -> AutomergeResult<ApplyResult> {
        let mut doc = self.get(id).await?.unwrap_or_else(AutoCommit::new);
        let mut applied: u32 = 0;
        for change in &changes {
            if let Ok(n) = doc.load_incremental(&change.bytes) {
                let n_u32 = n.min(u32::MAX as usize);
                debug_assert!(n == n_u32, "applied changes count overflow");
                applied = applied.saturating_add(n_u32 as u32);
            }
        }
        self.save(id, &mut doc).await?;
        let new_heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();
        let doc_len = doc.save().len();
        debug_assert!(doc_len <= u64::MAX as usize, "document too large for u64");
        let new_size = doc_len as u64;
        Ok(ApplyResult {
            changes_applied: applied > 0,
            change_count: applied,
            new_heads,
            new_size,
        })
    }

    async fn get_heads(&self, id: &DocumentId) -> AutomergeResult<Vec<String>> {
        match self.get(id).await? {
            Some(mut doc) => Ok(doc.get_heads().iter().map(|h| hex::encode(h.0)).collect()),
            None => Err(AutomergeError::DocumentNotFound {
                document_id: id.to_string(),
            }),
        }
    }

    async fn merge(&self, target_id: &DocumentId, source_id: &DocumentId) -> AutomergeResult<ApplyResult> {
        let mut target = self.get(target_id).await?.ok_or_else(|| AutomergeError::DocumentNotFound {
            document_id: target_id.to_string(),
        })?;
        let mut source = self.get(source_id).await?.ok_or_else(|| AutomergeError::DocumentNotFound {
            document_id: source_id.to_string(),
        })?;
        target.merge(&mut source).map_err(|e| AutomergeError::AutomergeLib {
            message: e.to_string(),
        })?;
        self.save(target_id, &mut target).await?;
        let new_heads: Vec<String> = target.get_heads().iter().map(|h| hex::encode(h.0)).collect();
        let doc_len = target.save().len();
        debug_assert!(doc_len <= u64::MAX as usize, "document too large for u64");
        let new_size = doc_len as u64;
        Ok(ApplyResult {
            changes_applied: true,
            change_count: 1,
            new_heads,
            new_size,
        })
    }

    async fn list(&self, _options: ListOptions) -> AutomergeResult<ListResult> {
        let metas = self.metas.read().await;
        let documents: Vec<DocumentMetadata> = metas.values().cloned().collect();
        Ok(ListResult {
            documents,
            has_more: false,
            continuation_token: None,
        })
    }

    async fn list_ids(&self, _namespace: Option<&str>, _limit: u32) -> AutomergeResult<Vec<DocumentId>> {
        let docs = self.docs.read().await;
        let ids: Vec<DocumentId> = docs.keys()
            .filter_map(|key| DocumentId::from_content_key(key))
            .collect();
        Ok(ids)
    }
}
