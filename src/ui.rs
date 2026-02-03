use std::collections::HashMap;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{
    App, BRUSHES, COLORS, GRID_SIZE, KeyboardShapeField, MessageSeverity, Mode, PendingAction,
    PopupKind, SnapOrientation, TOOLS, Tool,
};
use crate::canvas::{
    Position, arrow_points_styled, cloud_points, cylinder_points, diamond_points,
    double_rect_points, ellipse_points, hexagon_points, line_points_styled, parallelogram_points,
    rect_points, rounded_rect_points, star_points, trapezoid_points, triangle_points,
};
use crate::document::ShapeId;
use crate::layers::LayerId;
use crate::presence::{CursorActivity, PeerPresence, ToolKind, peer_color};
use crate::shapes::ShapeKind;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Canvas area (+ optional panel)
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Help bar
        ])
        .split(frame.area());

    // Determine which panels to show
    let show_participants = app.show_participants && app.presence.is_some();
    let show_layers = app.show_layers;

    // Split top area horizontally if any panel is shown
    let canvas_area = if show_layers || show_participants {
        // Calculate panel widths
        let mut constraints = vec![Constraint::Min(20)]; // Canvas (at least 20 chars)

        if show_layers {
            constraints.push(Constraint::Length(18)); // Layer panel
        }
        if show_participants {
            constraints.push(Constraint::Length(24)); // Participant panel
        }

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(chunks[0]);

        let mut panel_idx = 1;

        // Render layer panel (first panel if shown)
        if show_layers {
            let layer_area = horizontal[panel_idx];
            app.layer_panel_area = Some(layer_area);
            render_layer_panel(frame, app, layer_area);
            panel_idx += 1;
        } else {
            app.layer_panel_area = None;
        }

        // Render participants panel (second panel if shown)
        if show_participants {
            render_participants_panel(frame, app, horizontal[panel_idx]);
        }

        horizontal[0]
    } else {
        app.layer_panel_area = None;
        chunks[0]
    };

    render_canvas(frame, app, canvas_area);
    render_active_layer_indicator(frame, app, canvas_area);
    render_status_bar(frame, app, chunks[1]);
    render_help_bar(frame, app, chunks[2]);

    // Render input overlay if in input mode
    match &app.mode {
        Mode::TextInput { start_pos, text } => {
            render_text_cursor(frame, app, start_pos, text, canvas_area);
        }
        Mode::LabelInput {
            shape_id,
            text,
            cursor,
        } => {
            render_label_input(frame, app, *shape_id, text, *cursor, canvas_area);
        }
        Mode::FileSave { path } => {
            render_file_input(frame, "Export ASCII to:", path, canvas_area);
        }
        Mode::FileOpen { path } => {
            render_file_input(frame, "Import ASCII from:", path, canvas_area);
        }
        Mode::DocSave { path } => {
            render_file_input(frame, "Save document to:", path, canvas_area);
        }
        Mode::DocOpen { path } => {
            render_file_input(frame, "Open document:", path, canvas_area);
        }
        Mode::SvgExport { path } => {
            render_file_input(frame, "Export SVG:", path, canvas_area);
        }
        Mode::RecentFiles { selected } => {
            render_recent_files_menu(frame, app, *selected, canvas_area);
        }
        Mode::SelectionPopup { kind, selected } => {
            render_selection_popup(frame, *kind, *selected, canvas_area);
        }
        Mode::ConfirmDialog { action } => {
            render_confirm_dialog(frame, action, canvas_area);
        }
        Mode::HelpScreen { scroll } => {
            render_help_screen(frame, *scroll, frame.area());
        }
        Mode::SessionBrowser {
            selected,
            filter,
            show_pinned_only,
        } => {
            render_session_browser(
                frame,
                app,
                *selected,
                filter,
                *show_pinned_only,
                canvas_area,
            );
        }
        Mode::SessionCreate { name } => {
            render_session_create(frame, name, canvas_area);
        }
        Mode::KeyboardShapeCreate {
            tool,
            width,
            height,
            focus,
        } => {
            render_keyboard_shape_create(frame, *tool, width, height, *focus, canvas_area);
        }
        Mode::Normal => {}
        Mode::LayerRename { .. } => {} // Handled in layer panel
    }
}

/// Render the canvas area
fn render_canvas(frame: &mut Frame, app: &App, area: Rect) {
    let canvas_widget = CanvasWidget { app };
    frame.render_widget(canvas_widget, area);
}

/// Render active layer indicator in the top-right of the canvas
fn render_active_layer_indicator(frame: &mut Frame, app: &App, area: Rect) {
    // Only show when not in layer panel view (would be redundant)
    if app.show_layers {
        return;
    }

    if let Some(layer_id) = app.active_layer {
        if let Ok(Some(layer)) = app.doc.read_layer(layer_id) {
            // Build indicator text
            let mut text = layer.name.clone();
            if text.len() > 12 {
                text = text.chars().take(10).collect::<String>() + "..";
            }

            // Add status icons
            let mut icons = String::new();
            if !layer.visible {
                icons.push_str(" [H]");
            }
            if layer.locked {
                icons.push_str(" [L]");
            }

            let full_text = format!(" {} {}", text, icons);
            let text_len = full_text.len() as u16;

            // Position in top-right corner
            if area.width > text_len + 2 && area.height > 0 {
                let x = area.x + area.width - text_len - 1;
                let y = area.y;

                // Choose color based on layer state
                let (fg, bg) = if layer.locked {
                    (Color::Black, Color::Yellow)
                } else if !layer.visible {
                    (Color::White, Color::DarkGray)
                } else {
                    (Color::Black, Color::Cyan)
                };

                let style = Style::default().fg(fg).bg(bg);

                for (i, ch) in full_text.chars().enumerate() {
                    let px = x + i as u16;
                    if px < area.x + area.width {
                        frame.buffer_mut()[(px, y)].set_char(ch).set_style(style);
                    }
                }
            }
        }
    }
}

/// Custom widget for rendering the canvas
struct CanvasWidget<'a> {
    app: &'a App,
}

