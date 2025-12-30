//! Shape types and rendering cache for irohscii.
//!
//! ShapeKind defines the different shape variants.
//! ShapeView provides a fast read-only cache for rendering.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::canvas::{LineStyle, Position};
use crate::document::{Document, ShapeId};

/// Different types of shapes we can draw
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeKind {
    /// A line from start to end
    Line {
        start: Position,
        end: Position,
        style: LineStyle,
        start_connection: Option<u64>,
        end_connection: Option<u64>,
    },
    /// An arrow (line with arrowhead at end)
    Arrow {
        start: Position,
        end: Position,
        style: LineStyle,
        start_connection: Option<u64>,
        end_connection: Option<u64>,
    },
    /// A rectangle defined by two corners
    Rectangle {
        start: Position,
        end: Position,
        label: Option<String>,
    },
    /// A double-line rectangle
    DoubleBox {
        start: Position,
        end: Position,
        label: Option<String>,
    },
    /// A diamond (rhombus) defined by center and half-dimensions
    Diamond {
        center: Position,
        half_width: i32,
        half_height: i32,
        label: Option<String>,
    },
    /// An ellipse defined by center and radii
    Ellipse {
        center: Position,
        radius_x: i32,
        radius_y: i32,
        label: Option<String>,
    },
    /// Freehand stroke - series of points
    Freehand { points: Vec<Position>, char: char },
    /// Text at a position
    Text { pos: Position, content: String },
}

impl ShapeKind {
    /// Create a translated copy of this shape
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        match self {
            ShapeKind::Line { start, end, style, .. } => ShapeKind::Line {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                style: *style,
                start_connection: None,
                end_connection: None,
            },
            ShapeKind::Arrow { start, end, style, .. } => ShapeKind::Arrow {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                style: *style,
                start_connection: None,
                end_connection: None,
            },
            ShapeKind::Rectangle { start, end, label } => ShapeKind::Rectangle {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
            },
            ShapeKind::DoubleBox { start, end, label } => ShapeKind::DoubleBox {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
            },
            ShapeKind::Diamond { center, half_width, half_height, label } => ShapeKind::Diamond {
                center: Position { x: center.x + dx, y: center.y + dy },
                half_width: *half_width,
                half_height: *half_height,
                label: label.clone(),
            },
            ShapeKind::Ellipse { center, radius_x, radius_y, label } => ShapeKind::Ellipse {
                center: Position { x: center.x + dx, y: center.y + dy },
                radius_x: *radius_x,
                radius_y: *radius_y,
                label: label.clone(),
            },
            ShapeKind::Freehand { points, char } => ShapeKind::Freehand {
                points: points.iter().map(|p| Position { x: p.x + dx, y: p.y + dy }).collect(),
                char: *char,
            },
            ShapeKind::Text { pos, content } => ShapeKind::Text {
                pos: Position { x: pos.x + dx, y: pos.y + dy },
                content: content.clone(),
            },
        }
    }

    /// Get the label for this shape (if it supports labels)
    pub fn label(&self) -> Option<&str> {
        match self {
            ShapeKind::Rectangle { label, .. }
            | ShapeKind::DoubleBox { label, .. }
            | ShapeKind::Diamond { label, .. }
            | ShapeKind::Ellipse { label, .. } => label.as_deref(),
            _ => None,
        }
    }

    /// Set the label for this shape (if it supports labels)
    pub fn with_label(self, new_label: Option<String>) -> Self {
        match self {
            ShapeKind::Rectangle { start, end, .. } => ShapeKind::Rectangle { start, end, label: new_label },
            ShapeKind::DoubleBox { start, end, .. } => ShapeKind::DoubleBox { start, end, label: new_label },
            ShapeKind::Diamond { center, half_width, half_height, .. } => {
                ShapeKind::Diamond { center, half_width, half_height, label: new_label }
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, .. } => {
                ShapeKind::Ellipse { center, radius_x, radius_y, label: new_label }
            }
            other => other,
        }
    }

    /// Check if this shape supports labels
    pub fn supports_label(&self) -> bool {
        matches!(
            self,
            ShapeKind::Rectangle { .. }
                | ShapeKind::DoubleBox { .. }
                | ShapeKind::Diamond { .. }
                | ShapeKind::Ellipse { .. }
        )
    }

    /// Get snap points for this shape (used for connection updates during resize)
    pub fn snap_points(&self) -> Vec<Position> {
        match self {
            ShapeKind::Rectangle { start, end, .. } | ShapeKind::DoubleBox { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                let mid_x = (min_x + max_x) / 2;
                let mid_y = (min_y + max_y) / 2;

                vec![
                    Position::new(min_x, min_y),
                    Position::new(max_x, min_y),
                    Position::new(min_x, max_y),
                    Position::new(max_x, max_y),
                    Position::new(mid_x, min_y),
                    Position::new(mid_x, max_y),
                    Position::new(min_x, mid_y),
                    Position::new(max_x, mid_y),
                ]
            }
            ShapeKind::Line { start, end, .. } | ShapeKind::Arrow { start, end, .. } => {
                vec![*start, *end]
            }
            ShapeKind::Diamond { center, half_width, half_height, .. } => {
                vec![
                    Position::new(center.x, center.y - *half_height),
                    Position::new(center.x, center.y + *half_height),
                    Position::new(center.x - *half_width, center.y),
                    Position::new(center.x + *half_width, center.y),
                ]
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, .. } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Text { pos, content } => {
                let end_x = pos.x + content.len() as i32 - 1;
                vec![*pos, Position::new(end_x, pos.y)]
            }
            ShapeKind::Freehand { points, .. } => {
                let mut snaps = Vec::new();
                if let Some(first) = points.first() {
                    snaps.push(*first);
                }
                if let Some(last) = points.last() {
                    if points.len() > 1 {
                        snaps.push(*last);
                    }
                }
                snaps
            }
        }
    }
}

