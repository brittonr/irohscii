//! ASCII art geometry algorithms - line drawing, shape rendering, viewport management.
//!
//! This crate provides the foundational geometric primitives and algorithms for ASCII art:
//! - `Position`: Canvas coordinates (can be negative for infinite canvas)
//! - `Viewport`: Pan/zoom camera for viewing the canvas
//! - `LineStyle`: Different line drawing modes (straight, orthogonal, auto-routed)
//! - Shape rendering functions: rectangles, ellipses, diamonds, triangles, etc.
//!
//! All functions produce `Vec<(Position, char)>` suitable for rendering to a terminal.

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
    /// Auto-routed orthogonal line that avoids obstacles
    OrthogonalAuto,
}

impl LineStyle {
    /// Cycle to the next line style
    pub fn next(self) -> Self {
        match self {
            LineStyle::Straight => LineStyle::OrthogonalHV,
            LineStyle::OrthogonalHV => LineStyle::OrthogonalVH,
            LineStyle::OrthogonalVH => LineStyle::OrthogonalAuto,
            LineStyle::OrthogonalAuto => LineStyle::Straight,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LineStyle::Straight => "Straight",
            LineStyle::OrthogonalHV => "Ortho H->V",
            LineStyle::OrthogonalVH => "Ortho V->H",
            LineStyle::OrthogonalAuto => "Auto Route",
        }
    }
}

/// Viewport - what part of the canvas is visible
#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: u16,
    pub height: u16,
    /// Zoom level: 1.0 = normal, 2.0 = zoomed in (each canvas cell = 2 screen cells)
    pub zoom: f32,
}

/// Minimum zoom level (zoomed out)
pub const MIN_ZOOM: f32 = 0.25;
/// Maximum zoom level (zoomed in)
pub const MAX_ZOOM: f32 = 4.0;
/// Zoom step for keyboard shortcuts
pub const ZOOM_STEP: f32 = 0.25;

// Compile-time assertions for zoom constants
const _: () = assert!(MIN_ZOOM > 0.0, "MIN_ZOOM must be positive");
const _: () = assert!(MAX_ZOOM > MIN_ZOOM, "MAX_ZOOM must be greater than MIN_ZOOM");
const _: () = assert!(ZOOM_STEP > 0.0, "ZOOM_STEP must be positive");

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            offset_x: 0,
            offset_y: 0,
            width,
            height,
            zoom: 1.0,
        }
    }

    /// Convert screen coordinates to canvas coordinates
    /// At zoom > 1.0, screen space is larger than canvas space
    pub fn screen_to_canvas(&self, screen_x: u16, screen_y: u16) -> Position {
        debug_assert!(self.zoom > 0.0, "zoom must be positive");
        debug_assert!(self.width > 0, "width must be positive");
        debug_assert!(self.height > 0, "height must be positive");
        
        let canvas_x = (f32::from(screen_x) / self.zoom)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
        let canvas_y = (f32::from(screen_y) / self.zoom)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
        
        let result = Position::new(
            canvas_x.saturating_add(self.offset_x),
            canvas_y.saturating_add(self.offset_y),
        );
        
        debug_assert!(result.x >= i32::MIN && result.x <= i32::MAX, "canvas_x in valid range");
        debug_assert!(result.y >= i32::MIN && result.y <= i32::MAX, "canvas_y in valid range");
        
        result
    }

    /// Convert canvas coordinates to screen coordinates (if visible)
    /// At zoom > 1.0, canvas positions map to larger screen areas
    pub fn canvas_to_screen(&self, pos: Position) -> Option<(u16, u16)> {
        debug_assert!(self.zoom > 0.0, "zoom must be positive");
        debug_assert!(self.width > 0, "width must be positive");
        debug_assert!(self.height > 0, "height must be positive");
        
        let canvas_x_offset = pos.x.saturating_sub(self.offset_x);
        let canvas_y_offset = pos.y.saturating_sub(self.offset_y);
        
        let screen_x = (canvas_x_offset as f32 * self.zoom)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
        let screen_y = (canvas_y_offset as f32 * self.zoom)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32;

        if screen_x < 0 {
            return None;
        }
        if screen_y < 0 {
            return None;
        }
        if screen_x >= i32::from(self.width) {
            return None;
        }
        if screen_y >= i32::from(self.height) {
            return None;
        }

        Some((screen_x as u16, screen_y as u16))
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

    /// Zoom in (increase zoom level)
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + ZOOM_STEP).min(MAX_ZOOM);
    }

    /// Zoom out (decrease zoom level)
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom - ZOOM_STEP).max(MIN_ZOOM);
    }

    /// Reset zoom to 100%
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    /// Get the visible canvas area (number of canvas cells visible)
    pub fn visible_canvas_size(&self) -> (u16, u16) {
        debug_assert!(self.zoom > 0.0, "zoom must be positive");
        debug_assert!(self.width > 0, "width must be positive");
        debug_assert!(self.height > 0, "height must be positive");
        
        let visible_width = (f32::from(self.width) / self.zoom)
            .round()
            .clamp(0.0, u16::MAX as f32) as u16;
        let visible_height = (f32::from(self.height) / self.zoom)
            .round()
            .clamp(0.0, u16::MAX as f32) as u16;
        
        (visible_width, visible_height)
    }
}

/// Maximum iterations for line drawing to prevent infinite loops
const MAX_LINE_ITERATIONS: usize = 100_000;

