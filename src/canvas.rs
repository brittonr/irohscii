use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A position on the canvas (can be negative for infinite canvas feel)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// The drawing canvas - sparse representation for efficiency
#[derive(Debug, Clone)]
pub struct Canvas {
    cells: HashMap<Position, char>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    /// Get the character at a position, returns space if empty
    pub fn get(&self, pos: Position) -> char {
        *self.cells.get(&pos).unwrap_or(&' ')
    }

    /// Set a character at a position
    pub fn set(&mut self, pos: Position, ch: char) {
        if ch == ' ' {
            self.cells.remove(&pos);
        } else {
            self.cells.insert(pos, ch);
        }
    }

    /// Get the bounding box of all content (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        if self.cells.is_empty() {
            return None;
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for pos in self.cells.keys() {
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x);
            max_y = max_y.max(pos.y);
        }

        Some((min_x, min_y, max_x, max_y))
    }

    /// Clear the entire canvas
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Export canvas to string (for saving)
    pub fn to_string_content(&self) -> String {
        let Some((min_x, min_y, max_x, max_y)) = self.bounds() else {
            return String::new();
        };

        let mut lines = Vec::new();
        for y in min_y..=max_y {
            let mut line = String::new();
            for x in min_x..=max_x {
                line.push(self.get(Position::new(x, y)));
            }
            // Trim trailing spaces from each line
            let trimmed = line.trim_end();
            lines.push(trimmed.to_string());
        }

        // Remove trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }

        lines.join("\n")
    }

    /// Load canvas from string
    pub fn from_string(content: &str) -> Self {
        let mut canvas = Self::new();
        for (y, line) in content.lines().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if ch != ' ' {
                    canvas.set(Position::new(x as i32, y as i32), ch);
                }
            }
        }
        canvas
    }

    /// Draw a line between two points using Bresenham's algorithm
    pub fn draw_line(&mut self, from: Position, to: Position, ch: char) {
        for pos in line_points(from, to) {
            self.set(pos, ch);
        }
    }

    /// Draw a rectangle (box) with Unicode box-drawing characters
    pub fn draw_rect(&mut self, from: Position, to: Position) {
        let min_x = from.x.min(to.x);
        let max_x = from.x.max(to.x);
        let min_y = from.y.min(to.y);
        let max_y = from.y.max(to.y);

        // Corners
        self.set(Position::new(min_x, min_y), '┌');
        self.set(Position::new(max_x, min_y), '┐');
        self.set(Position::new(min_x, max_y), '└');
        self.set(Position::new(max_x, max_y), '┘');

        // Horizontal lines
        for x in (min_x + 1)..max_x {
            self.set(Position::new(x, min_y), '─');
            self.set(Position::new(x, max_y), '─');
        }

        // Vertical lines
        for y in (min_y + 1)..max_y {
            self.set(Position::new(min_x, y), '│');
            self.set(Position::new(max_x, y), '│');
        }
    }
}

/// Line drawing style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineStyle {
    /// Straight line with appropriate characters for direction
    #[default]
    Straight,
    /// Orthogonal: horizontal first, then vertical
    OrthogonalHV,
    /// Orthogonal: vertical first, then horizontal
    OrthogonalVH,
}

impl LineStyle {
    /// Cycle to the next line style
    pub fn next(self) -> Self {
        match self {
            LineStyle::Straight => LineStyle::OrthogonalHV,
            LineStyle::OrthogonalHV => LineStyle::OrthogonalVH,
            LineStyle::OrthogonalVH => LineStyle::Straight,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LineStyle::Straight => "Straight",
            LineStyle::OrthogonalHV => "Ortho H→V",
            LineStyle::OrthogonalVH => "Ortho V→H",
        }
    }
}