impl CanvasWidget<'_> {
    fn render_char(&self, buf: &mut Buffer, area: Rect, pos: Position, ch: char, style: Style) {
        if let Some((sx, sy)) = self.app.viewport.canvas_to_screen(pos) {
            let x = area.x + sx;
            let y = area.y + sy;
            if x < area.x + area.width && y < area.y + area.height {
                buf[(x, y)].set_char(ch).set_style(style);
            }
        }
    }

    /// Render a text string at a canvas position
    fn render_text(&self, buf: &mut Buffer, area: Rect, pos: Position, text: &str, style: Style) {
        for (i, ch) in text.chars().enumerate() {
            let char_pos = Position::new(pos.x + i as i32, pos.y);
            self.render_char(buf, area, char_pos, ch, style);
        }
    }

    /// Render a dashed rectangle outline (for ghost shapes during remote drag/resize)
    fn render_dashed_rect(
        &self,
        buf: &mut Buffer,
        area: Rect,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
        style: Style,
    ) {
        // Top edge (dashed horizontal line)
        for x in min_x..=max_x {
            let ch = if (x - min_x) % 2 == 0 { '╌' } else { ' ' };
            if ch != ' ' {
                self.render_char(buf, area, Position::new(x, min_y), ch, style);
            }
        }
        // Bottom edge (dashed horizontal line)
        for x in min_x..=max_x {
            let ch = if (x - min_x) % 2 == 0 { '╌' } else { ' ' };
            if ch != ' ' {
                self.render_char(buf, area, Position::new(x, max_y), ch, style);
            }
        }
        // Left edge (dashed vertical line)
        for y in min_y..=max_y {
            let ch = if (y - min_y) % 2 == 0 { '╎' } else { ' ' };
            if ch != ' ' {
                self.render_char(buf, area, Position::new(min_x, y), ch, style);
            }
        }
        // Right edge (dashed vertical line)
        for y in min_y..=max_y {
            let ch = if (y - min_y) % 2 == 0 { '╎' } else { ' ' };
            if ch != ' ' {
                self.render_char(buf, area, Position::new(max_x, y), ch, style);
            }
        }
        // Corners
        self.render_char(buf, area, Position::new(min_x, min_y), '┌', style);
        self.render_char(buf, area, Position::new(max_x, min_y), '┐', style);
        self.render_char(buf, area, Position::new(min_x, max_y), '└', style);
        self.render_char(buf, area, Position::new(max_x, max_y), '┘', style);
    }

    fn render_grid(&self, buf: &mut Buffer, area: Rect) {
        if !self.app.grid_enabled {
            return;
        }

        let grid_style = Style::default().fg(Color::DarkGray);

        for screen_y in 0..area.height {
            let canvas_y = self.app.viewport.offset_y + screen_y as i32;
            if canvas_y % GRID_SIZE == 0 {
                for screen_x in 0..area.width {
                    let canvas_x = self.app.viewport.offset_x + screen_x as i32;
                    if canvas_x % GRID_SIZE == 0 {
                        let x = area.x + screen_x;
                        let y = area.y + screen_y;
                        if x < area.x + area.width && y < area.y + area.height {
                            buf[(x, y)].set_char('·').set_style(grid_style);
                        }
                    }
                }
            }
        }
    }

    fn render_snap_guides(&self, buf: &mut Buffer, area: Rect) {
        let guide_style = Style::default().fg(Color::Magenta);

        for guide in &self.app.shape_snap_guides {
            match guide.orientation {
                SnapOrientation::Vertical => {
                    // Draw vertical line at x = guide.position
                    let screen_x = guide.position - self.app.viewport.offset_x;
                    if screen_x < 0 || screen_x >= area.width as i32 {
                        continue;
                    }
                    let x = area.x + screen_x as u16;

                    for canvas_y in guide.start..=guide.end {
                        let screen_y = canvas_y - self.app.viewport.offset_y;
                        if screen_y >= 0 && screen_y < area.height as i32 {
                            let y = area.y + screen_y as u16;
                            buf[(x, y)].set_char('│').set_style(guide_style);
                        }
                    }
                }
                SnapOrientation::Horizontal => {
                    // Draw horizontal line at y = guide.position
                    let screen_y = guide.position - self.app.viewport.offset_y;
                    if screen_y < 0 || screen_y >= area.height as i32 {
                        continue;
                    }
                    let y = area.y + screen_y as u16;

                    for canvas_x in guide.start..=guide.end {
                        let screen_x = canvas_x - self.app.viewport.offset_x;
                        if screen_x >= 0 && screen_x < area.width as i32 {
                            let x = area.x + screen_x as u16;
                            buf[(x, y)].set_char('─').set_style(guide_style);
                        }
                    }
                }
            }
        }
    }

    fn render_label(
        &self,
        buf: &mut Buffer,
        area: Rect,
        bounds: (i32, i32, i32, i32),
        text: &str,
        style: Style,
    ) {
        let (min_x, min_y, max_x, max_y) = bounds;
        // Center the text horizontally and vertically within the shape
        let center_y = (min_y + max_y) / 2;
        let shape_width = (max_x - min_x + 1) as usize;
        let text_len = text.chars().count();

        // Calculate starting x position to center text (inside the shape border)
        let inner_width = shape_width.saturating_sub(2); // Account for borders
        let start_offset = if text_len < inner_width {
            ((inner_width - text_len) / 2) as i32 + 1 // +1 for left border
        } else {
            1 // Start just inside left border
        };

        let start_x = min_x + start_offset;

        for (i, ch) in text.chars().enumerate() {
            let x = start_x + i as i32;
            // Don't render beyond the right border
            if x >= max_x {
                break;
            }
            self.render_char(buf, area, Position::new(x, center_y), ch, style);
        }
    }
}