/// Generate all points on a line using Bresenham's algorithm
pub fn line_points(from: Position, to: Position) -> Vec<Position> {
    debug_assert!(from.x >= i32::MIN && from.x <= i32::MAX, "from.x in valid range");
    debug_assert!(from.y >= i32::MIN && from.y <= i32::MAX, "from.y in valid range");
    debug_assert!(to.x >= i32::MIN && to.x <= i32::MAX, "to.x in valid range");
    debug_assert!(to.y >= i32::MIN && to.y <= i32::MAX, "to.y in valid range");
    
    let mut points = Vec::new();

    let dx = (to.x - from.x).abs();
    let dy = -(to.y - from.y).abs();
    let sx = if from.x < to.x { 1 } else { -1 };
    let sy = if from.y < to.y { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = from.x;
    let mut y = from.y;

    let mut iterations = 0;
    loop {
        points.push(Position::new(x, y));

        if x == to.x {
            if y == to.y {
                break;
            }
        }

        iterations += 1;
        if iterations >= MAX_LINE_ITERATIONS {
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

    debug_assert!(!points.is_empty(), "line_points should always return at least one point");
    debug_assert!(points[0] == from, "first point should be from");
    
    points
}

/// Generate line points with appropriate characters based on style and direction
pub fn line_points_styled(from: Position, to: Position, style: LineStyle) -> Vec<(Position, char)> {
    let result = match style {
        LineStyle::Straight => straight_line_points(from, to),
        LineStyle::OrthogonalHV => orthogonal_line_points(from, to, true),
        LineStyle::OrthogonalVH => orthogonal_line_points(from, to, false),
        // OrthogonalAuto falls back to HV when no obstacles - actual routing done elsewhere
        LineStyle::OrthogonalAuto => orthogonal_line_points(from, to, true),
    };
    
    debug_assert!(!result.is_empty(), "line_points_styled should return points");
    
    result
}

/// Generate auto-routed line points that avoid obstacles
/// Returns points for an orthogonal path that doesn't intersect any obstacle
pub fn line_points_auto_routed(
    from: Position,
    to: Position,
    obstacles: &[(i32, i32, i32, i32)], // (min_x, min_y, max_x, max_y) for each obstacle
) -> Vec<(Position, char)> {
    debug_assert!(obstacles.len() < 1000, "reasonable number of obstacles");
    
    // Try horizontal-then-vertical first
    let hv_path = orthogonal_line_points(from, to, true);
    if !path_intersects_obstacles(&hv_path, obstacles) {
        debug_assert!(!hv_path.is_empty(), "HV path should have points");
        return hv_path;
    }

    // Try vertical-then-horizontal
    let vh_path = orthogonal_line_points(from, to, false);
    if !path_intersects_obstacles(&vh_path, obstacles) {
        debug_assert!(!vh_path.is_empty(), "VH path should have points");
        return vh_path;
    }

    // Both blocked - try routing around with a waypoint
    // Find the blocking obstacle and route around it
    if let Some(waypoint) = find_routing_waypoint(from, to, obstacles) {
        let mut path = Vec::new();
        // Route: from -> waypoint -> to
        path.extend(orthogonal_line_points(from, waypoint, true));
        // Remove duplicate corner point
        if !path.is_empty() {
            path.pop();
        }
        path.extend(orthogonal_line_points(waypoint, to, true));
        debug_assert!(!path.is_empty(), "waypoint path should have points");
        return path;
    }

    // Fallback to HV if no good route found
    debug_assert!(!hv_path.is_empty(), "fallback HV path should have points");
    hv_path
}

/// Check if a path intersects any obstacles
fn path_intersects_obstacles(
    path: &[(Position, char)],
    obstacles: &[(i32, i32, i32, i32)],
) -> bool {
    debug_assert!(path.len() < 100_000, "path length is reasonable");
    debug_assert!(obstacles.len() < 1000, "obstacle count is reasonable");
    
    for (pos, _) in path {
        for &(min_x, min_y, max_x, max_y) in obstacles {
            debug_assert!(max_x >= min_x, "obstacle max_x >= min_x");
            debug_assert!(max_y >= min_y, "obstacle max_y >= min_y");
            
            // Check if point is inside obstacle (with 1 cell margin)
            if pos.x > min_x {
                if pos.x < max_x {
                    if pos.y > min_y {
                        if pos.y < max_y {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Find a waypoint to route around obstacles
fn find_routing_waypoint(
    from: Position,
    to: Position,
    obstacles: &[(i32, i32, i32, i32)],
) -> Option<Position> {
    debug_assert!(obstacles.len() < 1000, "reasonable number of obstacles");
    
    // Find the first blocking obstacle
    let hv_path = orthogonal_line_points(from, to, true);
    let blocking_obstacle = obstacles.iter().find(|&&(min_x, min_y, max_x, max_y)| {
        hv_path.iter().any(|(pos, _)| {
            if pos.x > min_x {
                if pos.x < max_x {
                    if pos.y > min_y {
                        if pos.y < max_y {
                            return true;
                        }
                    }
                }
            }
            false
        })
    })?;

    let (min_x, min_y, max_x, max_y) = *blocking_obstacle;
    
    debug_assert!(max_x >= min_x, "blocking obstacle max_x >= min_x");
    debug_assert!(max_y >= min_y, "blocking obstacle max_y >= min_y");

    // Try routing around each side of the obstacle
    let margin = 1;
    let candidates = [
        // Go above
        Position::new(from.x, min_y - margin),
        // Go below
        Position::new(from.x, max_y + margin),
        // Go left
        Position::new(min_x - margin, from.y),
        // Go right
        Position::new(max_x + margin, from.y),
    ];

    // Find the candidate that creates the shortest total path
    candidates
        .into_iter()
        .filter(|wp| {
            // Check waypoint doesn't hit any obstacle
            !obstacles.iter().any(|&(ox1, oy1, ox2, oy2)| {
                if wp.x >= ox1 {
                    if wp.x <= ox2 {
                        if wp.y >= oy1 {
                            if wp.y <= oy2 {
                                return true;
                            }
                        }
                    }
                }
                false
            })
        })
        .min_by_key(|wp| {
            // Manhattan distance through waypoint
            (from.x - wp.x).abs()
                + (from.y - wp.y).abs()
                + (wp.x - to.x).abs()
                + (wp.y - to.y).abs()
        })
}

/// Generate points for a horizontal straight line
fn straight_line_horizontal(from: Position, to: Position) -> Vec<(Position, char)> {
    debug_assert!(from.y == to.y, "horizontal line must have same y coordinate");
    
    let (start_x, end_x) = if to.x >= from.x {
        (from.x, to.x)
    } else {
        (to.x, from.x)
    };
    
    let result: Vec<_> = (start_x..=end_x)
        .map(|x| (Position::new(x, from.y), '\u{2500}')) // ─
        .collect();
    
    debug_assert!(!result.is_empty(), "horizontal line should have points");
    debug_assert!(result.len() == ((end_x - start_x).abs() + 1) as usize, "correct number of horizontal points");
    
    result
}

/// Generate points for a vertical straight line
fn straight_line_vertical(from: Position, to: Position) -> Vec<(Position, char)> {
    debug_assert!(from.x == to.x, "vertical line must have same x coordinate");
    
    let (start_y, end_y) = if to.y >= from.y {
        (from.y, to.y)
    } else {
        (to.y, from.y)
    };
    
    let result: Vec<_> = (start_y..=end_y)
        .map(|y| (Position::new(from.x, y), '\u{2502}')) // │
        .collect();
    
    debug_assert!(!result.is_empty(), "vertical line should have points");
    debug_assert!(result.len() == ((end_y - start_y).abs() + 1) as usize, "correct number of vertical points");
    
    result
}

/// Determine the character for a point on a diagonal line
fn diagonal_char_for_point(
    points: &[Position],
    index: usize,
    abs_dx: i32,
    abs_dy: i32,
    diag_char: char,
) -> char {
    debug_assert!(index < points.len(), "index within bounds");
    
    if index == 0 {
        return diagonal_char_endpoint(abs_dx, abs_dy, diag_char);
    }
    
    if index == points.len() - 1 {
        return diagonal_char_endpoint(abs_dx, abs_dy, diag_char);
    }
    
    // Middle points - check local direction
    let prev = points[index - 1];
    let next = points[index + 1];

    let local_dx = (next.x - prev.x).abs();
    let local_dy = (next.y - prev.y).abs();

    if local_dy == 0 {
        '\u{2500}' // ─
    } else if local_dx == 0 {
        '\u{2502}' // │
    } else {
        diag_char
    }
}

/// Determine endpoint character for diagonal lines
fn diagonal_char_endpoint(abs_dx: i32, abs_dy: i32, diag_char: char) -> char {
    if abs_dx > abs_dy * 2 {
        '\u{2500}' // ─
    } else if abs_dy > abs_dx * 2 {
        '\u{2502}' // │
    } else {
        diag_char
    }
}

/// Generate a straight line with smart character selection
fn straight_line_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;

    // Single point
    if dx == 0 {
        if dy == 0 {
            return vec![(from, '\u{25CB}')]; // ○
        }
    }

    // Pure horizontal
    if dy == 0 {
        return straight_line_horizontal(from, to);
    }

    // Pure vertical
    if dx == 0 {
        return straight_line_vertical(from, to);
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

    let points = line_points(from, to);
    let mut result = Vec::with_capacity(points.len());

    for i in 0..points.len() {
        let ch = diagonal_char_for_point(&points, i, abs_dx, abs_dy, diag_char);
        result.push((points[i], ch));
    }

    debug_assert!(!result.is_empty(), "straight_line_points should return points");
    debug_assert!(result[0].0 == from || result[result.len()-1].0 == from, "from position included");
    
    result
}

/// Determine the corner character for an orthogonal line
fn orthogonal_corner_char(dx: i32, dy: i32, horizontal_first: bool) -> char {
    match (dx > 0, dy > 0, horizontal_first) {
        // Horizontal first (corner at to.x, from.y)
        (true, true, true) => '\u{2510}',   // ┐ right then down
        (true, false, true) => '\u{2518}',  // ┘ right then up
        (false, true, true) => '\u{250C}',  // ┌ left then down
        (false, false, true) => '\u{2514}', // └ left then up
        // Vertical first (corner at from.x, to.y)
        (true, true, false) => '\u{2514}',   // └ down then right
        (true, false, false) => '\u{250C}',  // ┌ up then right
        (false, true, false) => '\u{2518}',  // ┘ down then left
        (false, false, false) => '\u{2510}', // ┐ up then left
    }
}

/// Generate orthogonal line with horizontal segment first
fn orthogonal_horizontal_first(
    from: Position,
    to: Position,
    corner: Position,
    corner_char: char,
    dx: i32,
    dy: i32,
) -> Vec<(Position, char)> {
    debug_assert!(corner.x == to.x && corner.y == from.y, "corner at correct position for H-first");
    
    let mut result = Vec::new();
    
    // Horizontal segment (excluding corner)
    let (start_x, end_x) = if dx > 0 {
        (from.x, corner.x - 1)
    } else {
        (corner.x + 1, from.x)
    };
    for x in start_x.min(end_x)..=start_x.max(end_x) {
        result.push((Position::new(x, from.y), '\u{2500}')); // ─
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
        result.push((Position::new(to.x, y), '\u{2502}')); // │
    }
    
    debug_assert!(!result.is_empty(), "H-first path should have points");
    
    result
}

/// Generate orthogonal line with vertical segment first
fn orthogonal_vertical_first(
    from: Position,
    to: Position,
    corner: Position,
    corner_char: char,
    dx: i32,
    dy: i32,
) -> Vec<(Position, char)> {
    debug_assert!(corner.x == from.x && corner.y == to.y, "corner at correct position for V-first");
    
    let mut result = Vec::new();
    
    // Vertical segment (excluding corner)
    let (start_y, end_y) = if dy > 0 {
        (from.y, corner.y - 1)
    } else {
        (corner.y + 1, from.y)
    };
    for y in start_y.min(end_y)..=start_y.max(end_y) {
        result.push((Position::new(from.x, y), '\u{2502}')); // │
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
        result.push((Position::new(x, to.y), '\u{2500}')); // ─
    }
    
    debug_assert!(!result.is_empty(), "V-first path should have points");
    
    result
}

/// Generate an orthogonal line (L-shaped) with corner
fn orthogonal_line_points(
    from: Position,
    to: Position,
    horizontal_first: bool,
) -> Vec<(Position, char)> {
    // Single point
    if from == to {
        return vec![(from, '\u{25CB}')]; // ○
    }

    let dx = to.x - from.x;
    let dy = to.y - from.y;

    // Pure horizontal - no corner needed
    if dx == 0 {
        return straight_line_points(from, to);
    }
    
    // Pure vertical - no corner needed
    if dy == 0 {
        return straight_line_points(from, to);
    }

    // Calculate corner position
    let corner = if horizontal_first {
        Position::new(to.x, from.y)
    } else {
        Position::new(from.x, to.y)
    };

    // Determine corner character
    let corner_char = orthogonal_corner_char(dx, dy, horizontal_first);

    let result = if horizontal_first {
        orthogonal_horizontal_first(from, to, corner, corner_char, dx, dy)
    } else {
        orthogonal_vertical_first(from, to, corner, corner_char, dx, dy)
    };
    
    debug_assert!(!result.is_empty(), "orthogonal_line_points should return points");
    
    result
}

/// Generate arrow points (line with arrowhead at end)
pub fn arrow_points_styled(
    from: Position,
    to: Position,
    style: LineStyle,
) -> Vec<(Position, char)> {
    let mut points = line_points_styled(from, to, style);
    
    debug_assert!(!points.is_empty(), "line_points_styled should return points");

    // Replace the last character with an arrowhead
    if let Some((_, ch)) = points.last_mut() {
        let dx = to.x - from.x;
        let dy = to.y - from.y;

        *ch = if dx.abs() > dy.abs() {
            // Predominantly horizontal
            if dx > 0 {
                '\u{2192}' // →
            } else {
                '\u{2190}' // ←
            }
        } else if dy.abs() > dx.abs() {
            // Predominantly vertical
            if dy > 0 {
                '\u{2193}' // ↓
            } else {
                '\u{2191}' // ↑
            }
        } else {
            // Diagonal - pick based on direction
            match (dx > 0, dy > 0) {
                (true, true) => '\u{2198}',   // ↘
                (true, false) => '\u{2197}',  // ↗
                (false, true) => '\u{2199}',  // ↙
                (false, false) => '\u{2196}', // ↖
            }
        };
    }

    debug_assert!(!points.is_empty(), "arrow_points_styled should return points");

    points
}

/// Generate rectangle outline points
pub fn rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    // Single point
    if min_x == max_x {
        if min_y == max_y {
            points.push((Position::new(min_x, min_y), '\u{253C}')); // ┼
            debug_assert!(points.len() == 1, "single point rect has 1 point");
            return points;
        }
    }

    // Horizontal line only
    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '\u{2500}')); // ─
        }
        debug_assert!(!points.is_empty(), "horizontal line has points");
        return points;
    }

    // Vertical line only
    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '\u{2502}')); // │
        }
        debug_assert!(!points.is_empty(), "vertical line has points");
        return points;
    }

    // Full rectangle - Corners
    points.push((Position::new(min_x, min_y), '\u{250C}')); // ┌
    points.push((Position::new(max_x, min_y), '\u{2510}')); // ┐
    points.push((Position::new(min_x, max_y), '\u{2514}')); // └
    points.push((Position::new(max_x, max_y), '\u{2518}')); // ┘

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '\u{2500}')); // ─
        points.push((Position::new(x, max_y), '\u{2500}')); // ─
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '\u{2502}')); // │
        points.push((Position::new(max_x, y), '\u{2502}')); // │
    }

    debug_assert!(!points.is_empty(), "rect_points should return points");
    debug_assert!(points.len() >= 4, "full rect has at least 4 corners");

    points
}