/// Snap point on a shape edge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapPoint {
    pub pos: Position,
    pub shape_id: ShapeId,
}

/// Handle for resizing shapes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Start,
    End,
}

/// A resize handle with its position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeHandleInfo {
    pub handle: ResizeHandle,
    pub pos: Position,
}

/// A cached shape for fast rendering (immutable view)
#[derive(Debug, Clone)]
pub struct CachedShape {
    pub id: ShapeId,
    pub kind: ShapeKind,
    bounds: (i32, i32, i32, i32),
    snap_points: Vec<Position>,
    resize_handles: Vec<ResizeHandleInfo>,
}

impl CachedShape {
    /// Create a cached shape from id and kind
    pub fn new(id: ShapeId, kind: ShapeKind) -> Self {
        let bounds = Self::compute_bounds(&kind);
        let snap_points = Self::compute_snap_points(&kind);
        let resize_handles = Self::compute_resize_handles(&kind);
        Self {
            id,
            kind,
            bounds,
            snap_points,
            resize_handles,
        }
    }

    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.bounds
    }

    pub fn snap_points(&self) -> &[Position] {
        &self.snap_points
    }

    pub fn resize_handles(&self) -> &[ResizeHandleInfo] {
        &self.resize_handles
    }

    pub fn contains(&self, pos: Position) -> bool {
        let (min_x, min_y, max_x, max_y) = self.bounds;
        pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y
    }

    pub fn label(&self) -> Option<&str> {
        self.kind.label()
    }

    pub fn supports_label(&self) -> bool {
        self.kind.supports_label()
    }

    fn compute_bounds(kind: &ShapeKind) -> (i32, i32, i32, i32) {
        match kind {
            ShapeKind::Line { start, end, .. } | ShapeKind::Arrow { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Rectangle { start, end, .. } | ShapeKind::DoubleBox { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Diamond { center, half_width, half_height, .. } => {
                (
                    center.x - *half_width,
                    center.y - *half_height,
                    center.x + *half_width,
                    center.y + *half_height,
                )
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, .. } => {
                (
                    center.x - *radius_x,
                    center.y - *radius_y,
                    center.x + *radius_x,
                    center.y + *radius_y,
                )
            }
            ShapeKind::Freehand { points, .. } => {
                if points.is_empty() {
                    return (0, 0, 0, 0);
                }
                let mut min_x = i32::MAX;
                let mut min_y = i32::MAX;
                let mut max_x = i32::MIN;
                let mut max_y = i32::MIN;
                for p in points {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Text { pos, content } => {
                (pos.x, pos.y, pos.x + content.len() as i32 - 1, pos.y)
            }
        }
    }

    fn compute_snap_points(kind: &ShapeKind) -> Vec<Position> {
        match kind {
            ShapeKind::Rectangle { start, end, .. } | ShapeKind::DoubleBox { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                let mid_x = (min_x + max_x) / 2;
                let mid_y = (min_y + max_y) / 2;

                vec![
                    Position::new(min_x, min_y),
                    Position::new(max_x, min_y),
                    Position::new(min_x, max_y),
                    Position::new(max_x, max_y),
                    Position::new(mid_x, min_y),
                    Position::new(mid_x, max_y),
                    Position::new(min_x, mid_y),
                    Position::new(max_x, mid_y),
                ]
            }
            ShapeKind::Line { start, end, .. } | ShapeKind::Arrow { start, end, .. } => {
                vec![*start, *end]
            }
            ShapeKind::Diamond { center, half_width, half_height, .. } => {
                vec![
                    Position::new(center.x, center.y - *half_height),
                    Position::new(center.x, center.y + *half_height),
                    Position::new(center.x - *half_width, center.y),
                    Position::new(center.x + *half_width, center.y),
                ]
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, .. } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Text { pos, content } => {
                let end_x = pos.x + content.len() as i32 - 1;
                vec![*pos, Position::new(end_x, pos.y)]
            }
            ShapeKind::Freehand { points, .. } => {
                let mut snaps = Vec::new();
                if let Some(first) = points.first() {
                    snaps.push(*first);
                }
                if let Some(last) = points.last() {
                    if points.len() > 1 {
                        snaps.push(*last);
                    }
                }
                snaps
            }
        }
    }

    fn compute_resize_handles(kind: &ShapeKind) -> Vec<ResizeHandleInfo> {
        match kind {
            ShapeKind::Rectangle { start, end, .. } | ShapeKind::DoubleBox { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);

                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: Position::new(min_x, min_y) },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: Position::new(max_x, min_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomLeft, pos: Position::new(min_x, max_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: Position::new(max_x, max_y) },
                ]
            }
            ShapeKind::Line { start, end, .. } | ShapeKind::Arrow { start, end, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::Start, pos: *start },
                    ResizeHandleInfo { handle: ResizeHandle::End, pos: *end },
                ]
            }
            ShapeKind::Diamond { center, half_width, half_height, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: Position::new(center.x, center.y - *half_height) },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: Position::new(center.x + *half_width, center.y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomLeft, pos: Position::new(center.x - *half_width, center.y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: Position::new(center.x, center.y + *half_height) },
                ]
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: Position::new(center.x - *radius_x, center.y - *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: Position::new(center.x + *radius_x, center.y - *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomLeft, pos: Position::new(center.x - *radius_x, center.y + *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: Position::new(center.x + *radius_x, center.y + *radius_y) },
                ]
            }
            _ => vec![],
        }
    }
}

