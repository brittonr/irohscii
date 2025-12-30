//! Iroh protocol handler for automerge synchronization
//!
//! Uses persistent bidirectional connections for real-time sync.

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use automerge::{sync::State as SyncState, sync::SyncDoc, Automerge};
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, watch, Mutex};

/// Protocol identifier for iroh
pub const ALPN: &[u8] = b"irohscii/automerge/1";

/// Automerge sync protocol over iroh connections
#[derive(Clone)]
pub struct IrohAutomergeProtocol {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for IrohAutomergeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohAutomergeProtocol").finish()
    }
}

struct Inner {
    /// The automerge document
    doc: Mutex<Automerge>,
    /// Channel to notify main thread of remote changes
    sync_tx: mpsc::Sender<Automerge>,
    /// Watch channel to signal local changes (never loses notifications)
    change_tx: watch::Sender<u64>,
    change_rx: watch::Receiver<u64>,
}

impl IrohAutomergeProtocol {
    /// Protocol identifier
    pub const ALPN: &'static [u8] = ALPN;

    /// Create a new protocol handler
    pub fn new(doc: Automerge, sync_tx: mpsc::Sender<Automerge>) -> Self {
        let (change_tx, change_rx) = watch::channel(0u64);
        Self {
            inner: Arc::new(Inner {
                doc: Mutex::new(doc),
                sync_tx,
                change_tx,
                change_rx,
            }),
        }
    }

    /// Merge local changes and signal all peers
    pub async fn merge_and_notify(&self, other: &Automerge) -> Result<()> {
        {
            let mut doc = self.inner.doc.lock().await;
            let mut other_clone = other.clone();
            doc.merge(&mut other_clone)?;
        }
        // Increment counter to signal change (watch never loses updates)
        let _ = self.inner.change_tx.send_modify(|v| *v += 1);
        Ok(())
    }

    /// Run a persistent sync connection (as initiator/client)
    pub async fn run_sync_loop(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.open_bi().await?;
        self.run_bidirectional_sync(&mut send, &mut recv).await
    }

    /// Handle incoming connection (as acceptor/server)
    async fn handle_peer(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.accept_bi().await?;
        self.run_bidirectional_sync(&mut send, &mut recv).await
    }

    /// Run bidirectional sync - same logic for both client and server
    async fn run_bidirectional_sync<S, R>(&self, send: &mut S, recv: &mut R) -> Result<()>
    where
        S: AsyncWriteExt + Unpin,
        R: AsyncReadExt + Unpin,
    {
        let mut sync_state = SyncState::new();
        let mut change_rx = self.inner.change_rx.clone();

        // Send initial sync messages
        self.send_all_sync_messages(send, &mut sync_state).await?;

        loop {
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
                            // Peer has no more to send right now, that's fine
                            // Don't break - connection stays open
                        }
                        Ok(msg_bytes) => {
                            // Process received message
                            let sync_msg = automerge::sync::Message::decode(&msg_bytes)?;
                            {
                                let mut doc = self.inner.doc.lock().await;
                                doc.receive_sync_message(&mut sync_state, sync_msg)?;
                            }
                            // Generate and send response
                            self.send_all_sync_messages(send, &mut sync_state).await?;
                            // Notify main thread of changes
                            let doc = self.inner.doc.lock().await;
                            let _ = self.inner.sync_tx.send(doc.clone()).await;
                        }
                        Err(_) => break, // Connection error
                    }
                }
            }
        }

        Ok(())
    }

    /// Send all pending sync messages until generate_sync_message returns None
    async fn send_all_sync_messages<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        sync_state: &mut SyncState,
    ) -> Result<()> {
        loop {
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

impl ProtocolHandler for IrohAutomergeProtocol {
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let this = self.clone();
        async move {
            this.handle_peer(conn).await.map_err(|e| {
                AcceptError::from_err(std::io::Error::other(e.to_string()))
            })
        }
    }
}

/// Send a message with length prefix
async fn send_msg<W: AsyncWriteExt + Unpin>(writer: &mut W, data: &[u8]) -> Result<()> {
    let len = data.len() as u64;
    writer.write_all(&len.to_le_bytes()).await?;
    if !data.is_empty() {
        writer.write_all(data).await?;
    }
    writer.flush().await?;
    Ok(())
}

/// Receive a message with length prefix
async fn recv_msg<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; 8];
    reader.read_exact(&mut len_bytes).await?;
    let len = u64::from_le_bytes(len_bytes) as usize;

    if len == 0 {
        return Ok(Vec::new());
    }

    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).await?;
    Ok(data)
}
