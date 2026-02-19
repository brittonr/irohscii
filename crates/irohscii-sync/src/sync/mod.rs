//! P2P synchronization for irohscii using iroh and automerge
//!
//! This module provides real-time collaborative editing via:
//! - iroh: P2P networking with cryptographic identity
//! - aspen-automerge: CRDT-based conflict-free data synchronization
//!
//! The automerge document IS the source of truth - this module just syncs it.

pub mod local_store;
pub mod presence_protocol;
pub mod protocol;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use automerge::Automerge;
use iroh::Endpoint;
use iroh::discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher};
use iroh::endpoint::Connection;
use iroh_base::EndpointAddr;
use tokio::sync::mpsc as tokio_mpsc;

use aspen_automerge::{DocumentId, DocumentStore};

use crate::{PeerId, PeerPresence, PresenceMessage};
use local_store::LocalDocumentStore;
use presence_protocol::PresenceProtocol;
pub use protocol::AutomergeSyncTicket;
pub use protocol::CapabilityToken;
use protocol::{AUTOMERGE_SYNC_ALPN, AutomergeSyncHandler, sync_with_peer, sync_with_peer_cap};

/// Fixed document ID for the irohscii session
const IROHSCII_DOC_ID: &str = "irohscii-session";

/// Configuration for sync behavior
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub mode: SyncMode,
    /// Ticket for an aspen cluster node to persist documents to
    pub cluster_ticket: Option<String>,
    /// Capability token for authenticating with the cluster node.
    /// Required when the cluster enforces capability-based auth.
    pub cluster_capability: Option<CapabilityToken>,
    /// Disable DNS/Pkarr discovery (for test isolation)
    pub disable_discovery: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::Active { join_ticket: None },
            cluster_ticket: None,
            cluster_capability: None,
            disable_discovery: false,
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
    Ready {
        endpoint_id: String,
        local_peer_id: PeerId,
    },
    /// Remote changes received (boxed to reduce enum size)
    RemoteChanges { doc: Box<Automerge> },
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
    /// Send local document state (boxed to reduce enum size)
    SyncDoc { doc: Box<Automerge> },
    /// Broadcast local presence to all peers
    BroadcastPresence(PeerPresence),
    /// Connect to an aspen cluster node for document persistence
    ConnectCluster {
        ticket: String,
        capability: Option<CapabilityToken>,
    },
    /// Shutdown sync
    Shutdown,
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
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = event_tx.send(SyncEvent::Error(format!(
                    "Failed to create tokio runtime: {}",
                    e
                )));
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = run_sync(config, event_tx.clone(), command_rx, endpoint_id_tx).await {
                let _ = event_tx.send(SyncEvent::Error(e.to_string()));
            }
        });
    });

    // Wait for endpoint ID (with timeout)
    let endpoint_id = endpoint_id_rx.recv_timeout(Duration::from_secs(10)).ok();

    Ok(SyncHandle {
        command_tx,
        event_rx,
        endpoint_id,
        _thread: thread,
    })
}

/// Decode a cluster ticket string into an endpoint address and optional capability.
///
/// Accepts two formats:
/// - `amsync1...` — automerge sync ticket (address + capability in one string)
/// - `irohscii1...` — bare endpoint address (no auth, for legacy/peer use)
pub fn decode_cluster_ticket(ticket: &str) -> Result<(EndpointAddr, Option<CapabilityToken>)> {
    if ticket.starts_with("amsync1") {
        let sync_ticket = AutomergeSyncTicket::deserialize(ticket)
            .map_err(|e| anyhow::anyhow!("Invalid sync ticket: {}", e))?;
        let token = sync_ticket
            .capability_token()
            .map_err(|e| anyhow::anyhow!("Invalid capability in ticket: {}", e))?;
        Ok((sync_ticket.addr, Some(token)))
    } else {
        let addr = decode_ticket(ticket)?;
        Ok((addr, None))
    }
}

/// Encode an EndpointAddr as a shareable ticket string
pub fn encode_ticket(addr: &EndpointAddr) -> String {
    let bytes = postcard::to_stdvec(addr).expect("EndpointAddr serialization should not fail");
    format!("irohscii1{}", data_encoding::BASE32_NOPAD.encode(&bytes))
}