/// Generate all points on a line using Bresenham's algorithm
pub fn line_points(from: Position, to: Position) -> Vec<Position> {
    let mut points = Vec::new();

    let dx = (to.x - from.x).abs();
    let dy = -(to.y - from.y).abs();
    let sx = if from.x < to.x { 1 } else { -1 };
    let sy = if from.y < to.y { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = from.x;
    let mut y = from.y;

    loop {
        points.push(Position::new(x, y));

        if x == to.x && y == to.y {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x == to.x {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == to.y {
                break;
            }
            err += dx;
            y += sy;
        }
    }

    points
}

/// Generate line points with appropriate characters based on style and direction
pub fn line_points_styled(from: Position, to: Position, style: LineStyle) -> Vec<(Position, char)> {
    match style {
        LineStyle::Straight => straight_line_points(from, to),
        LineStyle::OrthogonalHV => orthogonal_line_points(from, to, true),
        LineStyle::OrthogonalVH => orthogonal_line_points(from, to, false),
    }
}

/// Generate a straight line with smart character selection
fn straight_line_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;

    // Single point
    if dx == 0 && dy == 0 {
        return vec![(from, '○')];
    }

    // Pure horizontal
    if dy == 0 {
        let (start_x, end_x) = if dx > 0 { (from.x, to.x) } else { (to.x, from.x) };
        return (start_x..=end_x)
            .map(|x| (Position::new(x, from.y), '─'))
            .collect();
    }

    // Pure vertical
    if dx == 0 {
        let (start_y, end_y) = if dy > 0 { (from.y, to.y) } else { (to.y, from.y) };
        return (start_y..=end_y)
            .map(|y| (Position::new(from.x, y), '│'))
            .collect();
    }

    // Diagonal - determine predominant direction
    let abs_dx = dx.abs();
    let abs_dy = dy.abs();

    // Choose character based on diagonal direction
    let diag_char = if (dx > 0) == (dy > 0) {
        '\\' // going down-right or up-left
    } else {
        '/' // going down-left or up-right
    };

    // For mostly horizontal diagonals, use ─ with occasional diagonal
    // For mostly vertical diagonals, use │ with occasional diagonal
    // For 45° diagonals, use the diagonal character throughout

    let points = line_points(from, to);
    let mut result = Vec::with_capacity(points.len());

    for i in 0..points.len() {
        let ch = if i == 0 || i == points.len() - 1 {
            // Endpoints
            if abs_dx > abs_dy * 2 {
                '─'
            } else if abs_dy > abs_dx * 2 {
                '│'
            } else {
                diag_char
            }
        } else {
            // Middle points - check local direction
            let prev = points[i - 1];
            let next = points[i + 1];

            let local_dx = (next.x - prev.x).abs();
            let local_dy = (next.y - prev.y).abs();

            if local_dy == 0 {
                '─'
            } else if local_dx == 0 {
                '│'
            } else {
                diag_char
            }
        };
        result.push((points[i], ch));
    }

    result
}

/// Generate an orthogonal line (L-shaped) with corner
fn orthogonal_line_points(from: Position, to: Position, horizontal_first: bool) -> Vec<(Position, char)> {
    let mut result = Vec::new();

    // Single point
    if from == to {
        return vec![(from, '○')];
    }

    let dx = to.x - from.x;
    let dy = to.y - from.y;

    // Pure horizontal or vertical - no corner needed
    if dx == 0 || dy == 0 {
        return straight_line_points(from, to);
    }

    // Calculate corner position
    let corner = if horizontal_first {
        Position::new(to.x, from.y)
    } else {
        Position::new(from.x, to.y)
    };

    // Determine corner character
    let corner_char = match (dx > 0, dy > 0, horizontal_first) {
        // Horizontal first (corner at to.x, from.y)
        (true, true, true) => '┐',   // right then down
        (true, false, true) => '┘',  // right then up
        (false, true, true) => '┌',  // left then down
        (false, false, true) => '└', // left then up
        // Vertical first (corner at from.x, to.y)
        (true, true, false) => '└',  // down then right
        (true, false, false) => '┌', // up then right
        (false, true, false) => '┘', // down then left
        (false, false, false) => '┐', // up then left
    };

    if horizontal_first {
        // Horizontal segment (excluding corner)
        let (start_x, end_x) = if dx > 0 {
            (from.x, corner.x - 1)
        } else {
            (corner.x + 1, from.x)
        };
        for x in start_x.min(end_x)..=start_x.max(end_x) {
            result.push((Position::new(x, from.y), '─'));
        }

        // Corner
        result.push((corner, corner_char));

        // Vertical segment (excluding corner)
        let (start_y, end_y) = if dy > 0 {
            (corner.y + 1, to.y)
        } else {
            (to.y, corner.y - 1)
        };
        for y in start_y.min(end_y)..=start_y.max(end_y) {
            result.push((Position::new(to.x, y), '│'));
        }
    } else {
        // Vertical segment (excluding corner)
        let (start_y, end_y) = if dy > 0 {
            (from.y, corner.y - 1)
        } else {
            (corner.y + 1, from.y)
        };
        for y in start_y.min(end_y)..=start_y.max(end_y) {
            result.push((Position::new(from.x, y), '│'));
        }

        // Corner
        result.push((corner, corner_char));

        // Horizontal segment (excluding corner)
        let (start_x, end_x) = if dx > 0 {
            (corner.x + 1, to.x)
        } else {
            (to.x, corner.x - 1)
        };
        for x in start_x.min(end_x)..=start_x.max(end_x) {
            result.push((Position::new(x, to.y), '─'));
        }
    }

    result
}

/// Generate arrow points (line with arrowhead at end)
pub fn arrow_points_styled(from: Position, to: Position, style: LineStyle) -> Vec<(Position, char)> {
    let mut points = line_points_styled(from, to, style);

    // Replace the last character with an arrowhead
    if let Some((_, ch)) = points.last_mut() {
        let dx = to.x - from.x;
        let dy = to.y - from.y;

        *ch = if dx.abs() > dy.abs() {
            // Predominantly horizontal
            if dx > 0 { '→' } else { '←' }
        } else if dy.abs() > dx.abs() {
            // Predominantly vertical
            if dy > 0 { '↓' } else { '↑' }
        } else {
            // Diagonal - pick based on direction
            match (dx > 0, dy > 0) {
                (true, true) => '↘',
                (true, false) => '↗',
                (false, true) => '↙',
                (false, false) => '↖',
            }
        };
    }

    points
}

/// Generate double-line rectangle outline points
pub fn double_rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    // Single point
    if min_x == max_x && min_y == max_y {
        points.push((Position::new(min_x, min_y), '╬'));
        return points;
    }

    // Horizontal line only
    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '═'));
        }
        return points;
    }

    // Vertical line only
    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '║'));
        }
        return points;
    }

    // Full rectangle with double lines
    // Corners
    points.push((Position::new(min_x, min_y), '╔'));
    points.push((Position::new(max_x, min_y), '╗'));
    points.push((Position::new(min_x, max_y), '╚'));
    points.push((Position::new(max_x, max_y), '╝'));

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '═'));
        points.push((Position::new(x, max_y), '═'));
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '║'));
        points.push((Position::new(max_x, y), '║'));
    }

    points
}

