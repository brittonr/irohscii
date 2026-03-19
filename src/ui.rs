use std::collections::HashMap;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{
    App, BRUSHES, COLORS, GRID_SIZE, KeyboardShapeField, MessageSeverity, Mode, PendingAction,
    PopupKind, SnapOrientation, TOOLS, Tool,
};

// Import rat-widgets for the new UI components
use rat_widgets;
use crate::canvas::{
    Position, arrow_points_styled, cloud_points, cylinder_points, diamond_points,
    double_rect_points, ellipse_points, hexagon_points, line_points_styled, parallelogram_points,
    rect_points, rounded_rect_points, star_points, trapezoid_points, triangle_points,
};
use crate::document::ShapeId;
use crate::layers::LayerId;
use crate::presence::{CursorActivity, PeerPresence, ToolKind, peer_color};
use crate::shapes::ShapeKind;

// Compile-time assertions for UI constants
const _: () = assert!(GRID_SIZE > 0, "GRID_SIZE must be positive");
const _: () = assert!(MIN_CANVAS_WIDTH > 0, "MIN_CANVAS_WIDTH must be positive");
const MIN_CANVAS_WIDTH: u16 = 20;
const LAYER_PANEL_WIDTH: u16 = 18;
const PARTICIPANT_PANEL_WIDTH: u16 = 24;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &mut App) {
    debug_assert!(frame.area().width > 0, "Frame width must be positive");
    debug_assert!(frame.area().height > 0, "Frame height must be positive");
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Canvas area (+ optional panel)
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Help bar
        ])
        .split(frame.area());

    let canvas_area = render_main_area_with_panels(frame, app, chunks[0]);
    
    render_canvas(frame, app, canvas_area);
    render_active_layer_indicator(frame, app, canvas_area);
    render_status_bar(frame, app, chunks[1]);
    render_help_bar(frame, app, chunks[2]);

    render_mode_overlays(frame, app, canvas_area);
}

/// Render main area with optional side panels
fn render_main_area_with_panels(frame: &mut Frame, app: &mut App, area: Rect) -> Rect {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let show_participants = app.show_participants && app.presence.is_some();
    let show_layers = app.show_layers;

    if !show_layers && !show_participants {
        app.layer_panel_area = None;
        return area;
    }

    let mut constraints = vec![Constraint::Min(MIN_CANVAS_WIDTH)];
    if show_layers {
        constraints.push(Constraint::Length(LAYER_PANEL_WIDTH));
    }
    if show_participants {
        constraints.push(Constraint::Length(PARTICIPANT_PANEL_WIDTH));
    }

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let mut panel_idx = 1;

    if show_layers {
        let layer_area = horizontal[panel_idx];
        app.layer_panel_area = Some(layer_area);
        render_layer_panel(frame, app, layer_area);
        panel_idx += 1;
    } else {
        app.layer_panel_area = None;
    }

    if show_participants {
        render_participants_panel(frame, app, horizontal[panel_idx]);
    }

    horizontal[0]
}

/// Render mode-specific overlays
fn render_mode_overlays(frame: &mut Frame, app: &App, canvas_area: Rect) {
    debug_assert!(canvas_area.width > 0 && canvas_area.height > 0);
    
    match &app.mode {
        Mode::TextInput(state) => {
            render_text_cursor(frame, app, &state.start_pos, &state.text, canvas_area);
        }
        Mode::LabelInput(state) => {
            render_label_input(frame, app, state.shape_id, &state.text, state.cursor as usize, canvas_area);
        }
        Mode::PathInput(state) => {
            let prompt = state.kind.prompt();
            render_file_input(frame, prompt, &state.path, canvas_area);
        }
        Mode::RecentFiles(state) => {
            render_recent_files_menu(frame, app, state.selected as usize, canvas_area);
        }
        Mode::SelectionPopup(state) => {
            render_selection_popup(frame, state.kind, state.selected as usize, canvas_area);
        }
        Mode::ConfirmDialog(state) => {
            render_confirm_dialog(frame, &state.action, canvas_area);
        }
        Mode::HelpScreen(state) => {
            render_help_screen(frame, state.scroll as usize, frame.area());
        }
        Mode::SessionBrowser(state) => {
            render_session_browser(
                frame,
                app,
                state.selected as usize,
                &state.filter,
                state.show_pinned_only,
                canvas_area,
            );
        }
        Mode::SessionCreate(state) => {
            render_session_create(frame, &state.name, canvas_area);
        }
        Mode::KeyboardShapeCreate(state) => {
            render_keyboard_shape_create(frame, state.tool, &state.width, &state.height, state.focus, canvas_area);
        }
        Mode::QrCodeDisplay(state) => {
            render_qr_code_display(frame, &state.ticket, canvas_area);
        }
        Mode::Normal => {}
        Mode::LeaderMenu(_) => {
            app.leader_menu.render(frame, canvas_area);
        }
        Mode::LayerRename(_) => {} // Handled in layer panel
    }
}

/// Render the canvas area
fn render_canvas(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    let canvas_widget = CanvasWidget { app };
    frame.render_widget(canvas_widget, area);
}