/// Decode a ticket string back to EndpointAddr
pub fn decode_ticket(ticket: &str) -> Result<EndpointAddr> {
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

/// Trigger a sync with a peer, silently ignoring errors.
/// Clones the connection handle (cheap, QUIC connections are multiplexed).
async fn do_sync(
    store: &Arc<LocalDocumentStore>,
    doc_id: &DocumentId,
    conn: &Connection,
    capability: Option<&CapabilityToken>,
) {
    let result = match capability {
        Some(cap) => sync_with_peer_cap(store.as_ref(), doc_id, conn, Some(cap)).await,
        None => sync_with_peer(store.as_ref(), doc_id, conn).await,
    };
    if let Err(e) = result {
        // Only log non-connection errors (connection errors are expected during shutdown)
        let msg = e.to_string();
        if !msg.contains("closed") && !msg.contains("refused") {
            eprintln!("Sync error: {}", e);
        }
    }
}

/// Main async sync loop
async fn run_sync(
    config: SyncConfig,
    event_tx: std_mpsc::Sender<SyncEvent>,
    command_rx: std_mpsc::Receiver<SyncCommand>,
    endpoint_id_tx: std_mpsc::Sender<String>,
) -> Result<()> {
    // Create iroh endpoint, optionally with n0 discovery (DNS + Pkarr)
    let mut builder = Endpoint::builder();
    if !config.disable_discovery {
        builder = builder
            .discovery(DnsDiscovery::n0_dns())
            .discovery(PkarrPublisher::n0_dns());
    }
    let endpoint = builder.bind().await?;

    // Create a ticket from the endpoint address (includes relay URL and direct addresses)
    let addr = endpoint.addr();
    let ticket_string = encode_ticket(&addr);

    // Derive our local peer ID from the endpoint's public key
    let public_key = endpoint.id();
    let local_peer_id = PeerId::from_bytes(public_key.as_bytes())
        .ok_or_else(|| anyhow::anyhow!("PublicKey must be 32 bytes"))?;

    // Send our ticket string back to main thread
    let _ = endpoint_id_tx.send(ticket_string.clone());

    // Create the document store with a channel for sync notifications
    let (store_change_tx, mut store_change_rx) = tokio_mpsc::channel(10);
    let store = Arc::new(LocalDocumentStore::new(store_change_tx));

    // Create the document ID for our session
    let doc_id = DocumentId::from_string(IROHSCII_DOC_ID)?;

    // Initialize an empty document in the store if it doesn't exist
    if !store.as_ref().exists(&doc_id).await? {
        store.as_ref().create(Some(doc_id.clone()), None).await?;
    }

    // Create the automerge sync handler (for accepting incoming connections via router)
    let sync_handler = Arc::new(AutomergeSyncHandler::new(store.clone()));

    // Create the presence protocol handler
    let (presence_tx, mut presence_rx) = tokio_mpsc::channel(64);
    let presence_protocol = PresenceProtocol::new(local_peer_id, presence_tx);

    // Build the router with both protocols
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(AUTOMERGE_SYNC_ALPN, sync_handler.clone())
        .accept(PresenceProtocol::ALPN, presence_protocol.clone())
        .spawn();

    // Notify that we're ready with our peer ID
    let _ = event_tx.send(SyncEvent::Ready {
        endpoint_id: ticket_string.clone(),
        local_peer_id,
    });

    match config.mode {
        SyncMode::Active { join_ticket } => {
            // Shutdown flag for clean periodic sync termination
            let shutting_down = Arc::new(AtomicBool::new(false));

            // Resolve peer address (if joining)
            let peer_endpoint_addr = if let Some(ref remote_ticket) = join_ticket {
                Some(decode_ticket(remote_ticket)?)
            } else {
                None
            };

            // Establish a persistent sync connection to the peer (reused for all syncs).
            // QUIC connections support multiplexed streams, so sync_with_peer() opens
            // a new bi-stream each time on this same connection.
            let sync_conn: Option<Connection> = if let Some(ref endpoint_addr) = peer_endpoint_addr {
                match endpoint.connect(endpoint_addr.clone(), AUTOMERGE_SYNC_ALPN).await {
                    Ok(conn) => Some(conn),
                    Err(e) => {
                        let _ = event_tx.send(SyncEvent::Error(format!(
                            "Failed to connect to peer: {}", e
                        )));
                        None
                    }
                }
            } else {
                None
            };

            // Capability token for authenticating with the cluster. Mutable so
            // ConnectCluster can replace it mid-session.
            let mut cluster_cap: Option<CapabilityToken> = config.cluster_capability.clone();

            // Establish a persistent connection to the aspen cluster node (if configured).
            // This is independent of peer sync — the cluster provides durable storage
            // while peer sync provides real-time collaboration.
            // Mutable so ConnectCluster can attach one mid-session.
            let mut cluster_conn: Option<Connection> = if let Some(ref cluster_ticket) = config.cluster_ticket {
                // Parse the ticket — amsync1 tickets include the capability token
                let (cluster_addr, ticket_cap) = decode_cluster_ticket(cluster_ticket)?;
                if ticket_cap.is_some() {
                    cluster_cap = ticket_cap;
                }
                match endpoint.connect(cluster_addr, AUTOMERGE_SYNC_ALPN).await {
                    Ok(conn) => Some(conn),
                    Err(e) => {
                        let _ = event_tx.send(SyncEvent::Error(format!(
                            "Failed to connect to cluster: {}", e
                        )));
                        None
                    }
                }
            } else {
                None
            };

            // Do initial sync to pull any existing state from the peer
            if let Some(ref conn) = sync_conn {
                do_sync(&store, &doc_id, conn, None).await;
            }

            // Pull any existing state from the cluster
            if let Some(ref conn) = cluster_conn {
                do_sync(&store, &doc_id, conn, cluster_cap.as_ref()).await;
            }

            // Connect presence protocol on a separate connection (different ALPN)
            let presence_handle = if let Some(ref endpoint_addr) = peer_endpoint_addr {
                match endpoint.connect(endpoint_addr.clone(), PresenceProtocol::ALPN).await {
                    Ok(conn) => {
                        let presence_clone = presence_protocol.clone();
                        Some(tokio::spawn(async move {
                            if let Err(e) = presence_clone.run_presence_loop(conn).await {
                                eprintln!("Presence loop error: {}", e);
                            }
                        }))
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // Periodic sync timer (every 200ms) to pull remote changes from peers
            let mut sync_interval = tokio::time::interval(Duration::from_millis(200));
            sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Cluster sync timer (every 2s) — less frequent since it's for persistence
            let mut cluster_interval = tokio::time::interval(Duration::from_secs(2));
            cluster_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Main loop
            loop {
                tokio::select! {
                    // Store notified us of changes from sync
                    Some(_doc_id_str) = store_change_rx.recv() => {
                        if let Some(bytes) = store.get_bytes(&doc_id).await {
                            match Automerge::load(&bytes) {
                                Ok(doc) => {
                                    let _ = event_tx.send(SyncEvent::RemoteChanges {
                                        doc: Box::new(doc),
                                    });
                                }
                                Err(e) => {
                                    let _ = event_tx.send(SyncEvent::Error(
                                        format!("Failed to load doc: {}", e),
                                    ));
                                }
                            }
                        }
                    }
                    // Presence messages from peers
                    Some(msg) = presence_rx.recv() => {
                        match msg {
                            PresenceMessage::Update(presence) => {
                                let _ = event_tx.send(SyncEvent::PresenceUpdate(presence));
                            }
                            PresenceMessage::Leave { peer_id } => {
                                let _ = event_tx.send(SyncEvent::PresenceRemoved { peer_id });
                            }
                            PresenceMessage::RequestAll => {}
                        }
                    }
                    // Periodic sync with peer (reuses persistent connection)
                    _ = sync_interval.tick() => {
                        if !shutting_down.load(Ordering::Relaxed) {
                            if let Some(ref conn) = sync_conn {
                                let store_clone = store.clone();
                                let doc_id_clone = doc_id.clone();
                                let conn_clone = conn.clone();
                                let shutting_down_clone = shutting_down.clone();
                                tokio::spawn(async move {
                                    if !shutting_down_clone.load(Ordering::Relaxed) {
                                        do_sync(&store_clone, &doc_id_clone, &conn_clone, None).await;
                                    }
                                });
                            }
                        }
                    }
                    // Periodic sync with cluster (less frequent, for persistence)
                    _ = cluster_interval.tick() => {
                        if !shutting_down.load(Ordering::Relaxed) {
                            if let Some(ref conn) = cluster_conn {
                                let store_clone = store.clone();
                                let doc_id_clone = doc_id.clone();
                                let conn_clone = conn.clone();
                                let cap_clone = cluster_cap.clone();
                                let shutting_down_clone = shutting_down.clone();
                                tokio::spawn(async move {
                                    if !shutting_down_clone.load(Ordering::Relaxed) {
                                        do_sync(&store_clone, &doc_id_clone, &conn_clone, cap_clone.as_ref()).await;
                                    }
                                });
                            }
                        }
                    }
                    // Commands from main thread
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        match command_rx.try_recv() {
                            Ok(SyncCommand::SyncDoc { doc }) => {
                                // Update the local store (no notification to avoid feedback loop)
                                let bytes = doc.save();
                                if let Err(e) = store.update_from_app(&doc_id, &bytes).await {
                                    let _ = event_tx.send(SyncEvent::Error(
                                        format!("Failed to update store: {}", e),
                                    ));
                                }

                                // Push changes to peer via the persistent connection
                                if let Some(ref conn) = sync_conn {
                                    let store_clone = store.clone();
                                    let doc_id_clone = doc_id.clone();
                                    let conn_clone = conn.clone();
                                    tokio::spawn(async move {
                                        do_sync(&store_clone, &doc_id_clone, &conn_clone, None).await;
                                    });
                                }

                                // Push changes to cluster
                                if let Some(ref conn) = cluster_conn {
                                    let store_clone = store.clone();
                                    let doc_id_clone = doc_id.clone();
                                    let conn_clone = conn.clone();
                                    let cap_clone = cluster_cap.clone();
                                    tokio::spawn(async move {
                                        do_sync(&store_clone, &doc_id_clone, &conn_clone, cap_clone.as_ref()).await;
                                    });
                                }
                            }
                            Ok(SyncCommand::BroadcastPresence(presence)) => {
                                presence_protocol.broadcast(presence);
                            }
                            Ok(SyncCommand::ConnectCluster { ticket, capability }) => {
                                match decode_cluster_ticket(&ticket) {
                                    Ok((addr, ticket_cap)) => {
                                        match endpoint.connect(addr, AUTOMERGE_SYNC_ALPN).await {
                                            Ok(conn) => {
                                                // Prefer explicit capability, fall back to ticket's
                                                cluster_cap = capability.or(ticket_cap);
                                                // Pull existing state from cluster
                                                do_sync(&store, &doc_id, &conn, cluster_cap.as_ref()).await;
                                                cluster_conn = Some(conn);
                                            }
                                            Err(e) => {
                                                let _ = event_tx.send(SyncEvent::Error(
                                                    format!("Failed to connect to cluster: {}", e),
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(SyncEvent::Error(
                                            format!("Invalid cluster ticket: {}", e),
                                        ));
                                    }
                                }
                            }
                            Ok(SyncCommand::Shutdown) => {
                                // Signal all spawned tasks to stop
                                shutting_down.store(true, Ordering::Relaxed);
                                presence_protocol.broadcast_leave();
                                if let Some(h) = &presence_handle {
                                    h.abort();
                                }
                                // Close connections gracefully
                                if let Some(ref conn) = sync_conn {
                                    conn.close(0u32.into(), b"shutdown");
                                }
                                if let Some(ref conn) = cluster_conn {
                                    conn.close(0u32.into(), b"shutdown");
                                }
                                break;
                            }
                            Err(std_mpsc::TryRecvError::Empty) => {}
                            Err(std_mpsc::TryRecvError::Disconnected) => {
                                shutting_down.store(true, Ordering::Relaxed);
                                if let Some(h) = &presence_handle {
                                    h.abort();
                                }
                                if let Some(ref conn) = sync_conn {
                                    conn.close(0u32.into(), b"shutdown");
                                }
                                if let Some(ref conn) = cluster_conn {
                                    conn.close(0u32.into(), b"shutdown");
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        SyncMode::Disabled => {
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