/// Generate diamond outline points
pub fn diamond_points(center: Position, half_width: i32, half_height: i32) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let hw = half_width.abs().max(1);
    let hh = half_height.abs().max(1);

    // Single point
    if hw == 0 && hh == 0 {
        points.push((center, '◇'));
        return points;
    }

    // Very small diamond
    if hw == 1 && hh == 1 {
        points.push((Position::new(center.x, center.y - 1), '^'));
        points.push((Position::new(center.x - 1, center.y), '<'));
        points.push((Position::new(center.x + 1, center.y), '>'));
        points.push((Position::new(center.x, center.y + 1), 'v'));
        return points;
    }

    // Top point
    points.push((Position::new(center.x, center.y - hh), '^'));
    // Bottom point
    points.push((Position::new(center.x, center.y + hh), 'v'));
    // Left point
    points.push((Position::new(center.x - hw, center.y), '<'));
    // Right point
    points.push((Position::new(center.x + hw, center.y), '>'));

    // Draw the four edges using interpolation
    // Top-right edge
    for i in 1..hh {
        let x = center.x + (hw * i) / hh;
        let y = center.y - hh + i;
        points.push((Position::new(x, y), '\\'));
    }
    // Top-left edge
    for i in 1..hh {
        let x = center.x - (hw * i) / hh;
        let y = center.y - hh + i;
        points.push((Position::new(x, y), '/'));
    }
    // Bottom-right edge
    for i in 1..hh {
        let x = center.x + (hw * i) / hh;
        let y = center.y + hh - i;
        points.push((Position::new(x, y), '/'));
    }
    // Bottom-left edge
    for i in 1..hh {
        let x = center.x - (hw * i) / hh;
        let y = center.y + hh - i;
        points.push((Position::new(x, y), '\\'));
    }

    points
}

