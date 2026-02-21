//! SVG export functionality for irohscii
//!
//! Exports shapes to proper SVG elements with:
//! - 1 character = 10x16 SVG units (approximate monospace char aspect ratio)
//! - Arrow markers defined in <defs>
//! - Shape-specific rendering for each ShapeKind

use std::fmt::Write;
use std::path::Path;

use anyhow::Result;

use irohscii_core::{CachedShape, LineStyle, Position, ShapeKind, ShapeView};

/// Character dimensions in SVG units
const CHAR_WIDTH: i32 = 10;
const CHAR_HEIGHT: i32 = 16;

// Compile-time assertions for character dimensions
const _: () = assert!(CHAR_WIDTH > 0, "CHAR_WIDTH must be positive");
const _: () = assert!(CHAR_HEIGHT > 0, "CHAR_HEIGHT must be positive");
const _: () = assert!(CHAR_HEIGHT >= CHAR_WIDTH, "CHAR_HEIGHT should be >= CHAR_WIDTH for monospace aspect ratio");

/// Context for rendering shapes to SVG, containing common parameters
struct RenderContext<'a> {
    output: &'a mut String,
    color: &'a str,
    offset_x: i32,
    offset_y: i32,
}

impl<'a> RenderContext<'a> {
    /// Apply offset to a position and convert to SVG coordinates
    fn to_svg(&self, pos: Position) -> (i32, i32) {
        let adjusted = Position::new(pos.x + self.offset_x, pos.y + self.offset_y);
        to_svg_coords(adjusted)
    }
}

/// Convert canvas position to SVG coordinates
fn to_svg_coords(pos: Position) -> (i32, i32) {
    (pos.x * CHAR_WIDTH, pos.y * CHAR_HEIGHT)
}

/// Export shapes to SVG string
pub fn export_svg(shapes: &ShapeView) -> String {
    let mut output = String::new();

    // Calculate bounding box
    let (min_x, min_y, max_x, max_y) = calculate_bounds(shapes);
    debug_assert!(max_x >= min_x, "max_x must be >= min_x");
    debug_assert!(max_y >= min_y, "max_y must be >= min_y");
    
    let width = (max_x - min_x + 2) * CHAR_WIDTH;
    let height = (max_y - min_y + 2) * CHAR_HEIGHT;
    debug_assert!(width > 0, "SVG width must be positive");
    debug_assert!(height > 0, "SVG height must be positive");

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
    .expect("write to String is infallible");

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
    .expect("write to String is infallible");

    // Render each shape
    for shape in shapes.iter() {
        render_shape(&mut output, shape, offset_x, offset_y);
    }

    // SVG footer
    writeln!(&mut output, "</svg>").expect("write to String is infallible");

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
    debug_assert!(!shape.kind.color().to_css().is_empty(), "Shape color must be valid");
    
    let color = shape.kind.color().to_css();
    let mut ctx = RenderContext {
        output,
        color,
        offset_x,
        offset_y,
    };
    
    dispatch_shape_renderer(&mut ctx, &shape.kind);
}

