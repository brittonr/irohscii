//! Shape types and rendering cache for irohscii.
//!
//! ShapeKind defines the different shape variants.
//! ShapeView provides a fast read-only cache for rendering.

use std::collections::HashMap;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::canvas::{LineStyle, Position};
use crate::document::{Document, ShapeId};
use crate::layers::LayerId;

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
    #[allow(dead_code)]
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
    /// Get the human-readable name of this shape type
    pub fn type_name(&self) -> &'static str {
        match self {
            ShapeKind::Line { .. } => "Line",
            ShapeKind::Arrow { .. } => "Arrow",
            ShapeKind::Rectangle { .. } => "Rectangle",
            ShapeKind::DoubleBox { .. } => "DoubleBox",
            ShapeKind::Diamond { .. } => "Diamond",
            ShapeKind::Ellipse { .. } => "Ellipse",
            ShapeKind::Freehand { .. } => "Freehand",
            ShapeKind::Text { .. } => "Text",
            ShapeKind::Triangle { .. } => "Triangle",
            ShapeKind::Parallelogram { .. } => "Parallelogram",
            ShapeKind::Hexagon { .. } => "Hexagon",
            ShapeKind::Trapezoid { .. } => "Trapezoid",
            ShapeKind::RoundedRect { .. } => "RoundedRect",
            ShapeKind::Cylinder { .. } => "Cylinder",
            ShapeKind::Cloud { .. } => "Cloud",
            ShapeKind::Star { .. } => "Star",
        }
    }

    /// Get the bounding box of this shape as (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        CachedShape::compute_bounds(self)
    }

    /// Create a translated copy of this shape
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        match self {
            ShapeKind::Line {
                start,
                end,
                style,
                label,
                color,
                ..
            } => ShapeKind::Line {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                style: *style,
                start_connection: None,
                end_connection: None,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Arrow {
                start,
                end,
                style,
                label,
                color,
                ..
            } => ShapeKind::Arrow {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                style: *style,
                start_connection: None,
                end_connection: None,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Rectangle {
                start,
                end,
                label,
                color,
            } => ShapeKind::Rectangle {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::DoubleBox {
                start,
                end,
                label,
                color,
            } => ShapeKind::DoubleBox {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                label,
                color,
            } => ShapeKind::Diamond {
                center: Position {
                    x: center.x + dx,
                    y: center.y + dy,
                },
                half_width: *half_width,
                half_height: *half_height,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                label,
                color,
            } => ShapeKind::Ellipse {
                center: Position {
                    x: center.x + dx,
                    y: center.y + dy,
                },
                radius_x: *radius_x,
                radius_y: *radius_y,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Freehand {
                points,
                char,
                label,
                color,
            } => ShapeKind::Freehand {
                points: points
                    .iter()
                    .map(|p| Position {
                        x: p.x + dx,
                        y: p.y + dy,
                    })
                    .collect(),
                char: *char,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Text {
                pos,
                content,
                color,
            } => ShapeKind::Text {
                pos: Position {
                    x: pos.x + dx,
                    y: pos.y + dy,
                },
                content: content.clone(),
                color: *color,
            },
            ShapeKind::Triangle {
                p1,
                p2,
                p3,
                label,
                color,
            } => ShapeKind::Triangle {
                p1: Position {
                    x: p1.x + dx,
                    y: p1.y + dy,
                },
                p2: Position {
                    x: p2.x + dx,
                    y: p2.y + dy,
                },
                p3: Position {
                    x: p3.x + dx,
                    y: p3.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Parallelogram {
                start,
                end,
                label,
                color,
            } => ShapeKind::Parallelogram {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                label,
                color,
            } => ShapeKind::Hexagon {
                center: Position {
                    x: center.x + dx,
                    y: center.y + dy,
                },
                radius_x: *radius_x,
                radius_y: *radius_y,
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Trapezoid {
                start,
                end,
                label,
                color,
            } => ShapeKind::Trapezoid {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::RoundedRect {
                start,
                end,
                label,
                color,
            } => ShapeKind::RoundedRect {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Cylinder {
                start,
                end,
                label,
                color,
            } => ShapeKind::Cylinder {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Cloud {
                start,
                end,
                label,
                color,
            } => ShapeKind::Cloud {
                start: Position {
                    x: start.x + dx,
                    y: start.y + dy,
                },
                end: Position {
                    x: end.x + dx,
                    y: end.y + dy,
                },
                label: label.clone(),
                color: *color,
            },
            ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                label,
                color,
            } => ShapeKind::Star {
                center: Position {
                    x: center.x + dx,
                    y: center.y + dy,
                },
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
            ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                color,
                ..
            } => ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label: new_label,
                color,
            },
            ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                color,
                ..
            } => ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label: new_label,
                color,
            },
            ShapeKind::Rectangle {
                start, end, color, ..
            } => ShapeKind::Rectangle {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::DoubleBox {
                start, end, color, ..
            } => ShapeKind::DoubleBox {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                color,
                ..
            } => ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                label: new_label,
                color,
            },
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                color,
                ..
            } => ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                label: new_label,
                color,
            },
            ShapeKind::Freehand {
                points,
                char,
                color,
                ..
            } => ShapeKind::Freehand {
                points,
                char,
                label: new_label,
                color,
            },
            ShapeKind::Triangle {
                p1, p2, p3, color, ..
            } => ShapeKind::Triangle {
                p1,
                p2,
                p3,
                label: new_label,
                color,
            },
            ShapeKind::Parallelogram {
                start, end, color, ..
            } => ShapeKind::Parallelogram {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                color,
                ..
            } => ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                label: new_label,
                color,
            },
            ShapeKind::Trapezoid {
                start, end, color, ..
            } => ShapeKind::Trapezoid {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::RoundedRect {
                start, end, color, ..
            } => ShapeKind::RoundedRect {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::Cylinder {
                start, end, color, ..
            } => ShapeKind::Cylinder {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::Cloud {
                start, end, color, ..
            } => ShapeKind::Cloud {
                start,
                end,
                label: new_label,
                color,
            },
            ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                color,
                ..
            } => ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                label: new_label,
                color,
            },
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

    /// Set the color for this shape
    pub fn with_color(self, new_color: ShapeColor) -> Self {
        match self {
            ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                ..
            } => ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color: new_color,
            },
            ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                ..
            } => ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color: new_color,
            },
            ShapeKind::Rectangle {
                start, end, label, ..
            } => ShapeKind::Rectangle {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::DoubleBox {
                start, end, label, ..
            } => ShapeKind::DoubleBox {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                label,
                ..
            } => ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                label,
                color: new_color,
            },
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                label,
                ..
            } => ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                label,
                color: new_color,
            },
            ShapeKind::Freehand {
                points,
                char,
                label,
                ..
            } => ShapeKind::Freehand {
                points,
                char,
                label,
                color: new_color,
            },
            ShapeKind::Text { pos, content, .. } => ShapeKind::Text {
                pos,
                content,
                color: new_color,
            },
            ShapeKind::Triangle {
                p1, p2, p3, label, ..
            } => ShapeKind::Triangle {
                p1,
                p2,
                p3,
                label,
                color: new_color,
            },
            ShapeKind::Parallelogram {
                start, end, label, ..
            } => ShapeKind::Parallelogram {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                label,
                ..
            } => ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                label,
                color: new_color,
            },
            ShapeKind::Trapezoid {
                start, end, label, ..
            } => ShapeKind::Trapezoid {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::RoundedRect {
                start, end, label, ..
            } => ShapeKind::RoundedRect {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::Cylinder {
                start, end, label, ..
            } => ShapeKind::Cylinder {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::Cloud {
                start, end, label, ..
            } => ShapeKind::Cloud {
                start,
                end,
                label,
                color: new_color,
            },
            ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                label,
                ..
            } => ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                label,
                color: new_color,
            },
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

    /// Get connection IDs for this shape (start_connection, end_connection)
    #[allow(dead_code)]
    pub fn connections(&self) -> (Option<u64>, Option<u64>) {
        match self {
            ShapeKind::Line {
                start_connection,
                end_connection,
                ..
            }
            | ShapeKind::Arrow {
                start_connection,
                end_connection,
                ..
            } => (*start_connection, *end_connection),
            _ => (None, None),
        }
    }

    /// Translate line/arrow endpoints by dx, dy for connected ends
    pub fn translate_connected_endpoints(
        &self,
        target_id: u64,
        dx: i32,
        dy: i32,
    ) -> Option<ShapeKind> {
        match self {
            ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color,
            } => {
                let mut new_start = *start;
                let mut new_end = *end;
                let mut changed = false;

                if *start_connection == Some(target_id) {
                    new_start = Position::new(start.x + dx, start.y + dy);
                    changed = true;
                }
                if *end_connection == Some(target_id) {
                    new_end = Position::new(end.x + dx, end.y + dy);
                    changed = true;
                }

                if changed {
                    Some(ShapeKind::Line {
                        start: new_start,
                        end: new_end,
                        style: *style,
                        start_connection: *start_connection,
                        end_connection: *end_connection,
                        label: label.clone(),
                        color: *color,
                    })
                } else {
                    None
                }
            }
            ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color,
            } => {
                let mut new_start = *start;
                let mut new_end = *end;
                let mut changed = false;

                if *start_connection == Some(target_id) {
                    new_start = Position::new(start.x + dx, start.y + dy);
                    changed = true;
                }
                if *end_connection == Some(target_id) {
                    new_end = Position::new(end.x + dx, end.y + dy);
                    changed = true;
                }

                if changed {
                    Some(ShapeKind::Arrow {
                        start: new_start,
                        end: new_end,
                        style: *style,
                        start_connection: *start_connection,
                        end_connection: *end_connection,
                        label: label.clone(),
                        color: *color,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Update line/arrow endpoints for a resize operation where different snap points move differently
    /// Returns updated shape if connected to the resized shape
    pub fn update_for_resize(
        &self,
        resized_id: u64,
        old_snaps: &[Position],
        new_snaps: &[Position],
    ) -> Option<ShapeKind> {
        match self {
            ShapeKind::Line {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color,
            } => {
                let mut new_start = *start;
                let mut new_end = *end;
                let mut changed = false;

                if *start_connection == Some(resized_id) {
                    if let Some(new_pos) = find_corresponding_snap(start, old_snaps, new_snaps) {
                        new_start = new_pos;
                        changed = true;
                    }
                }
                if *end_connection == Some(resized_id) {
                    if let Some(new_pos) = find_corresponding_snap(end, old_snaps, new_snaps) {
                        new_end = new_pos;
                        changed = true;
                    }
                }

                if changed {
                    Some(ShapeKind::Line {
                        start: new_start,
                        end: new_end,
                        style: *style,
                        start_connection: *start_connection,
                        end_connection: *end_connection,
                        label: label.clone(),
                        color: *color,
                    })
                } else {
                    None
                }
            }
            ShapeKind::Arrow {
                start,
                end,
                style,
                start_connection,
                end_connection,
                label,
                color,
            } => {
                let mut new_start = *start;
                let mut new_end = *end;
                let mut changed = false;

                if *start_connection == Some(resized_id) {
                    if let Some(new_pos) = find_corresponding_snap(start, old_snaps, new_snaps) {
                        new_start = new_pos;
                        changed = true;
                    }
                }
                if *end_connection == Some(resized_id) {
                    if let Some(new_pos) = find_corresponding_snap(end, old_snaps, new_snaps) {
                        new_end = new_pos;
                        changed = true;
                    }
                }

                if changed {
                    Some(ShapeKind::Arrow {
                        start: new_start,
                        end: new_end,
                        style: *style,
                        start_connection: *start_connection,
                        end_connection: *end_connection,
                        label: label.clone(),
                        color: *color,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
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
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                ..
            } => {
                vec![
                    Position::new(center.x, center.y - *half_height),
                    Position::new(center.x, center.y + *half_height),
                    Position::new(center.x - *half_width, center.y),
                    Position::new(center.x + *half_width, center.y),
                ]
            }
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => {
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
            ShapeKind::Parallelogram { start, end, .. }
            | ShapeKind::Trapezoid { start, end, .. }
            | ShapeKind::RoundedRect { start, end, .. }
            | ShapeKind::Cylinder { start, end, .. }
            | ShapeKind::Cloud { start, end, .. } => {
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
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Star {
                center,
                outer_radius,
                ..
            } => {
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
    pub layer_id: Option<LayerId>,
    bounds: (i32, i32, i32, i32),
    snap_points: Vec<Position>,
    resize_handles: Vec<ResizeHandleInfo>,
}

impl CachedShape {
    /// Create a cached shape from id and kind
    #[allow(dead_code)]
    pub fn new(id: ShapeId, kind: ShapeKind) -> Self {
        let bounds = Self::compute_bounds(&kind);
        let snap_points = Self::compute_snap_points(&kind);
        let resize_handles = Self::compute_resize_handles(&kind);
        Self {
            id,
            kind,
            layer_id: None,
            bounds,
            snap_points,
            resize_handles,
        }
    }

    /// Create a cached shape with a layer ID
    pub fn with_layer(id: ShapeId, kind: ShapeKind, layer_id: Option<LayerId>) -> Self {
        let bounds = Self::compute_bounds(&kind);
        let snap_points = Self::compute_snap_points(&kind);
        let resize_handles = Self::compute_resize_handles(&kind);
        Self {
            id,
            kind,
            layer_id,
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

    /// Update the cached shape with new kind data and recompute derived fields
    pub fn update(&mut self, kind: ShapeKind) {
        self.bounds = Self::compute_bounds(&kind);
        self.snap_points = Self::compute_snap_points(&kind);
        self.resize_handles = Self::compute_resize_handles(&kind);
        self.kind = kind;
    }

    pub fn compute_bounds(kind: &ShapeKind) -> (i32, i32, i32, i32) {
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
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                ..
            } => (
                center.x - *half_width,
                center.y - *half_height,
                center.x + *half_width,
                center.y + *half_height,
            ),
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => (
                center.x - *radius_x,
                center.y - *radius_y,
                center.x + *radius_x,
                center.y + *radius_y,
            ),
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
            ShapeKind::Parallelogram { start, end, .. }
            | ShapeKind::Trapezoid { start, end, .. }
            | ShapeKind::RoundedRect { start, end, .. }
            | ShapeKind::Cylinder { start, end, .. }
            | ShapeKind::Cloud { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                (min_x, min_y, max_x, max_y)
            }
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                ..
            } => (
                center.x - *radius_x,
                center.y - *radius_y,
                center.x + *radius_x,
                center.y + *radius_y,
            ),
            ShapeKind::Star {
                center,
                outer_radius,
                ..
            } => (
                center.x - *outer_radius,
                center.y - *outer_radius,
                center.x + *outer_radius,
                center.y + *outer_radius,
            ),
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
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                ..
            } => {
                vec![
                    Position::new(center.x, center.y - *half_height),
                    Position::new(center.x, center.y + *half_height),
                    Position::new(center.x - *half_width, center.y),
                    Position::new(center.x + *half_width, center.y),
                ]
            }
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => {
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
            ShapeKind::Parallelogram { start, end, .. }
            | ShapeKind::Trapezoid { start, end, .. }
            | ShapeKind::RoundedRect { start, end, .. }
            | ShapeKind::Cylinder { start, end, .. }
            | ShapeKind::Cloud { start, end, .. } => {
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
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                vec![
                    Position::new(center.x, center.y - *radius_y),
                    Position::new(center.x, center.y + *radius_y),
                    Position::new(center.x - *radius_x, center.y),
                    Position::new(center.x + *radius_x, center.y),
                ]
            }
            ShapeKind::Star {
                center,
                outer_radius,
                ..
            } => {
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
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(min_x, min_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(max_x, min_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(min_x, max_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(max_x, max_y),
                    },
                ]
            }
            ShapeKind::Line { start, end, .. } | ShapeKind::Arrow { start, end, .. } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::Start,
                        pos: *start,
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::End,
                        pos: *end,
                    },
                ]
            }
            ShapeKind::Diamond {
                center,
                half_width,
                half_height,
                ..
            } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(center.x, center.y - *half_height),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(center.x + *half_width, center.y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(center.x - *half_width, center.y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(center.x, center.y + *half_height),
                    },
                ]
            }
            ShapeKind::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(center.x - *radius_x, center.y - *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(center.x + *radius_x, center.y - *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(center.x - *radius_x, center.y + *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(center.x + *radius_x, center.y + *radius_y),
                    },
                ]
            }
            ShapeKind::Triangle { p1, p2, p3, .. } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: *p1,
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: *p2,
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: *p3,
                    },
                ]
            }
            ShapeKind::Parallelogram { start, end, .. }
            | ShapeKind::Trapezoid { start, end, .. }
            | ShapeKind::RoundedRect { start, end, .. }
            | ShapeKind::Cylinder { start, end, .. }
            | ShapeKind::Cloud { start, end, .. } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);

                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(min_x, min_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(max_x, min_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(min_x, max_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(max_x, max_y),
                    },
                ]
            }
            ShapeKind::Hexagon {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(center.x - *radius_x, center.y - *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(center.x + *radius_x, center.y - *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(center.x - *radius_x, center.y + *radius_y),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(center.x + *radius_x, center.y + *radius_y),
                    },
                ]
            }
            ShapeKind::Star {
                center,
                outer_radius,
                ..
            } => {
                vec![
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopLeft,
                        pos: Position::new(center.x - *outer_radius, center.y - *outer_radius),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::TopRight,
                        pos: Position::new(center.x + *outer_radius, center.y - *outer_radius),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomLeft,
                        pos: Position::new(center.x - *outer_radius, center.y + *outer_radius),
                    },
                    ResizeHandleInfo {
                        handle: ResizeHandle::BottomRight,
                        pos: Position::new(center.x + *outer_radius, center.y + *outer_radius),
                    },
                ]
            }
            _ => vec![],
        }
    }
}

/// Read-only cache of shapes for rendering (rebuilds from document)
#[derive(Debug)]
pub struct ShapeView {
    /// Cached shapes in render order (layer-first, then z-order within layer)
    shapes: Vec<CachedShape>,
    /// Fast lookup by ID
    by_id: HashMap<ShapeId, usize>,
    /// Hidden layer IDs (for visibility toggle)
    hidden_layers: std::collections::HashSet<LayerId>,
}

impl ShapeView {
    pub fn new() -> Self {
        Self {
            shapes: Vec::new(),
            by_id: HashMap::new(),
            hidden_layers: std::collections::HashSet::new(),
        }
    }

    /// Rebuild cache from document (respects layer order and shape order for z-ordering)
    pub fn rebuild(&mut self, doc: &Document) -> anyhow::Result<()> {
        self.shapes.clear();
        self.by_id.clear();

        // Get layer order (bottom to top)
        let layer_order = doc.read_layer_order()?;

        // Get the ordered list of shape IDs
        let shape_order = doc.read_shape_order()?;

        // Build a map of all shapes for quick lookup
        let all_shapes: std::collections::HashMap<_, _> =
            doc.read_all_shapes()?.into_iter().collect();

        // Build shape -> layer mapping
        let mut shape_layers: std::collections::HashMap<ShapeId, Option<LayerId>> =
            std::collections::HashMap::new();
        for &id in all_shapes.keys() {
            shape_layers.insert(id, doc.get_shape_layer(id).ok().flatten());
        }

        // Get default layer for shapes without layer
        let default_layer = layer_order.first().copied();

        // Add shapes in layer order (layer-first rendering)
        // For each layer, add shapes in shape_order within that layer
        for layer_id in &layer_order {
            for shape_id in &shape_order {
                if let Some(kind) = all_shapes.get(shape_id) {
                    let shape_layer = shape_layers.get(shape_id).copied().flatten();
                    // Shape belongs to this layer if:
                    // - It has this layer ID, OR
                    // - It has no layer and this is the default layer
                    let belongs_to_layer = shape_layer == Some(*layer_id)
                        || (shape_layer.is_none() && default_layer == Some(*layer_id));

                    if belongs_to_layer && !self.by_id.contains_key(shape_id) {
                        let idx = self.shapes.len();
                        self.shapes.push(CachedShape::with_layer(
                            *shape_id,
                            kind.clone(),
                            Some(*layer_id),
                        ));
                        self.by_id.insert(*shape_id, idx);
                    }
                }
            }
        }

        // Add any shapes that might not be in any layer (migration case)
        for (id, kind) in all_shapes {
            if !self.by_id.contains_key(&id) {
                let idx = self.shapes.len();
                self.shapes
                    .push(CachedShape::with_layer(id, kind, default_layer));
                self.by_id.insert(id, idx);
            }
        }

        Ok(())
    }

    /// Set layer visibility
    pub fn set_layer_visible(&mut self, layer_id: LayerId, visible: bool) {
        if visible {
            self.hidden_layers.remove(&layer_id);
        } else {
            self.hidden_layers.insert(layer_id);
        }
    }

    /// Check if a layer is visible
    #[allow(dead_code)]
    pub fn is_layer_visible(&self, layer_id: LayerId) -> bool {
        !self.hidden_layers.contains(&layer_id)
    }

    /// Iterate all shapes for rendering
    pub fn iter(&self) -> impl Iterator<Item = &CachedShape> {
        self.shapes.iter()
    }

    /// Iterate only visible shapes (respecting layer visibility)
    pub fn iter_visible(&self) -> impl Iterator<Item = &CachedShape> {
        self.shapes.iter().filter(|shape| {
            match shape.layer_id {
                Some(layer_id) => !self.hidden_layers.contains(&layer_id),
                None => true, // Shapes without layer are always visible
            }
        })
    }

    /// Get total number of shapes
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
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
                points.push(SnapPoint {
                    pos,
                    shape_id: shape.id,
                });
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
                        best = Some((
                            SnapPoint {
                                pos: snap_pos,
                                shape_id: shape.id,
                            },
                            dist,
                        ));
                    }
                }
            }
        }
        best.map(|(s, _)| s)
    }

    /// Find resize handle on a shape near a position
    pub fn find_resize_handle(
        &self,
        shape_id: ShapeId,
        pos: Position,
        threshold: i32,
    ) -> Option<ResizeHandle> {
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
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Update only specific shapes in the cache (for incremental updates during drag)
    #[allow(dead_code)]
    pub fn update_shapes(&mut self, doc: &Document, ids: &[ShapeId]) {
        for &id in ids {
            if let Some(&idx) = self.by_id.get(&id) {
                if let Ok(Some(kind)) = doc.read_shape(id) {
                    self.shapes[idx].update(kind);
                }
            }
        }
    }

    /// Find all shapes connected to the given shape ID and return updated versions
    /// Returns Vec of (ShapeId, new ShapeKind) for shapes that need updating
    pub fn find_connected_updates(
        &self,
        target_id: ShapeId,
        dx: i32,
        dy: i32,
    ) -> Vec<(ShapeId, ShapeKind)> {
        let target_conn_id = target_id.0.as_u128() as u64;
        let mut updates = Vec::new();

        for shape in &self.shapes {
            if let Some(new_kind) = shape
                .kind
                .translate_connected_endpoints(target_conn_id, dx, dy)
            {
                updates.push((shape.id, new_kind));
            }
        }

        updates
    }

    /// Update a shape's cache entry directly with a new kind (without reading from document)
    pub fn update_shape_kind(&mut self, id: ShapeId, kind: ShapeKind) {
        if let Some(&idx) = self.by_id.get(&id) {
            self.shapes[idx].update(kind);
        }
    }

    /// Find all shapes connected to the resized shape and return updated versions
    /// This handles the case where different snap points move by different amounts
    /// Returns Vec of (ShapeId, new ShapeKind) for shapes that need updating
    pub fn find_connected_updates_for_resize(
        &self,
        resized_id: ShapeId,
        original_kind: &ShapeKind,
        new_kind: &ShapeKind,
    ) -> Vec<(ShapeId, ShapeKind)> {
        let old_snaps = original_kind.snap_points();
        let new_snaps = new_kind.snap_points();

        // If snap point counts don't match, we can't reliably update connections
        if old_snaps.len() != new_snaps.len() {
            return Vec::new();
        }

        let resized_conn_id = resized_id.0.as_u128() as u64;
        let mut updates = Vec::new();

        for shape in &self.shapes {
            if let Some(new_shape_kind) =
                shape
                    .kind
                    .update_for_resize(resized_conn_id, &old_snaps, &new_snaps)
            {
                updates.push((shape.id, new_shape_kind));
            }
        }

        updates
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
        ShapeKind::Rectangle {
            start,
            end,
            label,
            color,
        }
        | ShapeKind::DoubleBox {
            start,
            end,
            label,
            color,
        } => {
            let is_double = matches!(kind, ShapeKind::DoubleBox { .. });
            let is_start_left = start.x <= end.x;
            let is_start_top = start.y <= end.y;

            let (new_start, new_end) = match handle {
                ResizeHandle::TopLeft => {
                    if is_start_left && is_start_top {
                        (new_pos, *end)
                    } else if !is_start_left && is_start_top {
                        (
                            Position::new(start.x, new_pos.y),
                            Position::new(new_pos.x, end.y),
                        )
                    } else if is_start_left && !is_start_top {
                        (
                            Position::new(new_pos.x, start.y),
                            Position::new(end.x, new_pos.y),
                        )
                    } else {
                        (*start, new_pos)
                    }
                }
                ResizeHandle::TopRight => {
                    if is_start_left && is_start_top {
                        (
                            Position::new(start.x, new_pos.y),
                            Position::new(new_pos.x, end.y),
                        )
                    } else if !is_start_left && is_start_top {
                        (new_pos, *end)
                    } else if is_start_left && !is_start_top {
                        (*start, new_pos)
                    } else {
                        (
                            Position::new(new_pos.x, start.y),
                            Position::new(end.x, new_pos.y),
                        )
                    }
                }
                ResizeHandle::BottomLeft => {
                    if is_start_left && is_start_top {
                        (
                            Position::new(new_pos.x, start.y),
                            Position::new(end.x, new_pos.y),
                        )
                    } else if !is_start_left && is_start_top {
                        (*start, new_pos)
                    } else if is_start_left && !is_start_top {
                        (new_pos, *end)
                    } else {
                        (
                            Position::new(start.x, new_pos.y),
                            Position::new(new_pos.x, end.y),
                        )
                    }
                }
                ResizeHandle::BottomRight => {
                    if is_start_left && is_start_top {
                        (*start, new_pos)
                    } else if !is_start_left && is_start_top {
                        (
                            Position::new(new_pos.x, start.y),
                            Position::new(end.x, new_pos.y),
                        )
                    } else if is_start_left && !is_start_top {
                        (
                            Position::new(start.x, new_pos.y),
                            Position::new(new_pos.x, end.y),
                        )
                    } else {
                        (new_pos, *end)
                    }
                }
                _ => (*start, *end),
            };

            if is_double {
                ShapeKind::DoubleBox {
                    start: new_start,
                    end: new_end,
                    label: label.clone(),
                    color: *color,
                }
            } else {
                ShapeKind::Rectangle {
                    start: new_start,
                    end: new_end,
                    label: label.clone(),
                    color: *color,
                }
            }
        }
        ShapeKind::Line {
            start,
            end,
            style,
            start_connection,
            end_connection,
            label,
            color,
        } => match handle {
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
        },
        ShapeKind::Arrow {
            start,
            end,
            style,
            start_connection,
            end_connection,
            label,
            color,
        } => match handle {
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
        },
        ShapeKind::Diamond {
            center,
            half_width,
            half_height,
            label,
            color,
        } => match handle {
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
        },
        ShapeKind::Ellipse {
            center,
            label,
            color,
            ..
        } => match handle {
            ResizeHandle::TopLeft
            | ResizeHandle::TopRight
            | ResizeHandle::BottomLeft
            | ResizeHandle::BottomRight => ShapeKind::Ellipse {
                center: *center,
                radius_x: (new_pos.x - center.x).abs().max(1),
                radius_y: (new_pos.y - center.y).abs().max(1),
                label: label.clone(),
                color: *color,
            },
            _ => kind.clone(),
        },
        ShapeKind::Triangle {
            p1,
            p2,
            p3,
            label,
            color,
        } => match handle {
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
        },
        ShapeKind::Parallelogram {
            start,
            end,
            label,
            color,
        } => resize_rect_like(start, end, handle, new_pos, |s, e| {
            ShapeKind::Parallelogram {
                start: s,
                end: e,
                label: label.clone(),
                color: *color,
            }
        }),
        ShapeKind::Trapezoid {
            start,
            end,
            label,
            color,
        } => resize_rect_like(start, end, handle, new_pos, |s, e| ShapeKind::Trapezoid {
            start: s,
            end: e,
            label: label.clone(),
            color: *color,
        }),
        ShapeKind::RoundedRect {
            start,
            end,
            label,
            color,
        } => resize_rect_like(start, end, handle, new_pos, |s, e| ShapeKind::RoundedRect {
            start: s,
            end: e,
            label: label.clone(),
            color: *color,
        }),
        ShapeKind::Cylinder {
            start,
            end,
            label,
            color,
        } => resize_rect_like(start, end, handle, new_pos, |s, e| ShapeKind::Cylinder {
            start: s,
            end: e,
            label: label.clone(),
            color: *color,
        }),
        ShapeKind::Cloud {
            start,
            end,
            label,
            color,
        } => resize_rect_like(start, end, handle, new_pos, |s, e| ShapeKind::Cloud {
            start: s,
            end: e,
            label: label.clone(),
            color: *color,
        }),
        ShapeKind::Hexagon {
            center,
            label,
            color,
            ..
        } => match handle {
            ResizeHandle::TopLeft
            | ResizeHandle::TopRight
            | ResizeHandle::BottomLeft
            | ResizeHandle::BottomRight => ShapeKind::Hexagon {
                center: *center,
                radius_x: (new_pos.x - center.x).abs().max(1),
                radius_y: (new_pos.y - center.y).abs().max(1),
                label: label.clone(),
                color: *color,
            },
            _ => kind.clone(),
        },
        ShapeKind::Star {
            center,
            inner_radius,
            label,
            color,
            ..
        } => match handle {
            ResizeHandle::TopLeft
            | ResizeHandle::TopRight
            | ResizeHandle::BottomLeft
            | ResizeHandle::BottomRight => {
                let outer = ((new_pos.x - center.x)
                    .abs()
                    .max((new_pos.y - center.y).abs()))
                .max(2);
                ShapeKind::Star {
                    center: *center,
                    outer_radius: outer,
                    inner_radius: (*inner_radius).min(outer - 1).max(1),
                    label: label.clone(),
                    color: *color,
                }
            }
            _ => kind.clone(),
        },
        _ => kind.clone(),
    }
}

