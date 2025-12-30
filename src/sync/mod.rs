//! P2P synchronization for irohscii using iroh and automerge
//!
//! This module provides real-time collaborative editing via:
//! - iroh: P2P networking with cryptographic identity
//! - automerge: CRDT-based conflict-free data synchronization
//!
//! The automerge document IS the source of truth - this module just syncs it.

pub mod presence_protocol;
pub mod protocol;

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use automerge::Automerge;
use iroh::discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher};
use iroh::Endpoint;
use iroh_base::EndpointAddr;
use tokio::sync::mpsc as tokio_mpsc;

use crate::document::ShapeId;
use crate::presence::{PeerId, PeerPresence, PresenceMessage};
use crate::shapes::ShapeKind;
use presence_protocol::PresenceProtocol;
use protocol::IrohAutomergeProtocol;

/// Configuration for sync behavior
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub mode: SyncMode,
    pub storage_path: Option<PathBuf>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::Active { join_ticket: None },
            storage_path: None,
        }
    }
}

/// Sync operation mode
#[derive(Debug, Clone)]
pub enum SyncMode {
    /// No syncing, standalone mode
    Disabled,
    /// Active sync - always accept connections, optionally join a peer
    Active { join_ticket: Option<String> },
}

/// Events from sync thread to main thread
#[derive(Debug)]
pub enum SyncEvent {
    /// Connection established, here's our endpoint ID and peer ID for sharing
    Ready { endpoint_id: String, local_peer_id: PeerId },
    /// Remote changes received
    RemoteChanges { doc: Automerge },
    /// Peer connected/disconnected
    PeerStatus { peer_count: usize, connected: bool },
    /// Presence update from remote peer
    PresenceUpdate(PeerPresence),
    /// Peer presence removed (disconnect or leave)
    PresenceRemoved { peer_id: PeerId },
    /// Error occurred
    Error(String),
}

/// Commands from main thread to sync thread
#[derive(Debug)]
pub enum SyncCommand {
    /// Send local document state
    SyncDoc { doc: Automerge },
    /// Broadcast local presence to all peers
    BroadcastPresence(PeerPresence),
    /// Shutdown sync
    Shutdown,
}

/// Change to a shape that needs to be synced
#[derive(Debug, Clone)]
pub enum ShapeChange {
    Added {
        id: ShapeId,
        kind: ShapeKind,
    },
    Modified {
        id: ShapeId,
        kind: ShapeKind,
    },
    Deleted {
        id: ShapeId,
    },
}

/// Handle for communicating with the sync thread from the main thread
pub struct SyncHandle {
    /// Channel to send commands to sync thread
    pub command_tx: std_mpsc::Sender<SyncCommand>,
    /// Channel to receive events from sync thread
    pub event_rx: std_mpsc::Receiver<SyncEvent>,
    /// Our session endpoint ID (for others to join)
    pub endpoint_id: Option<String>,
    /// Thread handle
    _thread: JoinHandle<()>,
}

impl SyncHandle {
    /// Non-blocking check for sync events
    pub fn poll_event(&self) -> Option<SyncEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Send a command to the sync thread
    pub fn send_command(&self, cmd: SyncCommand) -> Result<()> {
        self.command_tx.send(cmd)?;
        Ok(())
    }
}

/// Start the sync thread with the given configuration
pub fn start_sync_thread(config: SyncConfig) -> Result<SyncHandle> {
    let (event_tx, event_rx) = std_mpsc::channel();
    let (command_tx, command_rx) = std_mpsc::channel();

    // Channel to get the endpoint ID back from the async context
    let (endpoint_id_tx, endpoint_id_rx) = std_mpsc::channel();

    let thread = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async move {
            if let Err(e) = run_sync(config, event_tx.clone(), command_rx, endpoint_id_tx).await {
                let _ = event_tx.send(SyncEvent::Error(e.to_string()));
            }
        });
    });

    // Wait for endpoint ID (with timeout)
    let endpoint_id = endpoint_id_rx
        .recv_timeout(Duration::from_secs(10))
        .ok();

    Ok(SyncHandle {
        command_tx,
        event_rx,
        endpoint_id,
        _thread: thread,
    })
}

/// Encode an EndpointAddr as a shareable ticket string
fn encode_ticket(addr: &EndpointAddr) -> String {
    let bytes = postcard::to_stdvec(addr).expect("EndpointAddr serialization should not fail");
    format!("irohscii1{}", data_encoding::BASE32_NOPAD.encode(&bytes))
}

