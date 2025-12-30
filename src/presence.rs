//! Presence tracking for remote cursor/activity visibility
//!
//! This module handles ephemeral presence data (cursor positions, selections, activities)
//! which is synced separately from the automerge document for efficiency.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::canvas::Position;
use crate::document::ShapeId;

/// Staleness threshold - remove cursors not updated in 5 seconds
const STALE_THRESHOLD: Duration = Duration::from_secs(5);

/// Color palette for remote cursors (8 distinct colors)
pub const PEER_COLORS: &[Color] = &[
    Color::Red,
    Color::Green,
    Color::Blue,
    Color::Magenta,
    Color::LightRed,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightMagenta,
];

/// Unique peer identifier (derived from iroh PublicKey)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes[..32]);
            Some(Self(arr))
        } else {
            None
        }
    }
}

/// Tool kind for presence (simplified from Tool enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolKind {
    Select,
    Freehand,
    Text,
    Line,
    Arrow,
    Rectangle,
    DoubleBox,
    Diamond,
    Ellipse,
    Triangle,
    Parallelogram,
    Hexagon,
    Trapezoid,
    RoundedRect,
    Cylinder,
    Cloud,
    Star,
}

/// Cursor activity state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorActivity {
    /// Idle - cursor visible but not actively doing anything
    Idle,
    /// Drawing a shape (with preview bounds)
    Drawing {
        tool: ToolKind,
        start: Position,
        current: Position,
    },
    /// Has a shape selected
    Selected { shape_id: ShapeId },
    /// Dragging a shape
    Dragging { shape_id: ShapeId },
    /// Resizing a shape
    Resizing { shape_id: ShapeId },
    /// Typing text
    Typing { position: Position },
}

impl CursorActivity {
    /// Get a human-readable activity label
    pub fn label(&self) -> &'static str {
        match self {
            CursorActivity::Idle => "Idle",
            CursorActivity::Drawing { .. } => "Drawing",
            CursorActivity::Selected { .. } => "Selected",
            CursorActivity::Dragging { .. } => "Moving",
            CursorActivity::Resizing { .. } => "Resizing",
            CursorActivity::Typing { .. } => "Typing",
        }
    }
}

/// A peer's presence state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerPresence {
    pub peer_id: PeerId,
    pub cursor_pos: Position,
    pub activity: CursorActivity,
    pub color_index: u8,
    pub timestamp_ms: u64,
}

impl PeerPresence {
    pub fn new(peer_id: PeerId, cursor_pos: Position, activity: CursorActivity) -> Self {
        let color_index = peer_id.0[0] % (PEER_COLORS.len() as u8);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            peer_id,
            cursor_pos,
            activity,
            color_index,
            timestamp_ms,
        }
    }

    /// Get a short display name from the peer ID (first 4 hex chars)
    pub fn display_name(&self) -> String {
        format!(
            "Peer-{:02x}{:02x}",
            self.peer_id.0[0],
            self.peer_id.0[1]
        )
    }
}

/// Presence message types for network protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresenceMessage {
    /// Full presence update
    Update(PeerPresence),
    /// Peer leaving gracefully
    Leave { peer_id: PeerId },
    /// Request all peers to send their presence (on connect)
    RequestAll,
}

/// Get color for a peer based on their color index
pub fn peer_color(presence: &PeerPresence) -> Color {
    PEER_COLORS[presence.color_index as usize % PEER_COLORS.len()]
}

/// Manages all peer presence states
#[derive(Debug)]
pub struct PresenceManager {
    /// Our own peer ID
    local_peer_id: PeerId,
    /// Remote peer presences, keyed by peer ID
    peers: HashMap<PeerId, (PeerPresence, Instant)>,
}

impl PresenceManager {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            peers: HashMap::new(),
        }
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Update or add a peer's presence
    pub fn update_peer(&mut self, presence: PeerPresence) {
        // Don't store our own presence
        if presence.peer_id != self.local_peer_id {
            self.peers.insert(presence.peer_id, (presence, Instant::now()));
        }
    }

    /// Remove a peer (graceful disconnect)
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    /// Remove stale peers (not updated recently)
    pub fn prune_stale(&mut self) {
        let now = Instant::now();
        self.peers.retain(|_, (_, last_update)| {
            now.duration_since(*last_update) < STALE_THRESHOLD
        });
    }

    /// Get all active peer presences for rendering
    pub fn active_peers(&self) -> impl Iterator<Item = &PeerPresence> {
        self.peers.values().map(|(p, _)| p)
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}