/// Render active layer indicator in the top-right of the canvas
fn render_active_layer_indicator(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    // Only show when not in layer panel view (would be redundant)
    if app.show_layers {
        return;
    }

    if let Some(layer_id) = app.active_layer
        && let Ok(Some(layer)) = app.doc.read_layer(layer_id)
    {
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

/// Custom widget for rendering the canvas
struct CanvasWidget<'a> {
    app: &'a App,
}

impl CanvasWidget<'_> {
    fn render_char(&self, buf: &mut Buffer, area: Rect, pos: Position, ch: char, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
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
        debug_assert!(area.width > 0 && area.height > 0);
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
        min_pos: Position,
        max_pos: Position,
        style: Style,
    ) {
        debug_assert!(area.width > 0 && area.height > 0);
        debug_assert!(min_pos.x <= max_pos.x);
        debug_assert!(min_pos.y <= max_pos.y);
        
        let min_x = min_pos.x;
        let min_y = min_pos.y;
        let max_x = max_pos.x;
        let max_y = max_pos.y;
        
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
        debug_assert!(area.width > 0 && area.height > 0);
        
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
        debug_assert!(area.width > 0 && area.height > 0);
        
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
        debug_assert!(area.width > 0 && area.height > 0);
        
        let (min_x, min_y, max_x, max_y) = bounds;
        debug_assert!(min_x <= max_x);
        debug_assert!(min_y <= max_y);
        
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
        debug_assert!(area.width > 0 && area.height > 0);
        
        let selected_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let preview_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let freehand_preview_style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        // Render background layers
        self.render_grid(buf, area);
        self.render_snap_guides(buf, area);

        // Render all shapes
        self.render_all_shapes(buf, area, selected_style);

        // Render preview layers
        self.render_freehand_preview(buf, area, freehand_preview_style);
        self.render_snap_points(buf, area);
        self.render_shape_preview(buf, area, preview_style);
        self.render_active_snap_indicator(buf, area);
        self.render_marquee_selection(buf, area);
        self.render_selection_boxes(buf, area, selected_style);

        // Render remote cursors (on top of everything)
        if let Some(ref presence_mgr) = self.app.presence {
            for peer in presence_mgr.active_peers() {
                self.render_remote_cursor(buf, area, peer);
            }
        }
    }
}