/// Dispatch rendering to the appropriate shape-specific function
fn dispatch_shape_renderer(ctx: &mut RenderContext<'_>, kind: &ShapeKind) {
    match kind {
        ShapeKind::Line {
            start, end, style, ..
        } => {
            render_line(ctx, *start, *end, *style, false);
        }
        ShapeKind::Arrow {
            start, end, style, ..
        } => {
            render_line(ctx, *start, *end, *style, true);
        }
        ShapeKind::Rectangle {
            start, end, label, ..
        } => {
            render_rectangle(ctx, *start, *end, label.as_deref(), false);
        }
        ShapeKind::DoubleBox {
            start, end, label, ..
        } => {
            render_rectangle(ctx, *start, *end, label.as_deref(), true);
        }
        ShapeKind::Diamond {
            center,
            half_width,
            half_height,
            label,
            ..
        } => {
            render_diamond(ctx, *center, *half_width, *half_height, label.as_deref());
        }
        ShapeKind::Ellipse {
            center,
            radius_x,
            radius_y,
            label,
            ..
        } => {
            render_ellipse(ctx, *center, *radius_x, *radius_y, label.as_deref());
        }
        ShapeKind::Freehand { points, .. } => {
            render_freehand(ctx, points);
        }
        ShapeKind::Text { pos, content, .. } => {
            render_text(ctx, *pos, content);
        }
        ShapeKind::Triangle {
            p1, p2, p3, label, ..
        } => {
            render_triangle(ctx, *p1, *p2, *p3, label.as_deref());
        }
        ShapeKind::Parallelogram {
            start, end, label, ..
        } => {
            render_parallelogram(ctx, *start, *end, label.as_deref());
        }
        ShapeKind::Hexagon {
            center,
            radius_x,
            radius_y,
            label,
            ..
        } => {
            render_hexagon(ctx, *center, *radius_x, *radius_y, label.as_deref());
        }
        ShapeKind::Trapezoid {
            start, end, label, ..
        } => {
            render_trapezoid(ctx, *start, *end, label.as_deref());
        }
        ShapeKind::RoundedRect {
            start, end, label, ..
        } => {
            render_rounded_rect(ctx, *start, *end, label.as_deref());
        }
        ShapeKind::Cylinder {
            start, end, label, ..
        } => {
            render_cylinder(ctx, *start, *end, label.as_deref());
        }
        ShapeKind::Cloud {
            start, end, label, ..
        } => {
            render_cloud(ctx, *start, *end, label.as_deref());
        }
        ShapeKind::Star {
            center,
            outer_radius,
            inner_radius,
            label,
            ..
        } => {
            render_star(ctx, *center, *outer_radius, *inner_radius, label.as_deref());
        }
    }
}

/// Render a line or arrow
fn render_line(ctx: &mut RenderContext<'_>, start: Position, end: Position, style: LineStyle, is_arrow: bool) {
    let (x1, y1) = ctx.to_svg(start);
    let (x2, y2) = ctx.to_svg(end);
    debug_assert!(!ctx.color.is_empty(), "Color must not be empty");

    let marker = if is_arrow {
        r#" marker-end="url(#arrowhead)""#
    } else {
        ""
    };

    match style {
        LineStyle::Straight => {
            writeln!(
                ctx.output,
                r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"{}/>"#,
                x1, y1, x2, y2, ctx.color, marker
            )
            .expect("write to String is infallible");
        }
        LineStyle::OrthogonalHV => {
            let path = format!("M {} {} L {} {} L {} {}", x1, y1, x2, y1, x2, y2);
            writeln!(
                ctx.output,
                r#"  <path d="{}" stroke="{}" stroke-width="1" fill="none"{}/>"#,
                path, ctx.color, marker
            )
            .expect("write to String is infallible");
        }
        LineStyle::OrthogonalVH => {
            let path = format!("M {} {} L {} {} L {} {}", x1, y1, x1, y2, x2, y2);
            writeln!(
                ctx.output,
                r#"  <path d="{}" stroke="{}" stroke-width="1" fill="none"{}/>"#,
                path, ctx.color, marker
            )
            .expect("write to String is infallible");
        }
        LineStyle::OrthogonalAuto => {
            let path = format!("M {} {} L {} {} L {} {}", x1, y1, x2, y1, x2, y2);
            writeln!(
                ctx.output,
                r#"  <path d="{}" stroke="{}" stroke-width="1" fill="none"{}/>"#,
                path, ctx.color, marker
            )
            .expect("write to String is infallible");
        }
    }
}