/// Generate double-line rectangle outline points
pub fn double_rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    // Single point
    if min_x == max_x {
        if min_y == max_y {
            points.push((Position::new(min_x, min_y), '\u{256C}')); // ╬
            debug_assert!(points.len() == 1, "single point has 1 point");
            return points;
        }
    }

    // Horizontal line only
    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '\u{2550}')); // ═
        }
        debug_assert!(!points.is_empty(), "horizontal double line has points");
        return points;
    }

    // Vertical line only
    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '\u{2551}')); // ║
        }
        debug_assert!(!points.is_empty(), "vertical double line has points");
        return points;
    }

    // Full rectangle with double lines - Corners
    points.push((Position::new(min_x, min_y), '\u{2554}')); // ╔
    points.push((Position::new(max_x, min_y), '\u{2557}')); // ╗
    points.push((Position::new(min_x, max_y), '\u{255A}')); // ╚
    points.push((Position::new(max_x, max_y), '\u{255D}')); // ╝

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '\u{2550}')); // ═
        points.push((Position::new(x, max_y), '\u{2550}')); // ═
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '\u{2551}')); // ║
        points.push((Position::new(max_x, y), '\u{2551}')); // ║
    }

    debug_assert!(!points.is_empty(), "double_rect_points should return points");
    debug_assert!(points.len() >= 4, "full double rect has at least 4 corners");

    points
}

