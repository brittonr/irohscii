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
        let name = name.into();
        debug_assert!(!name.is_empty(), "Layer name should not be empty");
        
        let layer = Self {
            id: LayerId::new(),
            name,
            visible: true,
            locked: false,
        };
        
        debug_assert!(layer.visible, "New layer should be visible by default");
        debug_assert!(!layer.locked, "New layer should be unlocked by default");
        layer
    }

    #[allow(dead_code)]
    pub fn with_id(id: LayerId, name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "Layer name should not be empty");
        
        let layer = Self {
            id,
            name,
            visible: true,
            locked: false,
        };
        
        debug_assert!(layer.visible, "New layer should be visible by default");
        debug_assert!(!layer.locked, "New layer should be unlocked by default");
        layer
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
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn layer_id_default() {
        let id1 = LayerId::default();
        let id2 = LayerId::default();
        assert_ne!(id1, id2); // Each default creates a new UUID
        assert_ne!(id1.0, id2.0);
        // Verify UUIDs are version 4 (random)
        assert_eq!(id1.0.get_version_num(), 4);
        assert_eq!(id2.0.get_version_num(), 4);
    }

    #[test]
    fn layer_id_display() {
        let id = LayerId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert_eq!(s.len(), 36); // UUID standard string length
        let parsed = uuid::Uuid::parse_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(parsed.expect("UUID should parse successfully"), id.0);
    }

    #[test]
    fn layer_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = LayerId(uuid);
        let id2 = LayerId(uuid);
        assert_eq!(id1, id2);
        assert_eq!(id1.0, id2.0);
        assert_eq!(id1.0, uuid);
        assert_eq!(id2.0, uuid);
    }

    #[test]
    fn layer_new() {
        let layer = Layer::new("Test Layer");
        assert_eq!(layer.name, "Test Layer");
        assert!(!layer.name.is_empty());
        assert!(layer.visible);
        assert_eq!(layer.visible, true);
        assert!(!layer.locked);
        assert_eq!(layer.locked, false);
        // Verify ID was created
        assert_eq!(layer.id.0.get_version_num(), 4);
    }

    #[test]
    fn layer_with_id() {
        let id = LayerId::new();
        let layer = Layer::with_id(id, "Custom");
        assert_eq!(layer.id, id);
        assert_eq!(layer.id.0, id.0);
        assert_eq!(layer.name, "Custom");
        assert!(!layer.name.is_empty());
        assert!(layer.visible);
        assert_eq!(layer.visible, true);
        assert!(!layer.locked);
        assert_eq!(layer.locked, false);
    }

    #[test]
    fn layer_default() {
        let layer = Layer::default();
        assert_eq!(layer.name, "Layer 1");
        assert!(!layer.name.is_empty());
        assert!(layer.visible);
        assert_eq!(layer.visible, true);
        assert!(!layer.locked);
        assert_eq!(layer.locked, false);
        // Verify ID was created
        assert_eq!(layer.id.0.get_version_num(), 4);
    }

    #[test]
    fn layer_id_hash() {
        use std::collections::HashSet;
        let id1 = LayerId::new();
        let id2 = LayerId::new();
        let mut set = HashSet::new();
        
        let inserted1 = set.insert(id1);
        assert!(inserted1);
        assert_eq!(set.len(), 1);
        
        let inserted_dup = set.insert(id1); // duplicate
        assert!(!inserted_dup);
        assert_eq!(set.len(), 1);
        
        let inserted2 = set.insert(id2);
        assert!(inserted2);
        assert_eq!(set.len(), 2);
        
        assert!(set.contains(&id1));
        assert!(set.contains(&id2));
    }

    #[test]
    fn layer_name_conversion() {
        let layer1 = Layer::new("Static str");
        assert_eq!(layer1.name, "Static str");
        assert!(!layer1.name.is_empty());
        
        let layer2 = Layer::new(String::from("String type"));
        assert_eq!(layer2.name, "String type");
        assert!(!layer2.name.is_empty());
        
        let owned = String::from("Owned");
        let layer3 = Layer::new(owned);
        assert_eq!(layer3.name, "Owned");
        assert!(!layer3.name.is_empty());
    }

    #[test]
    fn layer_clone() {
        let layer1 = Layer::new("Original");
        let layer2 = layer1.clone();
        
        assert_eq!(layer1.id, layer2.id);
        assert_eq!(layer1.name, layer2.name);
        assert_eq!(layer1.visible, layer2.visible);
        assert_eq!(layer1.locked, layer2.locked);
        
        // Verify they're independent after clone
        assert_ne!(&layer1 as *const _, &layer2 as *const _);
    }

    #[test]
    fn layer_id_copy() {
        let id1 = LayerId::new();
        let id2 = id1; // Should copy, not move
        assert_eq!(id1, id2);
        assert_eq!(id1.0, id2.0);
        // Verify we can still use id1
        let _display = format!("{}", id1);
    }

    #[test]
    fn layer_state_defaults() {
        let layer = Layer::new("Test");
        // Verify all default states
        assert!(layer.visible, "New layers must be visible");
        assert!(!layer.locked, "New layers must be unlocked");
        assert!(!layer.name.is_empty(), "Layer name must not be empty");
    }
}