/// Generate ellipse outline points
pub fn ellipse_points(center: Position, radius_x: i32, radius_y: i32) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let rx = radius_x.abs().max(1);
    let ry = radius_y.abs().max(1);

    // Very small ellipse
    if rx <= 1 && ry <= 1 {
        points.push((Position::new(center.x, center.y - 1), '─'));
        points.push((Position::new(center.x - 1, center.y), '('));
        points.push((Position::new(center.x + 1, center.y), ')'));
        points.push((Position::new(center.x, center.y + 1), '─'));
        return points;
    }

    // Draw ellipse using midpoint algorithm approximation
    // Top and bottom curves
    points.push((Position::new(center.x - rx, center.y), '('));
    points.push((Position::new(center.x + rx, center.y), ')'));

    // Top horizontal edge
    for x in (center.x - rx + 1)..(center.x + rx) {
        points.push((Position::new(x, center.y - ry), '─'));
    }
    // Bottom horizontal edge
    for x in (center.x - rx + 1)..(center.x + rx) {
        points.push((Position::new(x, center.y + ry), '─'));
    }

    // Corners
    points.push((Position::new(center.x - rx, center.y - ry), '╭'));
    points.push((Position::new(center.x + rx, center.y - ry), '╮'));
    points.push((Position::new(center.x - rx, center.y + ry), '╰'));
    points.push((Position::new(center.x + rx, center.y + ry), '╯'));

    // Left and right vertical edges (if tall enough)
    for y in (center.y - ry + 1)..(center.y + ry) {
        points.push((Position::new(center.x - rx, y), '│'));
        points.push((Position::new(center.x + rx, y), '│'));
    }

    // For wider ellipses, add parentheses at the widest point
    if ry > 1 {
        points.push((Position::new(center.x - rx, center.y), '('));
        points.push((Position::new(center.x + rx, center.y), ')'));
    }

    points
}

/// Generate rectangle outline points
pub fn rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    // Single point
    if min_x == max_x && min_y == max_y {
        points.push((Position::new(min_x, min_y), '┼'));
        return points;
    }

    // Horizontal line only
    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '─'));
        }
        return points;
    }

    // Vertical line only
    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '│'));
        }
        return points;
    }

    // Full rectangle
    // Corners
    points.push((Position::new(min_x, min_y), '┌'));
    points.push((Position::new(max_x, min_y), '┐'));
    points.push((Position::new(min_x, max_y), '└'));
    points.push((Position::new(max_x, max_y), '┘'));

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '─'));
        points.push((Position::new(x, max_y), '─'));
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '│'));
        points.push((Position::new(max_x, y), '│'));
    }

    points
}

/// Viewport - what part of the canvas is visible
#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: u16,
    pub height: u16,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            offset_x: 0,
            offset_y: 0,
            width,
            height,
        }
    }

    /// Convert screen coordinates to canvas coordinates
    pub fn screen_to_canvas(&self, screen_x: u16, screen_y: u16) -> Position {
        Position::new(
            screen_x as i32 + self.offset_x,
            screen_y as i32 + self.offset_y,
        )
    }

    /// Convert canvas coordinates to screen coordinates (if visible)
    pub fn canvas_to_screen(&self, pos: Position) -> Option<(u16, u16)> {
        let screen_x = pos.x - self.offset_x;
        let screen_y = pos.y - self.offset_y;

        if screen_x >= 0
            && screen_x < self.width as i32
            && screen_y >= 0
            && screen_y < self.height as i32
        {
            Some((screen_x as u16, screen_y as u16))
        } else {
            None
        }
    }

    /// Pan the viewport
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    /// Resize the viewport
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }
}

/// Generate triangle outline points
pub fn triangle_points(p1: Position, p2: Position, p3: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    // Draw three edges
    draw_line_edge(&mut points, p1, p2);
    draw_line_edge(&mut points, p2, p3);
    draw_line_edge(&mut points, p3, p1);

    points
}

/// Generate parallelogram outline points (slanted rectangle)
pub fn parallelogram_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    // Parallelogram slant offset (about 1/4 of width)
    let slant = (max_x - min_x) / 4;

    // Single point
    if min_x == max_x && min_y == max_y {
        points.push((Position::new(min_x, min_y), '◇'));
        return points;
    }

    // Top edge (offset right)
    for x in (min_x + slant)..=(max_x + slant) {
        if x == min_x + slant {
            points.push((Position::new(x, min_y), '/'));
        } else if x == max_x + slant {
            points.push((Position::new(x, min_y), '\\'));
        } else {
            points.push((Position::new(x, min_y), '─'));
        }
    }

    // Bottom edge
    for x in min_x..=max_x {
        if x == min_x {
            points.push((Position::new(x, max_y), '/'));
        } else if x == max_x {
            points.push((Position::new(x, max_y), '\\'));
        } else {
            points.push((Position::new(x, max_y), '─'));
        }
    }

    // Left edge (slanted)
    for y in (min_y + 1)..max_y {
        let progress = (y - min_y) as f32 / (max_y - min_y) as f32;
        let x = min_x + slant - (slant as f32 * progress) as i32;
        points.push((Position::new(x, y), '/'));
    }

    // Right edge (slanted)
    for y in (min_y + 1)..max_y {
        let progress = (y - min_y) as f32 / (max_y - min_y) as f32;
        let x = max_x + slant - (slant as f32 * progress) as i32;
        points.push((Position::new(x, y), '\\'));
    }

    points
}

