use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::canvas::{
    arrow_points_styled, cloud_points, cylinder_points, diamond_points, double_rect_points,
    ellipse_points, hexagon_points, line_points_styled, parallelogram_points, rect_points,
    rounded_rect_points, star_points, trapezoid_points, triangle_points, Position,
};
use crate::shapes::{ShapeColor, ShapeKind, ShapeView};

/// Render a label centered inside a shape's bounds
fn render_label_to_grid(grid: &mut HashMap<Position, char>, bounds: (i32, i32, i32, i32), text: &str) {
    let (min_x, min_y, max_x, max_y) = bounds;
    let center_y = (min_y + max_y) / 2;
    let shape_width = (max_x - min_x + 1) as usize;
    let text_len = text.chars().count();

    let inner_width = shape_width.saturating_sub(2);
    let start_offset = if text_len < inner_width {
        ((inner_width - text_len) / 2) as i32 + 1
    } else {
        1
    };
    let start_x = min_x + start_offset;

    for (i, ch) in text.chars().enumerate() {
        let x = start_x + i as i32;
        if x >= max_x {
            break;
        }
        grid.insert(Position::new(x, center_y), ch);
    }
}

/// Render shapes to a text string
fn render_shapes_to_text(shapes: &ShapeView) -> String {
    // Build a sparse grid
    let mut grid: HashMap<Position, char> = HashMap::new();

    for shape in shapes.iter() {
        match &shape.kind {
            ShapeKind::Line { start, end, style, .. } => {
                for (pos, ch) in line_points_styled(*start, *end, *style) {
                    grid.insert(pos, ch);
                }
            }
            ShapeKind::Arrow { start, end, style, .. } => {
                for (pos, ch) in arrow_points_styled(*start, *end, *style) {
                    grid.insert(pos, ch);
                }
            }
            ShapeKind::Rectangle { start, end, label, .. } => {
                for (pos, ch) in rect_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::DoubleBox { start, end, label, .. } => {
                for (pos, ch) in double_rect_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Diamond { center, half_width, half_height, label, .. } => {
                for (pos, ch) in diamond_points(*center, *half_width, *half_height) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Ellipse { center, radius_x, radius_y, label, .. } => {
                for (pos, ch) in ellipse_points(*center, *radius_x, *radius_y) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Freehand { points, char, .. } => {
                for &pos in points {
                    grid.insert(pos, *char);
                }
            }
            ShapeKind::Text { pos, content, .. } => {
                for (i, ch) in content.chars().enumerate() {
                    grid.insert(Position::new(pos.x + i as i32, pos.y), ch);
                }
            }
            ShapeKind::Triangle { p1, p2, p3, label, .. } => {
                for (pos, ch) in triangle_points(*p1, *p2, *p3) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Parallelogram { start, end, label, .. } => {
                for (pos, ch) in parallelogram_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Hexagon { center, radius_x, radius_y, label, .. } => {
                for (pos, ch) in hexagon_points(*center, *radius_x, *radius_y) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Trapezoid { start, end, label, .. } => {
                for (pos, ch) in trapezoid_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::RoundedRect { start, end, label, .. } => {
                for (pos, ch) in rounded_rect_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Cylinder { start, end, label, .. } => {
                for (pos, ch) in cylinder_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Cloud { start, end, label, .. } => {
                for (pos, ch) in cloud_points(*start, *end) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
            ShapeKind::Star { center, outer_radius, inner_radius, label, .. } => {
                for (pos, ch) in star_points(*center, *outer_radius, *inner_radius) {
                    grid.insert(pos, ch);
                }
                if let Some(text) = label {
                    render_label_to_grid(&mut grid, shape.bounds(), text);
                }
            }
        }
    }

    if grid.is_empty() {
        return String::new();
    }

    // Find bounds
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for pos in grid.keys() {
        min_x = min_x.min(pos.x);
        min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x);
        max_y = max_y.max(pos.y);
    }

    // Render to string
    let mut lines = Vec::new();
    for y in min_y..=max_y {
        let mut line = String::new();
        for x in min_x..=max_x {
            let ch = grid.get(&Position::new(x, y)).copied().unwrap_or(' ');
            line.push(ch);
        }
        lines.push(line.trim_end().to_string());
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Save shapes to a file (renders as ASCII art)
pub fn save_ascii(shapes: &ShapeView, path: &Path) -> Result<()> {
    let content = render_shapes_to_text(shapes);
    fs::write(path, content).with_context(|| format!("Failed to save to {:?}", path))?;
    Ok(())
}

/// Load shapes from a file (imports as text lines - shapes become Text entries)
pub fn load_ascii(path: &Path) -> Result<Vec<ShapeKind>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read from {:?}", path))?;

    let mut shapes = Vec::new();

    // Import each line as a separate text shape
    for (y, line) in content.lines().enumerate() {
        if !line.is_empty() {
            shapes.push(ShapeKind::Text {
                pos: Position::new(0, y as i32),
                content: line.to_string(),
                color: ShapeColor::default(),
            });
        }
    }

    Ok(shapes)
}