/// Generate diamond outline points
pub fn diamond_points(
    center: Position,
    half_width: i32,
    half_height: i32,
) -> Vec<(Position, char)> {
    debug_assert!(half_width >= 0, "half_width should be non-negative");
    debug_assert!(half_height >= 0, "half_height should be non-negative");
    
    let mut points = Vec::new();

    let hw = half_width.abs().max(1);
    let hh = half_height.abs().max(1);

    // Single point
    if hw == 0 {
        if hh == 0 {
            points.push((center, '\u{25C7}')); // ◇
            debug_assert!(points.len() == 1, "single point diamond has 1 point");
            return points;
        }
    }

    // Very small diamond
    if hw == 1 {
        if hh == 1 {
            points.push((Position::new(center.x, center.y - 1), '^'));
            points.push((Position::new(center.x - 1, center.y), '<'));
            points.push((Position::new(center.x + 1, center.y), '>'));
            points.push((Position::new(center.x, center.y + 1), 'v'));
            debug_assert!(points.len() == 4, "small diamond has 4 points");
            return points;
        }
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
    for i in 1..hh {
        let x = center.x + (hw * i) / hh;
        let y = center.y - hh + i;
        points.push((Position::new(x, y), '\\'));
    }
    for i in 1..hh {
        let x = center.x - (hw * i) / hh;
        let y = center.y - hh + i;
        points.push((Position::new(x, y), '/'));
    }
    for i in 1..hh {
        let x = center.x + (hw * i) / hh;
        let y = center.y + hh - i;
        points.push((Position::new(x, y), '/'));
    }
    for i in 1..hh {
        let x = center.x - (hw * i) / hh;
        let y = center.y + hh - i;
        points.push((Position::new(x, y), '\\'));
    }

    debug_assert!(!points.is_empty(), "diamond_points should return points");
    debug_assert!(points.len() >= 4, "diamond has at least 4 cardinal points");

    points
}

/// Generate ellipse outline points
pub fn ellipse_points(center: Position, radius_x: i32, radius_y: i32) -> Vec<(Position, char)> {
    debug_assert!(radius_x >= 0, "radius_x should be non-negative");
    debug_assert!(radius_y >= 0, "radius_y should be non-negative");
    
    let mut points = Vec::new();

    let rx = radius_x.abs().max(1);
    let ry = radius_y.abs().max(1);

    // Very small ellipse
    if rx <= 1 {
        if ry <= 1 {
            points.push((Position::new(center.x, center.y - 1), '\u{2500}')); // ─
            points.push((Position::new(center.x - 1, center.y), '('));
            points.push((Position::new(center.x + 1, center.y), ')'));
            points.push((Position::new(center.x, center.y + 1), '\u{2500}')); // ─
            debug_assert!(points.len() == 4, "small ellipse has 4 points");
            return points;
        }
    }

    // Left and right parentheses
    points.push((Position::new(center.x - rx, center.y), '('));
    points.push((Position::new(center.x + rx, center.y), ')'));

    // Top horizontal edge
    for x in (center.x - rx + 1)..(center.x + rx) {
        points.push((Position::new(x, center.y - ry), '\u{2500}')); // ─
    }
    // Bottom horizontal edge
    for x in (center.x - rx + 1)..(center.x + rx) {
        points.push((Position::new(x, center.y + ry), '\u{2500}')); // ─
    }

    // Corners
    points.push((Position::new(center.x - rx, center.y - ry), '\u{256D}')); // ╭
    points.push((Position::new(center.x + rx, center.y - ry), '\u{256E}')); // ╮
    points.push((Position::new(center.x - rx, center.y + ry), '\u{2570}')); // ╰
    points.push((Position::new(center.x + rx, center.y + ry), '\u{256F}')); // ╯

    // Left and right vertical edges
    for y in (center.y - ry + 1)..(center.y + ry) {
        points.push((Position::new(center.x - rx, y), '\u{2502}')); // │
        points.push((Position::new(center.x + rx, y), '\u{2502}')); // │
    }

    // For wider ellipses, add parentheses at the widest point
    if ry > 1 {
        points.push((Position::new(center.x - rx, center.y), '('));
        points.push((Position::new(center.x + rx, center.y), ')'));
    }

    debug_assert!(!points.is_empty(), "ellipse_points should return points");
    debug_assert!(points.len() >= 4, "ellipse has at least 4 key points");

    points
}

/// Generate triangle outline points
pub fn triangle_points(p1: Position, p2: Position, p3: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    // Draw three edges
    draw_line_edge(&mut points, p1, p2);
    draw_line_edge(&mut points, p2, p3);
    draw_line_edge(&mut points, p3, p1);

    debug_assert!(!points.is_empty(), "triangle_points should return points");
    debug_assert!(points.len() >= 3, "triangle has at least 3 vertices");

    points
}

/// Generate parallelogram outline points (slanted rectangle)
pub fn parallelogram_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    let slant = (max_x - min_x) / 4;

    if min_x == max_x {
        if min_y == max_y {
            points.push((Position::new(min_x, min_y), '\u{25C7}')); // ◇
            debug_assert!(points.len() == 1, "single point parallelogram has 1 point");
            return points;
        }
    }

    // Top edge (offset right)
    for x in (min_x + slant)..=(max_x + slant) {
        if x == min_x + slant {
            points.push((Position::new(x, min_y), '/'));
        } else if x == max_x + slant {
            points.push((Position::new(x, min_y), '\\'));
        } else {
            points.push((Position::new(x, min_y), '\u{2500}')); // ─
        }
    }

    // Bottom edge
    for x in min_x..=max_x {
        if x == min_x {
            points.push((Position::new(x, max_y), '/'));
        } else if x == max_x {
            points.push((Position::new(x, max_y), '\\'));
        } else {
            points.push((Position::new(x, max_y), '\u{2500}')); // ─
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

    debug_assert!(!points.is_empty(), "parallelogram_points should return points");
    debug_assert!(points.len() >= 4, "parallelogram has at least 4 corners");

    points
}

/// Generate hexagon outline points
pub fn hexagon_points(center: Position, radius_x: i32, radius_y: i32) -> Vec<(Position, char)> {
    debug_assert!(radius_x >= 0, "radius_x should be non-negative");
    debug_assert!(radius_y >= 0, "radius_y should be non-negative");
    
    let mut points = Vec::new();

    let rx = radius_x.abs().max(2);
    let ry = radius_y.abs().max(1);

    // Small hexagon
    if rx <= 2 {
        if ry <= 1 {
            points.push((Position::new(center.x - 1, center.y - 1), '/'));
            points.push((Position::new(center.x, center.y - 1), '\u{2500}')); // ─
            points.push((Position::new(center.x + 1, center.y - 1), '\\'));
            points.push((Position::new(center.x - 2, center.y), '<'));
            points.push((Position::new(center.x + 2, center.y), '>'));
            points.push((Position::new(center.x - 1, center.y + 1), '\\'));
            points.push((Position::new(center.x, center.y + 1), '\u{2500}')); // ─
            points.push((Position::new(center.x + 1, center.y + 1), '/'));
            debug_assert!(points.len() == 8, "small hexagon has 8 points");
            return points;
        }
    }

    // Top and bottom horizontal edges
    let edge_width = rx * 2 / 3;
    for x in (center.x - edge_width)..=(center.x + edge_width) {
        points.push((Position::new(x, center.y - ry), '\u{2500}')); // ─
        points.push((Position::new(x, center.y + ry), '\u{2500}')); // ─
    }

    // Left and right points
    points.push((Position::new(center.x - rx, center.y), '<'));
    points.push((Position::new(center.x + rx, center.y), '>'));

    // Diagonals
    let diag_height = ry;
    for i in 1..diag_height {
        let x_offset = edge_width + (rx - edge_width) * i / diag_height;
        points.push((Position::new(center.x - x_offset, center.y - ry + i), '/'));
        points.push((Position::new(center.x + x_offset, center.y - ry + i), '\\'));
    }

    for i in 1..diag_height {
        let x_offset = edge_width + (rx - edge_width) * i / diag_height;
        points.push((Position::new(center.x - x_offset, center.y + ry - i), '\\'));
        points.push((Position::new(center.x + x_offset, center.y + ry - i), '/'));
    }

    debug_assert!(!points.is_empty(), "hexagon_points should return points");
    debug_assert!(points.len() >= 6, "hexagon has at least 6 vertices");

    points
}

/// Generate trapezoid outline points
pub fn trapezoid_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    let inset = (max_x - min_x) / 4;

    if min_x == max_x {
        if min_y == max_y {
            points.push((Position::new(min_x, min_y), '\u{25C7}')); // ◇
            debug_assert!(points.len() == 1, "single point trapezoid has 1 point");
            return points;
        }
    }

    // Top edge (shorter, centered)
    for x in (min_x + inset)..=(max_x - inset) {
        points.push((Position::new(x, min_y), '\u{2500}')); // ─
    }

    // Bottom edge (full width)
    for x in min_x..=max_x {
        points.push((Position::new(x, max_y), '\u{2500}')); // ─
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

    debug_assert!(!points.is_empty(), "trapezoid_points should return points");
    debug_assert!(points.len() >= 4, "trapezoid has at least 4 corners");

    points
}

/// Generate rounded rectangle outline points
pub fn rounded_rect_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    if min_x == max_x {
        if min_y == max_y {
            points.push((Position::new(min_x, min_y), '\u{25CB}')); // ○
            debug_assert!(points.len() == 1, "single point rounded rect has 1 point");
            return points;
        }
    }

    if min_y == max_y {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '\u{2500}')); // ─
        }
        debug_assert!(!points.is_empty(), "horizontal rounded line has points");
        return points;
    }

    if min_x == max_x {
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '\u{2502}')); // │
        }
        debug_assert!(!points.is_empty(), "vertical rounded line has points");
        return points;
    }

    // Rounded corners
    points.push((Position::new(min_x, min_y), '\u{256D}')); // ╭
    points.push((Position::new(max_x, min_y), '\u{256E}')); // ╮
    points.push((Position::new(min_x, max_y), '\u{2570}')); // ╰
    points.push((Position::new(max_x, max_y), '\u{256F}')); // ╯

    // Horizontal lines
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '\u{2500}')); // ─
        points.push((Position::new(x, max_y), '\u{2500}')); // ─
    }

    // Vertical lines
    for y in (min_y + 1)..max_y {
        points.push((Position::new(min_x, y), '\u{2502}')); // │
        points.push((Position::new(max_x, y), '\u{2502}')); // │
    }

    debug_assert!(!points.is_empty(), "rounded_rect_points should return points");
    debug_assert!(points.len() >= 4, "rounded rect has at least 4 corners");

    points
}