/// Decode a ticket string back to EndpointAddr
fn decode_ticket(ticket: &str) -> Result<EndpointAddr> {
    if let Some(data) = ticket.strip_prefix("irohscii1") {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(data.as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid ticket encoding: {}", e))?;
        let addr: EndpointAddr = postcard::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Invalid ticket data: {}", e))?;
        Ok(addr)
    } else {
        // Try parsing as bare EndpointId for backwards compatibility
        let id: iroh_base::PublicKey = ticket
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid endpoint ID: {}", e))?;
        Ok(EndpointAddr::new(id))
    }
}

/// Main async sync loop
async fn run_sync(
    config: SyncConfig,
    event_tx: std_mpsc::Sender<SyncEvent>,
    command_rx: std_mpsc::Receiver<SyncCommand>,
    endpoint_id_tx: std_mpsc::Sender<String>,
) -> Result<()> {
    // Create iroh endpoint with n0 discovery (DNS + Pkarr)
    let endpoint = Endpoint::builder()
        .discovery(DnsDiscovery::n0_dns())
        .discovery(PkarrPublisher::n0_dns())
        .bind()
        .await?;

    // Create a ticket from the endpoint address (includes relay URL and direct addresses)
    let addr = endpoint.addr();
    let ticket_string = encode_ticket(&addr);

    // Derive our local peer ID from the endpoint's public key
    let public_key = endpoint.id();
    let local_peer_id = PeerId::from_bytes(public_key.as_bytes())
        .expect("PublicKey should be 32 bytes");

    // Send our ticket string back to main thread
    let _ = endpoint_id_tx.send(ticket_string.clone());

    // Create the automerge protocol handler
    let (sync_tx, mut sync_rx) = tokio_mpsc::channel(10);
    let protocol = IrohAutomergeProtocol::new(Automerge::new(), sync_tx);

    // Create the presence protocol handler
    let (presence_tx, mut presence_rx) = tokio_mpsc::channel(64);
    let presence_protocol = PresenceProtocol::new(local_peer_id, presence_tx);

    // Build the router with both protocols
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(IrohAutomergeProtocol::ALPN, protocol.clone())
        .accept(PresenceProtocol::ALPN, presence_protocol.clone())
        .spawn();

    // Notify that we're ready with our peer ID
    let _ = event_tx.send(SyncEvent::Ready {
        endpoint_id: ticket_string.clone(),
        local_peer_id,
    });

    match config.mode {
        SyncMode::Active { join_ticket } => {
            // If we have a ticket to join, connect to that peer for both protocols
            let sync_handle = if let Some(ref remote_ticket) = join_ticket {
                let endpoint_addr = decode_ticket(remote_ticket)?;
                let conn = endpoint
                    .connect(endpoint_addr.clone(), IrohAutomergeProtocol::ALPN)
                    .await?;

                let protocol_clone = protocol.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = protocol_clone.run_sync_loop(conn).await {
                        eprintln!("Sync loop error: {}", e);
                    }
                }))
            } else {
                None
            };

            // Also connect presence protocol if joining a peer
            let presence_handle = if let Some(ref remote_ticket) = join_ticket {
                let endpoint_addr = decode_ticket(remote_ticket)?;
                let conn = endpoint
                    .connect(endpoint_addr, PresenceProtocol::ALPN)
                    .await?;

                let presence_clone = presence_protocol.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = presence_clone.run_presence_loop(conn).await {
                        eprintln!("Presence loop error: {}", e);
                    }
                }))
            } else {
                None
            };

            // Main loop: accept connections (via router) and handle local changes
            loop {
                tokio::select! {
                    Some(doc) = sync_rx.recv() => {
                        let _ = event_tx.send(SyncEvent::RemoteChanges { doc });
                    }
                    Some(msg) = presence_rx.recv() => {
                        match msg {
                            PresenceMessage::Update(presence) => {
                                let _ = event_tx.send(SyncEvent::PresenceUpdate(presence));
                            }
                            PresenceMessage::Leave { peer_id } => {
                                let _ = event_tx.send(SyncEvent::PresenceRemoved { peer_id });
                            }
                            PresenceMessage::RequestAll => {
                                // Handled internally by presence protocol
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        match command_rx.try_recv() {
                            Ok(SyncCommand::SyncDoc { doc }) => {
                                if let Err(e) = protocol.merge_and_notify(&doc).await {
                                    let _ = event_tx.send(SyncEvent::Error(e.to_string()));
                                }
                            }
                            Ok(SyncCommand::BroadcastPresence(presence)) => {
                                presence_protocol.broadcast(presence);
                            }
                            Ok(SyncCommand::Shutdown) => {
                                presence_protocol.broadcast_leave();
                                if let Some(h) = &sync_handle {
                                    h.abort();
                                }
                                if let Some(h) = &presence_handle {
                                    h.abort();
                                }
                                break;
                            }
                            Err(std_mpsc::TryRecvError::Empty) => {}
                            Err(std_mpsc::TryRecvError::Disconnected) => {
                                if let Some(h) = &sync_handle {
                                    h.abort();
                                }
                                if let Some(h) = &presence_handle {
                                    h.abort();
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        SyncMode::Disabled => {
            // Should not reach here, but just wait for shutdown
            loop {
                if let Ok(SyncCommand::Shutdown) = command_rx.recv() {
                    break;
                }
            }
        }
    }

    router.shutdown().await?;
    Ok(())
}
