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

pub use document::{default_storage_path, Document, DocumentId, Group, GroupId, ShapeId};
pub use layers::{Layer, LayerId};
pub use shapes::{
    find_corresponding_snap, flip_horizontal, flip_vertical, resize_shape, rotate_90_ccw,
    rotate_90_cw, CachedShape, ResizeHandle, ResizeHandleInfo, ShapeColor, ShapeKind, ShapeView,
    SnapPoint,
};

// Re-export geometry types for convenience
pub use irohscii_geometry::{LineStyle, Position, Viewport};
