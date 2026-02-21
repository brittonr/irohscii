//! Peer identification types.

use serde::{Deserialize, Serialize};

/// Unique peer identifier (derived from iroh PublicKey).
///
/// This is a 32-byte identifier that uniquely identifies each peer in the network.
/// It's derived from the peer's cryptographic public key, ensuring global uniqueness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    /// Create a PeerId from raw bytes.
    ///
    /// Returns `None` if the slice is shorter than 32 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        const PEER_ID_SIZE: usize = 32;
        if bytes.len() >= PEER_ID_SIZE {
            let mut arr = [0u8; PEER_ID_SIZE];
            arr.copy_from_slice(&bytes[..PEER_ID_SIZE]);
            Some(Self(arr))
        } else {
            None
        }
    }

    /// Get a short display name from the peer ID (first 4 hex chars).
    ///
    /// Useful for UI display when full IDs are too long.
    pub fn short_name(&self) -> String {
        format!("{:02x}{:02x}", self.0[0], self.0[1])
    }

    /// Get a color index deterministically derived from this peer ID.
    ///
    /// Useful for assigning consistent colors to peers in collaborative UIs.
    /// The index is guaranteed to be in range `0..palette_size`.
    pub fn color_index(&self, palette_size: usize) -> usize {
        debug_assert!(palette_size > 0, "palette_size must be positive");
        usize::from(self.0[0]) % palette_size
    }

    /// Get the raw bytes of this peer ID.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display first 8 hex chars for readability
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_from_bytes_valid() {
        let bytes = [42u8; 32];
        let peer_id = PeerId::from_bytes(&bytes);
        assert!(peer_id.is_some());
        assert_eq!(peer_id.expect("should parse valid bytes").0, bytes);
    }

    #[test]
    fn peer_id_from_bytes_too_short() {
        let bytes = [42u8; 16];
        let peer_id = PeerId::from_bytes(&bytes);
        assert!(peer_id.is_none());
    }

    #[test]
    fn peer_id_from_bytes_longer_ok() {
        let bytes = [42u8; 64];
        let peer_id = PeerId::from_bytes(&bytes);
        assert!(peer_id.is_some());
        // Should only use first 32 bytes
        assert_eq!(peer_id.expect("should parse longer bytes").0, [42u8; 32]);
    }

    #[test]
    fn peer_id_short_name() {
        let peer_id = PeerId([
            0xAB, 0xCD, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);
        assert_eq!(peer_id.short_name(), "abcd");
    }

    #[test]
    fn peer_id_color_index_deterministic() {
        let peer_id = PeerId([42u8; 32]);
        let idx1 = peer_id.color_index(8);
        let idx2 = peer_id.color_index(8);
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn peer_id_color_index_within_bounds() {
        for i in 0u8..=255 {
            let peer_id = PeerId([i; 32]);
            let idx = peer_id.color_index(8);
            assert!(idx < 8);
        }
    }

    #[test]
    fn peer_id_display() {
        let peer_id = PeerId([
            0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);
        assert_eq!(format!("{}", peer_id), "deadbeef");
    }

    #[test]
    fn peer_id_serialization_roundtrip() {
        let peer_id = PeerId([42u8; 32]);
        let bytes = rmp_serde::to_vec(&peer_id).expect("serialization should succeed");
        let decoded: PeerId = rmp_serde::from_slice(&bytes).expect("deserialization should succeed");
        assert_eq!(peer_id, decoded);
    }
}
