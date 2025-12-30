use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

use crate::app::{App, Mode, PopupKind, Tool, BRUSHES, COLORS, GRID_SIZE, TOOLS};
use crate::canvas::{
    arrow_points_styled, cloud_points, cylinder_points, diamond_points, double_rect_points,
    ellipse_points, hexagon_points, line_points_styled, parallelogram_points, rect_points,
    rounded_rect_points, star_points, trapezoid_points, triangle_points, Position,
};
use crate::document::ShapeId;
use crate::presence::{peer_color, CursorActivity, PeerPresence, ToolKind};
use crate::shapes::ShapeKind;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Canvas area (+ optional panel)
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Help bar
        ])
        .split(frame.area());

    // Split top area horizontally if participants panel is shown
    let canvas_area = if app.show_participants && app.presence.is_some() {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),      // Canvas (at least 20 chars)
                Constraint::Length(24),   // Participant panel (fixed width)
            ])
            .split(chunks[0]);

        render_participants_panel(frame, app, horizontal[1]);
        horizontal[0]
    } else {
        chunks[0]
    };

    render_canvas(frame, app, canvas_area);
    render_status_bar(frame, app, chunks[1]);
    render_help_bar(frame, app, chunks[2]);

    // Render input overlay if in input mode
    match &app.mode {
        Mode::TextInput { start_pos, text } => {
            render_text_cursor(frame, app, start_pos, text, canvas_area);
        }
        Mode::LabelInput { shape_id, text } => {
            render_label_input(frame, app, *shape_id, text, canvas_area);
        }
        Mode::FileSave { path } => {
            render_file_input(frame, "Save as:", path, canvas_area);
        }
        Mode::FileOpen { path } => {
            render_file_input(frame, "Open file:", path, canvas_area);
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
        Mode::Normal => {}
    }
}