/// Read-only cache of shapes for rendering (rebuilds from document)
#[derive(Debug)]
pub struct ShapeView {
    /// Cached shapes in render order
    shapes: Vec<CachedShape>,
    /// Fast lookup by ID
    by_id: HashMap<ShapeId, usize>,
}

impl ShapeView {
    pub fn new() -> Self {
        Self {
            shapes: Vec::new(),
            by_id: HashMap::new(),
        }
    }

    /// Rebuild cache from document
    pub fn rebuild(&mut self, doc: &Document) -> anyhow::Result<()> {
        self.shapes.clear();
        self.by_id.clear();

        for (id, kind) in doc.read_all_shapes()? {
            let idx = self.shapes.len();
            self.shapes.push(CachedShape::new(id, kind));
            self.by_id.insert(id, idx);
        }

        Ok(())
    }

    /// Iterate all shapes for rendering
    pub fn iter(&self) -> impl Iterator<Item = &CachedShape> {
        self.shapes.iter()
    }

    /// Find shape at position (returns topmost)
    pub fn shape_at(&self, pos: Position) -> Option<ShapeId> {
        for shape in self.shapes.iter().rev() {
            if shape.contains(pos) {
                return Some(shape.id);
            }
        }
        None
    }

    /// Get shape by ID
    pub fn get(&self, id: ShapeId) -> Option<&CachedShape> {
        self.by_id.get(&id).map(|&idx| &self.shapes[idx])
    }

    /// Get all snap points
    pub fn all_snap_points(&self) -> Vec<SnapPoint> {
        let mut points = Vec::new();
        for shape in &self.shapes {
            for &pos in shape.snap_points() {
                points.push(SnapPoint { pos, shape_id: shape.id });
            }
        }
        points
    }

    /// Find snap point within threshold
    pub fn find_snap_point(&self, pos: Position, threshold: i32) -> Option<SnapPoint> {
        let mut best: Option<(SnapPoint, i32)> = None;
        for shape in &self.shapes {
            for &snap_pos in shape.snap_points() {
                let dist = (pos.x - snap_pos.x).abs() + (pos.y - snap_pos.y).abs();
                if dist <= threshold {
                    if best.is_none() || dist < best.as_ref().unwrap().1 {
                        best = Some((SnapPoint { pos: snap_pos, shape_id: shape.id }, dist));
                    }
                }
            }
        }
        best.map(|(s, _)| s)
    }

