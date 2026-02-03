//! P2P synchronization infrastructure using Iroh networking.
//!
//! This module provides the main entry points for collaborative applications:
//! - [`start_collab`]: Start a collaboration session
//! - [`CollabHandle`]: Handle for communicating with the sync thread
//! - [`CollabConfig`]: Configuration for sync behavior

pub mod presence_protocol;
pub mod protocol;

use std::sync::mpsc as std_mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use iroh::Endpoint;
use iroh::discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher};
use iroh_base::EndpointAddr;
use tokio::sync::mpsc as tokio_mpsc;

use crate::{PeerId, PresenceData, PresenceMessage, SyncableDocument};
use presence_protocol::PresenceProtocol;
use protocol::DocumentProtocol;

/// Configuration for collaboration behavior.
#[derive(Debug, Clone)]
pub struct CollabConfig {
    /// The collaboration mode.
    pub mode: CollabMode,
    /// Disable DNS/Pkarr discovery (useful for test isolation).
    pub disable_discovery: bool,
    /// Custom ALPN protocol identifier for document sync.
    /// Defaults to "iroh-collab/doc/1".
    pub doc_alpn: Option<Vec<u8>>,
    /// Custom ALPN protocol identifier for presence sync.
    /// Defaults to "iroh-collab/presence/1".
    pub presence_alpn: Option<Vec<u8>>,
}

impl Default for CollabConfig {
    fn default() -> Self {
        Self {
            mode: CollabMode::Active { join_ticket: None },
            disable_discovery: false,
            doc_alpn: None,
            presence_alpn: None,
        }
    }
}

impl CollabConfig {
    /// Create a new config that will host a session (others can join us).
    pub fn host() -> Self {
        Self::default()
    }

    /// Create a new config that will join an existing session.
    pub fn join(ticket: impl Into<String>) -> Self {
        Self {
            mode: CollabMode::Active {
                join_ticket: Some(ticket.into()),
            },
            ..Default::default()
        }
    }

    /// Create a config with sync disabled (offline mode).
    pub fn disabled() -> Self {
        Self {
            mode: CollabMode::Disabled,
            ..Default::default()
        }
    }

    /// Set custom ALPN identifiers for the protocols.
    ///
    /// Useful for application-specific protocol versioning.
    pub fn with_alpn(mut self, doc_alpn: &[u8], presence_alpn: &[u8]) -> Self {
        self.doc_alpn = Some(doc_alpn.to_vec());
        self.presence_alpn = Some(presence_alpn.to_vec());
        self
    }

    /// Disable discovery (for testing).
    pub fn without_discovery(mut self) -> Self {
        self.disable_discovery = true;
        self
    }
}

/// Collaboration mode.
#[derive(Debug, Clone)]
pub enum CollabMode {
    /// No syncing, standalone mode.
    Disabled,
    /// Active sync - always accept connections, optionally join a peer.
    Active {
        /// Ticket to join an existing session, or None to host.
        join_ticket: Option<String>,
    },
}

/// Events from the sync thread to the main thread.
#[derive(Debug)]
pub enum CollabEvent<D, P> {
    /// Connection established, here's our endpoint ID and peer ID for sharing.
    Ready {
        /// The ticket string that others can use to join this session.
        ticket: String,
        /// Our local peer ID.
        local_peer_id: PeerId,
    },
    /// Remote document changes received.
    DocumentChanged {
        /// The updated document after merging remote changes.
        doc: Box<D>,
    },
    /// Presence update from a remote peer.
    PresenceUpdate(P),
    /// A peer's presence was removed (disconnect or leave).
    PresenceRemoved {
        /// The peer ID that left.
        peer_id: PeerId,
    },
    /// An error occurred in the sync thread.
    Error(String),
}

/// Commands from the main thread to the sync thread.
#[derive(Debug)]
pub enum CollabCommand<D, P> {
    /// Sync local document state to all peers.
    SyncDocument {
        /// The current document state.
        doc: Box<D>,
    },
    /// Broadcast local presence to all peers.
    BroadcastPresence(P),
    /// Shutdown the sync thread.
    Shutdown,
}

/// Handle for communicating with the sync thread from the main thread.
pub struct CollabHandle<D, P> {
    /// Channel to send commands to sync thread.
    command_tx: std_mpsc::Sender<CollabCommand<D, P>>,
    /// Channel to receive events from sync thread.
    event_rx: std_mpsc::Receiver<CollabEvent<D, P>>,
    /// Our session ticket (for others to join).
    pub ticket: Option<String>,
    /// Thread handle.
    _thread: JoinHandle<()>,
}

impl<D: SyncableDocument, P: PresenceData> CollabHandle<D, P> {
    /// Non-blocking check for sync events.
    pub fn poll_event(&self) -> Option<CollabEvent<D, P>> {
        self.event_rx.try_recv().ok()
    }