/// Helper for resizing rectangle-like shapes
fn resize_rect_like<F>(
    start: &Position,
    end: &Position,
    handle: ResizeHandle,
    new_pos: Position,
    make_shape: F,
) -> ShapeKind
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
                (
                    Position::new(start.x, new_pos.y),
                    Position::new(new_pos.x, end.y),
                )
            } else if is_start_left && !is_start_top {
                (
                    Position::new(new_pos.x, start.y),
                    Position::new(end.x, new_pos.y),
                )
            } else {
                (*start, new_pos)
            }
        }
        ResizeHandle::TopRight => {
            if is_start_left && is_start_top {
                (
                    Position::new(start.x, new_pos.y),
                    Position::new(new_pos.x, end.y),
                )
            } else if !is_start_left && is_start_top {
                (new_pos, *end)
            } else if is_start_left && !is_start_top {
                (*start, new_pos)
            } else {
                (
                    Position::new(new_pos.x, start.y),
                    Position::new(end.x, new_pos.y),
                )
            }
        }
        ResizeHandle::BottomLeft => {
            if is_start_left && is_start_top {
                (
                    Position::new(new_pos.x, start.y),
                    Position::new(end.x, new_pos.y),
                )
            } else if !is_start_left && is_start_top {
                (*start, new_pos)
            } else if is_start_left && !is_start_top {
                (new_pos, *end)
            } else {
                (
                    Position::new(start.x, new_pos.y),
                    Position::new(new_pos.x, end.y),
                )
            }
        }
        ResizeHandle::BottomRight => {
            if is_start_left && is_start_top {
                (*start, new_pos)
            } else if !is_start_left && is_start_top {
                (
                    Position::new(new_pos.x, start.y),
                    Position::new(end.x, new_pos.y),
                )
            } else if is_start_left && !is_start_top {
                (
                    Position::new(start.x, new_pos.y),
                    Position::new(new_pos.x, end.y),
                )
            } else {
                (new_pos, *end)
            }
        }
        _ => (*start, *end),
    };

    make_shape(new_start, new_end)
}

