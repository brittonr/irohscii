//! Lightweight presence protocol for cursor/activity sync
//!
//! Uses ALPN "irohscii/presence/1" over iroh connections.
//! Separate from automerge sync since presence data is ephemeral and high-frequency.

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::presence::{PeerId, PeerPresence, PresenceMessage};

/// Protocol identifier for presence sync
pub const PRESENCE_ALPN: &[u8] = b"irohscii/presence/1";

/// Presence protocol handler for iroh
#[derive(Clone)]
pub struct PresenceProtocol {
    inner: Arc<PresenceInner>,
}

impl std::fmt::Debug for PresenceProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresenceProtocol").finish()
    }
}

struct PresenceInner {
    /// Our peer ID
    local_peer_id: PeerId,
    /// Current local presence (updated by main thread)
    local_presence: RwLock<Option<PeerPresence>>,
    /// Broadcast channel for outgoing presence updates
    outgoing_tx: broadcast::Sender<PresenceMessage>,
    /// Channel to notify main thread of incoming presence
    incoming_tx: mpsc::Sender<PresenceMessage>,
}

impl PresenceProtocol {
    /// Protocol identifier
    pub const ALPN: &'static [u8] = PRESENCE_ALPN;

    /// Create a new presence protocol handler
    pub fn new(
        local_peer_id: PeerId,
        incoming_tx: mpsc::Sender<PresenceMessage>,
    ) -> Self {
        let (outgoing_tx, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(PresenceInner {
                local_peer_id,
                local_presence: RwLock::new(None),
                outgoing_tx,
                incoming_tx,
            }),
        }
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.inner.local_peer_id
    }

    /// Broadcast our presence to all connected peers
    pub fn broadcast(&self, presence: PeerPresence) {
        // Update stored local presence
        if let Ok(mut guard) = self.inner.local_presence.try_write() {
            *guard = Some(presence.clone());
        }
        // Broadcast to all peer connections
        let _ = self.inner.outgoing_tx.send(PresenceMessage::Update(presence));
    }

    /// Notify peers we're leaving
    pub fn broadcast_leave(&self) {
        let _ = self.inner.outgoing_tx.send(PresenceMessage::Leave {
            peer_id: self.inner.local_peer_id,
        });
    }

    /// Handle incoming connection (as acceptor/server)
    async fn handle_peer(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.accept_bi().await?;
        self.run_presence_sync(&mut send, &mut recv).await
    }

    /// Run presence sync loop as initiator (client)
    pub async fn run_presence_loop(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.open_bi().await?;
        self.run_presence_sync(&mut send, &mut recv).await
    }

    /// Run bidirectional presence sync
    async fn run_presence_sync<S, R>(&self, send: &mut S, recv: &mut R) -> Result<()>
    where
        S: AsyncWriteExt + Unpin,
        R: AsyncReadExt + Unpin,
    {
        let mut outgoing_rx = self.inner.outgoing_tx.subscribe();

        // Request peer's presence on connect
        send_presence_msg(send, &PresenceMessage::RequestAll).await?;

        loop {
            tokio::select! {
                // Outgoing: broadcast our updates to this peer
                result = outgoing_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            send_presence_msg(send, &msg).await?;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Dropped some messages, send current state
                            if let Some(presence) = self.inner.local_presence.read().await.as_ref() {
                                send_presence_msg(send, &PresenceMessage::Update(presence.clone())).await?;
                            }
                        }
                    }
                }
                // Incoming: receive updates from this peer
                result = recv_presence_msg(recv) => {
                    match result {
                        Ok(msg) => {
                            // Handle RequestAll by sending our current presence
                            if matches!(msg, PresenceMessage::RequestAll) {
                                if let Some(presence) = self.inner.local_presence.read().await.as_ref() {
                                    send_presence_msg(send, &PresenceMessage::Update(presence.clone())).await?;
                                }
                            }
                            // Forward to main thread
                            let _ = self.inner.incoming_tx.send(msg).await;
                        }
                        Err(_) => break, // Connection closed
                    }
                }
            }
        }
        Ok(())
    }
}

impl ProtocolHandler for PresenceProtocol {
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let this = self.clone();
        async move {
            this.handle_peer(conn).await.map_err(|e| {
                AcceptError::from_err(std::io::Error::other(e.to_string()))
            })
        }
    }
}

/// Send a presence message (length-prefixed msgpack)
async fn send_presence_msg<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &PresenceMessage,
) -> Result<()> {
    let data = rmp_serde::to_vec(msg)?;
    let len = data.len() as u32;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&data).await?;
    writer.flush().await?;
    Ok(())
}

/// Receive a presence message
async fn recv_presence_msg<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<PresenceMessage> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).await?;

    Ok(rmp_serde::from_slice(&data)?)
}