impl Widget for CanvasWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let selected_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let preview_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let freehand_preview_style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        // Render grid dots (background layer)
        self.render_grid(buf, area);

        // Render snap guide lines
        self.render_snap_guides(buf, area);

        // Render all shapes
        for shape in self.app.shape_view.iter_visible() {
            let is_selected = self.app.selected.contains(&shape.id);
            // Use shape's color, but override with cyan when selected
            let style = if is_selected {
                selected_style
            } else {
                Style::default().fg(shape.kind.color().to_ratatui())
            };

            match &shape.kind {
                ShapeKind::Line {
                    start,
                    end,
                    style: line_style,
                    label,
                    ..
                } => {
                    for (pos, ch) in line_points_styled(*start, *end, *line_style) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Arrow {
                    start,
                    end,
                    style: line_style,
                    label,
                    ..
                } => {
                    for (pos, ch) in arrow_points_styled(*start, *end, *line_style) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Rectangle {
                    start, end, label, ..
                } => {
                    for (pos, ch) in rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::DoubleBox {
                    start, end, label, ..
                } => {
                    for (pos, ch) in double_rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Diamond {
                    center,
                    half_width,
                    half_height,
                    label,
                    ..
                } => {
                    for (pos, ch) in diamond_points(*center, *half_width, *half_height) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Ellipse {
                    center,
                    radius_x,
                    radius_y,
                    label,
                    ..
                } => {
                    for (pos, ch) in ellipse_points(*center, *radius_x, *radius_y) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Freehand {
                    points,
                    char,
                    label,
                    ..
                } => {
                    for &pos in points {
                        self.render_char(buf, area, pos, *char, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Text { pos, content, .. } => {
                    for (i, ch) in content.chars().enumerate() {
                        let char_pos = Position::new(pos.x + i as i32, pos.y);
                        self.render_char(buf, area, char_pos, ch, style);
                    }
                }
                ShapeKind::Triangle {
                    p1, p2, p3, label, ..
                } => {
                    for (pos, ch) in triangle_points(*p1, *p2, *p3) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Parallelogram {
                    start, end, label, ..
                } => {
                    for (pos, ch) in parallelogram_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Hexagon {
                    center,
                    radius_x,
                    radius_y,
                    label,
                    ..
                } => {
                    for (pos, ch) in hexagon_points(*center, *radius_x, *radius_y) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Trapezoid {
                    start, end, label, ..
                } => {
                    for (pos, ch) in trapezoid_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::RoundedRect {
                    start, end, label, ..
                } => {
                    for (pos, ch) in rounded_rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Cylinder {
                    start, end, label, ..
                } => {
                    for (pos, ch) in cylinder_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Cloud {
                    start, end, label, ..
                } => {
                    for (pos, ch) in cloud_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Star {
                    center,
                    outer_radius,
                    inner_radius,
                    label,
                    ..
                } => {
                    for (pos, ch) in star_points(*center, *outer_radius, *inner_radius) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
            }
        }

        // Render freehand preview while drawing
        if let Some(ref freehand_state) = self.app.freehand_state {
            for &pos in &freehand_state.points {
                self.render_char(buf, area, pos, self.app.brush_char, freehand_preview_style);
            }
        }

        // Show snap points when line/arrow tool is active (even before drawing)
        if self.app.current_tool == Tool::Line || self.app.current_tool == Tool::Arrow {
            let snap_style = Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::DIM);

            for snap in self.app.shape_view.all_snap_points() {
                self.render_char(buf, area, snap.pos, '◆', snap_style);
            }
        }

        // Render shape preview (line/rectangle/etc) while drawing
        if let Some(ref shape_state) = self.app.shape_state {
            // Use snapped positions if available
            let start = shape_state.start_snap.unwrap_or(shape_state.start);
            let end = shape_state.current_snap.unwrap_or(shape_state.current);

            match self.app.current_tool {
                Tool::Line => {
                    for (pos, ch) in line_points_styled(start, end, self.app.line_style) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Arrow => {
                    for (pos, ch) in arrow_points_styled(start, end, self.app.line_style) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Rectangle => {
                    for (pos, ch) in rect_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::DoubleBox => {
                    for (pos, ch) in double_rect_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Diamond => {
                    let half_width = (end.x - start.x).abs().max(1);
                    let half_height = (end.y - start.y).abs().max(1);
                    for (pos, ch) in diamond_points(start, half_width, half_height) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Ellipse => {
                    let radius_x = (end.x - start.x).abs().max(1);
                    let radius_y = (end.y - start.y).abs().max(1);
                    for (pos, ch) in ellipse_points(start, radius_x, radius_y) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Triangle => {
                    let mid_x = (start.x + end.x) / 2;
                    let height = (end.y - start.y).abs().max(1);
                    let p3 = Position::new(mid_x, start.y + height);
                    for (pos, ch) in triangle_points(start, end, p3) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Parallelogram => {
                    for (pos, ch) in parallelogram_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Hexagon => {
                    let radius_x = (end.x - start.x).abs().max(2);
                    let radius_y = (end.y - start.y).abs().max(1);
                    for (pos, ch) in hexagon_points(start, radius_x, radius_y) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Trapezoid => {
                    for (pos, ch) in trapezoid_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::RoundedRect => {
                    for (pos, ch) in rounded_rect_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Cylinder => {
                    for (pos, ch) in cylinder_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Cloud => {
                    for (pos, ch) in cloud_points(start, end) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                Tool::Star => {
                    let outer_radius = (end.x - start.x).abs().max((end.y - start.y).abs()).max(2);
                    let inner_radius = outer_radius / 2;
                    for (pos, ch) in star_points(start, outer_radius, inner_radius) {
                        self.render_char(buf, area, pos, ch, preview_style);
                    }
                }
                _ => {}
            }
        }

        // Render active snap indicator (when hovering near a snap point)
        if let Some(ref snap) = self.app.hover_snap {
            let snap_active_style = Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD);
            self.render_char(buf, area, snap.pos, '◉', snap_active_style);
        } else if let Some(grid_pos) = self.app.hover_grid_snap {
            // Render grid snap indicator (same style as shape snap)
            let snap_active_style = Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD);
            self.render_char(buf, area, grid_pos, '◉', snap_active_style);
        }

        // Render marquee selection box while dragging
        if let Some(ref marquee) = self.app.marquee_state {
            let marquee_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM);
            let min_x = marquee.start.x.min(marquee.current.x);
            let max_x = marquee.start.x.max(marquee.current.x);
            let min_y = marquee.start.y.min(marquee.current.y);
            let max_y = marquee.start.y.max(marquee.current.y);

            // Draw marquee border
            for x in min_x..=max_x {
                self.render_char(buf, area, Position::new(x, min_y), '-', marquee_style);
                self.render_char(buf, area, Position::new(x, max_y), '-', marquee_style);
            }
            for y in (min_y + 1)..max_y {
                self.render_char(buf, area, Position::new(min_x, y), '|', marquee_style);
                self.render_char(buf, area, Position::new(max_x, y), '|', marquee_style);
            }
        }

        // Render selection bounding box for all selected shapes
        let box_style = Style::default().fg(Color::Cyan);
        let single_selection = self.app.selected.len() == 1;

        for &id in &self.app.selected {
            if let Some(shape) = self.app.shape_view.get(id) {
                let (min_x, min_y, max_x, max_y) = shape.bounds();

                // Draw corners of selection box (offset from shape)
                for (x, y, ch) in [
                    (min_x - 1, min_y - 1, '┌'),
                    (max_x + 1, min_y - 1, '┐'),
                    (min_x - 1, max_y + 1, '└'),
                    (max_x + 1, max_y + 1, '┘'),
                ] {
                    self.render_char(buf, area, Position::new(x, y), ch, box_style);
                }

                // Draw resize handles only for single selection
                if single_selection {
                    let handle_style = Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD);

                    for handle_info in shape.resize_handles() {
                        self.render_char(buf, area, handle_info.pos, '■', handle_style);
                    }
                }
            }
        }

        // Render remote cursors (on top of everything)
        if let Some(ref presence_mgr) = self.app.presence {
            for peer in presence_mgr.active_peers() {
                self.render_remote_cursor(buf, area, peer);
            }
        }
    }
}

impl CanvasWidget<'_> {
    /// Render a remote peer's cursor and activity
    fn render_remote_cursor(&self, buf: &mut Buffer, area: Rect, peer: &PeerPresence) {
        let color = peer_color(peer);
        let cursor_style = Style::default().fg(color).add_modifier(Modifier::BOLD);

        // Render cursor marker
        self.render_char(buf, area, peer.cursor_pos, '█', cursor_style);

        // Render activity indicator based on what they're doing
        match &peer.activity {
            CursorActivity::Drawing {
                tool,
                start,
                current,
            } => {
                // Show ghost preview of shape being drawn
                let preview_style = Style::default().fg(color).add_modifier(Modifier::DIM);
                self.render_shape_preview(buf, area, *tool, *start, *current, preview_style);
            }
            CursorActivity::Selected { shape_id } => {
                // Highlight the shape they have selected
                if let Some(shape) = self.app.shape_view.get(*shape_id) {
                    let (min_x, min_y, max_x, max_y) = shape.bounds();
                    let highlight_style = Style::default().fg(color);
                    // Draw colored corners to show remote selection
                    for (x, y, ch) in [
                        (min_x - 1, min_y - 1, '╭'),
                        (max_x + 1, min_y - 1, '╮'),
                        (min_x - 1, max_y + 1, '╰'),
                        (max_x + 1, max_y + 1, '╯'),
                    ] {
                        self.render_char(buf, area, Position::new(x, y), ch, highlight_style);
                    }
                }
            }
            CursorActivity::Dragging { shape_id, delta } => {
                // Render ghost shape at the dragged position
                if let Some(shape) = self.app.shape_view.get(*shape_id) {
                    let (orig_min_x, orig_min_y, orig_max_x, orig_max_y) = shape.bounds();
                    let (dx, dy) = *delta;
                    // Ghost bounds at new position
                    let ghost_min_x = orig_min_x + dx;
                    let ghost_min_y = orig_min_y + dy;
                    let ghost_max_x = orig_max_x + dx;
                    let ghost_max_y = orig_max_y + dy;
                    let ghost_style = Style::default().fg(color);
                    // Draw dashed outline for the ghost shape
                    self.render_dashed_rect(
                        buf,
                        area,
                        ghost_min_x,
                        ghost_min_y,
                        ghost_max_x,
                        ghost_max_y,
                        ghost_style,
                    );
                    // Show peer label above ghost
                    let label = format!("{} moving", peer.display_name());
                    let label_pos = Position::new(ghost_min_x, ghost_min_y.saturating_sub(1));
                    self.render_text(buf, area, label_pos, &label, ghost_style);
                }
            }
            CursorActivity::Resizing {
                shape_id,
                preview_bounds,
            } => {
                // Render ghost shape at the resized bounds
                if let Some(bounds) = preview_bounds {
                    let (min_pos, max_pos) = bounds;
                    let ghost_style = Style::default().fg(color);
                    // Draw dashed outline for the ghost shape
                    self.render_dashed_rect(
                        buf,
                        area,
                        min_pos.x,
                        min_pos.y,
                        max_pos.x,
                        max_pos.y,
                        ghost_style,
                    );
                    // Show peer label above ghost
                    let label = format!("{} resizing", peer.display_name());
                    let label_pos = Position::new(min_pos.x, min_pos.y.saturating_sub(1));
                    self.render_text(buf, area, label_pos, &label, ghost_style);
                } else if let Some(shape) = self.app.shape_view.get(*shape_id) {
                    // Fallback: show original shape with highlight
                    let (min_x, min_y, max_x, max_y) = shape.bounds();
                    let highlight_style = Style::default().fg(color);
                    for (x, y, ch) in [
                        (min_x - 1, min_y - 1, '╭'),
                        (max_x + 1, min_y - 1, '╮'),
                        (min_x - 1, max_y + 1, '╰'),
                        (max_x + 1, max_y + 1, '╯'),
                    ] {
                        self.render_char(buf, area, Position::new(x, y), ch, highlight_style);
                    }
                }
            }
            CursorActivity::Typing { position } => {
                // Show typing indicator
                let typing_style = Style::default()
                    .fg(color)
                    .add_modifier(Modifier::SLOW_BLINK);
                self.render_char(buf, area, *position, '▌', typing_style);
            }
            CursorActivity::Idle => {
                // Just the cursor marker (already rendered above)
            }
        }
    }

    /// Render a preview shape for remote drawing activity
    fn render_shape_preview(
        &self,
        buf: &mut Buffer,
        area: Rect,
        tool: ToolKind,
        start: Position,
        current: Position,
        style: Style,
    ) {
        use crate::canvas::LineStyle;

        match tool {
            ToolKind::Line => {
                for (pos, ch) in line_points_styled(start, current, LineStyle::Straight) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Arrow => {
                for (pos, ch) in arrow_points_styled(start, current, LineStyle::Straight) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Rectangle => {
                for (pos, ch) in rect_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::DoubleBox => {
                for (pos, ch) in double_rect_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Diamond => {
                let half_width = (current.x - start.x).abs().max(1);
                let half_height = (current.y - start.y).abs().max(1);
                for (pos, ch) in diamond_points(start, half_width, half_height) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Ellipse => {
                let radius_x = (current.x - start.x).abs().max(1);
                let radius_y = (current.y - start.y).abs().max(1);
                for (pos, ch) in ellipse_points(start, radius_x, radius_y) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Triangle => {
                let mid_x = (start.x + current.x) / 2;
                let height = (current.y - start.y).abs().max(1);
                let p3 = Position::new(mid_x, start.y + height);
                for (pos, ch) in triangle_points(start, current, p3) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Parallelogram => {
                for (pos, ch) in parallelogram_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Hexagon => {
                let radius_x = (current.x - start.x).abs().max(2);
                let radius_y = (current.y - start.y).abs().max(1);
                for (pos, ch) in hexagon_points(start, radius_x, radius_y) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Trapezoid => {
                for (pos, ch) in trapezoid_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::RoundedRect => {
                for (pos, ch) in rounded_rect_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Cylinder => {
                for (pos, ch) in cylinder_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Cloud => {
                for (pos, ch) in cloud_points(start, current) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            ToolKind::Star => {
                let outer_radius = (current.x - start.x)
                    .abs()
                    .max((current.y - start.y).abs())
                    .max(2);
                let inner_radius = outer_radius / 2;
                for (pos, ch) in star_points(start, outer_radius, inner_radius) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            _ => {}
        }
    }
}

/// Render the participants panel
fn render_participants_panel(frame: &mut Frame, app: &App, area: Rect) {
    let Some(ref presence_mgr) = app.presence else {
        return;
    };

    let peer_count = presence_mgr.peer_count() + 1; // +1 for self
    let title = format!(" Participants ({}) ", peer_count);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Build list items
    let mut items: Vec<Line> = Vec::new();

    // Get local user's active layer name
    let local_layer_name = app
        .active_layer
        .and_then(|id| app.doc.read_layer(id).ok().flatten())
        .map(|l| truncate_name(&l.name, 8))
        .unwrap_or_else(|| "?".to_string());

    // Add "You" entry first (local user)
    items.push(Line::from(vec![
        Span::styled("● ", Style::default().fg(Color::White)),
        Span::raw("You"),
        Span::styled(
            format!(" [{}]", local_layer_name),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Add remote peers
    for peer in presence_mgr.active_peers() {
        let color = peer_color(peer);
        let activity = peer.activity.label();

        // Get peer's active layer name
        let layer_name = peer
            .active_layer_id
            .and_then(|id| app.doc.read_layer(id).ok().flatten())
            .map(|l| truncate_name(&l.name, 8))
            .unwrap_or_else(|| "?".to_string());

        items.push(Line::from(vec![
            Span::styled("█ ", Style::default().fg(color)),
            Span::raw(peer.display_name()),
            Span::styled(
                format!(" ({})", activity),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!(" [{}]", layer_name),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    let paragraph = Paragraph::new(items)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Truncate a name for display
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() > max_len {
        name.chars().take(max_len - 2).collect::<String>() + ".."
    } else {
        name.to_string()
    }
}

/// Render the layer panel
fn render_layer_panel(frame: &mut Frame, app: &App, area: Rect) {
    let layers = app.get_layers();
    let title = format!(" Layers ({}) ", layers.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Check if we're renaming a layer
    let renaming = if let Mode::LayerRename { layer_id, text } = &app.mode {
        Some((*layer_id, text.as_str()))
    } else {
        None
    };

    // Collect peers by their active layer
    let mut peers_by_layer: HashMap<LayerId, Vec<Color>> = HashMap::new();
    if let Some(ref presence_mgr) = app.presence {
        for peer in presence_mgr.active_peers() {
            if let Some(layer_id) = peer.active_layer_id {
                let color = peer_color(peer);
                peers_by_layer.entry(layer_id).or_default().push(color);
            }
        }
    }

    // Build list items
    let mut items: Vec<Line> = Vec::new();

    for layer in &layers {
        let is_active = app.active_layer == Some(layer.id);
        let is_renaming = renaming.map_or(false, |(id, _)| id == layer.id);

        // Active indicator
        let active_indicator = if is_active {
            Span::styled("● ", Style::default().fg(Color::Cyan))
        } else {
            Span::raw("  ")
        };

        // Layer name (truncate if needed) or rename input
        let max_name_len = 10;
        let (name_span, vis_span, lock_span) = if is_renaming {
            let text = renaming.unwrap().1;
            let display: String = text.chars().take(max_name_len).collect();
            let input_style = Style::default().fg(Color::Black).bg(Color::Yellow);
            (
                Span::styled(
                    format!(
                        "{:<width$}_",
                        display,
                        width = max_name_len.saturating_sub(1)
                    ),
                    input_style,
                ),
                Span::raw(" "),
                Span::raw(" "),
            )
        } else {
            let name: String = layer.name.chars().take(max_name_len).collect();
            let name_style = if is_active {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            // Visibility indicator - [H] for hidden is more accessible
            let visible_indicator = if layer.visible {
                Span::styled(" ", Style::default())
            } else {
                Span::styled("[H]", Style::default().fg(Color::DarkGray))
            };

            // Lock indicator - [L] for locked is more accessible
            let lock_indicator = if layer.locked {
                Span::styled("[L]", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("   ")
            };

            (
                Span::styled(
                    format!("{:<width$}", name, width = max_name_len),
                    name_style,
                ),
                visible_indicator,
                lock_indicator,
            )
        };

        // Peer indicators (colored blocks for peers on this layer)
        let peer_indicators: Vec<Span> = peers_by_layer
            .get(&layer.id)
            .map(|colors| {
                colors
                    .iter()
                    .take(3) // Max 3 indicators
                    .map(|color| Span::styled("█", Style::default().fg(*color)))
                    .collect()
            })
            .unwrap_or_default();

        let mut line_spans = vec![
            active_indicator,
            name_span,
            Span::raw(" "),
            vis_span,
            lock_span,
        ];
        line_spans.extend(peer_indicators);

        items.push(Line::from(line_spans));
    }

    let paragraph = Paragraph::new(items).block(block);

    frame.render_widget(paragraph, area);
}

/// Render the status bar (Helix-style with mode indicator)
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Determine mode name and color (Helix-style)
    let (mode_name, mode_bg) = match &app.mode {
        Mode::Normal => match app.current_tool {
            Tool::Select => ("SEL", Color::Blue),
            Tool::Text => ("TXT", Color::Green),
            _ => ("DRAW", Color::Yellow),
        },
        Mode::TextInput { .. } | Mode::LabelInput { .. } | Mode::LayerRename { .. } => {
            ("INS", Color::Green)
        }
        Mode::FileSave { .. }
        | Mode::FileOpen { .. }
        | Mode::DocSave { .. }
        | Mode::DocOpen { .. }
        | Mode::SvgExport { .. } => ("CMD", Color::Magenta),
        Mode::RecentFiles { .. } | Mode::SelectionPopup { .. } => ("MENU", Color::Cyan),
        Mode::ConfirmDialog { .. } => ("CONF", Color::Yellow),
        Mode::HelpScreen { .. } => ("HELP", Color::Cyan),
        Mode::SessionBrowser { .. } | Mode::SessionCreate { .. } => ("SESS", Color::Magenta),
        Mode::KeyboardShapeCreate { .. } => ("CREATE", Color::Cyan),
    };

    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(mode_bg)
        .add_modifier(Modifier::BOLD);

    let tool_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let file_name = app
        .file_path
        .as_ref()
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "[unsaved]".to_string());

    let dirty_marker = if app.is_dirty() { " *" } else { "" };

    let char_info = match app.current_tool {
        Tool::Freehand => format!(" brush:'{}' {}", app.brush_char, app.current_color.name()),
        Tool::Line | Tool::Arrow => {
            format!(" {} {}", app.line_style.name(), app.current_color.name())
        }
        Tool::Rectangle
        | Tool::DoubleBox
        | Tool::Diamond
        | Tool::Ellipse
        | Tool::Text
        | Tool::Triangle
        | Tool::Parallelogram
        | Tool::Hexagon
        | Tool::Trapezoid
        | Tool::RoundedRect
        | Tool::Cylinder
        | Tool::Cloud
        | Tool::Star => {
            format!(" {}", app.current_color.name())
        }
        _ => String::new(),
    };

    // Sync status indicator - show when in sync mode
    let (peer_info, peer_style) = if let Some(ref presence) = app.presence {
        let count = presence.peer_count();
        if count > 0 {
            (
                format!(" [SYNC {}]", count),
                Style::default().fg(Color::Green),
            )
        } else {
            (String::from(" [SYNC]"), Style::default().fg(Color::Yellow))
        }
    } else {
        (String::new(), Style::default())
    };

    // Format status message with appropriate styling
    let (status_text, status_color) = match &app.status_message {
        Some((msg, MessageSeverity::Info)) => (format!(" {}", msg), Color::White),
        Some((msg, MessageSeverity::Warning)) => (format!(" ⚠ {}", msg), Color::Yellow),
        Some((msg, MessageSeverity::Error)) => (format!(" ✗ {}", msg), Color::Red),
        None => (String::new(), Color::White),
    };

    // Tool info (only show if not in Select mode)
    let tool_info = if app.current_tool != Tool::Select {
        format!(" {}", app.current_tool.name().to_lowercase())
    } else {
        String::new()
    };

    // Shape properties info (only show when single shape selected and no status message)
    let shape_info = if app.status_message.is_none() && app.current_tool == Tool::Select {
        app.get_selected_shape_info()
            .map(|info| format!(" | {}", info))
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Session name (right-aligned context)
    let session_name = app
        .current_session_meta
        .as_ref()
        .map(|m| m.name.as_str())
        .unwrap_or("No session");

    // Shape/selection count
    let total_shapes = app.shape_view.shape_count();
    let selected_count = app.selected.len();
    let count_info = if selected_count > 0 {
        format!(" [{}/{}]", selected_count, total_shapes)
    } else if total_shapes > 0 {
        format!(" [{}]", total_shapes)
    } else {
        String::new()
    };

    let spans = vec![
        Span::styled(format!(" {} ", mode_name), mode_style),
        Span::styled(tool_info, tool_style),
        Span::raw(format!(" {}{}{}", file_name, dirty_marker, char_info)),
        Span::styled(count_info, Style::default().fg(Color::DarkGray)),
        Span::styled(peer_info, peer_style),
        Span::styled(shape_info, Style::default().fg(Color::Cyan)),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::styled(
            format!(" {} ", session_name),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render the help bar
fn render_help_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match &app.mode {
        Mode::Normal => {
            // Build context-sensitive help
            let base_help = match app.current_tool {
                Tool::Select => {
                    if app.selected.len() >= 2 {
                        // Multi-select: show group shortcuts
                        "[Shift+G] group [y]ank [Del] delete | [?] help"
                    } else if app.selected.len() == 1 {
                        // Single select: show shape operations
                        "[Enter] edit label [y]ank [Del] delete | []/{}:z-order [?] help"
                    } else {
                        // No selection
                        "[Space] tools [C] color | click to select | [?] help"
                    }
                }
                Tool::Freehand => "[Space] tools [c] brush [C] color | drag to draw | [?] help",
                Tool::Text => "[Space] tools [C] color | click to place text | [?] help",
                Tool::Line | Tool::Arrow => {
                    "[Space] tools [v] line style [C] color | drag to draw | [?] help"
                }
                _ => "[Space] tools [C] color | drag to draw | [?] help",
            };

            // Add layer hint if panel is visible
            if app.show_layers {
                if app.active_layer.is_some() {
                    "[F2] rename layer [Ctrl+D] delete | [Alt+1-9] switch layer"
                } else {
                    base_help
                }
            } else {
                base_help
            }
        }
        Mode::TextInput { .. } | Mode::LabelInput { .. } => {
            "type text | [Enter] confirm [Esc] cancel [Backspace] delete"
        }
        Mode::LayerRename { .. } => "type layer name | [Enter] confirm [Esc] cancel",
        Mode::FileSave { .. }
        | Mode::FileOpen { .. }
        | Mode::DocSave { .. }
        | Mode::DocOpen { .. }
        | Mode::SvgExport { .. } => "type path | [Tab] complete [Enter] confirm [Esc] cancel",
        Mode::RecentFiles { .. } => "[j/k] navigate [Enter] open [Esc] cancel",
        Mode::SelectionPopup { .. } => {
            "[hjkl] navigate | release key or [Enter] to select | [Esc] cancel"
        }
        Mode::ConfirmDialog { .. } => "[y] Yes [n] No | [Enter] confirm [Esc] cancel",
        Mode::HelpScreen { .. } => "[j/k] scroll [Space] page down [Esc/q/?] close",
        Mode::SessionBrowser { .. } => {
            "[j/k] navigate [n]ew [d]elete [p]in [*] pinned filter [Tab/Esc] close"
        }
        Mode::SessionCreate { .. } => "type session name | [Enter] create [Esc] cancel",
        Mode::KeyboardShapeCreate { .. } => {
            "[Tab] switch field | type dimensions | [Enter] create [Esc] cancel"
        }
    };

    let paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}

/// Render text input cursor on canvas
fn render_text_cursor(frame: &mut Frame, app: &App, start_pos: &Position, text: &str, area: Rect) {
    if let Some((screen_x, screen_y)) = app.viewport.canvas_to_screen(*start_pos) {
        let cursor_x = screen_x + text.len() as u16;
        if cursor_x < area.width && screen_y < area.height {
            // Render the text typed so far
            for (i, ch) in text.chars().enumerate() {
                let x = area.x + screen_x + i as u16;
                if x < area.x + area.width {
                    frame.buffer_mut()[(x, area.y + screen_y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::Yellow));
                }
            }

            // Show blinking cursor
            let cursor_screen_x = area.x + cursor_x;
            let cursor_screen_y = area.y + screen_y;
            if cursor_screen_x < area.x + area.width {
                frame.buffer_mut()[(cursor_screen_x, cursor_screen_y)]
                    .set_char('▏')
                    .set_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::SLOW_BLINK),
                    );
            }
        }
    }
}

/// Render label input inside a shape with cursor at specified position
fn render_label_input(
    frame: &mut Frame,
    app: &App,
    shape_id: ShapeId,
    text: &str,
    cursor: usize,
    area: Rect,
) {
    if let Some(shape) = app.shape_view.get(shape_id) {
        let (min_x, min_y, max_x, max_y) = shape.bounds();
        let center_y = (min_y + max_y) / 2;
        let shape_width = (max_x - min_x + 1) as usize;
        let text_len = text.chars().count();

        // Calculate starting x position (centered inside shape)
        let inner_width = shape_width.saturating_sub(2);
        let start_offset = if text_len < inner_width {
            ((inner_width - text_len) / 2) as i32 + 1
        } else {
            1
        };
        let start_x = min_x + start_offset;
        let label_pos = Position::new(start_x, center_y);

        if let Some((screen_x, screen_y)) = app.viewport.canvas_to_screen(label_pos) {
            let label_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);

            // Render the text typed so far
            for (i, ch) in text.chars().enumerate() {
                let x = area.x + screen_x + i as u16;
                let canvas_x = start_x + i as i32;
                if x < area.x + area.width && canvas_x < max_x {
                    // Highlight character at cursor position with underline
                    let style = if i == cursor {
                        label_style.add_modifier(Modifier::UNDERLINED)
                    } else {
                        label_style
                    };
                    frame.buffer_mut()[(x, area.y + screen_y)]
                        .set_char(ch)
                        .set_style(style);
                }
            }

            // Show blinking cursor at cursor position (not end of text)
            let cursor_x = screen_x + cursor as u16;
            let cursor_screen_x = area.x + cursor_x;
            let cursor_screen_y = area.y + screen_y;
            if cursor_screen_x < area.x + area.width && (start_x + cursor as i32) < max_x {
                // If cursor is within text, show cursor character; otherwise show bar
                if cursor >= text_len {
                    // Cursor at end - show bar cursor
                    frame.buffer_mut()[(cursor_screen_x, cursor_screen_y)]
                        .set_char('▏')
                        .set_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::SLOW_BLINK),
                        );
                }
                // If cursor is within text, the underline on the character shows position
            }
        }
    }
}

/// Render file path input overlay
fn render_file_input(frame: &mut Frame, label: &str, path: &str, area: Rect) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)].set_char(' ');
        }
    }

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(format!("{}▏", path))
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Render recent files menu overlay
fn render_recent_files_menu(frame: &mut Frame, app: &App, selected: usize, area: Rect) {
    let file_count = app.recent_files.len();
    let width = 50.min(area.width.saturating_sub(4));
    let height = (file_count as u16 + 2).min(12); // +2 for border, max 12 lines
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)].set_char(' ');
        }
    }

    let block = Block::default()
        .title(" Recent Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Build list items
    let items: Vec<Line> = app
        .recent_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if i == selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(format!(" {} ", file.name), style)
        })
        .collect();

    let paragraph = Paragraph::new(items)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Render selection popup for tools, colors, or brushes
fn render_selection_popup(frame: &mut Frame, kind: PopupKind, selected: usize, area: Rect) {
    let (title, cols, items, hint): (&str, usize, Vec<(String, Option<Color>)>, &str) = match kind {
        PopupKind::Tool => {
            let items: Vec<_> = TOOLS.iter().map(|t| (t.name().to_string(), None)).collect();
            (" Select Tool ", 3, items, "hjkl: move, release key: select")
        }
        PopupKind::Color => {
            let items: Vec<_> = COLORS
                .iter()
                .map(|c| (c.name().to_string(), Some(c.to_ratatui())))
                .collect();
            (
                " Select Color ",
                4,
                items,
                "hjkl: move, release key: select",
            )
        }
        PopupKind::Brush => {
            let items: Vec<_> = BRUSHES.iter().map(|&ch| (ch.to_string(), None)).collect();
            (
                " Select Brush ",
                6,
                items,
                "hjkl: move, release key: select",
            )
        }
    };

    let rows = (items.len() + cols - 1) / cols;

    // Calculate popup size based on grid
    let cell_width: u16 = match kind {
        PopupKind::Tool => 10, // Tool names are longer
        PopupKind::Color => 8, // Color names + colored block
        PopupKind::Brush => 3, // Just the character
    };
    let width = (cols as u16 * cell_width + 2).min(area.width.saturating_sub(4));
    let height = (rows as u16 + 3).min(area.height.saturating_sub(4)); // +3 for borders and hint
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    // Draw border with hint at bottom
    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(Span::styled(
            format!(" {} ", hint),
            Style::default().fg(Color::DarkGray),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block, popup_area);

    // Render grid items
    let inner_x = popup_area.x + 1;
    let inner_y = popup_area.y + 1;
    let inner_width = popup_area.width.saturating_sub(2);

    for (idx, (label, color_opt)) in items.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let item_x = inner_x + (col as u16 * cell_width);
        let item_y = inner_y + row as u16;

        if item_y >= popup_area.y + popup_area.height - 1 {
            break;
        }
        if item_x >= inner_x + inner_width {
            continue;
        }

        let is_selected = idx == selected;
        let base_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };

        match kind {
            PopupKind::Tool => {
                // Render tool name
                let display = format!("{:^width$}", label, width = (cell_width - 1) as usize);
                for (i, ch) in display.chars().take((cell_width - 1) as usize).enumerate() {
                    let px = item_x + i as u16;
                    if px < inner_x + inner_width {
                        frame.buffer_mut()[(px, item_y)]
                            .set_char(ch)
                            .set_style(base_style);
                    }
                }
            }
            PopupKind::Color => {
                // Render colored block + abbreviated name
                if let Some(color) = color_opt {
                    let color_style = if is_selected {
                        Style::default().fg(*color).bg(Color::Cyan)
                    } else {
                        Style::default().fg(*color).bg(Color::Black)
                    };
                    frame.buffer_mut()[(item_x, item_y)]
                        .set_char('█')
                        .set_style(color_style);
                    frame.buffer_mut()[(item_x + 1, item_y)]
                        .set_char('█')
                        .set_style(color_style);
                }
                // Render abbreviated name (first 4 chars)
                let abbrev: String = label.chars().take(4).collect();
                for (i, ch) in abbrev.chars().enumerate() {
                    let px = item_x + 2 + i as u16;
                    if px < inner_x + inner_width {
                        frame.buffer_mut()[(px, item_y)]
                            .set_char(ch)
                            .set_style(base_style);
                    }
                }
            }
            PopupKind::Brush => {
                // Render just the brush character, centered
                let ch = label.chars().next().unwrap_or(' ');
                frame.buffer_mut()[(item_x + 1, item_y)]
                    .set_char(ch)
                    .set_style(base_style);
            }
        }
    }
}

/// Render confirmation dialog overlay
fn render_confirm_dialog(frame: &mut Frame, action: &PendingAction, area: Rect) {
    let title = action.title();
    let message = action.message();

    let width = 50.min(area.width.saturating_sub(4));
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    // Build dialog content
    let content = vec![
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[y]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Yes  "),
            Span::styled(
                "[n]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" No"),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().bg(Color::Black))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}

/// Render the help screen overlay
fn render_help_screen(frame: &mut Frame, scroll: usize, area: Rect) {
    // Help content with all keyboard shortcuts
    let help_sections = vec![
        (
            "GENERAL",
            vec![
                ("q", "Quit"),
                ("Ctrl+c", "Quit"),
                ("Esc", "Cancel/deselect"),
                ("?/F1", "Toggle help screen"),
                ("u", "Undo"),
                ("U", "Redo"),
            ],
        ),
        (
            "TOOLS",
            vec![
                ("Space", "Open tool popup"),
                ("s", "Select tool"),
                ("f", "Freehand draw"),
                ("t", "Text tool"),
                ("l", "Line tool"),
                ("a", "Arrow tool"),
                ("r", "Rectangle tool"),
                ("b", "Double box tool"),
                ("d", "Diamond tool"),
                ("e", "Ellipse tool"),
            ],
        ),
        (
            "DRAWING",
            vec![
                ("c", "Open brush popup"),
                ("C", "Open color popup"),
                ("v", "Cycle line style"),
                ("g", "Toggle grid snap"),
            ],
        ),
        (
            "KEYBOARD CREATE",
            vec![
                ("Ctrl+Shift+R", "Create rectangle"),
                ("Ctrl+Shift+L", "Create line"),
                ("Ctrl+Shift+B", "Create double box"),
                ("Alt+d", "Create diamond"),
                ("Alt+e", "Create ellipse"),
                ("Alt+a", "Create arrow"),
            ],
        ),
        (
            "SELECTION",
            vec![
                ("Click", "Select shape"),
                ("Shift+Click", "Toggle selection"),
                ("Drag", "Marquee select"),
                ("y", "Yank (copy)"),
                ("p", "Paste"),
                ("Del/Backspace", "Delete selected"),
                ("Enter", "Edit label"),
            ],
        ),
        (
            "Z-ORDER",
            vec![
                ("]", "Bring forward"),
                ("[", "Send backward"),
                ("}", "Bring to front"),
                ("{", "Send to back"),
            ],
        ),
        (
            "TRANSFORM",
            vec![
                ("Alt+L", "Align left"),
                ("Alt+R", "Align right"),
                ("Alt+T", "Align top"),
                ("Alt+B", "Align bottom"),
                ("Alt+C", "Center horizontal"),
                ("Alt+M", "Center vertical"),
                ("Alt+H", "Flip horizontal"),
                ("Alt+V", "Flip vertical"),
                ("Alt+.", "Rotate 90 CW"),
                ("Alt+,", "Rotate 90 CCW"),
            ],
        ),
        (
            "GROUPING",
            vec![
                ("Shift+G", "Group selected"),
                ("Ctrl+Shift+G", "Ungroup selected"),
            ],
        ),
        (
            "LAYERS",
            vec![
                ("L", "Toggle layer panel"),
                ("Ctrl+n", "New layer"),
                ("Ctrl+D", "Delete layer (confirm)"),
                ("F2", "Rename layer"),
                ("Alt+1-9", "Select layer 1-9"),
                ("Ctrl+m", "Move selection to layer"),
                ("Ctrl+h", "Toggle layer visibility"),
            ],
        ),
        (
            "FILES",
            vec![
                ("Ctrl+s", "Export ASCII (.txt)"),
                ("Ctrl+o", "Import ASCII"),
                ("Ctrl+Shift+S", "Save document (.automerge)"),
                ("Ctrl+Shift+O", "Open document"),
                ("E", "Export SVG"),
                ("N", "New document (confirm)"),
                ("R", "Recent files"),
            ],
        ),
        (
            "COLLABORATION",
            vec![("T", "Copy sync ticket"), ("P", "Toggle participants")],
        ),
        ("NAVIGATION", vec![("Arrow keys", "Pan viewport")]),
    ];

    // Build help lines
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "═══ KEYBOARD SHORTCUTS ═══",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (section_name, shortcuts) in &help_sections {
        lines.push(Line::from(Span::styled(
            format!("─── {} ───", section_name),
            Style::default().fg(Color::Yellow),
        )));
        for (key, desc) in shortcuts {
            lines.push(Line::from(vec![
                Span::styled(format!("{:>14}", key), Style::default().fg(Color::Green)),
                Span::raw("  "),
                Span::raw(*desc),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Calculate visible area
    let visible_height = area.height.saturating_sub(4) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    let actual_scroll = scroll.min(max_scroll);

    // Clear the screen area
    for py in area.y..area.y + area.height {
        for px in area.x..area.x + area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    let block = Block::default()
        .title(" Help (press Esc to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Show scroll indicator
    let scroll_info = if max_scroll > 0 {
        format!(" [{}/{}] ", actual_scroll + 1, max_scroll + 1)
    } else {
        String::new()
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(actual_scroll)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(block.title_bottom(scroll_info))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render session browser popup
fn render_session_browser(
    frame: &mut Frame,
    app: &App,
    selected: usize,
    filter: &str,
    show_pinned_only: bool,
    area: Rect,
) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let filtered = app.get_filtered_sessions(filter, show_pinned_only);
    let session_count = filtered.len();

    let width = 60.min(area.width.saturating_sub(4));
    let height = (session_count as u16 + 5).min(20); // +5 for borders, filter, hint
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    // Title with current session indicator
    let title = if let Some(ref meta) = app.current_session_meta {
        format!(" Sessions [{}] ", meta.name)
    } else {
        " Sessions ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Build session list
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut lines: Vec<Line> = Vec::new();

    // Filter line
    let filter_display = if filter.is_empty() {
        if show_pinned_only {
            "Filter: (pinned only)".to_string()
        } else {
            "Filter: (type to search)".to_string()
        }
    } else {
        format!("Filter: {}_", filter)
    };
    lines.push(Line::styled(
        format!(" {}", filter_display),
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw("")); // Separator

    // Session items
    for (i, session) in filtered.iter().enumerate() {
        let is_selected = i == selected;
        let is_current = app.current_session.as_ref() == Some(&session.id);

        // Format time ago
        let time_ago = {
            let diff = now.saturating_sub(session.last_accessed);
            if diff < 60 {
                "now".to_string()
            } else if diff < 3600 {
                format!("{}m", diff / 60)
            } else if diff < 86400 {
                format!("{}h", diff / 3600)
            } else {
                format!("{}d", diff / 86400)
            }
        };

        // Build line
        let pinned = if session.pinned { "*" } else { " " };
        let current = if is_current { ">" } else { " " };
        let name = if session.name.len() > 30 {
            format!("{}..", &session.name[..28])
        } else {
            session.name.clone()
        };

        let line_text = format!("{}{} {:<30} {:>6}", pinned, current, name, time_ago);

        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_current {
            Style::default().fg(Color::Green)
        } else if session.pinned {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::styled(format!(" {}", line_text), style));
    }

    if filtered.is_empty() {
        lines.push(Line::styled(
            " (no sessions match filter)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
    }

    // Hint at bottom
    lines.push(Line::raw("")); // Separator
    lines.push(Line::styled(
        " n:new d:del p:pin *:pinned Tab:close",
        Style::default().fg(Color::DarkGray),
    ));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Render session create dialog
fn render_session_create(frame: &mut Frame, name: &str, area: Rect) {
    let width = 40.min(area.width.saturating_sub(4));
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    let block = Block::default()
        .title(" New Session ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines = vec![
        Line::raw(""),
        Line::styled(
            format!(" Name: {}_ ", name),
            Style::default().fg(Color::White),
        ),
        Line::styled(
            " Enter:create Esc:cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Render keyboard shape creation dialog
fn render_keyboard_shape_create(
    frame: &mut Frame,
    tool: Tool,
    width: &str,
    height: &str,
    focus: KeyboardShapeField,
    area: Rect,
) {
    let popup_width = 35.min(area.width.saturating_sub(4));
    let popup_height = 7;
    let x = (area.width.saturating_sub(popup_width)) / 2 + area.x;
    let y = (area.height.saturating_sub(popup_height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the popup area
    for py in popup_area.y..popup_area.y + popup_area.height {
        for px in popup_area.x..popup_area.x + popup_area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Black));
        }
    }

    let title = format!(" Create {} ", tool.name());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Determine labels based on tool type
    let (label1, label2) = match tool {
        Tool::Line | Tool::Arrow => ("Length", "Y-Offset"),
        Tool::Star => ("Outer R", "Inner R"),
        _ => ("Width", "Height"),
    };

    let width_style = if focus == KeyboardShapeField::Width {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let height_style = if focus == KeyboardShapeField::Height {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let cursor = "_";
    let width_cursor = if focus == KeyboardShapeField::Width {
        cursor
    } else {
        ""
    };
    let height_cursor = if focus == KeyboardShapeField::Height {
        cursor
    } else {
        ""
    };

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!(" {}: ", label1), Style::default().fg(Color::Gray)),
            Span::styled(format!("{}{} ", width, width_cursor), width_style),
        ]),
        Line::from(vec![
            Span::styled(format!(" {}: ", label2), Style::default().fg(Color::Gray)),
            Span::styled(format!("{}{} ", height, height_cursor), height_style),
        ]),
        Line::raw(""),
        Line::styled(
            " Tab:switch Enter:create Esc:cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_name_short_string() {
        // String shorter than max_len should be returned as-is
        assert_eq!(truncate_name("Layer 1", 10), "Layer 1");
        assert_eq!(truncate_name("abc", 8), "abc");
    }

    #[test]
    fn truncate_name_exact_length() {
        // String exactly at max_len should be returned as-is
        assert_eq!(truncate_name("12345678", 8), "12345678");
    }

    #[test]
    fn truncate_name_long_string() {
        // String longer than max_len should be truncated with ".."
        assert_eq!(truncate_name("VeryLongLayerName", 8), "VeryLo..");
        assert_eq!(truncate_name("1234567890", 8), "123456..");
    }

    #[test]
    fn truncate_name_empty_string() {
        // Empty string should return empty
        assert_eq!(truncate_name("", 8), "");
    }

    #[test]
    fn truncate_name_unicode() {
        // Unicode characters should be counted correctly
        // Japanese characters are single graphemes
        assert_eq!(truncate_name("...", 8), "...");
        // Mixed ASCII and unicode
        assert_eq!(truncate_name("Layer...", 8), "Layer...");
    }

    #[test]
    fn truncate_name_very_small_max() {
        // Edge case: max_len of 3 means only 1 char + ".."
        assert_eq!(truncate_name("Hello", 3), "H..");
        // max_len of 2 means 0 chars + ".."
        assert_eq!(truncate_name("Hello", 2), "..");
    }

    #[test]
    fn truncate_name_whitespace() {
        // Names with whitespace
        assert_eq!(truncate_name("Layer With Spaces", 8), "Layer ..");
        assert_eq!(truncate_name("   ", 2), "..");
    }
}