impl CanvasWidget<'_> {
    /// Render all visible shapes
    fn render_all_shapes(&self, buf: &mut Buffer, area: Rect, selected_style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        for shape in self.app.shape_view.iter_visible() {
            let is_selected = self.app.selected.contains(&shape.id);
            let style = if is_selected {
                selected_style
            } else {
                Style::default().fg(shape.kind.color().to_ratatui())
            };

            self.render_shape(buf, area, &shape.kind, style);
        }
    }

    /// Render a single shape
    fn render_shape(&self, buf: &mut Buffer, area: Rect, kind: &ShapeKind, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        match kind {
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
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::Rectangle {
                start, end, label, ..
            } => {
                for (pos, ch) in rect_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::DoubleBox {
                start, end, label, ..
            } => {
                for (pos, ch) in double_rect_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::Parallelogram {
                start, end, label, ..
            } => {
                for (pos, ch) in parallelogram_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::Trapezoid {
                start, end, label, ..
            } => {
                for (pos, ch) in trapezoid_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::RoundedRect {
                start, end, label, ..
            } => {
                for (pos, ch) in rounded_rect_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::Cylinder {
                start, end, label, ..
            } => {
                for (pos, ch) in cylinder_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
            ShapeKind::Cloud {
                start, end, label, ..
            } => {
                for (pos, ch) in cloud_points(*start, *end) {
                    self.render_char(buf, area, pos, ch, style);
                }
                if let Some(text) = label {
                    self.render_label_on_shape(buf, area, kind, text, style);
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
                    self.render_label_on_shape(buf, area, kind, text, style);
                }
            }
        }
    }

    /// Render label on a shape (helper to avoid code duplication)
    fn render_label_on_shape(&self, buf: &mut Buffer, area: Rect, kind: &ShapeKind, text: &str, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        let bounds = kind.bounds();
        self.render_label(buf, area, bounds, text, style);
    }

    /// Render freehand preview while drawing
    fn render_freehand_preview(&self, buf: &mut Buffer, area: Rect, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if let Some(ref freehand_state) = self.app.freehand_state {
            for &pos in &freehand_state.points {
                self.render_char(buf, area, pos, self.app.brush_char, style);
            }
        }
    }

    /// Show snap points when line/arrow tool is active
    fn render_snap_points(&self, buf: &mut Buffer, area: Rect) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if self.app.current_tool == Tool::Line || self.app.current_tool == Tool::Arrow {
            let snap_style = Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::DIM);

            for snap in self.app.shape_view.all_snap_points() {
                self.render_char(buf, area, snap.pos, '◆', snap_style);
            }
        }
    }

    /// Render shape preview while drawing
    fn render_shape_preview(&self, buf: &mut Buffer, area: Rect, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if let Some(ref shape_state) = self.app.shape_state {
            let start = shape_state.start_snap.unwrap_or(shape_state.start);
            let end = shape_state.current_snap.unwrap_or(shape_state.current);

            self.render_tool_preview(buf, area, self.app.current_tool, start, end, style);
        }
    }

    /// Render preview for a specific tool
    fn render_tool_preview(&self, buf: &mut Buffer, area: Rect, tool: Tool, start: Position, end: Position, style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        match tool {
            Tool::Line => {
                for (pos, ch) in line_points_styled(start, end, self.app.line_style) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Arrow => {
                for (pos, ch) in arrow_points_styled(start, end, self.app.line_style) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Rectangle => {
                for (pos, ch) in rect_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::DoubleBox => {
                for (pos, ch) in double_rect_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Diamond => {
                let half_width = (end.x - start.x).abs().max(1);
                let half_height = (end.y - start.y).abs().max(1);
                for (pos, ch) in diamond_points(start, half_width, half_height) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Ellipse => {
                let radius_x = (end.x - start.x).abs().max(1);
                let radius_y = (end.y - start.y).abs().max(1);
                for (pos, ch) in ellipse_points(start, radius_x, radius_y) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Triangle => {
                let mid_x = (start.x + end.x) / 2;
                let height = (end.y - start.y).abs().max(1);
                let p3 = Position::new(mid_x, start.y + height);
                for (pos, ch) in triangle_points(start, end, p3) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Parallelogram => {
                for (pos, ch) in parallelogram_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Hexagon => {
                let radius_x = (end.x - start.x).abs().max(2);
                let radius_y = (end.y - start.y).abs().max(1);
                for (pos, ch) in hexagon_points(start, radius_x, radius_y) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Trapezoid => {
                for (pos, ch) in trapezoid_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::RoundedRect => {
                for (pos, ch) in rounded_rect_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Cylinder => {
                for (pos, ch) in cylinder_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Cloud => {
                for (pos, ch) in cloud_points(start, end) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            Tool::Star => {
                let outer_radius = (end.x - start.x).abs().max((end.y - start.y).abs()).max(2);
                let inner_radius = outer_radius / 2;
                for (pos, ch) in star_points(start, outer_radius, inner_radius) {
                    self.render_char(buf, area, pos, ch, style);
                }
            }
            _ => {}
        }
    }

    /// Render active snap indicator
    fn render_active_snap_indicator(&self, buf: &mut Buffer, area: Rect) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        let snap_active_style = Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD);
            
        if let Some(ref snap) = self.app.hover_snap {
            self.render_char(buf, area, snap.pos, '◉', snap_active_style);
        } else if let Some(grid_pos) = self.app.hover_grid_snap {
            self.render_char(buf, area, grid_pos, '◉', snap_active_style);
        }
    }

    /// Render marquee selection box
    fn render_marquee_selection(&self, buf: &mut Buffer, area: Rect) {
        debug_assert!(area.width > 0 && area.height > 0);
        
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
    }

    /// Render selection bounding boxes
    fn render_selection_boxes(&self, buf: &mut Buffer, area: Rect, box_style: Style) {
        debug_assert!(area.width > 0 && area.height > 0);
        
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
    }

    /// Render a remote peer's cursor and activity
    fn render_remote_cursor(&self, buf: &mut Buffer, area: Rect, peer: &PeerPresence) {
        debug_assert!(area.width > 0 && area.height > 0);
        
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
                self.render_remote_drawing(buf, area, *tool, *start, *current, color);
            }
            CursorActivity::Selected { shape_id } => {
                self.render_remote_selection(buf, area, *shape_id, color);
            }
            CursorActivity::Dragging { shape_id, delta } => {
                self.render_remote_dragging(buf, area, *shape_id, *delta, peer, color);
            }
            CursorActivity::Resizing {
                shape_id,
                preview_bounds,
            } => {
                self.render_remote_resizing(buf, area, *shape_id, *preview_bounds, peer, color);
            }
            CursorActivity::Typing { position } => {
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

    /// Render remote drawing preview
    fn render_remote_drawing(&self, buf: &mut Buffer, area: Rect, tool: ToolKind, start: Position, current: Position, color: Color) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        let preview_style = Style::default().fg(color).add_modifier(Modifier::DIM);
        self.render_shape_preview_for_tool(buf, area, tool, start, current, preview_style);
    }

    /// Render remote selection highlight
    fn render_remote_selection(&self, buf: &mut Buffer, area: Rect, shape_id: ShapeId, color: Color) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if let Some(shape) = self.app.shape_view.get(shape_id) {
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

    /// Render remote dragging ghost
    fn render_remote_dragging(&self, buf: &mut Buffer, area: Rect, shape_id: ShapeId, delta: (i32, i32), peer: &PeerPresence, color: Color) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if let Some(shape) = self.app.shape_view.get(shape_id) {
            let (orig_min_x, orig_min_y, orig_max_x, orig_max_y) = shape.bounds();
            let (dx, dy) = delta;
            let ghost_min_pos = Position::new(orig_min_x + dx, orig_min_y + dy);
            let ghost_max_pos = Position::new(orig_max_x + dx, orig_max_y + dy);
            let ghost_style = Style::default().fg(color);
            
            self.render_dashed_rect(buf, area, ghost_min_pos, ghost_max_pos, ghost_style);
            
            let label = format!("{} moving", peer.display_name());
            let label_pos = Position::new(ghost_min_pos.x, ghost_min_pos.y.saturating_sub(1));
            self.render_text(buf, area, label_pos, &label, ghost_style);
        }
    }

    /// Render remote resizing ghost
    fn render_remote_resizing(&self, buf: &mut Buffer, area: Rect, shape_id: ShapeId, preview_bounds: Option<(Position, Position)>, peer: &PeerPresence, color: Color) {
        debug_assert!(area.width > 0 && area.height > 0);
        
        if let Some((min_pos, max_pos)) = preview_bounds {
            let ghost_style = Style::default().fg(color);
            self.render_dashed_rect(buf, area, min_pos, max_pos, ghost_style);
            
            let label = format!("{} resizing", peer.display_name());
            let label_pos = Position::new(min_pos.x, min_pos.y.saturating_sub(1));
            self.render_text(buf, area, label_pos, &label, ghost_style);
        } else if let Some(shape) = self.app.shape_view.get(shape_id) {
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

    /// Render a preview shape for remote drawing activity
    fn render_shape_preview_for_tool(
        &self,
        buf: &mut Buffer,
        area: Rect,
        tool: ToolKind,
        start: Position,
        current: Position,
        style: Style,
    ) {
        debug_assert!(area.width > 0 && area.height > 0);
        
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
    debug_assert!(area.width > 0 && area.height > 0);
    
    let Some(ref presence_mgr) = app.presence else {
        return;
    };

    let peer_count = presence_mgr.peer_count() + 1; // +1 for self
    let title = format!(" Participants ({}) ", peer_count);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut items: Vec<Line> = Vec::new();

    // Get local user's active layer name
    let local_layer_name = app
        .active_layer
        .and_then(|id| {
            match app.doc.read_layer(id) {
                Ok(opt) => opt,
                Err(_e) => {
                    // Log error in production; for now, silently handle
                    None
                }
            }
        })
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
            .and_then(|id| {
                match app.doc.read_layer(id) {
                    Ok(opt) => opt,
                    Err(_e) => {
                        // Log error in production; for now, silently handle
                        None
                    }
                }
            })
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
    let name_len = name.chars().count();
    if name_len > max_len {
        name.chars().take(max_len.saturating_sub(2)).collect::<String>() + ".."
    } else {
        name.to_string()
    }
}

/// Render the layer panel
fn render_layer_panel(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let layers = app.get_layers();
    let title = format!(" Layers ({}) ", layers.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Check if we're renaming a layer
    let renaming = if let Mode::LayerRename(state) = &app.mode {
        Some((state.layer_id, state.text.as_str()))
    } else {
        None
    };

    // Collect peers by their active layer
    let peers_by_layer = collect_peers_by_layer(app);

    // Build list items
    let mut items: Vec<Line> = Vec::new();

    for layer in &layers {
        let line = build_layer_line(app, layer, renaming, &peers_by_layer);
        items.push(line);
    }

    let paragraph = Paragraph::new(items).block(block);

    frame.render_widget(paragraph, area);
}

/// Collect peers grouped by their active layer
fn collect_peers_by_layer(app: &App) -> HashMap<LayerId, Vec<Color>> {
    let mut peers_by_layer: HashMap<LayerId, Vec<Color>> = HashMap::new();
    
    if let Some(ref presence_mgr) = app.presence {
        for peer in presence_mgr.active_peers() {
            if let Some(layer_id) = peer.active_layer_id {
                let color = peer_color(peer);
                peers_by_layer.entry(layer_id).or_default().push(color);
            }
        }
    }
    
    peers_by_layer
}

/// Build a single layer line for the layer panel
fn build_layer_line(
    app: &App,
    layer: &crate::layers::Layer,
    renaming: Option<(LayerId, &str)>,
    peers_by_layer: &HashMap<LayerId, Vec<Color>>,
) -> Line<'static> {
    let is_active = app.active_layer == Some(layer.id);
    let is_renaming = renaming.is_some_and(|(id, _)| id == layer.id);

    // Active indicator
    let active_indicator = if is_active {
        Span::styled("● ", Style::default().fg(Color::Cyan))
    } else {
        Span::raw("  ")
    };

    let max_name_len = 10usize;
    
    // Layer name or rename input
    let (name_span, vis_span, lock_span) = if is_renaming
        && let Some((_, text)) = renaming
    {
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

        let visible_indicator = if layer.visible {
            Span::styled(" ", Style::default())
        } else {
            Span::styled("[H]", Style::default().fg(Color::DarkGray))
        };

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

    Line::from(line_spans)
}

/// Render the status bar (Helix-style with mode indicator)
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let (mode_name, mode_bg) = get_mode_info(&app.mode, app.current_tool);
    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(mode_bg)
        .add_modifier(Modifier::BOLD);

    let file_name = get_file_name(app);
    let dirty_marker = if app.is_dirty() { " *" } else { "" };
    let char_info = get_char_info(app);
    let (peer_info, peer_style) = get_peer_info(app);
    let (status_text, status_color) = get_status_message(app);
    let tool_info = get_tool_info(app);
    let shape_info = get_shape_info(app);
    let session_name = get_session_name(app);
    let count_info = get_count_info(app);
    let zoom_info = get_zoom_info(app);

    let spans = vec![
        Span::styled(format!(" {} ", mode_name), mode_style),
        Span::styled(tool_info, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" {}{}{}", file_name, dirty_marker, char_info)),
        Span::styled(count_info, Style::default().fg(Color::DarkGray)),
        Span::styled(zoom_info, Style::default().fg(Color::Magenta)),
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

/// Get mode name and color for status bar
fn get_mode_info(mode: &Mode, current_tool: Tool) -> (&'static str, Color) {
    match mode {
        Mode::Normal => match current_tool {
            Tool::Select => ("SEL", Color::Blue),
            Tool::Text => ("TXT", Color::Green),
            _ => ("DRAW", Color::Yellow),
        },
        Mode::TextInput(_) | Mode::LabelInput(_) | Mode::LayerRename(_) => {
            ("INS", Color::Green)
        }
        Mode::PathInput(_) => ("CMD", Color::Magenta),
        Mode::RecentFiles(_) | Mode::SelectionPopup(_) => ("MENU", Color::Cyan),
        Mode::ConfirmDialog(_) => ("CONF", Color::Yellow),
        Mode::HelpScreen(_) => ("HELP", Color::Cyan),
        Mode::LeaderMenu(_) => ("SPACE", Color::Cyan),
        Mode::SessionBrowser(_) | Mode::SessionCreate(_) => ("SESS", Color::Magenta),
        Mode::KeyboardShapeCreate(_) => ("CREATE", Color::Cyan),
        Mode::QrCodeDisplay(_) => ("QR", Color::Magenta),
    }
}

/// Get file name for status bar
fn get_file_name(app: &App) -> String {
    app.file_path
        .as_ref()
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "[unsaved]".to_string())
}

/// Get character/style info for status bar
fn get_char_info(app: &App) -> String {
    match app.current_tool {
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
    }
}

/// Get peer/sync info for status bar
fn get_peer_info(app: &App) -> (String, Style) {
    if let Some(ref presence) = app.presence {
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
    }
}

/// Get status message for status bar
fn get_status_message(app: &App) -> (String, Color) {
    match &app.status_message {
        Some((msg, MessageSeverity::Info)) => (format!(" {}", msg), Color::White),
        Some((msg, MessageSeverity::Warning)) => (format!(" ⚠ {}", msg), Color::Yellow),
        Some((msg, MessageSeverity::Error)) => (format!(" ✗ {}", msg), Color::Red),
        None => (String::new(), Color::White),
    }
}

/// Get tool info for status bar
fn get_tool_info(app: &App) -> String {
    if app.current_tool != Tool::Select {
        format!(" {}", app.current_tool.name().to_lowercase())
    } else {
        String::new()
    }
}

/// Get shape info for status bar
fn get_shape_info(app: &App) -> String {
    if app.status_message.is_none() && app.current_tool == Tool::Select {
        app.get_selected_shape_info()
            .map(|info| format!(" | {}", info))
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Get session name for status bar
fn get_session_name(app: &App) -> &str {
    app.current_session_meta
        .as_ref()
        .map(|m| m.name.as_str())
        .unwrap_or("No session")
}

/// Get shape count info for status bar
fn get_count_info(app: &App) -> String {
    let total_shapes = app.shape_view.shape_count();
    let selected_count = app.selected.len();
    
    if selected_count > 0 {
        format!(" [{}/{}]", selected_count, total_shapes)
    } else if total_shapes > 0 {
        format!(" [{}]", total_shapes)
    } else {
        String::new()
    }
}

/// Get zoom info for status bar
fn get_zoom_info(app: &App) -> String {
    if (app.viewport.zoom - 1.0).abs() > 0.01 {
        format!(" {}%", (app.viewport.zoom * 100.0) as i32)
    } else {
        String::new()
    }
}

/// Render the help bar
fn render_help_bar(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let help_text = get_context_help(&app.mode, app.current_tool, app.show_layers, app.active_layer, &app.selected);

    let paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}

/// Get context-sensitive help text
fn get_context_help(mode: &Mode, current_tool: Tool, show_layers: bool, active_layer: Option<LayerId>, selected: &std::collections::HashSet<ShapeId>) -> &'static str {
    match mode {
        Mode::Normal => {
            get_normal_mode_help(current_tool, show_layers, active_layer, selected)
        }
        Mode::TextInput(_) | Mode::LabelInput(_) => {
            "type text | [Enter] confirm [Esc] cancel [Backspace] delete"
        }
        Mode::LayerRename(_) => "type layer name | [Enter] confirm [Esc] cancel",
        Mode::PathInput(_) => "type path | [Tab] complete [Enter] confirm [Esc] cancel",
        Mode::RecentFiles(_) => "[j/k] navigate [Enter] open [Esc] cancel",
        Mode::SelectionPopup(_) => "[hjkl] navigate [Enter] select [Esc] cancel",
        Mode::ConfirmDialog(_) => "[y] Yes [n] No | [Enter] confirm [Esc] cancel",
        Mode::HelpScreen(_) => "[j/k] scroll [Space] page down [Esc/q/?] close",
        Mode::LeaderMenu(_) => "Space:select t:tool c:color b:brush s:save o:open e:export g:grid l:layers ?:help q:quit",
        Mode::SessionBrowser(_) => {
            "[j/k] navigate [n]ew [d]elete [p]in [*] pinned filter [Tab/Esc] close"
        }
        Mode::SessionCreate(_) => "type session name | [Enter] create [Esc] cancel",
        Mode::KeyboardShapeCreate(_) => {
            "[Tab] switch field | type dimensions | [Enter] create [Esc] cancel"
        }
        Mode::QrCodeDisplay(_) => "[y] copy ticket | [w] save PNG | any key: close",
    }
}

/// Get help text for normal mode
fn get_normal_mode_help(current_tool: Tool, show_layers: bool, active_layer: Option<LayerId>, selected: &std::collections::HashSet<ShapeId>) -> &'static str {
    let base_help = match current_tool {
        Tool::Select => {
            if selected.len() >= 2 {
                "[Shift+G] group [y]ank [Del] delete | [Space] menu [?] help"
            } else if selected.len() == 1 {
                "[Enter] label [y]ank [Del] delete | []/{}:z-order | [Space] menu"
            } else {
                "[Ctrl+A] select all | click to select | [Space] menu [?] help"
            }
        }
        Tool::Freehand => "drag to draw | [Space] menu [?] help",
        Tool::Text => "click to place text | [Space] menu [?] help",
        Tool::Line | Tool::Arrow => {
            "[v] line style | drag to draw | [Space] menu [?] help"
        }
        _ => "drag to draw | [Space] menu [?] help",
    };

    if show_layers && active_layer.is_some() {
        "[F2] rename layer [Ctrl+D] delete | [Alt+1-9] switch layer"
    } else {
        base_help
    }
}

/// Render text input cursor on canvas
fn render_text_cursor(frame: &mut Frame, app: &App, start_pos: &Position, text: &str, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
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
    debug_assert!(area.width > 0 && area.height > 0);
    debug_assert!(cursor <= text.chars().count());
    
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
    debug_assert!(area.width > 0 && area.height > 0);
    
    let width = 50.min(area.width.saturating_sub(4));
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    // Clear the popup area
    clear_area(frame, popup_area);

    // Create a text input with the path value
    let text_input = rat_widgets::TextInput::new()
        .with_value(path)
        .with_focused(true);
    
    // Render with block wrapper
    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, popup_area);
    
    // Render text input in the inner area
    let inner = popup_area.inner(Margin::new(1, 1));
    text_input.render(frame, inner, None);
}

/// Render recent files menu overlay
fn render_recent_files_menu(frame: &mut Frame, app: &App, selected: usize, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let width = 50.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(4)); // Max 12 lines
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    clear_area(frame, popup_area);

    // Build list items from recent files
    let items: Vec<String> = app.recent_files.iter()
        .map(|file| file.name.clone())
        .collect();

    let mut list = rat_widgets::ScrollableList::new(items)
        .with_border_color(Color::Cyan);
    list.move_to(selected);
    
    let block = Block::default()
        .title(" Recent Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, popup_area);
    
    // Render list in the inner area
    let inner = popup_area.inner(Margin::new(1, 1));
    list.render(frame, inner, None);
}



/// Render selection popup for tools, colors, or brushes
fn render_selection_popup(frame: &mut Frame, kind: PopupKind, selected: usize, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let (title, cols, grid_items) = build_grid_select_items(kind);
    let mut grid_select = rat_widgets::GridSelect::new(title, grid_items, cols);
    grid_select.set_selected(selected);
    grid_select.render(frame, area);
}

/// Build grid select items for rat-widgets
fn build_grid_select_items(kind: PopupKind) -> (&'static str, usize, Vec<rat_widgets::GridItem>) {
    match kind {
        PopupKind::Tool => {
            let items: Vec<_> = TOOLS.iter()
                .map(|t| rat_widgets::GridItem::new(t.name()))
                .collect();
            (" Select Tool ", 3, items)
        }
        PopupKind::Color => {
            let items: Vec<_> = COLORS.iter()
                .map(|c| rat_widgets::GridItem::new(c.name()).with_color(c.to_ratatui()))
                .collect();
            (" Select Color ", 4, items)
        }
        PopupKind::Brush => {
            let items: Vec<_> = BRUSHES.iter()
                .map(|&ch| rat_widgets::GridItem::new(ch))
                .collect();
            (" Select Brush ", 6, items)
        }
    }
}



/// Render confirmation dialog overlay
fn render_confirm_dialog(frame: &mut Frame, action: &PendingAction, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let message = format!("{}: {}", action.title(), action.message());
    let dialog = rat_widgets::ConfirmDialog::new(message);
    dialog.render(frame, area);
}



/// Single source of truth for all documented keybindings.
pub fn help_sections() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    vec![
        ("GENERAL", get_general_help()),
        ("LEADER MENU (Space/:)", get_leader_help()),
        ("TOOLS", get_tools_help()),
        ("DRAWING", get_drawing_help()),
        ("KEYBOARD CREATE", get_keyboard_create_help()),
        ("SELECTION", get_selection_help()),
        ("Z-ORDER", get_zorder_help()),
        ("TRANSFORM", get_transform_help()),
        ("GROUPING", get_grouping_help()),
        ("LAYERS", get_layers_help()),
        ("FILES", get_files_help()),
        ("COLLABORATION", get_collaboration_help()),
        ("NAVIGATION", get_navigation_help()),
    ]
}

fn get_general_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("q", "Quit"),
        ("Ctrl+c", "Quit"),
        ("Esc", "Cancel/deselect"),
        ("Space/:", "Leader menu"),
        ("?/F1", "Toggle help screen"),
        ("u", "Undo"),
        ("U", "Redo"),
    ]
}

fn get_leader_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("t", "Tool picker"),
        ("c", "Color picker"),
        ("b", "Brush picker"),
        ("s", "Save file"),
        ("o", "Open file"),
        ("e", "Export SVG"),
        ("n", "New document"),
        ("g", "Toggle grid"),
        ("l", "Toggle layers"),
        ("p", "Toggle peers"),
        ("?", "Help"),
        ("q", "Quit"),
    ]
}

fn get_tools_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Space Space", "Select tool"),
        ("f", "Freehand draw"),
        ("t", "Text tool"),
        ("l", "Line tool"),
        ("a", "Arrow tool"),
        ("r", "Rectangle tool"),
        ("b", "DoubleBox tool"),
        ("d", "Diamond tool"),
        ("e", "Ellipse tool"),
        ("Space t", "Triangle (picker)"),
        ("Space t", "Parallelogram (picker)"),
        ("Space t", "Hexagon (picker)"),
        ("Space t", "Trapezoid (picker)"),
        ("Space t", "RoundedRect (picker)"),
        ("Space t", "Cylinder (picker)"),
        ("Space t", "Cloud (picker)"),
        ("s", "Star tool"),
    ]
}

fn get_drawing_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("v", "Cycle line style"),
        ("g", "Toggle grid snap"),
    ]
}

fn get_keyboard_create_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Ctrl+Shift+R", "Create rectangle"),
        ("Ctrl+Shift+L", "Create line"),
        ("Ctrl+Shift+B", "Create double box"),
        ("Alt+d", "Create diamond"),
        ("Alt+e", "Create ellipse"),
        ("Alt+a", "Create arrow"),
    ]
}

fn get_selection_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Click", "Select shape"),
        ("Shift+Click", "Toggle selection"),
        ("Drag", "Marquee select"),
        ("Ctrl+A", "Select all"),
        ("y", "Yank (copy)"),
        ("p", "Paste"),
        ("Del/Backspace", "Delete selected"),
        ("Enter", "Edit label"),
    ]
}

fn get_zorder_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("]", "Bring forward"),
        ("[", "Send backward"),
        ("}", "Bring to front"),
        ("{", "Send to back"),
    ]
}

fn get_transform_help() -> Vec<(&'static str, &'static str)> {
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
    ]
}

