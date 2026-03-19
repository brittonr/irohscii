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
use iroh::endpoint::presets;
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
    /// Connect to a peer for real-time sync
    ConnectPeer {
        ticket: String,
    },
    /// Shutdown sync
    Shutdown,
}

/// Handle for communicating with the sync thread from the main thread
pub struct SyncHandle {
    /// Channel to send commands to sync thread (tokio mpsc for zero-latency async recv)
    pub command_tx: tokio_mpsc::Sender<SyncCommand>,
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

    /// Send a command to the sync thread (non-blocking, drops if channel full)
    pub fn send_command(&self, cmd: SyncCommand) -> Result<()> {
        self.command_tx
            .try_send(cmd)
            .map_err(|e| anyhow::anyhow!("Failed to send sync command: {}", e))
    }
}

/// Start the sync thread with the given configuration
pub fn start_sync_thread(config: SyncConfig) -> Result<SyncHandle> {
    let (event_tx, event_rx) = std_mpsc::channel();
    let (command_tx, command_rx) = tokio_mpsc::channel(64);

    // Channel to get the endpoint ID back from the async context
    let (endpoint_id_tx, endpoint_id_rx) = std_mpsc::channel();

    let thread = thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                if let Err(send_err) = event_tx.send(SyncEvent::Error(format!(
                    "Failed to create tokio runtime: {}",
                    e
                ))) {
                    eprintln!("Failed to send runtime error event: {}", send_err);
                }
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = run_sync(config, event_tx.clone(), command_rx, endpoint_id_tx).await {
                if let Err(send_err) = event_tx.send(SyncEvent::Error(e.to_string())) {
                    eprintln!("Failed to send sync error event: {}", send_err);
                }
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

/// Setup context for the sync session
struct SyncSession {
    endpoint: Endpoint,
    store: Arc<LocalDocumentStore>,
    doc_id: DocumentId,
    local_peer_id: PeerId,
    presence_protocol: PresenceProtocol,
    store_change_rx: tokio_mpsc::Receiver<String>,
    presence_rx: tokio_mpsc::Receiver<PresenceMessage>,
    router: iroh::protocol::Router,
}

/// Initialize the sync session - create endpoint, store, and protocol handlers
async fn setup_sync_session(
    config: &SyncConfig,
    endpoint_id_tx: &std_mpsc::Sender<String>,
    event_tx: &std_mpsc::Sender<SyncEvent>,
) -> Result<SyncSession> {
    // Create iroh endpoint, optionally with n0 discovery (DNS + Pkarr)
    let builder = Endpoint::builder(presets::N0);
    let endpoint = builder.bind().await?;

    // Create a ticket from the endpoint address (includes relay URL and direct addresses)
    let addr = endpoint.addr();
    let ticket_string = encode_ticket(&addr);

    // Derive our local peer ID from the endpoint's public key
    let public_key = endpoint.id();
    debug_assert_eq!(
        public_key.as_bytes().len(),
        32,
        "Iroh PublicKey must be 32 bytes"
    );
    let local_peer_id = PeerId::from_bytes(public_key.as_bytes())
        .ok_or_else(|| anyhow::anyhow!("PublicKey must be 32 bytes"))?;

    // Send our ticket string back to main thread
    if let Err(e) = endpoint_id_tx.send(ticket_string.clone()) {
        eprintln!("Failed to send endpoint ID to main thread: {}", e);
    }

    // Create the document store with a channel for sync notifications
    let (store_change_tx, store_change_rx) = tokio_mpsc::channel(10);
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
    let (presence_tx, presence_rx) = tokio_mpsc::channel(64);
    let presence_protocol = PresenceProtocol::new(local_peer_id, presence_tx);

    // Build the router with both protocols
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(AUTOMERGE_SYNC_ALPN, sync_handler.clone())
        .accept(PresenceProtocol::ALPN, presence_protocol.clone())
        .spawn();

    // Notify that we're ready with our peer ID
    if let Err(e) = event_tx.send(SyncEvent::Ready {
        endpoint_id: ticket_string.clone(),
        local_peer_id,
    }) {
        eprintln!("Failed to send Ready event: {}", e);
    }

    debug_assert!(
        store.exists(&doc_id).await?,
        "Document must exist before starting sync"
    );

    Ok(SyncSession {
        endpoint,
        store,
        doc_id,
        local_peer_id,
        presence_protocol,
        store_change_rx,
        presence_rx,
        router,
    })
}

/// Establish initial peer connection (if join ticket provided)
async fn connect_to_peer(
    endpoint: &Endpoint,
    ticket: &str,
    event_tx: &std_mpsc::Sender<SyncEvent>,
) -> Option<Connection> {
    match decode_ticket(ticket) {
        Ok(endpoint_addr) => {
            match endpoint.connect(endpoint_addr, AUTOMERGE_SYNC_ALPN).await {
                Ok(conn) => Some(conn),
                Err(e) => {
                    if let Err(send_err) = event_tx.send(SyncEvent::Error(format!(
                        "Failed to connect to peer: {}", e
                    ))) {
                        eprintln!("Failed to send peer connection error: {}", send_err);
                    }
                    None
                }
            }
        }
        Err(e) => {
            if let Err(send_err) = event_tx.send(SyncEvent::Error(format!(
                "Invalid peer ticket: {}", e
            ))) {
                eprintln!("Failed to send ticket decode error: {}", send_err);
            }
            None
        }
    }
}

/// Establish initial cluster connection (if cluster ticket provided)
async fn connect_to_cluster(
    endpoint: &Endpoint,
    cluster_ticket: &str,
    event_tx: &std_mpsc::Sender<SyncEvent>,
) -> (Option<Connection>, Option<CapabilityToken>) {
    match decode_cluster_ticket(cluster_ticket) {
        Ok((cluster_addr, ticket_cap)) => {
            match endpoint.connect(cluster_addr, AUTOMERGE_SYNC_ALPN).await {
                Ok(conn) => (Some(conn), ticket_cap),
                Err(e) => {
                    if let Err(send_err) = event_tx.send(SyncEvent::Error(format!(
                        "Failed to connect to cluster: {}", e
                    ))) {
                        eprintln!("Failed to send cluster connection error: {}", send_err);
                    }
                    (None, ticket_cap)
                }
            }
        }
        Err(e) => {
            if let Err(send_err) = event_tx.send(SyncEvent::Error(format!(
                "Invalid cluster ticket: {}", e
            ))) {
                eprintln!("Failed to send cluster ticket error: {}", send_err);
            }
            (None, None)
        }
    }
}

/// Connect presence protocol to a peer
async fn connect_presence(
    endpoint: &Endpoint,
    ticket: &str,
    presence_protocol: &PresenceProtocol,
) -> Option<tokio::task::JoinHandle<()>> {
    match decode_ticket(ticket) {
        Ok(endpoint_addr) => {
            match endpoint.connect(endpoint_addr, PresenceProtocol::ALPN).await {
                Ok(conn) => {
                    let presence_clone = presence_protocol.clone();
                    Some(tokio::spawn(async move {
                        if let Err(e) = presence_clone.run_presence_loop(conn).await {
                            eprintln!("Presence loop error: {}", e);
                        }
                    }))
                }
                Err(e) => {
                    eprintln!("Failed to connect presence protocol: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("Invalid ticket for presence connection: {}", e);
            None
        }
    }
}

/// Main async sync loop
async fn run_sync(
    config: SyncConfig,
    event_tx: std_mpsc::Sender<SyncEvent>,
    mut command_rx: tokio_mpsc::Receiver<SyncCommand>,
    endpoint_id_tx: std_mpsc::Sender<String>,
) -> Result<()> {
    let session = setup_sync_session(&config, &endpoint_id_tx, &event_tx).await?;
    let SyncSession {
        endpoint,
        store,
        doc_id,
        local_peer_id,
        presence_protocol,
        mut store_change_rx,
        mut presence_rx,
        router,
    } = session;
    
    debug_assert!(!local_peer_id.as_bytes().is_empty(), "local_peer_id must be valid");
    
    match config.mode {
        SyncMode::Active { join_ticket } => {
            // Shutdown flag for clean periodic sync termination
            let shutting_down = Arc::new(AtomicBool::new(false));

            // Establish a persistent sync connection to the peer (if joining)
            let mut sync_conn: Option<Connection> = if let Some(ref remote_ticket) = join_ticket {
                connect_to_peer(&endpoint, remote_ticket, &event_tx).await
            } else {
                None
            };

            // Capability token for authenticating with the cluster. Mutable so
            // ConnectCluster can replace it mid-session.
            let mut cluster_cap: Option<CapabilityToken> = config.cluster_capability.clone();

            // Establish a persistent connection to the aspen cluster node (if configured).
            let mut cluster_conn: Option<Connection> = if let Some(ref cluster_ticket) = config.cluster_ticket {
                let (conn, ticket_cap) = connect_to_cluster(&endpoint, cluster_ticket, &event_tx).await;
                if ticket_cap.is_some() {
                    cluster_cap = ticket_cap;
                }
                conn
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
            let mut presence_handle = if let Some(ref remote_ticket) = join_ticket {
                connect_presence(&endpoint, remote_ticket, &presence_protocol).await
            } else {
                None
            };

            // Periodic peer sync (every 1s) to pull remote changes from the host.
            // The joiner pushes immediately on SyncDoc commands, but the host has no
            // outgoing connection, so the joiner must periodically poll to discover
            // host-side changes. Skipped when a SyncDoc push happened recently to
            // avoid redundant round-trips during active editing.
            let mut sync_interval = tokio::time::interval(Duration::from_secs(1));
            sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut last_explicit_sync = tokio::time::Instant::now();

            // Cluster sync timer (every 2s) — less frequent since it's for persistence
            let mut cluster_interval = tokio::time::interval(Duration::from_secs(2));
            cluster_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Main loop (bounded for safety)
            const MAX_LOOP_ITERATIONS: u32 = 1_000_000;
            let mut loop_count: u32 = 0;
            
            loop {
                loop_count = loop_count.saturating_add(1);
                if loop_count >= MAX_LOOP_ITERATIONS {
                    eprintln!("Sync loop exceeded max iterations, shutting down");
                    break;
                }
                tokio::select! {
                    // Store notified us of changes from sync
                    Some(_doc_id_str) = store_change_rx.recv() => {
                        if let Some(bytes) = store.get_bytes(&doc_id).await {
                            match Automerge::load(&bytes) {
                                Ok(doc) => {
                                    if let Err(send_err) = event_tx.send(SyncEvent::RemoteChanges {
                                        doc: Box::new(doc),
                                    }) {
                                        eprintln!("Failed to send RemoteChanges event: {}", send_err);
                                    }
                                }
                                Err(e) => {
                                    if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                        format!("Failed to load doc: {}", e),
                                    )) {
                                        eprintln!("Failed to send doc load error: {}", send_err);
                                    }
                                }
                            }
                        }
                    }
                    // Presence messages from peers
                    Some(msg) = presence_rx.recv() => {
                        match msg {
                            PresenceMessage::Update(presence) => {
                                if let Err(e) = event_tx.send(SyncEvent::PresenceUpdate(presence)) {
                                    eprintln!("Failed to send PresenceUpdate event: {}", e);
                                }
                            }
                            PresenceMessage::Leave { peer_id } => {
                                if let Err(e) = event_tx.send(SyncEvent::PresenceRemoved { peer_id }) {
                                    eprintln!("Failed to send PresenceRemoved event: {}", e);
                                }
                            }
                            PresenceMessage::RequestAll => {}
                        }
                    }
                    // Periodic sync with peer — pull remote changes (skipped if recent push)
                    _ = sync_interval.tick() => {
                        let not_shutting_down = !shutting_down.load(Ordering::Relaxed);
                        let not_recent_sync = last_explicit_sync.elapsed() >= Duration::from_millis(500);
                        let has_connection = sync_conn.is_some();
                        
                        if not_shutting_down && not_recent_sync && has_connection {
                            let Some(ref conn) = sync_conn else { continue; };
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
                    // Periodic sync with cluster (less frequent, for persistence)
                    _ = cluster_interval.tick() => {
                        let not_shutting_down = !shutting_down.load(Ordering::Relaxed);
                        let has_cluster = cluster_conn.is_some();
                        
                        if not_shutting_down && has_cluster {
                            let Some(ref conn) = cluster_conn else { continue; };
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
                    // Commands from main thread (immediate, no polling delay)
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(SyncCommand::SyncDoc { doc }) => {
                                // Update the local store (no notification to avoid feedback loop)
                                let bytes = doc.save();
                                if let Err(e) = store.update_from_app(&doc_id, &bytes).await {
                                    if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                        format!("Failed to update store: {}", e),
                                    )) {
                                        eprintln!("Failed to send store update error: {}", send_err);
                                    }
                                }

                                // Record that we just synced (suppresses redundant periodic pull)
                                last_explicit_sync = tokio::time::Instant::now();

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
                            Some(SyncCommand::BroadcastPresence(presence)) => {
                                presence_protocol.broadcast(presence);
                            }
                            Some(SyncCommand::ConnectCluster { ticket, capability }) => {
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
                                                if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                                    format!("Failed to connect to cluster: {}", e),
                                                )) {
                                                    eprintln!("Failed to send cluster connect error: {}", send_err);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                            format!("Invalid cluster ticket: {}", e),
                                        )) {
                                            eprintln!("Failed to send cluster ticket error: {}", send_err);
                                        }
                                    }
                                }
                            }
                            Some(SyncCommand::ConnectPeer { ticket }) => {
                                match decode_ticket(&ticket) {
                                    Ok(addr) => {
                                        // Close old presence connection if exists
                                        if let Some(h) = &presence_handle {
                                            h.abort();
                                        }
                                        // Connect to new peer for sync
                                        match endpoint.connect(addr.clone(), AUTOMERGE_SYNC_ALPN).await {
                                            Ok(conn) => {
                                                // Pull existing state from peer
                                                do_sync(&store, &doc_id, &conn, None).await;
                                                sync_conn = Some(conn);
                                            }
                                            Err(e) => {
                                                if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                                    format!("Failed to connect to peer: {}", e),
                                                )) {
                                                    eprintln!("Failed to send peer connect error: {}", send_err);
                                                }
                                                // Don't return early, still try to connect presence
                                            }
                                        }
                                        // Connect presence protocol on a separate connection
                                        match endpoint.connect(addr, PresenceProtocol::ALPN).await {
                                            Ok(conn) => {
                                                let presence_clone = presence_protocol.clone();
                                                presence_handle = Some(tokio::spawn(async move {
                                                    if let Err(e) = presence_clone.run_presence_loop(conn).await {
                                                        eprintln!("Presence loop error: {}", e);
                                                    }
                                                }));
                                            }
                                            Err(e) => {
                                                if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                                    format!("Failed to connect presence: {}", e),
                                                )) {
                                                    eprintln!("Failed to send presence connect error: {}", send_err);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        if let Err(send_err) = event_tx.send(SyncEvent::Error(
                                            format!("Invalid peer ticket: {}", e),
                                        )) {
                                            eprintln!("Failed to send peer ticket error: {}", send_err);
                                        }
                                    }
                                }
                            }
                            Some(SyncCommand::Shutdown) | None => {
                                // Shutdown command or channel closed (main thread dropped sender)
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
                        }
                    }
                }
            }
        }
        SyncMode::Disabled => {
            // Bounded loop for disabled mode
            const MAX_DISABLED_ITERATIONS: u32 = 100_000;
            let mut iteration_count: u32 = 0;
            
            loop {
                iteration_count = iteration_count.saturating_add(1);
                if iteration_count >= MAX_DISABLED_ITERATIONS {
                    eprintln!("Disabled sync loop exceeded max iterations");
                    break;
                }
                
                match command_rx.recv().await {
                    Some(SyncCommand::Shutdown) | None => break,
                    _ => {}
                }
            }
        }
    }

    router.shutdown().await?;
    Ok(())
}