/// Render a rectangle or double box
fn render_rectangle(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>, is_double: bool) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);

    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_x = start.x.max(end.x);
    let max_y = start.y.max(end.y);
    debug_assert!(max_x >= min_x, "Rectangle: max_x must be >= min_x");
    debug_assert!(max_y >= min_y, "Rectangle: max_y must be >= min_y");

    let (x, y) = to_svg_coords(Position::new(min_x, min_y));
    let (x2, y2) = to_svg_coords(Position::new(max_x, max_y));
    let width = x2 - x;
    let height = y2 - y;
    debug_assert!(width >= 0, "Rectangle width must be non-negative");
    debug_assert!(height >= 0, "Rectangle height must be non-negative");

    let stroke_width = if is_double { 3 } else { 1 };

    writeln!(
        ctx.output,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" stroke="{}" stroke-width="{}" fill="white"/>"#,
        x, y, width, height, ctx.color, stroke_width
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        let center_x = x + width / 2;
        let center_y = y + height / 2;
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            center_x, center_y, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a diamond shape
fn render_diamond(ctx: &mut RenderContext<'_>, center: Position, half_width: i32, half_height: i32, label: Option<&str>) {
    debug_assert!(half_width >= 0, "Diamond half_width must be non-negative");
    debug_assert!(half_height >= 0, "Diamond half_height must be non-negative");
    
    let (cx, cy) = ctx.to_svg(center);
    let hw = half_width * CHAR_WIDTH;
    let hh = half_height * CHAR_HEIGHT;

    let points = format!(
        "{},{} {},{} {},{} {},{}",
        cx, cy - hh, cx + hw, cy, cx, cy + hh, cx - hw, cy
    );

    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render an ellipse
fn render_ellipse(ctx: &mut RenderContext<'_>, center: Position, radius_x: i32, radius_y: i32, label: Option<&str>) {
    debug_assert!(radius_x > 0, "Ellipse radius_x must be positive");
    debug_assert!(radius_y > 0, "Ellipse radius_y must be positive");
    
    let (cx, cy) = ctx.to_svg(center);
    let rx = radius_x * CHAR_WIDTH;
    let ry = radius_y * CHAR_HEIGHT;

    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        cx, cy, rx, ry, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render freehand strokes as a path
fn render_freehand(ctx: &mut RenderContext<'_>, points: &[Position]) {
    debug_assert!(points.len() <= 100000, "Freehand should have reasonable number of points");
    
    if points.is_empty() {
        return;
    }

    for point in points {
        let (x, y) = ctx.to_svg(*point);
        writeln!(
            ctx.output,
            r#"  <circle cx="{}" cy="{}" r="3" fill="{}"/>"#,
            x, y, ctx.color
        )
        .expect("write to String is infallible");
    }
}

/// Render text
fn render_text(ctx: &mut RenderContext<'_>, pos: Position, content: &str) {
    debug_assert!(!content.is_empty(), "Text content should not be empty");
    debug_assert!(content.len() <= 10000, "Text should have reasonable length");
    
    let (x, y) = ctx.to_svg(pos);

    writeln!(
        ctx.output,
        r#"  <text x="{}" y="{}" font-family="monospace" font-size="14" dominant-baseline="middle" fill="{}">{}</text>"#,
        x, y, ctx.color, escape_xml(content)
    )
    .expect("write to String is infallible");
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Render a triangle
fn render_triangle(ctx: &mut RenderContext<'_>, p1: Position, p2: Position, p3: Position, label: Option<&str>) {
    debug_assert!(p1 != p2 && p2 != p3 && p1 != p3, "Triangle points should be distinct");
    
    let (x1, y1) = ctx.to_svg(p1);
    let (x2, y2) = ctx.to_svg(p2);
    let (x3, y3) = ctx.to_svg(p3);

    let points = format!("{},{} {},{} {},{}", x1, y1, x2, y2, x3, y3);
    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        let cx = (x1 + x2 + x3) / 3;
        let cy = (y1 + y2 + y3) / 3;
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a parallelogram
fn render_parallelogram(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);
    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    debug_assert!(max_x > min_x, "Parallelogram must have positive width");
    debug_assert!(max_y > min_y, "Parallelogram must have positive height");
    let slant = (max_x - min_x) / 4;

    let (x1, y1) = to_svg_coords(Position::new(min_x + slant, min_y));
    let (x2, y2) = to_svg_coords(Position::new(max_x + slant, min_y));
    let (x3, y3) = to_svg_coords(Position::new(max_x, max_y));
    let (x4, y4) = to_svg_coords(Position::new(min_x, max_y));

    let points = format!("{},{} {},{} {},{} {},{}", x1, y1, x2, y2, x3, y3, x4, y4);
    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        let cx = (x1 + x2 + x3 + x4) / 4;
        let cy = (y1 + y2 + y3 + y4) / 4;
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a hexagon
fn render_hexagon(ctx: &mut RenderContext<'_>, center: Position, radius_x: i32, radius_y: i32, label: Option<&str>) {
    debug_assert!(radius_x > 0, "Hexagon radius_x must be positive");
    debug_assert!(radius_y > 0, "Hexagon radius_y must be positive");
    
    let (cx, cy) = ctx.to_svg(center);
    let rx = radius_x * CHAR_WIDTH;
    let ry = radius_y * CHAR_HEIGHT;
    let edge_width = rx * 2 / 3;

    let points = format!(
        "{},{} {},{} {},{} {},{} {},{} {},{}",
        cx - edge_width, cy - ry,
        cx + edge_width, cy - ry,
        cx + rx, cy,
        cx + edge_width, cy + ry,
        cx - edge_width, cy + ry,
        cx - rx, cy
    );

    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a trapezoid
fn render_trapezoid(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);
    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    debug_assert!(max_x > min_x, "Trapezoid must have positive width");
    debug_assert!(max_y > min_y, "Trapezoid must have positive height");
    let inset = (max_x - min_x) / 4;

    let (x1, y1) = to_svg_coords(Position::new(min_x + inset, min_y));
    let (x2, y2) = to_svg_coords(Position::new(max_x - inset, min_y));
    let (x3, y3) = to_svg_coords(Position::new(max_x, max_y));
    let (x4, y4) = to_svg_coords(Position::new(min_x, max_y));

    let points = format!("{},{} {},{} {},{} {},{}", x1, y1, x2, y2, x3, y3, x4, y4);
    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        let cx = (x1 + x2 + x3 + x4) / 4;
        let cy = (y1 + y2 + y3 + y4) / 4;
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a rounded rectangle
fn render_rounded_rect(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);
    let (x1, y1) = to_svg_coords(start);
    let (x2, y2) = to_svg_coords(end);
    let x = x1.min(x2);
    let y = y1.min(y2);
    let width = (x2 - x1).abs();
    let height = (y2 - y1).abs();
    debug_assert!(width > 0, "RoundedRect width must be positive");
    debug_assert!(height > 0, "RoundedRect height must be positive");
    let radius = CHAR_WIDTH.min(CHAR_HEIGHT) / 2;

    writeln!(
        ctx.output,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x, y, width, height, radius, radius, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        let cx = x + width / 2;
        let cy = y + height / 2;
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a cylinder (database symbol)
fn render_cylinder(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);
    let (x1, y1) = to_svg_coords(start);
    let (x2, y2) = to_svg_coords(end);
    let x = x1.min(x2);
    let y = y1.min(y2);
    let width = (x2 - x1).abs();
    let height = (y2 - y1).abs();
    debug_assert!(width > 0, "Cylinder width must be positive");
    debug_assert!(height > CHAR_HEIGHT, "Cylinder height must be sufficient for ellipse");
    let ellipse_height = CHAR_HEIGHT;

    // Top ellipse
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + width / 2, y + ellipse_height / 2, width / 2, ellipse_height / 2, ctx.color
    )
    .expect("write to String is infallible");

    // Body
    writeln!(
        ctx.output,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" stroke="none" fill="white"/>"#,
        x, y + ellipse_height / 2, width, height - ellipse_height
    )
    .expect("write to String is infallible");

    // Left side
    writeln!(
        ctx.output,
        r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
        x, y + ellipse_height / 2, x, y + height - ellipse_height / 2, ctx.color
    )
    .expect("write to String is infallible");

    // Right side
    writeln!(
        ctx.output,
        r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
        x + width, y + ellipse_height / 2, x + width, y + height - ellipse_height / 2, ctx.color
    )
    .expect("write to String is infallible");

    // Bottom ellipse
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + width / 2, y + height - ellipse_height / 2, width / 2, ellipse_height / 2, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            x + width / 2, y + height / 2, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a cloud shape
fn render_cloud(ctx: &mut RenderContext<'_>, start: Position, end: Position, label: Option<&str>) {
    let start = Position::new(start.x + ctx.offset_x, start.y + ctx.offset_y);
    let end = Position::new(end.x + ctx.offset_x, end.y + ctx.offset_y);
    let (x1, y1) = to_svg_coords(start);
    let (x2, y2) = to_svg_coords(end);
    let x = x1.min(x2);
    let y = y1.min(y2);
    let width = (x2 - x1).abs();
    let height = (y2 - y1).abs();
    debug_assert!(width > 0, "Cloud width must be positive");
    debug_assert!(height > 0, "Cloud height must be positive");

    // Approximate cloud with overlapping ellipses
    let r = height / 3;
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + r, y + height / 2, r, r, ctx.color
    )
    .expect("write to String is infallible");
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + width / 2, y + r, r * 3 / 2, r, ctx.color
    )
    .expect("write to String is infallible");
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + width - r, y + height / 2, r, r, ctx.color
    )
    .expect("write to String is infallible");
    writeln!(
        ctx.output,
        r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        x + width / 2, y + height - r, r * 3 / 2, r, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            x + width / 2, y + height / 2, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

/// Render a star shape
fn render_star(ctx: &mut RenderContext<'_>, center: Position, outer_radius: i32, _inner_radius: i32, label: Option<&str>) {
    debug_assert!(outer_radius > 0, "Star outer_radius must be positive");
    
    let (cx, cy) = ctx.to_svg(center);
    let r_out = outer_radius * CHAR_WIDTH;
    let r_in = r_out / 2;
    debug_assert!(r_out > r_in, "Star outer radius must be greater than inner radius");

    // 5-pointed star
    let mut points = String::new();
    for i in 0..10 {
        let angle = std::f64::consts::PI / 2.0 + (i as f64) * std::f64::consts::PI / 5.0;
        let r = if i % 2 == 0 { r_out } else { r_in };
        let x = cx + (angle.cos() * r as f64) as i32;
        let y = cy - (angle.sin() * r as f64) as i32;
        if !points.is_empty() {
            points.push(' ');
        }
        points.push_str(&format!("{},{}", x, y));
    }

    writeln!(
        ctx.output,
        r#"  <polygon points="{}" stroke="{}" stroke-width="1" fill="white"/>"#,
        points, ctx.color
    )
    .expect("write to String is infallible");

    if let Some(text) = label {
        writeln!(
            ctx.output,
            r#"  <text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="monospace" font-size="12" fill="{}">{}</text>"#,
            cx, cy, ctx.color, escape_xml(text)
        )
        .expect("write to String is infallible");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use irohscii_core::ShapeColor;

    fn make_rect(x: i32, y: i32, w: i32, h: i32) -> ShapeKind {
        ShapeKind::Rectangle {
            start: Position::new(x, y),
            end: Position::new(x + w, y + h),
            color: ShapeColor::default(),
            label: None,
        }
    }

    fn make_labeled_rect(x: i32, y: i32, w: i32, h: i32, label: &str) -> ShapeKind {
        ShapeKind::Rectangle {
            start: Position::new(x, y),
            end: Position::new(x + w, y + h),
            color: ShapeColor::default(),
            label: Some(label.to_string()),
        }
    }

    fn make_line(x1: i32, y1: i32, x2: i32, y2: i32) -> ShapeKind {
        ShapeKind::Line {
            start: Position::new(x1, y1),
            end: Position::new(x2, y2),
            color: ShapeColor::default(),
            style: LineStyle::Straight,
            start_connection: None,
            end_connection: None,
            label: None,
        }
    }

    fn make_arrow(x1: i32, y1: i32, x2: i32, y2: i32) -> ShapeKind {
        ShapeKind::Arrow {
            start: Position::new(x1, y1),
            end: Position::new(x2, y2),
            color: ShapeColor::default(),
            style: LineStyle::Straight,
            start_connection: None,
            end_connection: None,
            label: None,
        }
    }

    fn make_ellipse(cx: i32, cy: i32, rx: i32, ry: i32) -> ShapeKind {
        ShapeKind::Ellipse {
            center: Position::new(cx, cy),
            radius_x: rx,
            radius_y: ry,
            color: ShapeColor::default(),
            label: None,
        }
    }

    fn make_text(x: i32, y: i32, content: &str) -> ShapeKind {
        ShapeKind::Text {
            pos: Position::new(x, y),
            content: content.to_string(),
            color: ShapeColor::default(),
        }
    }

    fn build_shape_view(shapes: Vec<ShapeKind>) -> ShapeView {
        use irohscii_core::Document;

        let mut doc = Document::new();
        for kind in shapes {
            doc.add_shape(kind).expect("write to String is infallible");
        }
        let mut view = ShapeView::default();
        view.rebuild(&doc).expect("write to String is infallible");
        view
    }

    // --- Coordinate conversion tests ---

    #[test]
    fn to_svg_coords_origin() {
        let (x, y) = to_svg_coords(Position::new(0, 0));
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn to_svg_coords_positive() {
        let (x, y) = to_svg_coords(Position::new(10, 5));
        assert_eq!(x, 10 * CHAR_WIDTH);
        assert_eq!(y, 5 * CHAR_HEIGHT);
    }

    #[test]
    fn to_svg_coords_negative() {
        let (x, y) = to_svg_coords(Position::new(-5, -3));
        assert_eq!(x, -5 * CHAR_WIDTH);
        assert_eq!(y, -3 * CHAR_HEIGHT);
    }

    // --- Bounds calculation tests ---

    #[test]
    fn calculate_bounds_empty() {
        let view = build_shape_view(vec![]);
        let bounds = calculate_bounds(&view);
        assert_eq!(bounds, (0, 0, 10, 10)); // Default for empty
    }

    #[test]
    fn calculate_bounds_single_rect() {
        let view = build_shape_view(vec![make_rect(5, 5, 10, 8)]);
        let (min_x, min_y, max_x, max_y) = calculate_bounds(&view);
        assert!(min_x <= 5);
        assert!(min_y <= 5);
        assert!(max_x >= 15);
        assert!(max_y >= 13);
    }

    #[test]
    fn calculate_bounds_multiple_shapes() {
        let view = build_shape_view(vec![make_rect(0, 0, 5, 5), make_rect(20, 20, 5, 5)]);
        let (min_x, min_y, max_x, max_y) = calculate_bounds(&view);
        assert!(min_x <= 0);
        assert!(min_y <= 0);
        assert!(max_x >= 25);
        assert!(max_y >= 25);
    }

    // --- SVG export tests ---

    #[test]
    fn export_svg_has_header() {
        let view = build_shape_view(vec![make_rect(0, 0, 5, 5)]);
        let svg = export_svg(&view);

        assert!(svg.starts_with("<?xml version=\"1.0\""));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("xmlns="));
    }

    #[test]
    fn export_svg_has_footer() {
        let view = build_shape_view(vec![make_rect(0, 0, 5, 5)]);
        let svg = export_svg(&view);

        assert!(svg.trim().ends_with("</svg>"));
    }

    #[test]
    fn export_svg_has_arrow_marker() {
        let view = build_shape_view(vec![make_arrow(0, 0, 10, 10)]);
        let svg = export_svg(&view);

        assert!(svg.contains("<defs>"));
        assert!(svg.contains("id=\"arrowhead\""));
        assert!(svg.contains("</defs>"));
    }

    #[test]
    fn export_svg_rectangle() {
        let view = build_shape_view(vec![make_rect(0, 0, 10, 5)]);
        let svg = export_svg(&view);

        assert!(svg.contains("<rect"));
        assert!(svg.contains("stroke="));
        assert!(svg.contains("fill="));
    }

    #[test]
    fn export_svg_line() {
        let view = build_shape_view(vec![make_line(0, 0, 10, 10)]);
        let svg = export_svg(&view);

        assert!(svg.contains("<line"));
        assert!(svg.contains("x1="));
        assert!(svg.contains("y1="));
        assert!(svg.contains("x2="));
        assert!(svg.contains("y2="));
    }

    #[test]
    fn export_svg_arrow() {
        let view = build_shape_view(vec![make_arrow(0, 0, 10, 10)]);
        let svg = export_svg(&view);

        // Arrow should reference the marker
        assert!(svg.contains("marker-end=\"url(#arrowhead)\""));
    }

    #[test]
    fn export_svg_ellipse() {
        let view = build_shape_view(vec![make_ellipse(10, 10, 5, 3)]);
        let svg = export_svg(&view);

        assert!(svg.contains("<ellipse"));
        assert!(svg.contains("cx="));
        assert!(svg.contains("cy="));
        assert!(svg.contains("rx="));
        assert!(svg.contains("ry="));
    }

    #[test]
    fn export_svg_text() {
        let view = build_shape_view(vec![make_text(0, 0, "Hello World")]);
        let svg = export_svg(&view);

        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello World"));
    }

    #[test]
    fn export_svg_with_label() {
        let view = build_shape_view(vec![make_labeled_rect(0, 0, 10, 5, "Label")]);
        let svg = export_svg(&view);

        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("Label"));
    }

    // --- XML escaping tests ---

    #[test]
    fn escape_xml_ampersand() {
        assert_eq!(escape_xml("A & B"), "A &amp; B");
    }

    #[test]
    fn escape_xml_less_than() {
        assert_eq!(escape_xml("A < B"), "A &lt; B");
    }

    #[test]
    fn escape_xml_greater_than() {
        assert_eq!(escape_xml("A > B"), "A &gt; B");
    }

    #[test]
    fn escape_xml_quote() {
        assert_eq!(escape_xml("A \"B\" C"), "A &quot;B&quot; C");
    }

    #[test]
    fn escape_xml_apostrophe() {
        assert_eq!(escape_xml("A 'B' C"), "A &apos;B&apos; C");
    }

    #[test]
    fn escape_xml_multiple() {
        assert_eq!(escape_xml("<>&\"'"), "&lt;&gt;&amp;&quot;&apos;");
    }

    #[test]
    fn escape_xml_none_needed() {
        assert_eq!(escape_xml("Hello World"), "Hello World");
    }

    // --- Save SVG tests ---

    #[test]
    fn save_svg_creates_file() {
        let temp_dir = tempfile::tempdir().expect("write to String is infallible");
        let file_path = temp_dir.path().join("test.svg");

        let view = build_shape_view(vec![make_rect(0, 0, 10, 5)]);
        save_svg(&view, &file_path).expect("write to String is infallible");

        assert!(file_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&file_path).expect("write to String is infallible");
        assert!(content.contains("<svg"));
        assert!(content.contains("</svg>"));
    }

    // --- Color export tests ---

    #[test]
    fn export_svg_with_color() {
        let rect = ShapeKind::Rectangle {
            start: Position::new(0, 0),
            end: Position::new(10, 5),
            color: ShapeColor::Red,
            label: None,
        };
        let view = build_shape_view(vec![rect]);
        let svg = export_svg(&view);

        // Should contain the red color
        assert!(svg.contains("red") || svg.contains("Red") || svg.contains("#"));
    }
}