/// Render the canvas area
fn render_canvas(frame: &mut Frame, app: &App, area: Rect) {
    let canvas_widget = CanvasWidget { app };
    frame.render_widget(canvas_widget, area);
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

    fn render_label(&self, buf: &mut Buffer, area: Rect, bounds: (i32, i32, i32, i32), text: &str, style: Style) {
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

        // Render all shapes
        for shape in self.app.shape_view.iter() {
            let is_selected = self.app.selected.contains(&shape.id);
            // Use shape's color, but override with cyan when selected
            let style = if is_selected {
                selected_style
            } else {
                Style::default().fg(shape.kind.color().to_ratatui())
            };

            match &shape.kind {
                ShapeKind::Line { start, end, style: line_style, label, .. } => {
                    for (pos, ch) in line_points_styled(*start, *end, *line_style) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Arrow { start, end, style: line_style, label, .. } => {
                    for (pos, ch) in arrow_points_styled(*start, *end, *line_style) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Rectangle { start, end, label, .. } => {
                    for (pos, ch) in rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::DoubleBox { start, end, label, .. } => {
                    for (pos, ch) in double_rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Diamond { center, half_width, half_height, label, .. } => {
                    for (pos, ch) in diamond_points(*center, *half_width, *half_height) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Ellipse { center, radius_x, radius_y, label, .. } => {
                    for (pos, ch) in ellipse_points(*center, *radius_x, *radius_y) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Freehand { points, char, label, .. } => {
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
                ShapeKind::Triangle { p1, p2, p3, label, .. } => {
                    for (pos, ch) in triangle_points(*p1, *p2, *p3) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Parallelogram { start, end, label, .. } => {
                    for (pos, ch) in parallelogram_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Hexagon { center, radius_x, radius_y, label, .. } => {
                    for (pos, ch) in hexagon_points(*center, *radius_x, *radius_y) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Trapezoid { start, end, label, .. } => {
                    for (pos, ch) in trapezoid_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::RoundedRect { start, end, label, .. } => {
                    for (pos, ch) in rounded_rect_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Cylinder { start, end, label, .. } => {
                    for (pos, ch) in cylinder_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Cloud { start, end, label, .. } => {
                    for (pos, ch) in cloud_points(*start, *end) {
                        self.render_char(buf, area, pos, ch, style);
                    }
                    if let Some(text) = label {
                        self.render_label(buf, area, shape.bounds(), text, style);
                    }
                }
                ShapeKind::Star { center, outer_radius, inner_radius, label, .. } => {
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
            let marquee_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::DIM);
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
        let cursor_style = Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD);

        // Render cursor marker
        self.render_char(buf, area, peer.cursor_pos, '█', cursor_style);

        // Render activity indicator based on what they're doing
        match &peer.activity {
            CursorActivity::Drawing { tool, start, current } => {
                // Show ghost preview of shape being drawn
                let preview_style = Style::default()
                    .fg(color)
                    .add_modifier(Modifier::DIM);
                self.render_shape_preview(buf, area, *tool, *start, *current, preview_style);
            }
            CursorActivity::Selected { shape_id }
            | CursorActivity::Dragging { shape_id }
            | CursorActivity::Resizing { shape_id } => {
                // Highlight the shape they're working with
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
                let outer_radius = (current.x - start.x).abs().max((current.y - start.y).abs()).max(2);
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
    let Some(ref presence_mgr) = app.presence else { return };

    let peer_count = presence_mgr.peer_count() + 1; // +1 for self
    let title = format!(" Participants ({}) ", peer_count);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Build list items
    let mut items: Vec<Line> = Vec::new();

    // Add "You" entry first (local user)
    items.push(Line::from(vec![
        Span::styled("● ", Style::default().fg(Color::White)),
        Span::raw("You"),
    ]));

    // Add remote peers
    for peer in presence_mgr.active_peers() {
        let color = peer_color(peer);
        let activity = peer.activity.label();

        items.push(Line::from(vec![
            Span::styled("█ ", Style::default().fg(color)),
            Span::raw(peer.display_name()),
            Span::styled(format!(" ({})", activity), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let paragraph = Paragraph::new(items)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

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
        Mode::TextInput { .. } | Mode::LabelInput { .. } => ("INS", Color::Green),
        Mode::FileSave { .. } | Mode::FileOpen { .. } | Mode::SvgExport { .. } => ("CMD", Color::Magenta),
        Mode::RecentFiles { .. } | Mode::SelectionPopup { .. } => ("MENU", Color::Cyan),
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
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .unwrap_or_else(|| "[unsaved]".to_string());

    let dirty_marker = if app.is_dirty() { " *" } else { "" };

    let char_info = match app.current_tool {
        Tool::Freehand => format!(" brush:'{}' {}", app.brush_char, app.current_color.name()),
        Tool::Line | Tool::Arrow => format!(" {} {}", app.line_style.name(), app.current_color.name()),
        Tool::Rectangle | Tool::DoubleBox | Tool::Diamond | Tool::Ellipse | Tool::Text |
        Tool::Triangle | Tool::Parallelogram | Tool::Hexagon | Tool::Trapezoid |
        Tool::RoundedRect | Tool::Cylinder | Tool::Cloud | Tool::Star => {
            format!(" {}", app.current_color.name())
        }
        _ => String::new(),
    };

    let peer_info = if let Some(ref presence) = app.presence {
        let count = presence.peer_count();
        if count > 0 {
            format!(" {}p", count)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let status_text = app
        .status_message
        .as_ref()
        .map(|m| format!(" {}", m))
        .unwrap_or_default();

    // Tool info (only show if not in Select mode)
    let tool_info = if app.current_tool != Tool::Select {
        format!(" {}", app.current_tool.name().to_lowercase())
    } else {
        String::new()
    };

    let spans = vec![
        Span::styled(format!(" {} ", mode_name), mode_style),
        Span::styled(tool_info, tool_style),
        Span::raw(format!(
            " {}{}{}{}{}",
            file_name, dirty_marker, char_info, peer_info, status_text
        )),
    ];

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render the help bar
fn render_help_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match &app.mode {
        Mode::Normal => match app.current_tool {
            Tool::Select => {
                "[Space] tools [c] brush [C] color | [d]el [y]ank [p]aste [u]ndo U:redo | [q]uit"
            }
            Tool::Freehand => {
                "[Space] tools [c] brush [C] color | drag to draw | [s] select [Esc] cancel"
            }
            Tool::Text => {
                "[Space] tools [C] color | click to place text | [s] select [Esc] cancel"
            }
            _ => {
                "[Space] tools [v] style [C] color | drag to draw | [s] select [Esc] cancel"
            }
        }
        Mode::TextInput { .. } | Mode::LabelInput { .. } => {
            "type text | [Enter] confirm [Esc] cancel [Backspace] delete"
        }
        Mode::FileSave { .. } | Mode::FileOpen { .. } | Mode::SvgExport { .. } => {
            "type path | [Enter] confirm [Esc] cancel"
        }
        Mode::RecentFiles { .. } => {
            "[j/k] navigate [Enter] open [Esc] cancel"
        }
        Mode::SelectionPopup { .. } => {
            "[hjkl] navigate | release key or [Enter] to select | [Esc] cancel"
        }
    };

    let paragraph =
        Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

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

/// Render label input inside a shape
fn render_label_input(frame: &mut Frame, app: &App, shape_id: ShapeId, text: &str, area: Rect) {
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
                    frame.buffer_mut()[(x, area.y + screen_y)]
                        .set_char(ch)
                        .set_style(label_style);
                }
            }

            // Show blinking cursor
            let cursor_x = screen_x + text_len as u16;
            let cursor_screen_x = area.x + cursor_x;
            let cursor_screen_y = area.y + screen_y;
            if cursor_screen_x < area.x + area.width && (start_x + text_len as i32) < max_x {
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
    let (title, cols, items): (&str, usize, Vec<(String, Option<Color>)>) = match kind {
        PopupKind::Tool => {
            let items: Vec<_> = TOOLS.iter().map(|t| (t.name().to_string(), None)).collect();
            (" Select Tool ", 3, items)
        }
        PopupKind::Color => {
            let items: Vec<_> = COLORS.iter().map(|c| {
                (c.name().to_string(), Some(c.to_ratatui()))
            }).collect();
            (" Select Color ", 4, items)
        }
        PopupKind::Brush => {
            let items: Vec<_> = BRUSHES.iter().map(|&ch| (ch.to_string(), None)).collect();
            (" Select Brush ", 6, items)
        }
    };

    let rows = (items.len() + cols - 1) / cols;

    // Calculate popup size based on grid
    let cell_width: u16 = match kind {
        PopupKind::Tool => 10,  // Tool names are longer
        PopupKind::Color => 8,  // Color names + colored block
        PopupKind::Brush => 3,  // Just the character
    };
    let width = (cols as u16 * cell_width + 2).min(area.width.saturating_sub(4));
    let height = (rows as u16 + 2).min(area.height.saturating_sub(4));
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

    // Draw border
    let block = Block::default()
        .title(title)
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