/// Generate hexagon outline points
pub fn hexagon_points(center: Position, radius_x: i32, radius_y: i32) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let rx = radius_x.abs().max(2);
    let ry = radius_y.abs().max(1);

    // Small hexagon
    if rx <= 2 && ry <= 1 {
        points.push((Position::new(center.x - 1, center.y - 1), '/'));
        points.push((Position::new(center.x, center.y - 1), '─'));
        points.push((Position::new(center.x + 1, center.y - 1), '\\'));
        points.push((Position::new(center.x - 2, center.y), '<'));
        points.push((Position::new(center.x + 2, center.y), '>'));
        points.push((Position::new(center.x - 1, center.y + 1), '\\'));
        points.push((Position::new(center.x, center.y + 1), '─'));
        points.push((Position::new(center.x + 1, center.y + 1), '/'));
        return points;
    }

    // Top and bottom horizontal edges (shorter than full width)
    let edge_width = rx * 2 / 3;
    for x in (center.x - edge_width)..=(center.x + edge_width) {
        points.push((Position::new(x, center.y - ry), '─'));
        points.push((Position::new(x, center.y + ry), '─'));
    }

    // Left and right points
    points.push((Position::new(center.x - rx, center.y), '<'));
    points.push((Position::new(center.x + rx, center.y), '>'));

    // Top-left and top-right diagonals
    let diag_height = ry;
    for i in 1..diag_height {
        let x_offset = edge_width + (rx - edge_width) * i / diag_height;
        points.push((Position::new(center.x - x_offset, center.y - ry + i), '/'));
        points.push((Position::new(center.x + x_offset, center.y - ry + i), '\\'));
    }

    // Bottom-left and bottom-right diagonals
    for i in 1..diag_height {
        let x_offset = edge_width + (rx - edge_width) * i / diag_height;
        points.push((Position::new(center.x - x_offset, center.y + ry - i), '\\'));
        points.push((Position::new(center.x + x_offset, center.y + ry - i), '/'));
    }

    points
}

/// Generate trapezoid outline points
pub fn trapezoid_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    // Inset for top edge (about 1/4 of width on each side)
    let inset = (max_x - min_x) / 4;

    // Single point
    if min_x == max_x && min_y == max_y {
        points.push((Position::new(min_x, min_y), '◇'));
        return points;
    }

    // Top edge (shorter, centered)
    for x in (min_x + inset)..=(max_x - inset) {
        points.push((Position::new(x, min_y), '─'));
    }

    // Bottom edge (full width)
    for x in min_x..=max_x {
        points.push((Position::new(x, max_y), '─'));
    }

    // Left edge (slanted inward)
    for y in (min_y + 1)..max_y {
        let progress = (y - min_y) as f32 / (max_y - min_y) as f32;
        let x = min_x + inset - (inset as f32 * progress) as i32;
        points.push((Position::new(x, y), '/'));
    }

    // Right edge (slanted inward)
    for y in (min_y + 1)..max_y {
        let progress = (y - min_y) as f32 / (max_y - min_y) as f32;
        let x = max_x - inset + (inset as f32 * progress) as i32;
        points.push((Position::new(x, y), '\\'));
    }

    // Corners
    points.push((Position::new(min_x + inset, min_y), '/'));
    points.push((Position::new(max_x - inset, min_y), '\\'));
    points.push((Position::new(min_x, max_y), '/'));
    points.push((Position::new(max_x, max_y), '\\'));

    points
}