fn get_grouping_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Shift+G", "Group selected"),
        ("Ctrl+Shift+G", "Ungroup selected"),
    ]
}

fn get_layers_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("L", "Toggle layer panel"),
        ("Ctrl+n", "New layer"),
        ("Ctrl+D", "Delete layer (confirm)"),
        ("F2", "Rename layer"),
        ("Alt+1-9", "Select layer 1-9"),
        ("Ctrl+m", "Move selection to layer"),
        ("Ctrl+h", "Toggle layer visibility"),
    ]
}

fn get_files_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Ctrl+s", "Export ASCII (.txt)"),
        ("Ctrl+o", "Import ASCII"),
        ("Ctrl+Shift+S", "Save document (.automerge)"),
        ("Ctrl+Shift+O", "Open document"),
        ("E", "Export SVG"),
        ("N", "New document (confirm)"),
        ("R", "Recent files"),
    ]
}

fn get_collaboration_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("T", "Copy sync ticket"),
        ("Q", "Show ticket as QR code (w: save PNG)"),
        ("D", "Decode QR code from image"),
        ("J", "Join peer session"),
        ("P", "Toggle participants"),
    ]
}

fn get_navigation_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Arrow keys", "Pan viewport"),
        ("Ctrl++/=", "Zoom in"),
        ("Ctrl+-", "Zoom out"),
        ("Ctrl+0", "Reset zoom"),
        ("Ctrl+Scroll", "Mouse zoom"),
    ]
}