    /// Find resize handle on a shape near a position
    pub fn find_resize_handle(&self, shape_id: ShapeId, pos: Position, threshold: i32) -> Option<ResizeHandle> {
        let shape = self.get(shape_id)?;
        for handle_info in shape.resize_handles() {
            let dist = (pos.x - handle_info.pos.x).abs() + (pos.y - handle_info.pos.y).abs();
            if dist <= threshold {
                return Some(handle_info.handle);
            }
        }
        None
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Get shape count
    pub fn len(&self) -> usize {
        self.shapes.len()
    }
}

impl Default for ShapeView {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply resize to a shape kind
pub fn resize_shape(kind: &ShapeKind, handle: ResizeHandle, new_pos: Position) -> ShapeKind {
    match kind {
        ShapeKind::Rectangle { start, end, label } | ShapeKind::DoubleBox { start, end, label } => {
            let is_double = matches!(kind, ShapeKind::DoubleBox { .. });
            let is_start_left = start.x <= end.x;
            let is_start_top = start.y <= end.y;

            let (new_start, new_end) = match handle {
                ResizeHandle::TopLeft => {
                    if is_start_left && is_start_top {
                        (new_pos, *end)
                    } else if !is_start_left && is_start_top {
                        (Position::new(start.x, new_pos.y), Position::new(new_pos.x, end.y))
                    } else if is_start_left && !is_start_top {
                        (Position::new(new_pos.x, start.y), Position::new(end.x, new_pos.y))
                    } else {
                        (*start, new_pos)
                    }
                }
                ResizeHandle::TopRight => {
                    if is_start_left && is_start_top {
                        (Position::new(start.x, new_pos.y), Position::new(new_pos.x, end.y))
                    } else if !is_start_left && is_start_top {
                        (new_pos, *end)
                    } else if is_start_left && !is_start_top {
                        (*start, new_pos)
                    } else {
                        (Position::new(new_pos.x, start.y), Position::new(end.x, new_pos.y))
                    }
                }
                ResizeHandle::BottomLeft => {
                    if is_start_left && is_start_top {
                        (Position::new(new_pos.x, start.y), Position::new(end.x, new_pos.y))
                    } else if !is_start_left && is_start_top {
                        (*start, new_pos)
                    } else if is_start_left && !is_start_top {
                        (new_pos, *end)
                    } else {
                        (Position::new(start.x, new_pos.y), Position::new(new_pos.x, end.y))
                    }
                }
                ResizeHandle::BottomRight => {
                    if is_start_left && is_start_top {
                        (*start, new_pos)
                    } else if !is_start_left && is_start_top {
                        (Position::new(new_pos.x, start.y), Position::new(end.x, new_pos.y))
                    } else if is_start_left && !is_start_top {
                        (Position::new(start.x, new_pos.y), Position::new(new_pos.x, end.y))
                    } else {
                        (new_pos, *end)
                    }
                }
                _ => (*start, *end),
            };

            if is_double {
                ShapeKind::DoubleBox { start: new_start, end: new_end, label: label.clone() }
            } else {
                ShapeKind::Rectangle { start: new_start, end: new_end, label: label.clone() }
            }
        }
        ShapeKind::Line { start, end, style, start_connection, end_connection } => {
            match handle {
                ResizeHandle::Start => ShapeKind::Line {
                    start: new_pos,
                    end: *end,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                },
                ResizeHandle::End => ShapeKind::Line {
                    start: *start,
                    end: new_pos,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Arrow { start, end, style, start_connection, end_connection } => {
            match handle {
                ResizeHandle::Start => ShapeKind::Arrow {
                    start: new_pos,
                    end: *end,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                },
                ResizeHandle::End => ShapeKind::Arrow {
                    start: *start,
                    end: new_pos,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Diamond { center, half_width, half_height, label } => {
            match handle {
                ResizeHandle::TopLeft => ShapeKind::Diamond {
                    center: *center,
                    half_width: *half_width,
                    half_height: (center.y - new_pos.y).abs().max(1),
                    label: label.clone(),
                },
                ResizeHandle::TopRight => ShapeKind::Diamond {
                    center: *center,
                    half_width: (new_pos.x - center.x).abs().max(1),
                    half_height: *half_height,
                    label: label.clone(),
                },
                ResizeHandle::BottomLeft => ShapeKind::Diamond {
                    center: *center,
                    half_width: (center.x - new_pos.x).abs().max(1),
                    half_height: *half_height,
                    label: label.clone(),
                },
                ResizeHandle::BottomRight => ShapeKind::Diamond {
                    center: *center,
                    half_width: *half_width,
                    half_height: (new_pos.y - center.y).abs().max(1),
                    label: label.clone(),
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Ellipse { center, label, .. } => {
            match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight |
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => {
                    ShapeKind::Ellipse {
                        center: *center,
                        radius_x: (new_pos.x - center.x).abs().max(1),
                        radius_y: (new_pos.y - center.y).abs().max(1),
                        label: label.clone(),
                    }
                }
                _ => kind.clone(),
            }
        }
        _ => kind.clone(),
    }
}