/// Generate rounded rectangle outline points
pub fn rounded_rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    // Single point
    if min_x == max_x && min_y == max_y {
        points.push((Position::new(min_x, min_y), '○'));
        return points;
    }

    // Horizontal line only
    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '─'));
        }
        return points;
    }

    // Vertical line only
    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '│'));
        }
        return points;
    }

    // Rounded corners (using curved Unicode characters)
    points.push((Position::new(min_x, min_y), '╭'));
    points.push((Position::new(max_x, min_y), '╮'));
    points.push((Position::new(min_x, max_y), '╰'));
    points.push((Position::new(max_x, max_y), '╯'));

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '─'));
        points.push((Position::new(x, max_y), '─'));
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '│'));
        points.push((Position::new(max_x, y), '│'));
    }

    points
}

/// Generate cylinder outline points (database symbol)
/// Looks like:
///  .--------.
/// (          )
///  '--------'
///  |        |
///  |        |
/// (          )
///  '--------'
/// Generate cylinder outline points
/// Looks like:
///    .------.
///   (        )
///   |`------'|
///   |        |
///   |        |
///   (        )
///    '------'
pub fn cylinder_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    let width = max_x - min_x;
    let height = max_y - min_y;

    // Single point
    if width == 0 && height == 0 {
        points.push((Position::new(min_x, min_y), 'O'));
        return points;
    }

    // Too small for cylinder shape - use simple tube
    if width < 4 || height < 3 {
        // Simple small cylinder
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '-'));
            points.push((Position::new(x, max_y), '-'));
        }
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '|'));
            points.push((Position::new(max_x, y), '|'));
        }
        return points;
    }

    // Top ellipse - top edge: .------.
    points.push((Position::new(min_x + 1, min_y), '.'));
    points.push((Position::new(max_x - 1, min_y), '.'));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, min_y), '-'));
    }

    // Top ellipse - sides: (        )
    points.push((Position::new(min_x, min_y + 1), '('));
    points.push((Position::new(max_x, min_y + 1), ')'));

    // Inner ellipse showing 3D depth: |`------'|
    points.push((Position::new(min_x, min_y + 2), '|'));
    points.push((Position::new(max_x, min_y + 2), '|'));
    points.push((Position::new(min_x + 1, min_y + 2), '`'));
    points.push((Position::new(max_x - 1, min_y + 2), '\''));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, min_y + 2), '-'));
    }

    // Vertical sides of body
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(min_x, y), '|'));
        points.push((Position::new(max_x, y), '|'));
    }

    // Bottom ellipse - sides: (        )
    points.push((Position::new(min_x, max_y - 1), '('));
    points.push((Position::new(max_x, max_y - 1), ')'));

    // Bottom edge: '------'
    points.push((Position::new(min_x + 1, max_y), '\''));
    points.push((Position::new(max_x - 1, max_y), '\''));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, max_y), '-'));
    }

    points
}

/// Generate cloud outline points
/// Looks like:
///      _   _
///    _( )_( )_
///   (         )
///    `._____.'
pub fn cloud_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);

    let width = max_x - min_x;
    let height = max_y - min_y;

    // Single point
    if width == 0 && height == 0 {
        points.push((Position::new(min_x, min_y), 'o'));
        return points;
    }

    // Very small cloud - simple oval
    if width < 6 || height < 3 {
        points.push((Position::new(min_x, (min_y + max_y) / 2), '('));
        points.push((Position::new(max_x, (min_y + max_y) / 2), ')'));
        for x in (min_x + 1)..max_x {
            points.push((Position::new(x, min_y), '_'));
            points.push((Position::new(x, max_y), '_'));
        }
        return points;
    }

    // Calculate number of bumps based on width
    let bump_width = 4;
    let num_bumps = ((width - 2) / bump_width).max(2).min(5);
    let total_bump_space = num_bumps * 3; // each bump takes ~3 chars: _( )
    let start_offset = (width as i32 - total_bump_space as i32) / 2;

    // Top bumps - row 0: _ between bumps
    // Row 1: ( ) for each bump
    for i in 0..num_bumps {
        let bump_center = min_x + start_offset as i32 + 1 + (i as i32 * 3);
        // Underscore on top
        points.push((Position::new(bump_center, min_y), '_'));
        // Parentheses on second row
        points.push((Position::new(bump_center - 1, min_y + 1), '('));
        points.push((Position::new(bump_center + 1, min_y + 1), ')'));
        // Connector underscore between bumps
        if i < num_bumps - 1 {
            points.push((Position::new(bump_center + 2, min_y + 1), '_'));
        }
    }

    // Left edge underscore before first bump
    let first_bump_x = min_x + start_offset as i32;
    if first_bump_x > min_x + 1 {
        for x in (min_x + 1)..first_bump_x {
            points.push((Position::new(x, min_y + 1), '_'));
        }
    }
    points.push((Position::new(min_x, min_y + 1), '_'));

    // Right edge underscore after last bump
    let last_bump_x = min_x + start_offset as i32 + 2 + ((num_bumps - 1) as i32 * 3);
    if last_bump_x < max_x - 1 {
        for x in (last_bump_x + 1)..max_x {
            points.push((Position::new(x, min_y + 1), '_'));
        }
    }
    points.push((Position::new(max_x, min_y + 1), '_'));

    // Left side
    points.push((Position::new(min_x, min_y + 2), '('));
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(min_x, y), '('));
    }

    // Right side
    points.push((Position::new(max_x, min_y + 2), ')'));
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(max_x, y), ')'));
    }

    // Bottom curve
    points.push((Position::new(min_x + 1, max_y - 1), '`'));
    points.push((Position::new(max_x - 1, max_y - 1), '\''));
    points.push((Position::new(min_x + 2, max_y), '.'));
    for x in (min_x + 3)..(max_x - 2) {
        points.push((Position::new(x, max_y), '_'));
    }
    points.push((Position::new(max_x - 2, max_y), '.'));

    points
}