/// Find the corresponding new snap point for a position based on the closest old snap point
fn find_corresponding_snap(
    pos: &Position,
    old_snaps: &[Position],
    new_snaps: &[Position],
) -> Option<Position> {
    // Find the closest old snap point to this position
    let mut best_idx = None;
    let mut best_dist = i32::MAX;

    for (idx, old_snap) in old_snaps.iter().enumerate() {
        let dist = (pos.x - old_snap.x).abs() + (pos.y - old_snap.y).abs();
        if dist < best_dist {
            best_dist = dist;
            best_idx = Some(idx);
        }
    }

    // If we found a matching snap point and the new snaps have the same index, return it
    best_idx.and_then(|idx| new_snaps.get(idx).copied())
}

/// Flip a shape horizontally (mirror across vertical axis through center_x).
pub fn flip_horizontal(kind: &ShapeKind, center_x: i32) -> ShapeKind {
    let mirror_x = |x: i32| 2 * center_x - x;

    match kind {
        ShapeKind::Line {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Line {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Arrow {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Arrow {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Rectangle {
            start,
            end,
            label,
            color,
        } => ShapeKind::Rectangle {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::DoubleBox {
            start,
            end,
            label,
            color,
        } => ShapeKind::DoubleBox {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Diamond {
            center,
            half_width,
            half_height,
            label,
            color,
        } => ShapeKind::Diamond {
            center: Position::new(mirror_x(center.x), center.y),
            half_width: *half_width,
            half_height: *half_height,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Ellipse {
            center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Ellipse {
            center: Position::new(mirror_x(center.x), center.y),
            radius_x: *radius_x,
            radius_y: *radius_y,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Freehand {
            points,
            char,
            label,
            color,
        } => ShapeKind::Freehand {
            points: points
                .iter()
                .map(|p| Position::new(mirror_x(p.x), p.y))
                .collect(),
            char: *char,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Text {
            pos,
            content,
            color,
        } => ShapeKind::Text {
            pos: Position::new(mirror_x(pos.x), pos.y),
            content: content.clone(),
            color: *color,
        },
        ShapeKind::Triangle {
            p1,
            p2,
            p3,
            label,
            color,
        } => ShapeKind::Triangle {
            p1: Position::new(mirror_x(p1.x), p1.y),
            p2: Position::new(mirror_x(p2.x), p2.y),
            p3: Position::new(mirror_x(p3.x), p3.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Parallelogram {
            start,
            end,
            label,
            color,
        } => ShapeKind::Parallelogram {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Hexagon {
            center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Hexagon {
            center: Position::new(mirror_x(center.x), center.y),
            radius_x: *radius_x,
            radius_y: *radius_y,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Trapezoid {
            start,
            end,
            label,
            color,
        } => ShapeKind::Trapezoid {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::RoundedRect {
            start,
            end,
            label,
            color,
        } => ShapeKind::RoundedRect {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cylinder {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cylinder {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cloud {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cloud {
            start: Position::new(mirror_x(start.x), start.y),
            end: Position::new(mirror_x(end.x), end.y),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Star {
            center,
            outer_radius,
            inner_radius,
            label,
            color,
        } => ShapeKind::Star {
            center: Position::new(mirror_x(center.x), center.y),
            outer_radius: *outer_radius,
            inner_radius: *inner_radius,
            label: label.clone(),
            color: *color,
        },
    }
}

/// Flip a shape vertically (mirror across horizontal axis through center_y).
pub fn flip_vertical(kind: &ShapeKind, center_y: i32) -> ShapeKind {
    let mirror_y = |y: i32| 2 * center_y - y;

    match kind {
        ShapeKind::Line {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Line {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Arrow {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Arrow {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Rectangle {
            start,
            end,
            label,
            color,
        } => ShapeKind::Rectangle {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::DoubleBox {
            start,
            end,
            label,
            color,
        } => ShapeKind::DoubleBox {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Diamond {
            center,
            half_width,
            half_height,
            label,
            color,
        } => ShapeKind::Diamond {
            center: Position::new(center.x, mirror_y(center.y)),
            half_width: *half_width,
            half_height: *half_height,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Ellipse {
            center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Ellipse {
            center: Position::new(center.x, mirror_y(center.y)),
            radius_x: *radius_x,
            radius_y: *radius_y,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Freehand {
            points,
            char,
            label,
            color,
        } => ShapeKind::Freehand {
            points: points
                .iter()
                .map(|p| Position::new(p.x, mirror_y(p.y)))
                .collect(),
            char: *char,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Text {
            pos,
            content,
            color,
        } => ShapeKind::Text {
            pos: Position::new(pos.x, mirror_y(pos.y)),
            content: content.clone(),
            color: *color,
        },
        ShapeKind::Triangle {
            p1,
            p2,
            p3,
            label,
            color,
        } => ShapeKind::Triangle {
            p1: Position::new(p1.x, mirror_y(p1.y)),
            p2: Position::new(p2.x, mirror_y(p2.y)),
            p3: Position::new(p3.x, mirror_y(p3.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Parallelogram {
            start,
            end,
            label,
            color,
        } => ShapeKind::Parallelogram {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Hexagon {
            center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Hexagon {
            center: Position::new(center.x, mirror_y(center.y)),
            radius_x: *radius_x,
            radius_y: *radius_y,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Trapezoid {
            start,
            end,
            label,
            color,
        } => ShapeKind::Trapezoid {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::RoundedRect {
            start,
            end,
            label,
            color,
        } => ShapeKind::RoundedRect {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cylinder {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cylinder {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cloud {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cloud {
            start: Position::new(start.x, mirror_y(start.y)),
            end: Position::new(end.x, mirror_y(end.y)),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Star {
            center,
            outer_radius,
            inner_radius,
            label,
            color,
        } => ShapeKind::Star {
            center: Position::new(center.x, mirror_y(center.y)),
            outer_radius: *outer_radius,
            inner_radius: *inner_radius,
            label: label.clone(),
            color: *color,
        },
    }
}

/// Rotate a shape 90 degrees clockwise around a center point.
pub fn rotate_90_cw(kind: &ShapeKind, center: Position) -> ShapeKind {
    // Rotation formula: (x', y') = (cx + (y - cy), cy - (x - cx))
    let rotate_point = |p: Position| -> Position {
        Position::new(center.x + (p.y - center.y), center.y - (p.x - center.x))
    };

    match kind {
        ShapeKind::Line {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Line {
            start: rotate_point(*start),
            end: rotate_point(*end),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Arrow {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Arrow {
            start: rotate_point(*start),
            end: rotate_point(*end),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Rectangle {
            start,
            end,
            label,
            color,
        } => ShapeKind::Rectangle {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::DoubleBox {
            start,
            end,
            label,
            color,
        } => ShapeKind::DoubleBox {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Diamond {
            center: shape_center,
            half_width,
            half_height,
            label,
            color,
        } => ShapeKind::Diamond {
            center: rotate_point(*shape_center),
            half_width: *half_height, // Swap dimensions
            half_height: *half_width,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Ellipse {
            center: shape_center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Ellipse {
            center: rotate_point(*shape_center),
            radius_x: *radius_y, // Swap dimensions
            radius_y: *radius_x,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Freehand {
            points,
            char,
            label,
            color,
        } => ShapeKind::Freehand {
            points: points.iter().map(|p| rotate_point(*p)).collect(),
            char: *char,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Text {
            pos,
            content,
            color,
        } => ShapeKind::Text {
            pos: rotate_point(*pos),
            content: content.clone(),
            color: *color,
        },
        ShapeKind::Triangle {
            p1,
            p2,
            p3,
            label,
            color,
        } => ShapeKind::Triangle {
            p1: rotate_point(*p1),
            p2: rotate_point(*p2),
            p3: rotate_point(*p3),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Parallelogram {
            start,
            end,
            label,
            color,
        } => ShapeKind::Parallelogram {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Hexagon {
            center: shape_center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Hexagon {
            center: rotate_point(*shape_center),
            radius_x: *radius_y, // Swap dimensions
            radius_y: *radius_x,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Trapezoid {
            start,
            end,
            label,
            color,
        } => ShapeKind::Trapezoid {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::RoundedRect {
            start,
            end,
            label,
            color,
        } => ShapeKind::RoundedRect {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cylinder {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cylinder {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cloud {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cloud {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Star {
            center: shape_center,
            outer_radius,
            inner_radius,
            label,
            color,
        } => ShapeKind::Star {
            center: rotate_point(*shape_center),
            outer_radius: *outer_radius, // Star radii don't need swapping (radial symmetry)
            inner_radius: *inner_radius,
            label: label.clone(),
            color: *color,
        },
    }
}

/// Rotate a shape 90 degrees counter-clockwise around a center point.
pub fn rotate_90_ccw(kind: &ShapeKind, center: Position) -> ShapeKind {
    // Rotation formula: (x', y') = (cx - (y - cy), cy + (x - cx))
    let rotate_point = |p: Position| -> Position {
        Position::new(center.x - (p.y - center.y), center.y + (p.x - center.x))
    };

    match kind {
        ShapeKind::Line {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Line {
            start: rotate_point(*start),
            end: rotate_point(*end),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Arrow {
            start,
            end,
            style,
            label,
            color,
            ..
        } => ShapeKind::Arrow {
            start: rotate_point(*start),
            end: rotate_point(*end),
            style: *style,
            start_connection: None,
            end_connection: None,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Rectangle {
            start,
            end,
            label,
            color,
        } => ShapeKind::Rectangle {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::DoubleBox {
            start,
            end,
            label,
            color,
        } => ShapeKind::DoubleBox {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Diamond {
            center: shape_center,
            half_width,
            half_height,
            label,
            color,
        } => ShapeKind::Diamond {
            center: rotate_point(*shape_center),
            half_width: *half_height, // Swap dimensions
            half_height: *half_width,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Ellipse {
            center: shape_center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Ellipse {
            center: rotate_point(*shape_center),
            radius_x: *radius_y, // Swap dimensions
            radius_y: *radius_x,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Freehand {
            points,
            char,
            label,
            color,
        } => ShapeKind::Freehand {
            points: points.iter().map(|p| rotate_point(*p)).collect(),
            char: *char,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Text {
            pos,
            content,
            color,
        } => ShapeKind::Text {
            pos: rotate_point(*pos),
            content: content.clone(),
            color: *color,
        },
        ShapeKind::Triangle {
            p1,
            p2,
            p3,
            label,
            color,
        } => ShapeKind::Triangle {
            p1: rotate_point(*p1),
            p2: rotate_point(*p2),
            p3: rotate_point(*p3),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Parallelogram {
            start,
            end,
            label,
            color,
        } => ShapeKind::Parallelogram {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Hexagon {
            center: shape_center,
            radius_x,
            radius_y,
            label,
            color,
        } => ShapeKind::Hexagon {
            center: rotate_point(*shape_center),
            radius_x: *radius_y, // Swap dimensions
            radius_y: *radius_x,
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Trapezoid {
            start,
            end,
            label,
            color,
        } => ShapeKind::Trapezoid {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::RoundedRect {
            start,
            end,
            label,
            color,
        } => ShapeKind::RoundedRect {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cylinder {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cylinder {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Cloud {
            start,
            end,
            label,
            color,
        } => ShapeKind::Cloud {
            start: rotate_point(*start),
            end: rotate_point(*end),
            label: label.clone(),
            color: *color,
        },
        ShapeKind::Star {
            center: shape_center,
            outer_radius,
            inner_radius,
            label,
            color,
        } => ShapeKind::Star {
            center: rotate_point(*shape_center),
            outer_radius: *outer_radius, // Star radii don't need swapping (radial symmetry)
            inner_radius: *inner_radius,
            label: label.clone(),
            color: *color,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== ShapeColor tests ==========

    #[test]
    fn shape_color_default() {
        assert_eq!(ShapeColor::default(), ShapeColor::White);
    }

    #[test]
    fn shape_color_cycle_all() {
        let mut color = ShapeColor::White;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..16 {
            seen.insert(std::mem::discriminant(&color));
            color = color.next();
        }
        assert_eq!(seen.len(), 16);
        assert_eq!(color, ShapeColor::White); // Completes cycle
    }

    #[test]
    fn shape_color_to_css() {
        assert_eq!(ShapeColor::White.to_css(), "white");
        assert_eq!(ShapeColor::Black.to_css(), "black");
        assert_eq!(ShapeColor::Red.to_css(), "#cd0000");
        assert_eq!(ShapeColor::LightRed.to_css(), "#ff0000");
    }

    #[test]
    fn shape_color_name() {
        assert_eq!(ShapeColor::White.name(), "White");
        assert_eq!(ShapeColor::LightMagenta.name(), "LightMagenta");
    }

    // ========== ShapeKind tests ==========

    fn make_rect(x1: i32, y1: i32, x2: i32, y2: i32) -> ShapeKind {
        ShapeKind::Rectangle {
            start: Position::new(x1, y1),
            end: Position::new(x2, y2),
            label: None,
            color: ShapeColor::White,
        }
    }

    fn make_line(x1: i32, y1: i32, x2: i32, y2: i32) -> ShapeKind {
        ShapeKind::Line {
            start: Position::new(x1, y1),
            end: Position::new(x2, y2),
            style: LineStyle::Straight,
            start_connection: None,
            end_connection: None,
            label: None,
            color: ShapeColor::White,
        }
    }

    #[test]
    fn shape_kind_type_name() {
        assert_eq!(make_rect(0, 0, 10, 10).type_name(), "Rectangle");
        assert_eq!(make_line(0, 0, 10, 10).type_name(), "Line");
        assert_eq!(
            ShapeKind::Text {
                pos: Position::new(0, 0),
                content: "test".to_string(),
                color: ShapeColor::White
            }
            .type_name(),
            "Text"
        );
    }

    #[test]
    fn shape_kind_translated_rectangle() {
        let rect = make_rect(0, 0, 10, 5);
        let translated = rect.translated(5, 3);
        if let ShapeKind::Rectangle { start, end, .. } = translated {
            assert_eq!(start, Position::new(5, 3));
            assert_eq!(end, Position::new(15, 8));
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn shape_kind_translated_line() {
        let line = make_line(0, 0, 10, 10);
        let translated = line.translated(-5, 2);
        if let ShapeKind::Line { start, end, .. } = translated {
            assert_eq!(start, Position::new(-5, 2));
            assert_eq!(end, Position::new(5, 12));
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn shape_kind_translated_freehand() {
        let freehand = ShapeKind::Freehand {
            points: vec![
                Position::new(0, 0),
                Position::new(1, 1),
                Position::new(2, 2),
            ],
            char: '*',
            label: None,
            color: ShapeColor::White,
        };
        let translated = freehand.translated(10, 10);
        if let ShapeKind::Freehand { points, .. } = translated {
            assert_eq!(points[0], Position::new(10, 10));
            assert_eq!(points[1], Position::new(11, 11));
            assert_eq!(points[2], Position::new(12, 12));
        } else {
            panic!("Expected Freehand");
        }
    }

    #[test]
    fn shape_kind_translated_text() {
        let text = ShapeKind::Text {
            pos: Position::new(5, 5),
            content: "Hello".to_string(),
            color: ShapeColor::White,
        };
        let translated = text.translated(3, 2);
        if let ShapeKind::Text { pos, content, .. } = translated {
            assert_eq!(pos, Position::new(8, 7));
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn shape_kind_label() {
        let rect = ShapeKind::Rectangle {
            start: Position::new(0, 0),
            end: Position::new(10, 5),
            label: Some("Test".to_string()),
            color: ShapeColor::White,
        };
        assert_eq!(rect.label(), Some("Test"));

        let rect_no_label = make_rect(0, 0, 10, 5);
        assert_eq!(rect_no_label.label(), None);

        // Text doesn't use label
        let text = ShapeKind::Text {
            pos: Position::new(0, 0),
            content: "Hello".to_string(),
            color: ShapeColor::White,
        };
        assert_eq!(text.label(), None);
    }

    #[test]
    fn shape_kind_with_label() {
        let rect = make_rect(0, 0, 10, 5);
        let labeled = rect.with_label(Some("New Label".to_string()));
        assert_eq!(labeled.label(), Some("New Label"));
    }

    #[test]
    fn shape_kind_color() {
        let rect = make_rect(0, 0, 10, 5);
        assert_eq!(rect.color(), ShapeColor::White);
    }

    #[test]
    fn shape_kind_with_color() {
        let rect = make_rect(0, 0, 10, 5);
        let colored = rect.with_color(ShapeColor::Red);
        assert_eq!(colored.color(), ShapeColor::Red);
    }

    #[test]
    fn shape_kind_supports_label() {
        assert!(make_rect(0, 0, 10, 5).supports_label());
        assert!(make_line(0, 0, 10, 5).supports_label());
        assert!(
            ShapeKind::Ellipse {
                center: Position::new(5, 5),
                radius_x: 3,
                radius_y: 2,
                label: None,
                color: ShapeColor::White
            }
            .supports_label()
        );

        // Text does NOT support label (its content IS the label)
        assert!(
            !ShapeKind::Text {
                pos: Position::new(0, 0),
                content: "test".to_string(),
                color: ShapeColor::White
            }
            .supports_label()
        );
    }

    #[test]
    fn shape_kind_snap_points_rectangle() {
        let rect = make_rect(0, 0, 10, 6);
        let snaps = rect.snap_points();
        assert_eq!(snaps.len(), 8);
        // Corners
        assert!(snaps.contains(&Position::new(0, 0)));
        assert!(snaps.contains(&Position::new(10, 0)));
        assert!(snaps.contains(&Position::new(0, 6)));
        assert!(snaps.contains(&Position::new(10, 6)));
        // Midpoints
        assert!(snaps.contains(&Position::new(5, 0)));
        assert!(snaps.contains(&Position::new(5, 6)));
        assert!(snaps.contains(&Position::new(0, 3)));
        assert!(snaps.contains(&Position::new(10, 3)));
    }

    #[test]
    fn shape_kind_snap_points_line() {
        let line = make_line(0, 0, 10, 10);
        let snaps = line.snap_points();
        assert_eq!(snaps.len(), 2);
        assert!(snaps.contains(&Position::new(0, 0)));
        assert!(snaps.contains(&Position::new(10, 10)));
    }

    // ========== CachedShape tests ==========

    #[test]
    fn cached_shape_bounds_rectangle() {
        let shape = CachedShape::new(ShapeId::new(), make_rect(5, 10, 15, 20));
        assert_eq!(shape.bounds(), (5, 10, 15, 20));
    }

    #[test]
    fn cached_shape_bounds_rectangle_swapped() {
        let shape = CachedShape::new(ShapeId::new(), make_rect(15, 20, 5, 10));
        assert_eq!(shape.bounds(), (5, 10, 15, 20));
    }

    #[test]
    fn cached_shape_bounds_ellipse() {
        let shape = CachedShape::new(
            ShapeId::new(),
            ShapeKind::Ellipse {
                center: Position::new(10, 10),
                radius_x: 5,
                radius_y: 3,
                label: None,
                color: ShapeColor::White,
            },
        );
        assert_eq!(shape.bounds(), (5, 7, 15, 13));
    }

    #[test]
    fn cached_shape_bounds_text() {
        let shape = CachedShape::new(
            ShapeId::new(),
            ShapeKind::Text {
                pos: Position::new(5, 10),
                content: "Hello".to_string(),
                color: ShapeColor::White,
            },
        );
        assert_eq!(shape.bounds(), (5, 10, 9, 10)); // 5 chars wide
    }

    #[test]
    fn cached_shape_bounds_freehand_empty() {
        let shape = CachedShape::new(
            ShapeId::new(),
            ShapeKind::Freehand {
                points: vec![],
                char: '*',
                label: None,
                color: ShapeColor::White,
            },
        );
        assert_eq!(shape.bounds(), (0, 0, 0, 0));
    }

    #[test]
    fn cached_shape_bounds_freehand() {
        let shape = CachedShape::new(
            ShapeId::new(),
            ShapeKind::Freehand {
                points: vec![
                    Position::new(5, 5),
                    Position::new(10, 3),
                    Position::new(7, 8),
                ],
                char: '*',
                label: None,
                color: ShapeColor::White,
            },
        );
        assert_eq!(shape.bounds(), (5, 3, 10, 8));
    }

    #[test]
    fn cached_shape_snap_points() {
        let shape = CachedShape::new(ShapeId::new(), make_rect(0, 0, 10, 6));
        assert_eq!(shape.snap_points().len(), 8);
    }

    #[test]
    fn cached_shape_contains() {
        let shape = CachedShape::new(ShapeId::new(), make_rect(0, 0, 10, 10));
        assert!(shape.contains(Position::new(5, 5)));
        assert!(shape.contains(Position::new(0, 0)));
        assert!(shape.contains(Position::new(10, 10)));
        assert!(!shape.contains(Position::new(-1, 5)));
        assert!(!shape.contains(Position::new(11, 5)));
    }

    #[test]
    fn cached_shape_label() {
        let shape = CachedShape::new(
            ShapeId::new(),
            ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 5),
                label: Some("Test".to_string()),
                color: ShapeColor::White,
            },
        );
        assert_eq!(shape.label(), Some("Test"));
    }

    #[test]
    fn cached_shape_resize_handles_rectangle() {
        let shape = CachedShape::new(ShapeId::new(), make_rect(0, 0, 10, 10));
        let handles = shape.resize_handles();
        assert_eq!(handles.len(), 4);
        assert!(handles.iter().any(|h| h.handle == ResizeHandle::TopLeft));
        assert!(handles.iter().any(|h| h.handle == ResizeHandle::TopRight));
        assert!(handles.iter().any(|h| h.handle == ResizeHandle::BottomLeft));
        assert!(
            handles
                .iter()
                .any(|h| h.handle == ResizeHandle::BottomRight)
        );
    }

    #[test]
    fn cached_shape_resize_handles_line() {
        let shape = CachedShape::new(ShapeId::new(), make_line(0, 0, 10, 10));
        let handles = shape.resize_handles();
        assert_eq!(handles.len(), 2);
        assert!(handles.iter().any(|h| h.handle == ResizeHandle::Start));
        assert!(handles.iter().any(|h| h.handle == ResizeHandle::End));
    }

    // ========== resize_shape tests ==========

    #[test]
    fn resize_rectangle_bottom_right() {
        let rect = make_rect(0, 0, 10, 10);
        let resized = resize_shape(&rect, ResizeHandle::BottomRight, Position::new(20, 15));
        if let ShapeKind::Rectangle { start, end, .. } = resized {
            assert_eq!(start, Position::new(0, 0));
            assert_eq!(end, Position::new(20, 15));
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn resize_rectangle_top_left() {
        let rect = make_rect(0, 0, 10, 10);
        let resized = resize_shape(&rect, ResizeHandle::TopLeft, Position::new(-5, -5));
        if let ShapeKind::Rectangle { start, end, .. } = resized {
            assert_eq!(start, Position::new(-5, -5));
            assert_eq!(end, Position::new(10, 10));
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn resize_line_start() {
        let line = make_line(0, 0, 10, 10);
        let resized = resize_shape(&line, ResizeHandle::Start, Position::new(-5, -5));
        if let ShapeKind::Line { start, end, .. } = resized {
            assert_eq!(start, Position::new(-5, -5));
            assert_eq!(end, Position::new(10, 10));
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn resize_line_end() {
        let line = make_line(0, 0, 10, 10);
        let resized = resize_shape(&line, ResizeHandle::End, Position::new(20, 5));
        if let ShapeKind::Line { start, end, .. } = resized {
            assert_eq!(start, Position::new(0, 0));
            assert_eq!(end, Position::new(20, 5));
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn resize_ellipse() {
        let ellipse = ShapeKind::Ellipse {
            center: Position::new(10, 10),
            radius_x: 5,
            radius_y: 3,
            label: None,
            color: ShapeColor::White,
        };
        let resized = resize_shape(&ellipse, ResizeHandle::BottomRight, Position::new(20, 15));
        if let ShapeKind::Ellipse {
            center,
            radius_x,
            radius_y,
            ..
        } = resized
        {
            assert_eq!(center, Position::new(10, 10));
            assert_eq!(radius_x, 10);
            assert_eq!(radius_y, 5);
        } else {
            panic!("Expected Ellipse");
        }
    }

    // ========== ShapeView tests ==========

    #[test]
    fn shape_view_new() {
        let view = ShapeView::new();
        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn shape_view_default() {
        let view = ShapeView::default();
        assert!(view.is_empty());
    }
}
