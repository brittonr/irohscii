//! Generic document sync protocol over Iroh connections.
//!
//! This protocol handles synchronization of any document implementing [`SyncableDocument`].

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, mpsc, watch};

use crate::SyncableDocument;
use crate::document::SyncMessage;

// Protocol constants
const MAX_MESSAGE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
const MAX_SYNC_ITERATIONS: usize = 100_000;
const MAX_MESSAGES_PER_ROUND: usize = 1000;

// Compile-time assertions
const _: () = assert!(MAX_MESSAGE_SIZE > 0);
const _: () = assert!(MAX_SYNC_ITERATIONS > 0);
const _: () = assert!(MAX_MESSAGES_PER_ROUND > 0);

/// Document sync protocol handler for Iroh.
///
/// Generic over the document type `D` which must implement [`SyncableDocument`].
#[derive(Clone)]
pub struct DocumentProtocol<D: SyncableDocument> {
    inner: Arc<Inner<D>>,
}

impl<D: SyncableDocument> std::fmt::Debug for DocumentProtocol<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentProtocol").finish()
    }
}

struct Inner<D: SyncableDocument> {
    /// The document being synced.
    doc: Mutex<D>,
    /// Channel to notify main thread of remote changes.
    sync_tx: mpsc::Sender<D>,
    /// Watch channel to signal local changes.
    change_tx: watch::Sender<u64>,
    change_rx: watch::Receiver<u64>,
    /// Protocol ALPN identifier.
    #[allow(dead_code)]
    alpn: Vec<u8>,
}

impl<D: SyncableDocument> DocumentProtocol<D> {
    /// Create a new protocol handler.
    pub fn new(doc: D, sync_tx: mpsc::Sender<D>, alpn: Vec<u8>) -> Self {
        let (change_tx, change_rx) = watch::channel(0u64);
        Self {
            inner: Arc::new(Inner {
                doc: Mutex::new(doc),
                sync_tx,
                change_tx,
                change_rx,
                alpn,
            }),
        }
    }

    /// Merge local changes and signal all peers.
    pub async fn merge_and_notify(&self, other: &D) -> Result<()> {
        {
            let mut doc = self.inner.doc.lock().await;
            let mut other_clone = other.clone();
            doc.merge(&mut other_clone)?;
        }
        // Increment counter to signal change
        self.inner.change_tx.send_modify(|v| *v += 1);
        Ok(())
    }

    /// Run a persistent sync connection (as initiator/client).
    pub async fn run_sync_loop(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.open_bi().await?;
        self.run_bidirectional_sync(&mut send, &mut recv).await
    }

    /// Handle incoming connection (as acceptor/server).
    async fn handle_peer(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.accept_bi().await?;
        self.run_bidirectional_sync(&mut send, &mut recv).await
    }

    /// Run bidirectional sync - same logic for client and server.
    async fn run_bidirectional_sync<S, R>(&self, send: &mut S, recv: &mut R) -> Result<()>
    where
        S: AsyncWriteExt + Unpin,
        R: AsyncReadExt + Unpin,
    {
        let mut sync_state = {
            let doc = self.inner.doc.lock().await;
            doc.new_sync_state()
        };
        let mut change_rx = self.inner.change_rx.clone();

        // Send initial sync messages
        self.send_all_sync_messages(send, &mut sync_state).await?;

        let mut iteration = 0;
        loop {
            iteration += 1;
            debug_assert!(iteration <= MAX_SYNC_ITERATIONS, "sync loop exceeded max iterations");
            if iteration > MAX_SYNC_ITERATIONS {
                tracing::warn!("sync loop exceeded {} iterations, terminating", MAX_SYNC_ITERATIONS);
                break;
            }
            
            tokio::select! {
                // Local changes - send sync messages
                result = change_rx.changed() => {
                    if result.is_err() {
                        break; // Channel closed
                    }
                    self.send_all_sync_messages(send, &mut sync_state).await?;
                }
                // Incoming message from peer
                result = recv_msg(recv) => {
                    match result {
                        Ok(msg_bytes) if msg_bytes.is_empty() => {
                            // Peer has no more to send right now
                        }
                        Ok(msg_bytes) => {
                            // Decode and process message
                            let sync_msg = D::SyncMessage::decode(&msg_bytes)?;
                            {
                                let mut doc = self.inner.doc.lock().await;
                                doc.receive_sync_message(&mut sync_state, sync_msg)?;
                            }
                            // Generate and send response
                            self.send_all_sync_messages(send, &mut sync_state).await?;
                            // Notify main thread of changes
                            let doc = self.inner.doc.lock().await;
                            if let Err(e) = self.inner.sync_tx.send(doc.clone()).await {
                                tracing::warn!("failed to send document update to main thread: {}", e);
                            }
                        }
                        Err(_) => break, // Connection error
                    }
                }
            }
        }

        Ok(())
    }

    /// Send all pending sync messages until generate returns None.
    async fn send_all_sync_messages<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        sync_state: &mut D::SyncState,
    ) -> Result<()> {
        let mut count = 0;
        loop {
            count += 1;
            if count > MAX_MESSAGES_PER_ROUND {
                tracing::warn!("send_all_sync_messages exceeded {} messages, stopping", MAX_MESSAGES_PER_ROUND);
                send_msg(writer, &[]).await?;
                break;
            }
            
            let msg = {
                let doc = self.inner.doc.lock().await;
                doc.generate_sync_message(sync_state)
            };
            match msg {
                Some(msg) => {
                    send_msg(writer, &msg.encode()).await?;
                }
                None => {
                    // No more messages - send empty to signal "done for now"
                    send_msg(writer, &[]).await?;
                    break;
                }
            }
        }
        Ok(())
    }
}

impl<D: SyncableDocument> ProtocolHandler for DocumentProtocol<D> {
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let this = self.clone();
        async move {
            this.handle_peer(conn)
                .await
                .map_err(|e| AcceptError::from_err(std::io::Error::other(e.to_string())))
        }
    }
}

/// Send a message with length prefix.
async fn send_msg<W: AsyncWriteExt + Unpin>(writer: &mut W, data: &[u8]) -> Result<()> {
    let len = u64::try_from(data.len())
        .map_err(|_| anyhow::anyhow!("message too large: {} bytes", data.len()))?;
    debug_assert!(data.len() as u64 <= MAX_MESSAGE_SIZE, "message size {} exceeds limit", data.len());
    
    writer.write_all(&len.to_le_bytes()).await?;
    if !data.is_empty() {
        writer.write_all(data).await?;
    }
    writer.flush().await?;
    Ok(())
}

/// Receive a message with length prefix.
async fn recv_msg<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; 8];
    reader.read_exact(&mut len_bytes).await?;
    let len = u64::from_le_bytes(len_bytes);
    
    if len > MAX_MESSAGE_SIZE {
        return Err(anyhow::anyhow!("message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE));
    }

    if len == 0 {
        return Ok(Vec::new());
    }

    let len_usize = usize::try_from(len)
        .map_err(|_| anyhow::anyhow!("message size {} exceeds usize", len))?;
    let mut data = vec![0u8; len_usize];
    reader.read_exact(&mut data).await?;
    Ok(data)
}
