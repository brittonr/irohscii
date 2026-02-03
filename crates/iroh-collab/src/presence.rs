//! Generic presence tracking for collaborative applications.
//!
//! Presence data represents ephemeral, high-frequency state like cursor positions,
//! selection highlights, or activity indicators. It's separate from document sync
//! because it doesn't need CRDT semantics - latest-wins is sufficient.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::PeerId;

/// Default staleness threshold - remove peers not updated in 5 seconds.
pub const DEFAULT_STALE_THRESHOLD: Duration = Duration::from_secs(5);

/// Trait for application-specific presence data.
///
/// Implement this trait to define what presence information your application tracks.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Serialize, Deserialize)]
/// struct EditorPresence {
///     cursor_line: u32,
///     cursor_col: u32,
///     selection: Option<(u32, u32)>,
///     is_typing: bool,
///     timestamp_ms: u64,
/// }
///
/// impl PresenceData for EditorPresence {
///     fn peer_id(&self) -> PeerId { ... }
///     fn timestamp_ms(&self) -> u64 { self.timestamp_ms }
///     fn with_peer_id(self, peer_id: PeerId) -> Self { ... }
/// }
/// ```
pub trait PresenceData: Clone + Send + Sync + Serialize + DeserializeOwned + 'static {
    /// Get the peer ID associated with this presence.
    fn peer_id(&self) -> PeerId;

    /// Get the timestamp in milliseconds since UNIX epoch.
    ///
    /// Used for ordering presence updates and detecting stale data.
    fn timestamp_ms(&self) -> u64;

    /// Create a copy of this presence with a different peer ID.
    ///
    /// Used when forwarding presence from one peer to another.
    fn with_peer_id(self, peer_id: PeerId) -> Self;
}

/// Messages for the presence protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresenceMessage<P> {
    /// Full presence update from a peer.
    Update(P),
    /// Peer is leaving gracefully.
    Leave { peer_id: PeerId },
    /// Request all peers to send their current presence (sent on connect).
    RequestAll,
}

/// Manages presence state for all connected peers.
///
/// Generic over the presence data type `P`, allowing applications to track
/// whatever information is relevant (cursor positions, selections, activity, etc.).
#[derive(Debug)]
pub struct PresenceManager<P> {
    /// Our own peer ID.
    local_peer_id: PeerId,
    /// Remote peer presences, keyed by peer ID.
    /// The Instant tracks when we last received an update.
    peers: HashMap<PeerId, (P, Instant)>,
    /// How long before a peer is considered stale.
    stale_threshold: Duration,
}

