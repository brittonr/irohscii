//! Core types, shapes, layers, and CRDT document for irohscii.
//!
//! This crate provides the fundamental types and document structure for
//! collaborative ASCII art editing:
//!
//! - Shape types and rendering data
//! - Layer management
//! - CRDT document backed by Automerge
//! - ID types for shapes, layers, groups, and documents

mod document;
mod layers;
mod shapes;

pub use document::{Document, DocumentId, Group, GroupId, ShapeId, default_storage_path};
pub use layers::{Layer, LayerId};
pub use shapes::{
    CachedShape, ResizeHandle, ResizeHandleInfo, ShapeColor, ShapeKind, ShapeView, SnapPoint,
    find_corresponding_snap, flip_horizontal, flip_vertical, resize_shape, rotate_90_ccw,
    rotate_90_cw,
};

// Re-export geometry types for convenience
pub use irohscii_geometry::{LineStyle, Position, Viewport};

// Compile-time assertions for core type properties
const _: () = {
    // Ensure ID types are reasonably sized (should be 16 bytes for UUID)
    const EXPECTED_UUID_SIZE: usize = 16;
    assert!(std::mem::size_of::<LayerId>() == EXPECTED_UUID_SIZE);
    assert!(std::mem::size_of::<DocumentId>() == EXPECTED_UUID_SIZE);
    assert!(std::mem::size_of::<GroupId>() == EXPECTED_UUID_SIZE);
    assert!(std::mem::size_of::<ShapeId>() == EXPECTED_UUID_SIZE);
};

// Compile-time assertions for type safety guarantees
const _: () = {
    // Core types must be Send + Sync for thread safety
    const fn assert_send_sync<T: Send + Sync>() {}
    
    assert_send_sync::<LayerId>();
    assert_send_sync::<DocumentId>();
    assert_send_sync::<GroupId>();
    assert_send_sync::<ShapeId>();
    assert_send_sync::<Layer>();
    assert_send_sync::<Document>();
};
