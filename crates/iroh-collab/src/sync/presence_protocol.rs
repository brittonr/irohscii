//! Generic presence protocol for cursor/activity sync.
//!
//! Separate from document sync since presence data is ephemeral and high-frequency.

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{PeerId, PresenceData, PresenceMessage};

// Protocol constants
const MAX_PRESENCE_MSG_SIZE: u32 = 1024 * 1024; // 1 MB
const MAX_PRESENCE_ITERATIONS: usize = 100_000;

// Compile-time assertions
const _: () = assert!(MAX_PRESENCE_MSG_SIZE > 0);
const _: () = assert!(MAX_PRESENCE_ITERATIONS > 0);

/// Presence protocol handler for Iroh.
///
/// Generic over the presence type `P` which must implement [`PresenceData`].
#[derive(Clone)]
pub struct PresenceProtocol<P: PresenceData> {
    inner: Arc<PresenceInner<P>>,
}

impl<P: PresenceData> std::fmt::Debug for PresenceProtocol<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresenceProtocol").finish()
    }
}

struct PresenceInner<P: PresenceData> {
    /// Our peer ID.
    local_peer_id: PeerId,
    /// Current local presence (updated by main thread).
    local_presence: RwLock<Option<P>>,
    /// Broadcast channel for outgoing presence updates.
    outgoing_tx: broadcast::Sender<PresenceMessage<P>>,
    /// Channel to notify main thread of incoming presence.
    incoming_tx: mpsc::Sender<PresenceMessage<P>>,
    /// Protocol ALPN identifier.
    #[allow(dead_code)]
    alpn: Vec<u8>,
}

impl<P: PresenceData> PresenceProtocol<P> {
    /// Create a new presence protocol handler.
    pub fn new(
        local_peer_id: PeerId,
        incoming_tx: mpsc::Sender<PresenceMessage<P>>,
        alpn: Vec<u8>,
    ) -> Self {
        let (outgoing_tx, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(PresenceInner {
                local_peer_id,
                local_presence: RwLock::new(None),
                outgoing_tx,
                incoming_tx,
                alpn,
            }),
        }
    }

    /// Get our local peer ID.
    #[allow(dead_code)]
    pub fn local_peer_id(&self) -> PeerId {
        self.inner.local_peer_id
    }

    /// Broadcast our presence to all connected peers.
    pub fn broadcast(&self, presence: P) {
        // Update stored local presence
        if let Ok(mut guard) = self.inner.local_presence.try_write() {
            *guard = Some(presence.clone());
        }
        // Broadcast to all peer connections
        if let Err(e) = self.inner.outgoing_tx.send(PresenceMessage::Update(presence)) {
            tracing::debug!("failed to broadcast presence update: {}", e);
        }
    }

    /// Notify peers we're leaving.
    pub fn broadcast_leave(&self) {
        if let Err(e) = self.inner.outgoing_tx.send(PresenceMessage::Leave {
            peer_id: self.inner.local_peer_id,
        }) {
            tracing::debug!("failed to broadcast leave message: {}", e);
        }
    }

    /// Handle incoming connection (as acceptor/server).
    async fn handle_peer(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.accept_bi().await?;
        self.run_presence_sync(&mut send, &mut recv).await
    }

    /// Run presence sync loop as initiator (client).
    pub async fn run_presence_loop(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.open_bi().await?;
        self.run_presence_sync(&mut send, &mut recv).await
    }

    /// Run bidirectional presence sync.
    async fn run_presence_sync<S, R>(&self, send: &mut S, recv: &mut R) -> Result<()>
    where
        S: AsyncWriteExt + Unpin,
        R: AsyncReadExt + Unpin,
    {
        let mut outgoing_rx = self.inner.outgoing_tx.subscribe();

        // Request peer's presence on connect
        send_presence_msg(send, &PresenceMessage::<P>::RequestAll).await?;

        let mut iteration = 0;
        loop {
            iteration += 1;
            debug_assert!(iteration <= MAX_PRESENCE_ITERATIONS, "presence loop exceeded max iterations");
            if iteration > MAX_PRESENCE_ITERATIONS {
                tracing::warn!("presence loop exceeded {} iterations, terminating", MAX_PRESENCE_ITERATIONS);
                break;
            }
            
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
                result = recv_presence_msg::<P, _>(recv) => {
                    match result {
                        Ok(msg) => {
                            // Handle RequestAll by sending our current presence
                            if matches!(msg, PresenceMessage::RequestAll)
                                && let Some(presence) = self.inner.local_presence.read().await.as_ref() {
                                    send_presence_msg(send, &PresenceMessage::Update(presence.clone())).await?;
                                }
                            // Forward to main thread
                            if let Err(e) = self.inner.incoming_tx.send(msg).await {
                                tracing::warn!("failed to forward presence message to main thread: {}", e);
                            }
                        }
                        Err(_) => break, // Connection closed
                    }
                }
            }
        }
        Ok(())
    }
}

impl<P: PresenceData> ProtocolHandler for PresenceProtocol<P> {
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let this = self.clone();
        async move {
            this.handle_peer(conn)
                .await
                .map_err(|e| AcceptError::from_err(std::io::Error::other(e.to_string())))
        }
    }
}

/// Send a presence message (length-prefixed msgpack).
async fn send_presence_msg<P: PresenceData, W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &PresenceMessage<P>,
) -> Result<()> {
    let data = rmp_serde::to_vec(msg)?;
    let len = u32::try_from(data.len())
        .map_err(|_| anyhow::anyhow!("presence message too large: {} bytes", data.len()))?;
    debug_assert!(len <= MAX_PRESENCE_MSG_SIZE, "presence message size {} exceeds limit", len);
    
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&data).await?;
    writer.flush().await?;
    Ok(())
}

/// Receive a presence message.
async fn recv_presence_msg<P: PresenceData, R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<PresenceMessage<P>> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes);
    
    if len > MAX_PRESENCE_MSG_SIZE {
        return Err(anyhow::anyhow!("presence message too large: {} bytes (max: {})", len, MAX_PRESENCE_MSG_SIZE));
    }

    let len_usize = usize::try_from(len)
        .map_err(|_| anyhow::anyhow!("message size {} exceeds usize", len))?;
    let mut data = vec![0u8; len_usize];
    reader.read_exact(&mut data).await?;

    Ok(rmp_serde::from_slice(&data)?)
}
