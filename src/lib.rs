//! irohscii library - ASCII art drawing tool with real-time collaboration
//!
//! This library re-exports from workspace crates for backwards compatibility.

// Re-export core types (shapes, document, layers)
pub use irohscii_core as core;
pub use irohscii_core::{
    flip_horizontal, flip_vertical, resize_shape, rotate_90_ccw, rotate_90_cw, CachedShape,
    Document, DocumentId, Group, GroupId, Layer, LayerId, LineStyle, Position, ResizeHandle,
    ResizeHandleInfo, ShapeColor, ShapeId, ShapeKind, ShapeView, SnapPoint, Viewport,
};

// Re-export geometry functions
pub use irohscii_geometry as geometry;
pub use irohscii_geometry::{
    arrow_points_styled, cloud_points, cylinder_points, diamond_points, double_rect_points,
    ellipse_points, hexagon_points, line_points, line_points_auto_routed, line_points_styled,
    parallelogram_points, rect_points, rounded_rect_points, star_points, trapezoid_points,
    triangle_points,
};

// Re-export sync types
pub use irohscii_sync as sync;
pub use irohscii_sync::{
    decode_ticket, encode_ticket, peer_color, start_sync_thread, CursorActivity, PeerId,
    PeerPresence, PresenceManager, PresenceMessage, SyncCommand, SyncConfig, SyncEvent,
    SyncHandle, SyncMode, ToolKind, PEER_COLORS,
};

// Re-export export functions
pub use irohscii_export as export;
pub use irohscii_export::{export_svg, load_ascii, save_ascii, save_svg};

// Re-export session management
pub use irohscii_session as session;
pub use irohscii_session::{
    Collaborator, SessionId, SessionManager, SessionMeta, SessionRegistry, TicketInfo,
    UndoManager,
};

// Legacy module aliases for backwards compatibility with internal code
pub mod canvas {
    pub use irohscii_geometry::*;
}

pub mod document {
    pub use irohscii_core::{
        default_storage_path, Document, DocumentId, Group, GroupId, ShapeId,
    };
}

pub mod layers {
    pub use irohscii_core::{Layer, LayerId};
}

pub mod shapes {
    pub use irohscii_core::{
        flip_horizontal, flip_vertical, resize_shape, rotate_90_ccw, rotate_90_cw, CachedShape,
        ResizeHandle, ResizeHandleInfo, ShapeColor, ShapeKind, ShapeView, SnapPoint,
        find_corresponding_snap,
    };
}

pub mod presence {
    pub use irohscii_sync::{
        peer_color, CursorActivity, PeerId, PeerPresence, PresenceManager, PresenceMessage,
        ToolKind, PEER_COLORS,
    };
}

pub mod file_io {
    pub use irohscii_export::{load_ascii, save_ascii};
}

pub mod svg_export {
    pub use irohscii_export::{export_svg, save_svg};
}

pub mod undo {
    pub use irohscii_session::UndoManager;
}
