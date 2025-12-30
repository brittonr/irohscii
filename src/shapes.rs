//! Shape types and rendering cache for irohscii.
//!
//! ShapeKind defines the different shape variants.
//! ShapeView provides a fast read-only cache for rendering.

use std::collections::HashMap;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::canvas::{LineStyle, Position};
use crate::document::{Document, ShapeId};

/// Color for shapes - 16-color terminal palette
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShapeColor {
    #[default]
    White,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
}

impl ShapeColor {
    /// Convert to ratatui Color for terminal rendering
    pub fn to_ratatui(self) -> Color {
        match self {
            ShapeColor::White => Color::White,
            ShapeColor::Black => Color::Black,
            ShapeColor::Red => Color::Red,
            ShapeColor::Green => Color::Green,
            ShapeColor::Yellow => Color::Yellow,
            ShapeColor::Blue => Color::Blue,
            ShapeColor::Magenta => Color::Magenta,
            ShapeColor::Cyan => Color::Cyan,
            ShapeColor::Gray => Color::Gray,
            ShapeColor::DarkGray => Color::DarkGray,
            ShapeColor::LightRed => Color::LightRed,
            ShapeColor::LightGreen => Color::LightGreen,
            ShapeColor::LightYellow => Color::LightYellow,
            ShapeColor::LightBlue => Color::LightBlue,
            ShapeColor::LightMagenta => Color::LightMagenta,
            ShapeColor::LightCyan => Color::LightCyan,
        }
    }

