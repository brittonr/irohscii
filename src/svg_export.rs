//! SVG export functionality for irohscii
//!
//! Exports shapes to proper SVG elements with:
//! - 1 character = 10x16 SVG units (approximate monospace char aspect ratio)
//! - Arrow markers defined in <defs>
//! - Shape-specific rendering for each ShapeKind

use std::fmt::Write;
use std::path::Path;

use anyhow::Result;

use crate::canvas::{LineStyle, Position};
use crate::shapes::{CachedShape, ShapeKind, ShapeView};

/// Character dimensions in SVG units
const CHAR_WIDTH: i32 = 10;
const CHAR_HEIGHT: i32 = 16;

/// Convert canvas position to SVG coordinates
fn to_svg_coords(pos: Position) -> (i32, i32) {
    (pos.x * CHAR_WIDTH, pos.y * CHAR_HEIGHT)
}

/// Export shapes to SVG string
pub fn export_svg(shapes: &ShapeView) -> String {
    let mut output = String::new();

    // Calculate bounding box
    let (min_x, min_y, max_x, max_y) = calculate_bounds(shapes);
    let width = (max_x - min_x + 2) * CHAR_WIDTH;
    let height = (max_y - min_y + 2) * CHAR_HEIGHT;

    // Offset to translate shapes to start at (CHAR_WIDTH, CHAR_HEIGHT)
    let offset_x = -min_x + 1;
    let offset_y = -min_y + 1;

    // SVG header
    writeln!(
        &mut output,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="{}" height="{}"
     viewBox="0 0 {} {}"
     style="background-color: white;">"#,
        width, height, width, height
    )
    .unwrap();

    // Arrow marker definitions
    writeln!(
        &mut output,
        r#"  <defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="7"
            refX="9" refY="3.5" orient="auto" fill="black">
      <polygon points="0 0, 10 3.5, 0 7" />
    </marker>
  </defs>"#
    )
    .unwrap();

    // Render each shape
    for shape in shapes.iter() {
        render_shape(&mut output, shape, offset_x, offset_y);
    }

    // SVG footer
    writeln!(&mut output, "</svg>").unwrap();

    output
}

/// Save SVG to a file
pub fn save_svg(shapes: &ShapeView, path: &Path) -> Result<()> {
    let svg = export_svg(shapes);
    std::fs::write(path, svg)?;
    Ok(())
}

