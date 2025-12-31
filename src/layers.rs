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