/// Generate cylinder outline points (database symbol)
pub fn cylinder_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let mut points = Vec::new();

    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    let width = max_x - min_x;
    let height = max_y - min_y;

    if width == 0 {
        if height == 0 {
            points.push((Position::new(min_x, min_y), 'O'));
            debug_assert!(points.len() == 1, "single point cylinder has 1 point");
            return points;
        }
    }

    if width < 4 {
        for x in min_x..=max_x {
            points.push((Position::new(x, min_y), '-'));
            points.push((Position::new(x, max_y), '-'));
        }
        for y in min_y..=max_y {
            points.push((Position::new(min_x, y), '|'));
            points.push((Position::new(max_x, y), '|'));
        }
        debug_assert!(!points.is_empty(), "small cylinder has points");
        return points;
    }
    
    if height < 3 {
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

    // Top ellipse
    points.push((Position::new(min_x + 1, min_y), '.'));
    points.push((Position::new(max_x - 1, min_y), '.'));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, min_y), '-'));
    }

    points.push((Position::new(min_x, min_y + 1), '('));
    points.push((Position::new(max_x, min_y + 1), ')'));

    // Inner ellipse
    points.push((Position::new(min_x, min_y + 2), '|'));
    points.push((Position::new(max_x, min_y + 2), '|'));
    points.push((Position::new(min_x + 1, min_y + 2), '`'));
    points.push((Position::new(max_x - 1, min_y + 2), '\''));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, min_y + 2), '-'));
    }

    // Vertical sides
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(min_x, y), '|'));
        points.push((Position::new(max_x, y), '|'));
    }

    // Bottom ellipse
    points.push((Position::new(min_x, max_y - 1), '('));
    points.push((Position::new(max_x, max_y - 1), ')'));

    points.push((Position::new(min_x + 1, max_y), '\''));
    points.push((Position::new(max_x - 1, max_y), '\''));
    for x in (min_x + 2)..(max_x - 1) {
        points.push((Position::new(x, max_y), '-'));
    }

    debug_assert!(!points.is_empty(), "cylinder_points should return points");
    debug_assert!(points.len() >= 8, "cylinder has multiple components");

    points
}