/// Render the help screen overlay
fn render_help_screen(frame: &mut Frame, scroll: usize, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let help_sections = help_sections();

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

    clear_area_with_bg(frame, area, Color::Black);

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
    debug_assert!(area.width > 0 && area.height > 0);
    
    use std::time::{SystemTime, UNIX_EPOCH};

    let filtered = app.get_filtered_sessions(filter, show_pinned_only);
    let session_count = filtered.len();

    let width = 60.min(area.width.saturating_sub(4));
    let height = (session_count as u16 + 5).min(20);
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    clear_area_with_bg(frame, popup_area, Color::Black);

    let title = build_session_browser_title(app);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let lines = build_session_browser_lines(filter, show_pinned_only, &filtered, selected, app, now);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Build title for session browser
fn build_session_browser_title(app: &App) -> String {
    if let Some(ref meta) = app.current_session_meta {
        format!(" Sessions [{}] ", meta.name)
    } else {
        " Sessions ".to_string()
    }
}

/// Build lines for session browser
fn build_session_browser_lines(
    filter: &str,
    show_pinned_only: bool,
    filtered: &[&crate::session::SessionMeta],
    selected: usize,
    app: &App,
    now: u64,
) -> Vec<Line<'static>> {
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
    lines.push(Line::raw(""));

    // Session items
    for (i, session) in filtered.iter().enumerate() {
        let line = build_session_line(*session, i == selected, app, now);
        lines.push(line);
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
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        " n:new d:del p:pin *:pinned Tab:close",
        Style::default().fg(Color::DarkGray),
    ));

    lines
}