    /// Blocking wait for the next event (with timeout).
    pub fn recv_event_timeout(&self, timeout: Duration) -> Option<CollabEvent<D, P>> {
        self.event_rx.recv_timeout(timeout).ok()
    }

    /// Send a command to the sync thread.
    pub fn send_command(&self, cmd: CollabCommand<D, P>) -> Result<()> {
        self.command_tx.send(cmd)?;
        Ok(())
    }

    /// Convenience: sync document to all peers.
    pub fn sync_document(&self, doc: D) -> Result<()> {
        self.send_command(CollabCommand::SyncDocument { doc: Box::new(doc) })
    }

    /// Convenience: broadcast presence to all peers.
    pub fn broadcast_presence(&self, presence: P) -> Result<()> {
        self.send_command(CollabCommand::BroadcastPresence(presence))
    }

    /// Request shutdown of the sync thread.
    pub fn shutdown(&self) -> Result<()> {
        self.send_command(CollabCommand::Shutdown)
    }

    /// Get the ticket for others to join this session.
    pub fn ticket(&self) -> Option<&str> {
        self.ticket.as_deref()
    }
}

/// Start a collaboration session with the given configuration.
///
/// Returns a handle for communicating with the sync thread.
///
/// # Type Parameters
///
/// - `D`: The document type implementing [`SyncableDocument`]
/// - `P`: The presence type implementing [`PresenceData`]
///
/// # Example
///
/// ```ignore
/// let config = CollabConfig::host();
/// let handle = start_collab::<MyDocument, MyPresence>(config, initial_doc)?;
///
/// // Share the ticket with collaborators
/// if let Some(ticket) = handle.ticket() {
///     println!("Join with: {}", ticket);
/// }
///
/// // Main loop
/// loop {
///     if let Some(event) = handle.poll_event() {
///         match event {
///             CollabEvent::DocumentChanged { doc } => { ... }
///             CollabEvent::PresenceUpdate(presence) => { ... }
///             _ => {}
///         }
///     }
///
///     // Sync local changes
///     handle.sync_document(my_doc.clone())?;
/// }
/// ```
pub fn start_collab<D, P>(config: CollabConfig, initial_doc: D) -> Result<CollabHandle<D, P>>
where
    D: SyncableDocument,
    P: PresenceData,
{
    let (event_tx, event_rx) = std_mpsc::channel();
    let (command_tx, command_rx) = std_mpsc::channel();
    let (ticket_tx, ticket_rx) = std_mpsc::channel();

    let doc_alpn = config
        .doc_alpn
        .clone()
        .unwrap_or_else(|| b"iroh-collab/doc/1".to_vec());
    let presence_alpn = config
        .presence_alpn
        .clone()
        .unwrap_or_else(|| b"iroh-collab/presence/1".to_vec());

    let thread = thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = event_tx.send(CollabEvent::Error(format!(
                    "Failed to create tokio runtime: {}",
                    e
                )));
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = run_collab::<D, P>(
                config,
                initial_doc,
                event_tx.clone(),
                command_rx,
                ticket_tx,
                doc_alpn,
                presence_alpn,
            )
            .await
            {
                let _ = event_tx.send(CollabEvent::Error(e.to_string()));
            }
        });
    });

    // Wait for ticket (with timeout)
    let ticket = ticket_rx.recv_timeout(Duration::from_secs(10)).ok();

    Ok(CollabHandle {
        command_tx,
        event_rx,
        ticket,
        _thread: thread,
    })
}

/// Encode an EndpointAddr as a shareable ticket string.
pub fn encode_ticket(addr: &EndpointAddr, prefix: &str) -> String {
    let bytes = postcard::to_stdvec(addr).expect("EndpointAddr serialization should not fail");
    format!("{}{}", prefix, data_encoding::BASE32_NOPAD.encode(&bytes))
}