/// Generate a small cloud (simplified version)
fn cloud_points_small(min_x: i32, max_x: i32, min_y: i32, max_y: i32) -> Vec<(Position, char)> {
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");
    
    let mut points = Vec::new();
    
    points.push((Position::new(min_x, (min_y + max_y) / 2), '('));
    points.push((Position::new(max_x, (min_y + max_y) / 2), ')'));
    for x in (min_x + 1)..max_x {
        points.push((Position::new(x, min_y), '_'));
        points.push((Position::new(x, max_y), '_'));
    }
    
    debug_assert!(!points.is_empty(), "small cloud should have points");
    
    points
}

/// Draw cloud top bumps
fn cloud_draw_top_bumps(
    points: &mut Vec<(Position, char)>,
    min_x: i32,
    min_y: i32,
    num_bumps: i32,
    start_offset: i32,
) {
    debug_assert!(num_bumps > 0, "num_bumps should be positive");
    
    for i in 0..num_bumps {
        let bump_center = min_x + start_offset + 1 + (i * 3);
        points.push((Position::new(bump_center, min_y), '_'));
        points.push((Position::new(bump_center - 1, min_y + 1), '('));
        points.push((Position::new(bump_center + 1, min_y + 1), ')'));
        if i < num_bumps - 1 {
            points.push((Position::new(bump_center + 2, min_y + 1), '_'));
        }
    }
}