    /// Convert to CSS color string for SVG export
    pub fn to_css(self) -> &'static str {
        match self {
            ShapeColor::White => "white",
            ShapeColor::Black => "black",
            ShapeColor::Red => "#cd0000",
            ShapeColor::Green => "#00cd00",
            ShapeColor::Yellow => "#cdcd00",
            ShapeColor::Blue => "#0000cd",
            ShapeColor::Magenta => "#cd00cd",
            ShapeColor::Cyan => "#00cdcd",
            ShapeColor::Gray => "#808080",
            ShapeColor::DarkGray => "#555555",
            ShapeColor::LightRed => "#ff0000",
            ShapeColor::LightGreen => "#00ff00",
            ShapeColor::LightYellow => "#ffff00",
            ShapeColor::LightBlue => "#0000ff",
            ShapeColor::LightMagenta => "#ff00ff",
            ShapeColor::LightCyan => "#00ffff",
        }
    }

    /// Get display name for status bar
    pub fn name(self) -> &'static str {
        match self {
            ShapeColor::White => "White",
            ShapeColor::Black => "Black",
            ShapeColor::Red => "Red",
            ShapeColor::Green => "Green",
            ShapeColor::Yellow => "Yellow",
            ShapeColor::Blue => "Blue",
            ShapeColor::Magenta => "Magenta",
            ShapeColor::Cyan => "Cyan",
            ShapeColor::Gray => "Gray",
            ShapeColor::DarkGray => "DarkGray",
            ShapeColor::LightRed => "LightRed",
            ShapeColor::LightGreen => "LightGreen",
            ShapeColor::LightYellow => "LightYellow",
            ShapeColor::LightBlue => "LightBlue",
            ShapeColor::LightMagenta => "LightMagenta",
            ShapeColor::LightCyan => "LightCyan",
        }
    }

    /// Cycle to next color
    pub fn next(self) -> Self {
        match self {
            ShapeColor::White => ShapeColor::Red,
            ShapeColor::Red => ShapeColor::Green,
            ShapeColor::Green => ShapeColor::Yellow,
            ShapeColor::Yellow => ShapeColor::Blue,
            ShapeColor::Blue => ShapeColor::Magenta,
            ShapeColor::Magenta => ShapeColor::Cyan,
            ShapeColor::Cyan => ShapeColor::LightRed,
            ShapeColor::LightRed => ShapeColor::LightGreen,
            ShapeColor::LightGreen => ShapeColor::LightYellow,
            ShapeColor::LightYellow => ShapeColor::LightBlue,
            ShapeColor::LightBlue => ShapeColor::LightMagenta,
            ShapeColor::LightMagenta => ShapeColor::LightCyan,
            ShapeColor::LightCyan => ShapeColor::Gray,
            ShapeColor::Gray => ShapeColor::DarkGray,
            ShapeColor::DarkGray => ShapeColor::Black,
            ShapeColor::Black => ShapeColor::White,
        }
    }
}

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
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// An arrow (line with arrowhead at end)
    Arrow {
        start: Position,
        end: Position,
        style: LineStyle,
        start_connection: Option<u64>,
        end_connection: Option<u64>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A rectangle defined by two corners
    Rectangle {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A double-line rectangle
    DoubleBox {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A diamond (rhombus) defined by center and half-dimensions
    Diamond {
        center: Position,
        half_width: i32,
        half_height: i32,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// An ellipse defined by center and radii
    Ellipse {
        center: Position,
        radius_x: i32,
        radius_y: i32,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// Freehand stroke - series of points
    Freehand {
        points: Vec<Position>,
        char: char,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// Text at a position
    Text {
        pos: Position,
        content: String,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A triangle defined by three points
    Triangle {
        p1: Position,
        p2: Position,
        p3: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A parallelogram defined by start and end (slanted rectangle)
    Parallelogram {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A hexagon defined by center and radii
    Hexagon {
        center: Position,
        radius_x: i32,
        radius_y: i32,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A trapezoid defined by start and end
    Trapezoid {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A rounded rectangle
    RoundedRect {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A cylinder (database symbol)
    Cylinder {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A cloud shape
    Cloud {
        start: Position,
        end: Position,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
    /// A star shape
    Star {
        center: Position,
        outer_radius: i32,
        inner_radius: i32,
        label: Option<String>,
        #[serde(default)]
        color: ShapeColor,
    },
}

impl ShapeKind {
    /// Create a translated copy of this shape
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        match self {
            ShapeKind::Line { start, end, style, label, color, .. } => ShapeKind::Line {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                style: *style,
                start_connection: None,
                end_connection: None,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Arrow { start, end, style, label, color, .. } => ShapeKind::Arrow {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                style: *style,
                start_connection: None,
                end_connection: None,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Rectangle { start, end, label, color } => ShapeKind::Rectangle {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::DoubleBox { start, end, label, color } => ShapeKind::DoubleBox {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Diamond { center, half_width, half_height, label, color } => ShapeKind::Diamond {
                center: Position { x: center.x + dx, y: center.y + dy },
                half_width: *half_width,
                half_height: *half_height,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Ellipse { center, radius_x, radius_y, label, color } => ShapeKind::Ellipse {
                center: Position { x: center.x + dx, y: center.y + dy },
                radius_x: *radius_x,
                radius_y: *radius_y,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Freehand { points, char, label, color } => ShapeKind::Freehand {
                points: points.iter().map(|p| Position { x: p.x + dx, y: p.y + dy }).collect(),
                char: *char,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Text { pos, content, color } => ShapeKind::Text {
                pos: Position { x: pos.x + dx, y: pos.y + dy },
                content: content.clone(),
                color: *color,
            },
            ShapeKind::Triangle { p1, p2, p3, label, color } => ShapeKind::Triangle {
                p1: Position { x: p1.x + dx, y: p1.y + dy },
                p2: Position { x: p2.x + dx, y: p2.y + dy },
                p3: Position { x: p3.x + dx, y: p3.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Parallelogram { start, end, label, color } => ShapeKind::Parallelogram {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Hexagon { center, radius_x, radius_y, label, color } => ShapeKind::Hexagon {
                center: Position { x: center.x + dx, y: center.y + dy },
                radius_x: *radius_x,
                radius_y: *radius_y,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Trapezoid { start, end, label, color } => ShapeKind::Trapezoid {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::RoundedRect { start, end, label, color } => ShapeKind::RoundedRect {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Cylinder { start, end, label, color } => ShapeKind::Cylinder {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Cloud { start, end, label, color } => ShapeKind::Cloud {
                start: Position { x: start.x + dx, y: start.y + dy },
                end: Position { x: end.x + dx, y: end.y + dy },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Star { center, outer_radius, inner_radius, label, color } => ShapeKind::Star {
                center: Position { x: center.x + dx, y: center.y + dy },
                outer_radius: *outer_radius,
                inner_radius: *inner_radius,
                label: label.clone(),
                color: *color,
            },
        }
    }

    /// Get the label for this shape (if it supports labels)
    pub fn label(&self) -> Option<&str> {
        match self {
            ShapeKind::Line { label, .. }
            | ShapeKind::Arrow { label, .. }
            | ShapeKind::Rectangle { label, .. }
            | ShapeKind::DoubleBox { label, .. }
            | ShapeKind::Diamond { label, .. }
            | ShapeKind::Ellipse { label, .. }
            | ShapeKind::Freehand { label, .. }
            | ShapeKind::Triangle { label, .. }
            | ShapeKind::Parallelogram { label, .. }
            | ShapeKind::Hexagon { label, .. }
            | ShapeKind::Trapezoid { label, .. }
            | ShapeKind::RoundedRect { label, .. }
            | ShapeKind::Cylinder { label, .. }
            | ShapeKind::Cloud { label, .. }
            | ShapeKind::Star { label, .. } => label.as_deref(),
            ShapeKind::Text { .. } => None, // Text content is the label
        }
    }

    /// Set the label for this shape (if it supports labels)
    pub fn with_label(self, new_label: Option<String>) -> Self {
        match self {
            ShapeKind::Line { start, end, style, start_connection, end_connection, color, .. } => {
                ShapeKind::Line { start, end, style, start_connection, end_connection, label: new_label, color }
            }
            ShapeKind::Arrow { start, end, style, start_connection, end_connection, color, .. } => {
                ShapeKind::Arrow { start, end, style, start_connection, end_connection, label: new_label, color }
            }
            ShapeKind::Rectangle { start, end, color, .. } => ShapeKind::Rectangle { start, end, label: new_label, color },
            ShapeKind::DoubleBox { start, end, color, .. } => ShapeKind::DoubleBox { start, end, label: new_label, color },
            ShapeKind::Diamond { center, half_width, half_height, color, .. } => {
                ShapeKind::Diamond { center, half_width, half_height, label: new_label, color }
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, color, .. } => {
                ShapeKind::Ellipse { center, radius_x, radius_y, label: new_label, color }
            }
            ShapeKind::Freehand { points, char, color, .. } => {
                ShapeKind::Freehand { points, char, label: new_label, color }
            }
            ShapeKind::Triangle { p1, p2, p3, color, .. } => {
                ShapeKind::Triangle { p1, p2, p3, label: new_label, color }
            }
            ShapeKind::Parallelogram { start, end, color, .. } => {
                ShapeKind::Parallelogram { start, end, label: new_label, color }
            }
            ShapeKind::Hexagon { center, radius_x, radius_y, color, .. } => {
                ShapeKind::Hexagon { center, radius_x, radius_y, label: new_label, color }
            }
            ShapeKind::Trapezoid { start, end, color, .. } => {
                ShapeKind::Trapezoid { start, end, label: new_label, color }
            }
            ShapeKind::RoundedRect { start, end, color, .. } => {
                ShapeKind::RoundedRect { start, end, label: new_label, color }
            }
            ShapeKind::Cylinder { start, end, color, .. } => {
                ShapeKind::Cylinder { start, end, label: new_label, color }
            }
            ShapeKind::Cloud { start, end, color, .. } => {
                ShapeKind::Cloud { start, end, label: new_label, color }
            }
            ShapeKind::Star { center, outer_radius, inner_radius, color, .. } => {
                ShapeKind::Star { center, outer_radius, inner_radius, label: new_label, color }
            }
            // Text doesn't have a separate label - its content IS the label
            other => other,
        }
    }

    /// Get the color of this shape
    pub fn color(&self) -> ShapeColor {
        match self {
            ShapeKind::Line { color, .. }
            | ShapeKind::Arrow { color, .. }
            | ShapeKind::Rectangle { color, .. }
            | ShapeKind::DoubleBox { color, .. }
            | ShapeKind::Diamond { color, .. }
            | ShapeKind::Ellipse { color, .. }
            | ShapeKind::Freehand { color, .. }
            | ShapeKind::Text { color, .. }
            | ShapeKind::Triangle { color, .. }
            | ShapeKind::Parallelogram { color, .. }
            | ShapeKind::Hexagon { color, .. }
            | ShapeKind::Trapezoid { color, .. }
            | ShapeKind::RoundedRect { color, .. }
            | ShapeKind::Cylinder { color, .. }
            | ShapeKind::Cloud { color, .. }
            | ShapeKind::Star { color, .. } => *color,
        }
    }

    /// Check if this shape supports labels
    pub fn supports_label(&self) -> bool {
        matches!(
            self,
            ShapeKind::Line { .. }
                | ShapeKind::Arrow { .. }
                | ShapeKind::Rectangle { .. }
                | ShapeKind::DoubleBox { .. }
                | ShapeKind::Diamond { .. }
                | ShapeKind::Ellipse { .. }
                | ShapeKind::Freehand { .. }
                | ShapeKind::Triangle { .. }
                | ShapeKind::Parallelogram { .. }
                | ShapeKind::Hexagon { .. }
                | ShapeKind::Trapezoid { .. }
                | ShapeKind::RoundedRect { .. }
                | ShapeKind::Cylinder { .. }
                | ShapeKind::Cloud { .. }
                | ShapeKind::Star { .. }
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
            ShapeKind::Text { pos, content, .. } => {
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
            ShapeKind::Triangle { p1, p2, p3, .. } => {
                vec![*p1, *p2, *p3]
            }
            ShapeKind::Parallelogram { start, end, .. } | ShapeKind::Trapezoid { start, end, .. } |
            ShapeKind::RoundedRect { start, end, .. } | ShapeKind::Cylinder { start, end, .. } |
            ShapeKind::Cloud { start, end, .. } => {
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
            ShapeKind::Hexagon { center, radius_x, radius_y, .. } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Star { center, outer_radius, .. } => {
                vec![
                    Position::new(center.x, center.y - *outer_radius),
                    Position::new(center.x, center.y + *outer_radius),
                    Position::new(center.x - *outer_radius, center.y),
                    Position::new(center.x + *outer_radius, center.y),
                ]
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
            ShapeKind::Text { pos, content, .. } => {
                (pos.x, pos.y, pos.x + content.len() as i32 - 1, pos.y)
            }
            ShapeKind::Triangle { p1, p2, p3, .. } => {
                let min_x = p1.x.min(p2.x).min(p3.x);
                let max_x = p1.x.max(p2.x).max(p3.x);
                let min_y = p1.y.min(p2.y).min(p3.y);
                let max_y = p1.y.max(p2.y).max(p3.y);
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Parallelogram { start, end, .. } | ShapeKind::Trapezoid { start, end, .. } |
            ShapeKind::RoundedRect { start, end, .. } | ShapeKind::Cylinder { start, end, .. } |
            ShapeKind::Cloud { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Hexagon { center, radius_x, radius_y, .. } => {
                (
                    center.x - *radius_x,
                    center.y - *radius_y,
                    center.x + *radius_x,
                    center.y + *radius_y,
                )
            }
            ShapeKind::Star { center, outer_radius, .. } => {
                (
                    center.x - *outer_radius,
                    center.y - *outer_radius,
                    center.x + *outer_radius,
                    center.y + *outer_radius,
                )
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
            ShapeKind::Text { pos, content, .. } => {
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
            ShapeKind::Triangle { p1, p2, p3, .. } => {
                vec![*p1, *p2, *p3]
            }
            ShapeKind::Parallelogram { start, end, .. } | ShapeKind::Trapezoid { start, end, .. } |
            ShapeKind::RoundedRect { start, end, .. } | ShapeKind::Cylinder { start, end, .. } |
            ShapeKind::Cloud { start, end, .. } => {
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
            ShapeKind::Hexagon { center, radius_x, radius_y, .. } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Star { center, outer_radius, .. } => {
                vec![
                    Position::new(center.x, center.y - *outer_radius),
                    Position::new(center.x, center.y + *outer_radius),
                    Position::new(center.x - *outer_radius, center.y),
                    Position::new(center.x + *outer_radius, center.y),
                ]
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
            ShapeKind::Triangle { p1, p2, p3, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: *p1 },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: *p2 },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: *p3 },
                ]
            }
            ShapeKind::Parallelogram { start, end, .. } | ShapeKind::Trapezoid { start, end, .. } |
            ShapeKind::RoundedRect { start, end, .. } | ShapeKind::Cylinder { start, end, .. } |
            ShapeKind::Cloud { start, end, .. } => {
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
            ShapeKind::Hexagon { center, radius_x, radius_y, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: Position::new(center.x - *radius_x, center.y - *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: Position::new(center.x + *radius_x, center.y - *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomLeft, pos: Position::new(center.x - *radius_x, center.y + *radius_y) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: Position::new(center.x + *radius_x, center.y + *radius_y) },
                ]
            }
            ShapeKind::Star { center, outer_radius, .. } => {
                vec![
                    ResizeHandleInfo { handle: ResizeHandle::TopLeft, pos: Position::new(center.x - *outer_radius, center.y - *outer_radius) },
                    ResizeHandleInfo { handle: ResizeHandle::TopRight, pos: Position::new(center.x + *outer_radius, center.y - *outer_radius) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomLeft, pos: Position::new(center.x - *outer_radius, center.y + *outer_radius) },
                    ResizeHandleInfo { handle: ResizeHandle::BottomRight, pos: Position::new(center.x + *outer_radius, center.y + *outer_radius) },
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
        ShapeKind::Rectangle { start, end, label, color } | ShapeKind::DoubleBox { start, end, label, color } => {
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
                ShapeKind::DoubleBox { start: new_start, end: new_end, label: label.clone(), color: *color }
            } else {
                ShapeKind::Rectangle { start: new_start, end: new_end, label: label.clone(), color: *color }
            }
        }
        ShapeKind::Line { start, end, style, start_connection, end_connection, label, color } => {
            match handle {
                ResizeHandle::Start => ShapeKind::Line {
                    start: new_pos,
                    end: *end,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::End => ShapeKind::Line {
                    start: *start,
                    end: new_pos,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                    label: label.clone(),
                    color: *color,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Arrow { start, end, style, start_connection, end_connection, label, color } => {
            match handle {
                ResizeHandle::Start => ShapeKind::Arrow {
                    start: new_pos,
                    end: *end,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::End => ShapeKind::Arrow {
                    start: *start,
                    end: new_pos,
                    style: *style,
                    start_connection: *start_connection,
                    end_connection: *end_connection,
                    label: label.clone(),
                    color: *color,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Diamond { center, half_width, half_height, label, color } => {
            match handle {
                ResizeHandle::TopLeft => ShapeKind::Diamond {
                    center: *center,
                    half_width: *half_width,
                    half_height: (center.y - new_pos.y).abs().max(1),
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::TopRight => ShapeKind::Diamond {
                    center: *center,
                    half_width: (new_pos.x - center.x).abs().max(1),
                    half_height: *half_height,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::BottomLeft => ShapeKind::Diamond {
                    center: *center,
                    half_width: (center.x - new_pos.x).abs().max(1),
                    half_height: *half_height,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::BottomRight => ShapeKind::Diamond {
                    center: *center,
                    half_width: *half_width,
                    half_height: (new_pos.y - center.y).abs().max(1),
                    label: label.clone(),
                    color: *color,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Ellipse { center, label, color, .. } => {
            match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight |
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => {
                    ShapeKind::Ellipse {
                        center: *center,
                        radius_x: (new_pos.x - center.x).abs().max(1),
                        radius_y: (new_pos.y - center.y).abs().max(1),
                        label: label.clone(),
                        color: *color,
                    }
                }
                _ => kind.clone(),
            }
        }
        ShapeKind::Triangle { p1, p2, p3, label, color } => {
            match handle {
                ResizeHandle::TopLeft => ShapeKind::Triangle {
                    p1: new_pos,
                    p2: *p2,
                    p3: *p3,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::TopRight => ShapeKind::Triangle {
                    p1: *p1,
                    p2: new_pos,
                    p3: *p3,
                    label: label.clone(),
                    color: *color,
                },
                ResizeHandle::BottomRight => ShapeKind::Triangle {
                    p1: *p1,
                    p2: *p2,
                    p3: new_pos,
                    label: label.clone(),
                    color: *color,
                },
                _ => kind.clone(),
            }
        }
        ShapeKind::Parallelogram { start, end, label, color } => {
            resize_rect_like(start, end, handle, new_pos, |s, e| {
                ShapeKind::Parallelogram { start: s, end: e, label: label.clone(), color: *color }
            })
        }
        ShapeKind::Trapezoid { start, end, label, color } => {
            resize_rect_like(start, end, handle, new_pos, |s, e| {
                ShapeKind::Trapezoid { start: s, end: e, label: label.clone(), color: *color }
            })
        }
        ShapeKind::RoundedRect { start, end, label, color } => {
            resize_rect_like(start, end, handle, new_pos, |s, e| {
                ShapeKind::RoundedRect { start: s, end: e, label: label.clone(), color: *color }
            })
        }
        ShapeKind::Cylinder { start, end, label, color } => {
            resize_rect_like(start, end, handle, new_pos, |s, e| {
                ShapeKind::Cylinder { start: s, end: e, label: label.clone(), color: *color }
            })
        }
        ShapeKind::Cloud { start, end, label, color } => {
            resize_rect_like(start, end, handle, new_pos, |s, e| {
                ShapeKind::Cloud { start: s, end: e, label: label.clone(), color: *color }
            })
        }
        ShapeKind::Hexagon { center, label, color, .. } => {
            match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight |
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => {
                    ShapeKind::Hexagon {
                        center: *center,
                        radius_x: (new_pos.x - center.x).abs().max(1),
                        radius_y: (new_pos.y - center.y).abs().max(1),
                        label: label.clone(),
                        color: *color,
                    }
                }
                _ => kind.clone(),
            }
        }
        ShapeKind::Star { center, inner_radius, label, color, .. } => {
            match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight |
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => {
                    let outer = ((new_pos.x - center.x).abs().max((new_pos.y - center.y).abs())).max(2);
                    ShapeKind::Star {
                        center: *center,
                        outer_radius: outer,
                        inner_radius: (*inner_radius).min(outer - 1).max(1),
                        label: label.clone(),
                        color: *color,
                    }
                }
                _ => kind.clone(),
            }
        }
        _ => kind.clone(),
    }
}

/// Helper for resizing rectangle-like shapes
fn resize_rect_like<F>(start: &Position, end: &Position, handle: ResizeHandle, new_pos: Position, make_shape: F) -> ShapeKind
where
    F: FnOnce(Position, Position) -> ShapeKind,
{
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

    make_shape(new_start, new_end)
}
