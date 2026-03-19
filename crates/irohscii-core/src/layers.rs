//! Layer types for organizing shapes in the document.
//!
//! Layers provide visibility and lock controls, with layer-first rendering
//! (shapes on higher layers always appear above lower layers).

// Re-export types from rat-layers
pub use rat_layers::{Layer, LayerId};

#[cfg(test)]
use uuid;

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
        assert_eq!(s.len(), 36); // UUID standard string length
        // Should be parseable as UUID
        let parsed = uuid::Uuid::parse_str(&s);
        assert!(parsed.is_ok());
    }

    #[test]
    fn layer_new() {
        let layer = Layer::new("Test Layer");
        assert_eq!(layer.name, "Test Layer");
        assert!(!layer.name.is_empty());
        assert!(layer.visible);
        assert!(!layer.locked);
    }

    #[test]
    fn layer_with_id() {
        let id = LayerId::new();
        let layer = Layer::with_id(id, "Custom");
        assert_eq!(layer.id, id);
        assert_eq!(layer.name, "Custom");
        assert!(!layer.name.is_empty());
        assert!(layer.visible);
        assert!(!layer.locked);
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
    }

    #[test]
    fn layer_id_copy() {
        let id1 = LayerId::new();
        let id2 = id1; // Should copy, not move
        assert_eq!(id1, id2);
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