/// Generate star outline points - 5-pointed star
pub fn star_points(center: Position, outer_radius: i32, _inner_radius: i32) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let r = outer_radius.abs().max(2);

    // Very small star
    if r <= 2 {
        points.push((Position::new(center.x, center.y - 1), '*'));
        points.push((Position::new(center.x - 1, center.y), '*'));
        points.push((Position::new(center.x, center.y), '*'));
        points.push((Position::new(center.x + 1, center.y), '*'));
        points.push((Position::new(center.x, center.y + 1), '*'));
        return points;
    }

    // 5-pointed star with hand-tuned coordinates for ASCII
    // Points: top, upper-right, lower-right, lower-left, upper-left
    let top = Position::new(center.x, center.y - r);
    let upper_right = Position::new(center.x + r, center.y - r / 3);
    let lower_right = Position::new(center.x + r * 2 / 3, center.y + r);
    let lower_left = Position::new(center.x - r * 2 / 3, center.y + r);
    let upper_left = Position::new(center.x - r, center.y - r / 3);

    // Draw pentagram: connect every other point
    // top -> lower_left -> upper_right -> upper_left -> lower_right -> top
    draw_star_segment(&mut points, top, lower_left);
    draw_star_segment(&mut points, lower_left, upper_right);
    draw_star_segment(&mut points, upper_right, upper_left);
    draw_star_segment(&mut points, upper_left, lower_right);
    draw_star_segment(&mut points, lower_right, top);

    // Mark the 5 points
    points.push((top, '*'));
    points.push((upper_right, '*'));
    points.push((lower_right, '*'));
    points.push((lower_left, '*'));
    points.push((upper_left, '*'));

    points
}

/// Draw a line segment for star, choosing char based on direction
fn draw_star_segment(points: &mut Vec<(Position, char)>, from: Position, to: Position) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let steps = dx.abs().max(dy.abs());

    if steps == 0 {
        return;
    }

    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let x = from.x as f32 + dx as f32 * t;
        let y = from.y as f32 + dy as f32 * t;
        let pos = Position::new(x.round() as i32, y.round() as i32);

        // Choose character based on local slope
        let slope = if dx == 0 {
            f32::INFINITY
        } else {
            dy as f32 / dx as f32
        };

        let ch = if slope.abs() > 2.0 {
            '|'
        } else if slope.abs() < 0.5 {
            '-'
        } else if slope > 0.0 {
            '\\'
        } else {
            '/'
        };

        points.push((pos, ch));
    }
}

/// Helper function to draw a line edge between two points
fn draw_line_edge(points: &mut Vec<(Position, char)>, from: Position, to: Position) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;

    if dx == 0 && dy == 0 {
        points.push((from, '*'));
        return;
    }

    // Determine character based on direction
    let ch = if dy == 0 {
        '─'
    } else if dx == 0 {
        '│'
    } else if (dx > 0) == (dy > 0) {
        '\\'
    } else {
        '/'
    };

    for pos in line_points(from, to) {
        points.push((pos, ch));
    }
}