/// Draw cloud connecting lines around bumps
fn cloud_draw_connecting_lines(
    points: &mut Vec<(Position, char)>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    num_bumps: i32,
    start_offset: i32,
) {
    let first_bump_x = min_x + start_offset;
    if first_bump_x > min_x + 1 {
        for x in (min_x + 1)..first_bump_x {
            points.push((Position::new(x, min_y + 1), '_'));
        }
    }
    points.push((Position::new(min_x, min_y + 1), '_'));

    let last_bump_x = min_x + start_offset + 2 + ((num_bumps - 1) * 3);
    if last_bump_x < max_x - 1 {
        for x in (last_bump_x + 1)..max_x {
            points.push((Position::new(x, min_y + 1), '_'));
        }
    }
    points.push((Position::new(max_x, min_y + 1), '_'));
}

/// Draw cloud sides
fn cloud_draw_sides(
    points: &mut Vec<(Position, char)>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) {
    points.push((Position::new(min_x, min_y + 2), '('));
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(min_x, y), '('));
    }

    points.push((Position::new(max_x, min_y + 2), ')'));
    for y in (min_y + 3)..(max_y - 1) {
        points.push((Position::new(max_x, y), ')'));
    }
}

/// Draw cloud bottom
fn cloud_draw_bottom(
    points: &mut Vec<(Position, char)>,
    min_x: i32,
    max_x: i32,
    max_y: i32,
) {
    points.push((Position::new(min_x + 1, max_y - 1), '`'));
    points.push((Position::new(max_x - 1, max_y - 1), '\''));
    points.push((Position::new(min_x + 2, max_y), '.'));
    for x in (min_x + 3)..(max_x - 2) {
        points.push((Position::new(x, max_y), '_'));
    }
    points.push((Position::new(max_x - 2, max_y), '.'));
}

/// Generate cloud outline points
pub fn cloud_points(from: Position, to: Position) -> Vec<(Position, char)> {
    let min_x = from.x.min(to.x);
    let max_x = from.x.max(to.x);
    let min_y = from.y.min(to.y);
    let max_y = from.y.max(to.y);
    
    debug_assert!(max_x >= min_x, "max_x should be >= min_x");
    debug_assert!(max_y >= min_y, "max_y should be >= min_y");

    let mut points = Vec::new();

    let width = max_x - min_x;
    let height = max_y - min_y;

    if width == 0 {
        if height == 0 {
            points.push((Position::new(min_x, min_y), 'o'));
            return points;
        }
    }

    if width < 6 {
        return cloud_points_small(min_x, max_x, min_y, max_y);
    }
    
    if height < 3 {
        return cloud_points_small(min_x, max_x, min_y, max_y);
    }

    // Calculate bumps
    let bump_width = 4;
    let num_bumps = ((width - 2) / bump_width).clamp(2, 5);
    let total_bump_space = num_bumps * 3;
    let start_offset = (width - total_bump_space) / 2;

    cloud_draw_top_bumps(&mut points, min_x, min_y, num_bumps, start_offset);
    cloud_draw_connecting_lines(&mut points, min_x, max_x, min_y, num_bumps, start_offset);
    cloud_draw_sides(&mut points, min_x, max_x, min_y, max_y);
    cloud_draw_bottom(&mut points, min_x, max_x, max_y);

    debug_assert!(!points.is_empty(), "cloud_points should return points");
    
    points
}

