//! Layer types for organizing shapes in the document.
//!
//! Layers provide visibility and lock controls, with layer-first rendering
//! (shapes on higher layers always appear above lower layers).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Layer identifier - UUID for global uniqueness (CRDT-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Layer data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_id(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new("Layer 1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_id_new_unique() {
        let id1 = LayerId::new();
        let id2 = LayerId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn layer_id_default() {
        let id1 = LayerId::default();
        let id2 = LayerId::default();
        assert_ne!(id1, id2); // Each default creates a new UUID
    }

    #[test]
    fn layer_id_display() {
        let id = LayerId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert!(uuid::Uuid::parse_str(&s).is_ok());
    }

    #[test]
    fn layer_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = LayerId(uuid);
        let id2 = LayerId(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn layer_new() {
        let layer = Layer::new("Test Layer");
        assert_eq!(layer.name, "Test Layer");
        assert!(layer.visible);
        assert!(!layer.locked);
    }

    #[test]
    fn layer_with_id() {
        let id = LayerId::new();
        let layer = Layer::with_id(id, "Custom");
        assert_eq!(layer.id, id);
        assert_eq!(layer.name, "Custom");
        assert!(layer.visible);
        assert!(!layer.locked);
    }

    #[test]
    fn layer_default() {
        let layer = Layer::default();
        assert_eq!(layer.name, "Layer 1");
        assert!(layer.visible);
        assert!(!layer.locked);
    }

    #[test]
    fn layer_id_hash() {
        use std::collections::HashSet;
        let id1 = LayerId::new();
        let id2 = LayerId::new();
        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id1); // duplicate
        set.insert(id2);
        assert_eq!(set.len(), 2);
    }
}