/// Calculate the bounding box of all shapes
fn calculate_bounds(shapes: &ShapeView) -> (i32, i32, i32, i32) {
    if shapes.is_empty() {
        return (0, 0, 10, 10);
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for shape in shapes.iter() {
        let (bmin_x, bmin_y, bmax_x, bmax_y) = shape.bounds();
        min_x = min_x.min(bmin_x);
        min_y = min_y.min(bmin_y);
        max_x = max_x.max(bmax_x);
        max_y = max_y.max(bmax_y);
    }

    (min_x, min_y, max_x, max_y)
}

/// Render a single shape to SVG
fn render_shape(output: &mut String, shape: &CachedShape, offset_x: i32, offset_y: i32) {
    match &shape.kind {
        ShapeKind::Line { start, end, style, .. } => {
            render_line(output, *start, *end, *style, false, offset_x, offset_y);
        }
        ShapeKind::Arrow { start, end, style, .. } => {
            render_line(output, *start, *end, *style, true, offset_x, offset_y);
        }
        ShapeKind::Rectangle { start, end, label } => {
            render_rectangle(output, *start, *end, label.as_deref(), false, offset_x, offset_y);
        }
        ShapeKind::DoubleBox { start, end, label } => {
            render_rectangle(output, *start, *end, label.as_deref(), true, offset_x, offset_y);
        }
        ShapeKind::Diamond { center, half_width, half_height, label } => {
            render_diamond(output, *center, *half_width, *half_height, label.as_deref(), offset_x, offset_y);
        }
        ShapeKind::Ellipse { center, radius_x, radius_y, label } => {
            render_ellipse(output, *center, *radius_x, *radius_y, label.as_deref(), offset_x, offset_y);
        }
        ShapeKind::Freehand { points, .. } => {
            render_freehand(output, points, offset_x, offset_y);
        }
        ShapeKind::Text { pos, content } => {
            render_text(output, *pos, content, offset_x, offset_y);
        }
    }
}

/// Render a line or arrow
fn render_line(
    output: &mut String,
    start: Position,
    end: Position,
    style: LineStyle,
    is_arrow: bool,
    offset_x: i32,
    offset_y: i32,
) {
    let start = Position::new(start.x + offset_x, start.y + offset_y);
    let end = Position::new(end.x + offset_x, end.y + offset_y);
    let (x1, y1) = to_svg_coords(start);
    let (x2, y2) = to_svg_coords(end);

    let marker = if is_arrow {
        r#" marker-end="url(#arrowhead)""#
    } else {
        ""
    };

    match style {
        LineStyle::Straight => {
            writeln!(
                output,
                r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="1"{}/>"#,
                x1, y1, x2, y2, marker
            )
            .unwrap();
        }
        LineStyle::OrthogonalHV => {
            // Horizontal first, then vertical
            let path = format!("M {} {} L {} {} L {} {}", x1, y1, x2, y1, x2, y2);
            writeln!(
                output,
                r#"  <path d="{}" stroke="black" stroke-width="1" fill="none"{}/>"#,
                path, marker
            )
            .unwrap();
        }
        LineStyle::OrthogonalVH => {
            // Vertical first, then horizontal
            let path = format!("M {} {} L {} {} L {} {}", x1, y1, x1, y2, x2, y2);
            writeln!(
                output,
                r#"  <path d="{}" stroke="black" stroke-width="1" fill="none"{}/>"#,
                path, marker
            )
            .unwrap();
        }
    }
}

/// Render a rectangle or double box
fn render_rectangle(
    output: &mut String,
    start: Position,
    end: Position,
    label: Option<&str>,
    is_double: bool,
    offset_x: i32,
    offset_y: i32,
) {
    let start = Position::new(start.x + offset_x, start.y + offset_y);
    let end = Position::new(end.x + offset_x, end.y + offset_y);

    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_x = start.x.max(end.x);
    let max_y = start.y.max(end.y);

    let (x, y) = to_svg_coords(Position::new(min_x, min_y));
    let (x2, y2) = to_svg_coords(Position::new(max_x, max_y));
    let width = x2 - x;
    let height = y2 - y;

    let stroke_width = if is_double { 3 } else { 1 };

    writeln!(
        output,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" stroke="black" stroke-width="{}" fill="white"/>"#,
        x, y, width, height, stroke_width
    )
    .unwrap();

    // Render label if present
    if let Some(text) = label {
        let center_x = x + width / 2;
        let center_y = y + height / 2;
        writeln!(
            output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12">{}</text>"#,
            center_x, center_y, escape_xml(text)
        )
        .unwrap();
    }
}

/// Render a diamond shape
fn render_diamond(
    output: &mut String,
    center: Position,
    half_width: i32,
    half_height: i32,
    label: Option<&str>,
    offset_x: i32,
    offset_y: i32,
) {
    let center = Position::new(center.x + offset_x, center.y + offset_y);
    let (cx, cy) = to_svg_coords(center);
    let hw = half_width * CHAR_WIDTH;
    let hh = half_height * CHAR_HEIGHT;

    // Diamond is a rotated square - 4 points
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        cx, cy - hh,       // top
        cx + hw, cy,       // right
        cx, cy + hh,       // bottom
        cx - hw, cy        // left
    );

    writeln!(
        output,
        r#"  <polygon points="{}" stroke="black" stroke-width="1" fill="white"/>"#,
        points
    )
    .unwrap();

    // Render label if present
    if let Some(text) = label {
        writeln!(
            output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12">{}</text>"#,
            cx, cy, escape_xml(text)
        )
        .unwrap();
    }
}

/// Render an ellipse
fn render_ellipse(
    output: &mut String,
    center: Position,
    radius_x: i32,
    radius_y: i32,
    label: Option<&str>,
    offset_x: i32,
    offset_y: i32,
) {
    let center = Position::new(center.x + offset_x, center.y + offset_y);
    let (cx, cy) = to_svg_coords(center);
    let rx = radius_x * CHAR_WIDTH;
    let ry = radius_y * CHAR_HEIGHT;

    writeln!(
        output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="black" stroke-width="1" fill="white"/>"#,
        cx, cy, rx, ry
    )
    .unwrap();

    // Render label if present
    if let Some(text) = label {
        writeln!(
            output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12">{}</text>"#,
            cx, cy, escape_xml(text)
        )
        .unwrap();
    }
}

/// Render freehand strokes as a path
fn render_freehand(output: &mut String, points: &[Position], offset_x: i32, offset_y: i32) {
    if points.is_empty() {
        return;
    }

    // For freehand, render circles at each point since it's character-based
    for point in points {
        let point = Position::new(point.x + offset_x, point.y + offset_y);
        let (x, y) = to_svg_coords(point);
        // Small circle at each point
        writeln!(
            output,
            r#"  <circle cx="{}" cy="{}" r="3" fill="black"/>"#,
            x, y
        )
        .unwrap();
    }
}

/// Render text
fn render_text(output: &mut String, pos: Position, content: &str, offset_x: i32, offset_y: i32) {
    let pos = Position::new(pos.x + offset_x, pos.y + offset_y);
    let (x, y) = to_svg_coords(pos);

    writeln!(
        output,
        r#"  <text x="{}" y="{}" font-family="monospace" font-size="14" dominant-baseline="middle">{}</text>"#,
        x, y, escape_xml(content)
    )
    .unwrap();
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