impl<P: PresenceData> PresenceManager<P> {
    /// Create a new presence manager for the given local peer.
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            peers: HashMap::new(),
            stale_threshold: DEFAULT_STALE_THRESHOLD,
        }
    }

    /// Create a presence manager with a custom stale threshold.
    pub fn with_stale_threshold(local_peer_id: PeerId, stale_threshold: Duration) -> Self {
        Self {
            local_peer_id,
            peers: HashMap::new(),
            stale_threshold,
        }
    }

    /// Get our local peer ID.
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Update or add a peer's presence.
    ///
    /// Ignores updates for our own peer ID.
    pub fn update_peer(&mut self, presence: P) {
        let peer_id = presence.peer_id();
        if peer_id != self.local_peer_id {
            self.peers.insert(peer_id, (presence, Instant::now()));
        }
    }

    /// Remove a peer (graceful disconnect).
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    /// Remove peers that haven't been updated within the stale threshold.
    pub fn prune_stale(&mut self) {
        let now = Instant::now();
        let threshold = self.stale_threshold;
        self.peers
            .retain(|_, (_, last_update)| now.duration_since(*last_update) < threshold);
    }

    /// Get all active peer presences for rendering.
    pub fn active_peers(&self) -> impl Iterator<Item = &P> {
        self.peers.values().map(|(p, _)| p)
    }

    /// Get a specific peer's presence.
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&P> {
        self.peers.get(peer_id).map(|(p, _)| p)
    }

    /// Get the number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Check if a peer is currently tracked.
    pub fn has_peer(&self, peer_id: &PeerId) -> bool {
        self.peers.contains_key(peer_id)
    }

    /// Find peers matching a predicate.
    pub fn find_peers<F>(&self, predicate: F) -> impl Iterator<Item = &P>
    where
        F: Fn(&P) -> bool,
    {
        self.peers
            .values()
            .map(|(p, _)| p)
            .filter(move |p| predicate(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestPresence {
        peer_id: PeerId,
        cursor_x: i32,
        cursor_y: i32,
        timestamp_ms: u64,
    }

    impl TestPresence {
        fn new(peer_id: PeerId, x: i32, y: i32) -> Self {
            Self {
                peer_id,
                cursor_x: x,
                cursor_y: y,
                timestamp_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            }
        }
    }

    impl PresenceData for TestPresence {
        fn peer_id(&self) -> PeerId {
            self.peer_id
        }

        fn timestamp_ms(&self) -> u64 {
            self.timestamp_ms
        }

        fn with_peer_id(mut self, peer_id: PeerId) -> Self {
            self.peer_id = peer_id;
            self
        }
    }

    #[test]
    fn manager_new() {
        let local_id = PeerId([1u8; 32]);
        let manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);
        assert_eq!(manager.local_peer_id(), local_id);
        assert_eq!(manager.peer_count(), 0);
    }

    #[test]
    fn manager_update_peer() {
        let local_id = PeerId([1u8; 32]);
        let remote_id = PeerId([2u8; 32]);
        let mut manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);

        let presence = TestPresence::new(remote_id, 10, 20);
        manager.update_peer(presence.clone());

        assert_eq!(manager.peer_count(), 1);
        assert!(manager.has_peer(&remote_id));

        let stored = manager.get_peer(&remote_id).unwrap();
        assert_eq!(stored.cursor_x, 10);
        assert_eq!(stored.cursor_y, 20);
    }

    #[test]
    fn manager_ignores_local_peer() {
        let local_id = PeerId([1u8; 32]);
        let mut manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);

        let presence = TestPresence::new(local_id, 10, 20);
        manager.update_peer(presence);

        assert_eq!(manager.peer_count(), 0);
    }

    #[test]
    fn manager_remove_peer() {
        let local_id = PeerId([1u8; 32]);
        let remote_id = PeerId([2u8; 32]);
        let mut manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);

        manager.update_peer(TestPresence::new(remote_id, 10, 20));
        assert_eq!(manager.peer_count(), 1);

        manager.remove_peer(&remote_id);
        assert_eq!(manager.peer_count(), 0);
    }

    #[test]
    fn manager_updates_existing_peer() {
        let local_id = PeerId([1u8; 32]);
        let remote_id = PeerId([2u8; 32]);
        let mut manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);

        manager.update_peer(TestPresence::new(remote_id, 10, 20));
        manager.update_peer(TestPresence::new(remote_id, 30, 40));

        assert_eq!(manager.peer_count(), 1);
        let stored = manager.get_peer(&remote_id).unwrap();
        assert_eq!(stored.cursor_x, 30);
        assert_eq!(stored.cursor_y, 40);
    }

    #[test]
    fn manager_active_peers_iterator() {
        let local_id = PeerId([1u8; 32]);
        let mut manager: PresenceManager<TestPresence> = PresenceManager::new(local_id);

        manager.update_peer(TestPresence::new(PeerId([2u8; 32]), 10, 20));
        manager.update_peer(TestPresence::new(PeerId([3u8; 32]), 30, 40));

        let peers: Vec<_> = manager.active_peers().collect();
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn presence_message_serialization() {
        let peer_id = PeerId([1u8; 32]);
        let presence = TestPresence::new(peer_id, 10, 20);

        let msg: PresenceMessage<TestPresence> = PresenceMessage::Update(presence.clone());
        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded: PresenceMessage<TestPresence> = rmp_serde::from_slice(&bytes).unwrap();

        if let PresenceMessage::Update(p) = decoded {
            assert_eq!(p.cursor_x, 10);
            assert_eq!(p.cursor_y, 20);
        } else {
            panic!("Expected Update message");
        }
    }

    #[test]
    fn presence_message_leave_serialization() {
        let peer_id = PeerId([1u8; 32]);
        let msg: PresenceMessage<TestPresence> = PresenceMessage::Leave { peer_id };

        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded: PresenceMessage<TestPresence> = rmp_serde::from_slice(&bytes).unwrap();

        if let PresenceMessage::Leave {
            peer_id: decoded_id,
        } = decoded
        {
            assert_eq!(decoded_id, peer_id);
        } else {
            panic!("Expected Leave message");
        }
    }
}