/// Decode a ticket string back to EndpointAddr.
pub fn decode_ticket(ticket: &str, prefix: &str) -> Result<EndpointAddr> {
    if let Some(data) = ticket.strip_prefix(prefix) {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(data.as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid ticket encoding: {}", e))?;
        let addr: EndpointAddr = postcard::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Invalid ticket data: {}", e))?;
        Ok(addr)
    } else {
        // Try parsing as bare PublicKey for backwards compatibility
        let id: iroh_base::PublicKey = ticket
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid endpoint ID: {}", e))?;
        Ok(EndpointAddr::new(id))
    }
}

/// Main async collaboration loop.
async fn run_collab<D, P>(
    config: CollabConfig,
    initial_doc: D,
    event_tx: std_mpsc::Sender<CollabEvent<D, P>>,
    command_rx: std_mpsc::Receiver<CollabCommand<D, P>>,
    ticket_tx: std_mpsc::Sender<String>,
    doc_alpn: Vec<u8>,
    presence_alpn: Vec<u8>,
) -> Result<()>
where
    D: SyncableDocument,
    P: PresenceData,
{
    // Create iroh endpoint with optional discovery
    let mut builder = Endpoint::builder();
    if !config.disable_discovery {
        builder = builder
            .discovery(DnsDiscovery::n0_dns())
            .discovery(PkarrPublisher::n0_dns());
    }
    let endpoint = builder.bind().await?;

    // Create ticket from endpoint address
    let addr = endpoint.addr();
    let ticket_string = encode_ticket(&addr, "collab1");

    // Derive local peer ID from endpoint's public key
    let public_key = endpoint.id();
    let local_peer_id = PeerId::from_bytes(public_key.as_bytes())
        .ok_or_else(|| anyhow::anyhow!("PublicKey must be 32 bytes"))?;

    // Send ticket back to main thread
    let _ = ticket_tx.send(ticket_string.clone());

    // Create document protocol handler
    let (doc_tx, mut doc_rx) = tokio_mpsc::channel(10);
    let doc_protocol = DocumentProtocol::new(initial_doc, doc_tx, doc_alpn.clone());

    // Create presence protocol handler
    let (presence_tx, mut presence_rx) = tokio_mpsc::channel(64);
    let presence_protocol =
        PresenceProtocol::<P>::new(local_peer_id, presence_tx, presence_alpn.clone());

    // Build router with both protocols
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(doc_alpn.clone(), doc_protocol.clone())
        .accept(presence_alpn.clone(), presence_protocol.clone())
        .spawn();

    // Notify that we're ready
    let _ = event_tx.send(CollabEvent::Ready {
        ticket: ticket_string.clone(),
        local_peer_id,
    });

    match config.mode {
        CollabMode::Active { join_ticket } => {
            // Connect to remote peer if we have a ticket
            let doc_handle = if let Some(ref remote_ticket) = join_ticket {
                let endpoint_addr = decode_ticket(remote_ticket, "collab1")?;
                let conn = endpoint.connect(endpoint_addr.clone(), &doc_alpn).await?;

                let protocol_clone = doc_protocol.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = protocol_clone.run_sync_loop(conn).await {
                        eprintln!("Document sync loop error: {}", e);
                    }
                }))
            } else {
                None
            };

            // Connect presence protocol if joining
            let presence_handle = if let Some(ref remote_ticket) = join_ticket {
                let endpoint_addr = decode_ticket(remote_ticket, "collab1")?;
                let conn = endpoint.connect(endpoint_addr, &presence_alpn).await?;

                let presence_clone = presence_protocol.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = presence_clone.run_presence_loop(conn).await {
                        eprintln!("Presence loop error: {}", e);
                    }
                }))
            } else {
                None
            };

            // Main event loop
            loop {
                tokio::select! {
                    Some(doc) = doc_rx.recv() => {
                        let _ = event_tx.send(CollabEvent::DocumentChanged { doc: Box::new(doc) });
                    }
                    Some(msg) = presence_rx.recv() => {
                        match msg {
                            PresenceMessage::Update(presence) => {
                                let _ = event_tx.send(CollabEvent::PresenceUpdate(presence));
                            }
                            PresenceMessage::Leave { peer_id } => {
                                let _ = event_tx.send(CollabEvent::PresenceRemoved { peer_id });
                            }
                            PresenceMessage::RequestAll => {
                                // Handled internally by presence protocol
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        match command_rx.try_recv() {
                            Ok(CollabCommand::SyncDocument { doc }) => {
                                if let Err(e) = doc_protocol.merge_and_notify(&doc).await {
                                    let _ = event_tx.send(CollabEvent::Error(e.to_string()));
                                }
                            }
                            Ok(CollabCommand::BroadcastPresence(presence)) => {
                                presence_protocol.broadcast(presence);
                            }
                            Ok(CollabCommand::Shutdown) => {
                                presence_protocol.broadcast_leave();
                                if let Some(h) = &doc_handle {
                                    h.abort();
                                }
                                if let Some(h) = &presence_handle {
                                    h.abort();
                                }
                                break;
                            }
                            Err(std_mpsc::TryRecvError::Empty) => {}
                            Err(std_mpsc::TryRecvError::Disconnected) => {
                                if let Some(h) = &doc_handle {
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
        CollabMode::Disabled => {
            // Wait for shutdown command
            loop {
                if let Ok(CollabCommand::Shutdown) = command_rx.recv() {
                    break;
                }
            }
        }
    }

    router.shutdown().await?;
    Ok(())
}