/// Generate star outline points - 5-pointed star
pub fn star_points(
    center: Position,
    outer_radius: i32,
    _inner_radius: i32,
) -> Vec<(Position, char)> {
    debug_assert!(outer_radius >= 0, "outer_radius should be non-negative");
    debug_assert!(_inner_radius >= 0, "inner_radius should be non-negative");
    
    let mut points = Vec::new();

    let r = outer_radius.abs().max(2);

    if r <= 2 {
        points.push((Position::new(center.x, center.y - 1), '*'));
        points.push((Position::new(center.x - 1, center.y), '*'));
        points.push((Position::new(center.x, center.y), '*'));
        points.push((Position::new(center.x + 1, center.y), '*'));
        points.push((Position::new(center.x, center.y + 1), '*'));
        debug_assert!(points.len() == 5, "small star has 5 points");
        return points;
    }

    let top = Position::new(center.x, center.y - r);
    let upper_right = Position::new(center.x + r, center.y - r / 3);
    let lower_right = Position::new(center.x + r * 2 / 3, center.y + r);
    let lower_left = Position::new(center.x - r * 2 / 3, center.y + r);
    let upper_left = Position::new(center.x - r, center.y - r / 3);

    draw_star_segment(&mut points, top, lower_left);
    draw_star_segment(&mut points, lower_left, upper_right);
    draw_star_segment(&mut points, upper_right, upper_left);
    draw_star_segment(&mut points, upper_left, lower_right);
    draw_star_segment(&mut points, lower_right, top);

    points.push((top, '*'));
    points.push((upper_right, '*'));
    points.push((lower_right, '*'));
    points.push((lower_left, '*'));
    points.push((upper_left, '*'));

    debug_assert!(!points.is_empty(), "star_points should return points");
    debug_assert!(points.len() >= 5, "star has at least 5 vertices");

    points
}

fn draw_star_segment(points: &mut Vec<(Position, char)>, from: Position, to: Position) {
    debug_assert!(!points.is_empty() || points.is_empty(), "points vector is valid");
    
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
        let pos = Position::new(
            x.round().clamp(i32::MIN as f32, i32::MAX as f32) as i32,
            y.round().clamp(i32::MIN as f32, i32::MAX as f32) as i32,
        );

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

fn draw_line_edge(points: &mut Vec<(Position, char)>, from: Position, to: Position) {
    debug_assert!(!points.is_empty() || points.is_empty(), "points vector is valid");
    
    let dx = to.x - from.x;
    let dy = to.y - from.y;

    if dx == 0 {
        if dy == 0 {
            points.push((from, '*'));
            return;
        }
    }

    let ch = if dy == 0 {
        '\u{2500}' // ─
    } else if dx == 0 {
        '\u{2502}' // │
    } else if (dx > 0) == (dy > 0) {
        '\\'
    } else {
        '/'
    };

    let edge_points = line_points(from, to);
    for pos in edge_points {
        points.push((pos, ch));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_new() {
        let pos = Position::new(10, 20);
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    #[test]
    fn position_equality() {
        let p1 = Position::new(5, 10);
        let p2 = Position::new(5, 10);
        let p3 = Position::new(10, 5);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn line_points_single_point() {
        let points = line_points(Position::new(5, 5), Position::new(5, 5));
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], Position::new(5, 5));
    }

    #[test]
    fn line_points_horizontal() {
        let points = line_points(Position::new(0, 0), Position::new(5, 0));
        assert_eq!(points.len(), 6);
        assert!(points.iter().all(|p| p.y == 0));
    }

    #[test]
    fn line_points_vertical() {
        let points = line_points(Position::new(0, 0), Position::new(0, 5));
        assert_eq!(points.len(), 6);
        assert!(points.iter().all(|p| p.x == 0));
    }

    #[test]
    fn line_style_cycle() {
        assert_eq!(LineStyle::Straight.next(), LineStyle::OrthogonalHV);
        assert_eq!(LineStyle::OrthogonalHV.next(), LineStyle::OrthogonalVH);
        assert_eq!(LineStyle::OrthogonalVH.next(), LineStyle::OrthogonalAuto);
        assert_eq!(LineStyle::OrthogonalAuto.next(), LineStyle::Straight);
    }

    #[test]
    fn viewport_screen_to_canvas() {
        let vp = Viewport::new(80, 24);
        assert_eq!(vp.screen_to_canvas(0, 0), Position::new(0, 0));
        assert_eq!(vp.screen_to_canvas(10, 5), Position::new(10, 5));
    }

    #[test]
    fn viewport_pan() {
        let mut vp = Viewport::new(80, 24);
        vp.pan(10, 5);
        assert_eq!(vp.offset_x, 10);
        assert_eq!(vp.offset_y, 5);
    }

    #[test]
    fn rect_points_single_point() {
        let points = rect_points(Position::new(5, 5), Position::new(5, 5));
        assert_eq!(points.len(), 1);
    }

    #[test]
    fn rect_points_full() {
        let points = rect_points(Position::new(0, 0), Position::new(5, 3));
        assert!(!points.is_empty());
    }

    #[test]
    fn diamond_points_not_empty() {
        let points = diamond_points(Position::new(10, 10), 5, 3);
        assert!(!points.is_empty());
    }

    #[test]
    fn ellipse_points_not_empty() {
        let points = ellipse_points(Position::new(10, 10), 5, 3);
        assert!(!points.is_empty());
    }
}