/// Build a single session line
fn build_session_line(session: &crate::session::SessionMeta, is_selected: bool, app: &App, now: u64) -> Line<'static> {
    let is_current = app.current_session.as_ref() == Some(&session.id);

    let time_ago = format_time_ago(now.saturating_sub(session.last_accessed));

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

    Line::styled(format!(" {}", line_text), style)
}

/// Format seconds as time ago string
fn format_time_ago(diff: u64) -> String {
    if diff < 60 {
        "now".to_string()
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else {
        format!("{}d", diff / 86400)
    }
}

/// Render session create dialog
fn render_session_create(frame: &mut Frame, name: &str, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let width = 40.min(area.width.saturating_sub(4));
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2 + area.x;
    let y = (area.height.saturating_sub(height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, width, height);

    clear_area_with_bg(frame, popup_area, Color::Black);

    // Create a text input with the name value
    let text_input = rat_widgets::TextInput::new()
        .with_value(name)
        .with_focused(true)
        .with_placeholder("Session name");
    
    let block = Block::default()
        .title(" New Session ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    frame.render_widget(block, popup_area);
    
    // Render text input in the inner area
    let inner = popup_area.inner(Margin::new(1, 1));
    text_input.render(frame, inner, None);
    
    // Add hint at bottom
    let hint = " Enter:create Esc:cancel";
    let hint_y = popup_area.y + popup_area.height - 2;
    let hint_x = popup_area.x + 1;
    for (i, ch) in hint.chars().enumerate() {
        let px = hint_x + i as u16;
        if px < popup_area.x + popup_area.width - 1 {
            frame.buffer_mut()[(px, hint_y)]
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
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
    debug_assert!(area.width > 0 && area.height > 0);
    
    let popup_width = 35.min(area.width.saturating_sub(4));
    let popup_height = 7;
    let x = (area.width.saturating_sub(popup_width)) / 2 + area.x;
    let y = (area.height.saturating_sub(popup_height)) / 2 + area.y;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    clear_area_with_bg(frame, popup_area, Color::Black);

    let title = format!(" Create {} ", tool.name());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let (label1, label2) = get_shape_create_labels(tool);
    let lines = build_shape_create_lines(label1, label2, width, height, focus);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, popup_area);
}

/// Get labels for shape creation dialog
fn get_shape_create_labels(tool: Tool) -> (&'static str, &'static str) {
    match tool {
        Tool::Line | Tool::Arrow => ("Length", "Y-Offset"),
        Tool::Star => ("Outer R", "Inner R"),
        _ => ("Width", "Height"),
    }
}

/// Build lines for shape creation dialog
fn build_shape_create_lines(
    label1: &str,
    label2: &str,
    width: &str,
    height: &str,
    focus: KeyboardShapeField,
) -> Vec<Line<'static>> {
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

    vec![
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
    ]
}

/// Render a QR code display popup showing a ticket as a scannable QR code.
fn render_qr_code_display(frame: &mut Frame, ticket: &str, area: Rect) {
    debug_assert!(area.width > 0 && area.height > 0);
    
    use crate::app::qr::ticket_to_qr_lines;

    let qr_lines = match ticket_to_qr_lines(ticket) {
        Ok(lines) => lines,
        Err(e) => {
            render_qr_error(frame, &e, area);
            return;
        }
    };

    let qr_width = qr_lines.first().map(|l| l.text.chars().count()).unwrap_or(0);
    let qr_height = qr_lines.len();
    let popup_w = (qr_width + 4) as u16;
    let popup_h = (qr_height + 6) as u16;

    let popup_area = centered_rect_fixed(popup_w, popup_h, area);

    let lines = build_qr_display_lines(&qr_lines, ticket, popup_w);

    let block = Block::default()
        .title(" QR Code ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}

/// Render QR code error
fn render_qr_error(frame: &mut Frame, error: &str, area: Rect) {
    let block = Block::default()
        .title(" QR Code Error ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let paragraph = Paragraph::new(error).block(block);
    let popup_area = centered_rect_fixed(40, 5, area);
    frame.render_widget(paragraph, popup_area);
}

/// Build lines for QR code display
fn build_qr_display_lines(qr_lines: &[crate::app::qr::QrLine], ticket: &str, popup_w: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::styled(
        " Scan to join session ",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));

    lines.push(Line::raw(""));

    let qr_style = Style::default().fg(Color::White).bg(Color::Black);
    for qr_line in qr_lines {
        let padded = format!(" {} ", qr_line.text);
        lines.push(Line::styled(padded, qr_style));
    }

    lines.push(Line::raw(""));

    let display_ticket = if ticket.len() > (popup_w as usize).saturating_sub(4) {
        let max = (popup_w as usize).saturating_sub(7);
        format!("{}...", &ticket[..max])
    } else {
        ticket.to_string()
    };
    lines.push(Line::styled(
        display_ticket,
        Style::default().fg(Color::DarkGray),
    ));

    lines.push(Line::styled(
        " y:copy  any key:close ",
        Style::default().fg(Color::DarkGray),
    ));

    lines
}

/// Create a fixed-size centered rectangle within the given area.
fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    debug_assert!(area.width > 0 && area.height > 0);
    
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Clear an area by setting all cells to space
fn clear_area(frame: &mut Frame, area: Rect) {
    for py in area.y..area.y + area.height {
        for px in area.x..area.x + area.width {
            frame.buffer_mut()[(px, py)].set_char(' ');
        }
    }
}

/// Clear an area with a specific background color
fn clear_area_with_bg(frame: &mut Frame, area: Rect, bg: Color) {
    for py in area.y..area.y + area.height {
        for px in area.x..area.x + area.width {
            frame.buffer_mut()[(px, py)]
                .set_char(' ')
                .set_style(Style::default().bg(bg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_covers_all_tools() {
        use crate::app::TOOLS;
        let sections = help_sections();
        let tools_section = sections
            .iter()
            .find(|(name, _)| *name == "TOOLS")
            .expect("TOOLS section must exist in help");

        for tool in TOOLS {
            let tool_name_lower = tool.name().to_lowercase();
            let found = tools_section
                .1
                .iter()
                .any(|(_, desc)| desc.to_lowercase().contains(&tool_name_lower));
            assert!(
                found,
                "Tool {:?} (name='{}') not found in help TOOLS section",
                tool,
                tool.name()
            );
        }
    }

    #[test]
    fn help_has_leader_menu_section() {
        let sections = help_sections();
        let leader = sections
            .iter()
            .find(|(name, _)| name.contains("LEADER"))
            .expect("LEADER section must exist in help");

        // These keys are handled in leader.rs and must be documented
        let expected_keys = ["t", "c", "b", "s", "o", "e", "n", "g", "l", "p", "?", "q"];
        for key in expected_keys {
            let found = leader.1.iter().any(|(k, _)| k.contains(key));
            assert!(
                found,
                "Leader key '{}' not documented in help LEADER section",
                key
            );
        }
    }

    #[test]
    fn truncate_name_short_string() {
        assert_eq!(truncate_name("Layer 1", 10), "Layer 1");
        assert_eq!(truncate_name("abc", 8), "abc");
    }

    #[test]
    fn truncate_name_exact_length() {
        assert_eq!(truncate_name("12345678", 8), "12345678");
    }

    #[test]
    fn truncate_name_long_string() {
        assert_eq!(truncate_name("VeryLongLayerName", 8), "VeryLo..");
        assert_eq!(truncate_name("1234567890", 8), "123456..");
    }

    #[test]
    fn truncate_name_empty_string() {
        assert_eq!(truncate_name("", 8), "");
    }

    #[test]
    fn truncate_name_unicode() {
        assert_eq!(truncate_name("...", 8), "...");
        assert_eq!(truncate_name("Layer...", 8), "Layer...");
    }

    #[test]
    fn truncate_name_very_small_max() {
        assert_eq!(truncate_name("Hello", 3), "H..");
        assert_eq!(truncate_name("Hello", 2), "..");
    }

    #[test]
    fn truncate_name_whitespace() {
        assert_eq!(truncate_name("Layer With Spaces", 8), "Layer ..");
        assert_eq!(truncate_name("   ", 2), "..");
    }
}
