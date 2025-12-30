use std::path::PathBuf;

use crate::canvas::{LineStyle, Position, Viewport};
use crate::document::{default_storage_path, Document, ShapeId};
use crate::presence::{CursorActivity, PeerId, PeerPresence, PresenceManager, ToolKind};
use crate::shapes::{resize_shape, ResizeHandle, ShapeKind, ShapeView, SnapPoint};
use crate::recent_files::RecentFiles;

/// Snap distance threshold (in characters)
pub const SNAP_THRESHOLD: i32 = 3;

/// Grid size for snap-to-grid (in characters)
pub const GRID_SIZE: i32 = 5;

/// Available drawing tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Freehand,
    Text,
    Line,
    Arrow,
    Rectangle,
    DoubleBox,
    Diamond,
    Ellipse,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Freehand => "Freehand",
            Tool::Text => "Text",
            Tool::Line => "Line",
            Tool::Arrow => "Arrow",
            Tool::Rectangle => "Rectangle",
            Tool::DoubleBox => "DoubleBox",
            Tool::Diamond => "Diamond",
            Tool::Ellipse => "Ellipse",
        }
    }
}

/// Application mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    TextInput { start_pos: Position, text: String },
    LabelInput { shape_id: ShapeId, text: String },
    FileSave { path: String },
    FileOpen { path: String },
    SvgExport { path: String },
    RecentFiles { selected: usize },
}

/// State for shape drawing (line, rectangle)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeState {
    pub start: Position,
    pub current: Position,
    pub start_snap: Option<Position>,   // Snapped start position
    pub current_snap: Option<Position>, // Snapped current position
    pub start_snap_id: Option<ShapeId>,   // Shape ID snapped to at start
    pub current_snap_id: Option<ShapeId>, // Shape ID snapped to at current
}

/// State for dragging/moving a shape
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragState {
    pub shape_id: ShapeId,
    pub last_mouse: Position,
}

/// State for resizing a shape
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeState {
    pub shape_id: ShapeId,
    pub handle: ResizeHandle,
}

/// Mouse button state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseState {
    pub pressed: bool,
    pub last_pos: Option<Position>,
}

/// Freehand drawing state
#[derive(Debug, Clone, Default)]
pub struct FreehandState {
    pub points: Vec<Position>,
}

/// Main application state
pub struct App {
    /// The automerge document - THE source of truth
    pub doc: Document,
    /// Cached shape view for fast rendering
    pub shape_view: ShapeView,
    /// Currently selected shape
    pub selected: Option<ShapeId>,
    pub viewport: Viewport,
    pub current_tool: Tool,
    pub mode: Mode,
    pub brush_char: char,
    pub line_style: LineStyle,
    pub running: bool,
    pub file_path: Option<PathBuf>,
    pub mouse: MouseState,
    pub shape_state: Option<ShapeState>,
    pub drag_state: Option<DragState>,
    pub resize_state: Option<ResizeState>,
    pub freehand_state: Option<FreehandState>,
    pub status_message: Option<String>,
    pub hover_snap: Option<SnapPoint>,
    pub hover_grid_snap: Option<Position>,
    clipboard: Option<ShapeKind>,
    /// Sync session ticket for sharing
    pub sync_ticket: Option<String>,
    /// Presence manager for remote cursors
    pub presence: Option<PresenceManager>,
    /// Our local peer ID (if syncing)
    pub local_peer_id: Option<PeerId>,
    /// Last cursor position (for presence tracking)
    pub last_cursor_pos: Position,
    /// Whether to show the participant list panel
    pub show_participants: bool,
    /// Whether grid snapping is enabled
    pub grid_enabled: bool,
    /// Recent files manager
    pub recent_files: RecentFiles,
}

impl App {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            doc: Document::new(),
            shape_view: ShapeView::new(),
            selected: None,
            viewport: Viewport::new(width, height),
            current_tool: Tool::Select,
            mode: Mode::Normal,
            brush_char: '*',
            line_style: LineStyle::default(),
            running: true,
            file_path: None,
            mouse: MouseState::default(),
            shape_state: None,
            drag_state: None,
            resize_state: None,
            freehand_state: None,
            status_message: None,
            hover_snap: None,
            hover_grid_snap: None,
            clipboard: None,
            sync_ticket: None,
            presence: None,
            local_peer_id: None,
            last_cursor_pos: Position::new(0, 0),
            show_participants: false,
            grid_enabled: false,
            recent_files: RecentFiles::load(),
        }
    }

    /// Initialize presence manager with local peer ID
    pub fn init_presence(&mut self, local_peer_id: PeerId) {
        self.local_peer_id = Some(local_peer_id);
        self.presence = Some(PresenceManager::new(local_peer_id));
    }

    /// Get current cursor activity for presence
    pub fn current_activity(&self) -> CursorActivity {
        if let Some(ref shape_state) = self.shape_state {
            CursorActivity::Drawing {
                tool: self.tool_to_presence_kind(),
                start: shape_state.start,
                current: shape_state.current,
            }
        } else if let Some(ref drag_state) = self.drag_state {
            CursorActivity::Dragging {
                shape_id: drag_state.shape_id,
            }
        } else if let Some(ref resize_state) = self.resize_state {
            CursorActivity::Resizing {
                shape_id: resize_state.shape_id,
            }
        } else if let Mode::TextInput { start_pos, .. } = &self.mode {
            CursorActivity::Typing { position: *start_pos }
        } else if let Some(id) = self.selected {
            CursorActivity::Selected { shape_id: id }
        } else {
            CursorActivity::Idle
        }
    }

    /// Convert current tool to presence ToolKind
    fn tool_to_presence_kind(&self) -> ToolKind {
        match self.current_tool {
            Tool::Select => ToolKind::Select,
            Tool::Freehand => ToolKind::Freehand,
            Tool::Text => ToolKind::Text,
            Tool::Line => ToolKind::Line,
            Tool::Arrow => ToolKind::Arrow,
            Tool::Rectangle => ToolKind::Rectangle,
            Tool::DoubleBox => ToolKind::DoubleBox,
            Tool::Diamond => ToolKind::Diamond,
            Tool::Ellipse => ToolKind::Ellipse,
        }
    }

    /// Build presence data for broadcasting
    pub fn build_presence(&self, cursor_pos: Position) -> Option<PeerPresence> {
        let peer_id = self.local_peer_id?;
        Some(PeerPresence::new(peer_id, cursor_pos, self.current_activity()))
    }

    /// Copy sync ticket to system clipboard
    pub fn copy_ticket_to_clipboard(&mut self) {
        if let Some(ref ticket) = self.sync_ticket {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Try wl-copy (Wayland) first - spawn and forget
            if let Ok(mut child) = Command::new("wl-copy")
                .arg(ticket)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                // Don't wait, just detach
                std::thread::spawn(move || { let _ = child.wait(); });
                self.set_status(format!("Copied: {}", ticket));
                return;
            }

            // Fall back to xsel
            if let Ok(mut child) = Command::new("xsel")
                .args(["--clipboard", "--input"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let ticket_clone = ticket.clone();
                    std::thread::spawn(move || {
                        let _ = stdin.write_all(ticket_clone.as_bytes());
                        drop(stdin);
                        let _ = child.wait();
                    });
                    self.set_status(format!("Copied: {}", ticket));
                    return;
                }
            }

            self.set_status("Install wl-copy or xsel for clipboard");
        } else {
            self.set_status("No sync session active");
        }
    }

    /// Rebuild shape view from document (call after mutations)
    fn rebuild_view(&mut self) {
        if let Err(e) = self.shape_view.rebuild(&self.doc) {
            self.set_status(format!("Error rebuilding view: {}", e));
        }
    }

    /// Check if document is dirty
    pub fn is_dirty(&self) -> bool {
        self.doc.is_dirty()
    }

    /// Save document to storage if dirty
    pub fn autosave(&mut self) {
        if self.doc.is_dirty() {
            if let Err(e) = self.doc.save() {
                self.set_status(format!("Autosave error: {}", e));
            }
        }
    }

    /// Set a status message to display
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Save current state for undo (global, synced via CRDT)
    fn save_undo_state(&mut self) {
        if let Err(e) = self.doc.push_undo_checkpoint() {
            self.set_status(format!("Undo checkpoint error: {}", e));
        }
    }

    /// Undo the last action (global, synced via CRDT)
    pub fn undo(&mut self) {
        match self.doc.global_undo() {
            Ok(true) => {
                self.rebuild_view();
                self.set_status("Undo");
            }
            Ok(false) => {
                self.set_status("Nothing to undo");
            }
            Err(e) => {
                self.set_status(format!("Undo error: {}", e));
            }
        }
    }

    /// Redo the last undone action (global, synced via CRDT)
    pub fn redo(&mut self) {
        match self.doc.global_redo() {
            Ok(true) => {
                self.rebuild_view();
                self.set_status("Redo");
            }
            Ok(false) => {
                self.set_status("Nothing to redo");
            }
            Err(e) => {
                self.set_status(format!("Redo error: {}", e));
            }
        }
    }

    /// Copy selected shape to clipboard
    pub fn yank(&mut self) {
        if let Some(id) = self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                self.clipboard = Some(shape.kind.clone());
                self.set_status("Yanked shape");
            }
        }
    }

    /// Paste shape from clipboard
    pub fn paste(&mut self) {
        if let Some(kind) = self.clipboard.clone() {
            self.save_undo_state();
            let new_kind = kind.translated(2, 1);
            if let Ok(id) = self.doc.add_shape(new_kind) {
                self.rebuild_view();
                self.selected = Some(id);
                self.doc.mark_dirty();
                self.set_status("Pasted shape");
            }
        }
    }

    /// Switch to a different tool
    pub fn set_tool(&mut self, tool: Tool) {
        // If we're in text input mode, commit the text first
        if let Mode::TextInput { .. } = &self.mode {
            self.commit_text();
        }
        // Cancel any shape/drag/resize in progress
        self.shape_state = None;
        self.drag_state = None;
        self.resize_state = None;
        self.freehand_state = None;
        self.current_tool = tool;
        self.mode = Mode::Normal;
        self.set_status(format!("Tool: {}", tool.name()));
    }

    /// Commit current text input as a shape
    pub fn commit_text(&mut self) {
        // Extract values before borrowing self mutably
        let text_data = if let Mode::TextInput { start_pos, text } = &self.mode {
            if !text.is_empty() {
                Some((*start_pos, text.clone()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((pos, content)) = text_data {
            self.save_undo_state();
            if self.doc.add_shape(ShapeKind::Text { pos, content }).is_ok() {
                self.rebuild_view();
                self.doc.mark_dirty();
            }
        }
        self.mode = Mode::Normal;
    }

    /// Start freehand drawing
    pub fn start_freehand(&mut self, pos: Position) {
        self.freehand_state = Some(FreehandState { points: vec![pos] });
    }

    /// Continue freehand drawing
    pub fn continue_freehand(&mut self, pos: Position) {
        if let Some(ref mut state) = self.freehand_state {
            // Add intermediate points for smooth lines
            if let Some(&last) = state.points.last() {
                let points = crate::canvas::line_points(last, pos);
                for p in points.into_iter().skip(1) {
                    state.points.push(p);
                }
            } else {
                state.points.push(pos);
            }
        }
    }

    /// Finish freehand drawing and create shape
    pub fn finish_freehand(&mut self) {
        if let Some(state) = self.freehand_state.take() {
            if !state.points.is_empty() {
                self.save_undo_state();
                if self.doc.add_shape(ShapeKind::Freehand {
                    points: state.points,
                    char: self.brush_char,
                }).is_ok() {
                    self.rebuild_view();
                    self.doc.mark_dirty();
                }
            }
        }
    }

    /// Start drawing a shape (line or rectangle)
    pub fn start_shape(&mut self, pos: Position) {
        // Check for snap point at start (shape snap takes priority)
        let shape_snap = self.shape_view.find_snap_point(pos, SNAP_THRESHOLD);
        let grid_snap = if shape_snap.is_none() {
            self.find_grid_snap(pos, SNAP_THRESHOLD)
        } else {
            None
        };

        let (start_snap, start_snap_id) = if let Some(snap) = shape_snap {
            (Some(snap.pos), Some(snap.shape_id))
        } else if let Some(grid_pos) = grid_snap {
            (Some(grid_pos), None)
        } else {
            (None, None)
        };

        self.shape_state = Some(ShapeState {
            start: pos,
            current: pos,
            start_snap,
            current_snap: None,
            start_snap_id,
            current_snap_id: None,
        });
        self.hover_snap = shape_snap;
        self.hover_grid_snap = grid_snap;
    }

    /// Update shape preview position
    pub fn update_shape(&mut self, pos: Position) {
        // Check for snap point at current position (shape snap takes priority)
        let shape_snap = self.shape_view.find_snap_point(pos, SNAP_THRESHOLD);
        let grid_snap = if shape_snap.is_none() {
            self.find_grid_snap(pos, SNAP_THRESHOLD)
        } else {
            None
        };

        self.hover_snap = shape_snap;
        self.hover_grid_snap = grid_snap;

        if let Some(ref mut state) = self.shape_state {
            state.current = pos;
            // Use shape snap if available, otherwise grid snap
            if let Some(snap) = shape_snap {
                state.current_snap = Some(snap.pos);
                state.current_snap_id = Some(snap.shape_id);
            } else if let Some(grid_pos) = grid_snap {
                state.current_snap = Some(grid_pos);
                state.current_snap_id = None; // No connection for grid snap
            } else {
                state.current_snap = None;
                state.current_snap_id = None;
            }
        }
    }

    /// Commit the current shape
    pub fn commit_shape(&mut self) {
        self.hover_snap = None;
        self.hover_grid_snap = None;

        if let Some(state) = self.shape_state.take() {
            // Use snapped positions if available
            let start = state.start_snap.unwrap_or(state.start);
            let end = state.current_snap.unwrap_or(state.current);

            // Convert ShapeId to u64 for connection tracking
            let start_conn = state.start_snap_id.map(|id| id.0.as_u128() as u64);
            let current_conn = state.current_snap_id.map(|id| id.0.as_u128() as u64);

            self.save_undo_state();
            let result = match self.current_tool {
                Tool::Line => {
                    self.doc.add_shape(ShapeKind::Line {
                        start,
                        end,
                        style: self.line_style,
                        start_connection: start_conn,
                        end_connection: current_conn,
                    })
                }
                Tool::Arrow => {
                    self.doc.add_shape(ShapeKind::Arrow {
                        start,
                        end,
                        style: self.line_style,
                        start_connection: start_conn,
                        end_connection: current_conn,
                    })
                }
                Tool::Rectangle => {
                    self.doc.add_shape(ShapeKind::Rectangle {
                        start,
                        end,
                        label: None,
                    })
                }
                Tool::DoubleBox => {
                    self.doc.add_shape(ShapeKind::DoubleBox {
                        start,
                        end,
                        label: None,
                    })
                }
                Tool::Diamond => {
                    // Diamond uses center + half dimensions
                    let center = start;
                    let half_width = (end.x - start.x).abs().max(1);
                    let half_height = (end.y - start.y).abs().max(1);
                    self.doc.add_shape(ShapeKind::Diamond {
                        center,
                        half_width,
                        half_height,
                        label: None,
                    })
                }
                Tool::Ellipse => {
                    // Ellipse uses center + radii
                    let center = start;
                    let radius_x = (end.x - start.x).abs().max(1);
                    let radius_y = (end.y - start.y).abs().max(1);
                    self.doc.add_shape(ShapeKind::Ellipse {
                        center,
                        radius_x,
                        radius_y,
                        label: None,
                    })
                }
                _ => return,
            };

            if result.is_ok() {
                self.rebuild_view();
                self.doc.mark_dirty();
            }
        }
    }

    /// Cancel the current shape
    pub fn cancel_shape(&mut self) {
        self.shape_state = None;
        self.drag_state = None;
        self.resize_state = None;
        self.freehand_state = None;
        self.hover_snap = None;
        self.hover_grid_snap = None;
    }

    /// Update hover snap point (for when mouse moves without drawing)
    pub fn update_hover_snap(&mut self, pos: Position) {
        let shape_snap = self.shape_view.find_snap_point(pos, SNAP_THRESHOLD);
        let grid_snap = if shape_snap.is_none() {
            self.find_grid_snap(pos, SNAP_THRESHOLD)
        } else {
            None
        };
        self.hover_snap = shape_snap;
        self.hover_grid_snap = grid_snap;
    }

    /// Clear hover snap
    pub fn clear_hover_snap(&mut self) {
        self.hover_snap = None;
        self.hover_grid_snap = None;
    }

    /// Try to select a shape at position
    pub fn try_select(&mut self, pos: Position) -> bool {
        if let Some(id) = self.shape_view.shape_at(pos) {
            self.selected = Some(id);
            self.set_status("Selected shape - drag to move, [Del] to delete");
            true
        } else {
            self.selected = None;
            false
        }
    }

    /// Start dragging the selected shape
    pub fn start_drag(&mut self, pos: Position) {
        if let Some(id) = self.selected {
            self.save_undo_state();
            self.drag_state = Some(DragState {
                shape_id: id,
                last_mouse: pos,
            });
        }
    }

    /// Continue dragging
    pub fn continue_drag(&mut self, pos: Position) {
        let Some(drag) = self.drag_state.as_ref() else {
            return;
        };

        let dx = pos.x - drag.last_mouse.x;
        let dy = pos.y - drag.last_mouse.y;
        let shape_id = drag.shape_id;

        if dx == 0 && dy == 0 {
            return;
        }

        if self.doc.translate_shape(shape_id, dx, dy).is_ok() {
            // Update connected lines
            let _ = self.doc.update_connections_for_shape(shape_id, dx, dy);
            self.rebuild_view();
            if let Some(ref mut drag) = self.drag_state {
                drag.last_mouse = pos;
            }
            self.doc.mark_dirty();
        }
    }

    /// Finish dragging
    pub fn finish_drag(&mut self) {
        self.drag_state = None;
    }

    /// Try to start resizing at a position (returns true if resize handle found)
    pub fn try_start_resize(&mut self, pos: Position) -> bool {
        if let Some(id) = self.selected {
            if let Some(handle) = self.shape_view.find_resize_handle(id, pos, SNAP_THRESHOLD) {
                self.save_undo_state();
                self.resize_state = Some(ResizeState {
                    shape_id: id,
                    handle,
                });
                self.set_status("Resizing shape");
                return true;
            }
        }
        false
    }

    /// Continue resizing
    pub fn continue_resize(&mut self, pos: Position) {
        if let Some(ref resize) = self.resize_state {
            if let Some(shape) = self.shape_view.get(resize.shape_id) {
                let new_kind = resize_shape(&shape.kind, resize.handle, pos);
                if self.doc.update_shape(resize.shape_id, new_kind).is_ok() {
                    self.rebuild_view();
                    self.doc.mark_dirty();
                }
            }
        }
    }

    /// Finish resizing
    pub fn finish_resize(&mut self) {
        self.resize_state = None;
    }

    /// Delete selected shape
    pub fn delete_selected(&mut self) {
        if let Some(id) = self.selected {
            self.save_undo_state();
            if self.doc.delete_shape(id).is_ok() {
                self.selected = None;
                self.rebuild_view();
                self.doc.mark_dirty();
                self.set_status("Deleted shape");
            }
        }
    }

    /// Start text input at a position
    pub fn start_text_input(&mut self, pos: Position) {
        self.mode = Mode::TextInput {
            start_pos: pos,
            text: String::new(),
        };
    }

    /// Add a character to current text input
    pub fn add_text_char(&mut self, ch: char) {
        if let Mode::TextInput { text, .. } = &mut self.mode {
            text.push(ch);
        }
    }

    /// Remove last character from text input
    pub fn backspace_text(&mut self) {
        if let Mode::TextInput { text, .. } = &mut self.mode {
            text.pop();
        }
    }

    /// Enter file save mode
    pub fn start_save(&mut self) {
        let initial_path = self
            .file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "drawing.txt".to_string());
        self.mode = Mode::FileSave { path: initial_path };
    }

    /// Enter file open mode
    pub fn start_open(&mut self) {
        self.mode = Mode::FileOpen {
            path: String::new(),
        };
    }

    /// Enter SVG export mode
    pub fn start_svg_export(&mut self) {
        let initial_path = self
            .file_path
            .as_ref()
            .map(|p| {
                let mut path = p.clone();
                path.set_extension("svg");
                path.to_string_lossy().to_string()
            })
            .unwrap_or_else(|| "drawing.svg".to_string());
        self.mode = Mode::SvgExport { path: initial_path };
    }

    /// Add character to current path input
    pub fn add_path_char(&mut self, ch: char) {
        match &mut self.mode {
            Mode::FileSave { path } | Mode::FileOpen { path } | Mode::SvgExport { path } => {
                path.push(ch);
            }
            _ => {}
        }
    }

    /// Remove last character from path input
    pub fn backspace_path(&mut self) {
        match &mut self.mode {
            Mode::FileSave { path } | Mode::FileOpen { path } | Mode::SvgExport { path } => {
                path.pop();
            }
            _ => {}
        }
    }

    /// Cycle through brush characters
    pub fn cycle_brush(&mut self) {
        const BRUSHES: &[char] = &['*', '#', '@', '+', '.', 'o', 'x', '█', '░', '▒', '▓'];
        let current_idx = BRUSHES.iter().position(|&c| c == self.brush_char);
        let next_idx = match current_idx {
            Some(i) => (i + 1) % BRUSHES.len(),
            None => 0,
        };
        self.brush_char = BRUSHES[next_idx];
        self.set_status(format!("Brush: '{}'", self.brush_char));
    }

    /// Cycle through line styles
    pub fn cycle_line_style(&mut self) {
        self.line_style = self.line_style.next();
        self.set_status(format!("Line style: {}", self.line_style.name()));
    }

    /// Start label input for the selected shape
    pub fn start_label_input(&mut self) -> bool {
        if let Some(id) = self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                if shape.supports_label() {
                    let existing_label = shape.label().unwrap_or("").to_string();
                    self.mode = Mode::LabelInput {
                        shape_id: id,
                        text: existing_label,
                    };
                    return true;
                }
            }
        }
        false
    }

    /// Add a character to current label input
    pub fn add_label_char(&mut self, ch: char) {
        if let Mode::LabelInput { text, .. } = &mut self.mode {
            text.push(ch);
        }
    }

    /// Remove last character from label input
    pub fn backspace_label(&mut self) {
        if let Mode::LabelInput { text, .. } = &mut self.mode {
            text.pop();
        }
    }

    /// Commit the label to the shape
    pub fn commit_label(&mut self) {
        // Extract values before borrowing self mutably
        let label_data = if let Mode::LabelInput { shape_id, text } = &self.mode {
            let label = if text.is_empty() {
                None
            } else {
                Some(text.clone())
            };
            Some((*shape_id, label))
        } else {
            None
        };

        if let Some((shape_id, label)) = label_data {
            self.save_undo_state();
            if let Some(shape) = self.shape_view.get(shape_id) {
                let new_kind = shape.kind.clone().with_label(label);
                if self.doc.update_shape(shape_id, new_kind).is_ok() {
                    self.rebuild_view();
                    self.doc.mark_dirty();
                }
            }
        }
        self.mode = Mode::Normal;
    }

    /// Get the automerge document for syncing
    pub fn automerge(&self) -> &automerge::Automerge {
        self.doc.automerge()
    }

    /// Clone the automerge document for syncing
    pub fn clone_automerge(&self) -> automerge::Automerge {
        self.doc.clone_automerge()
    }

    /// Merge remote changes and rebuild view
    pub fn merge_remote(&mut self, other: &mut automerge::Automerge) {
        if let Err(e) = self.doc.merge(other) {
            self.set_status(format!("Merge error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Synced with peer");
    }

    /// Toggle grid snapping
    pub fn toggle_grid(&mut self) {
        self.grid_enabled = !self.grid_enabled;
        self.set_status(if self.grid_enabled { "Grid: ON" } else { "Grid: OFF" });
    }

    /// Create a new empty document
    pub fn new_document(&mut self) {
        self.doc = Document::new();
        self.doc.set_storage_path(default_storage_path());
        self.shape_view = ShapeView::new();
        self.selected = None;
        self.file_path = None;
        // Clear global undo history (it's stored in the new doc already)
        let _ = self.doc.clear_undo_history();
        self.set_status("New document");
    }

    /// Find the nearest grid snap point if within threshold
    pub fn find_grid_snap(&self, pos: Position, threshold: i32) -> Option<Position> {
        if !self.grid_enabled {
            return None;
        }
        let snapped = snap_to_grid(pos);
        let dist = (pos.x - snapped.x).abs() + (pos.y - snapped.y).abs();
        if dist <= threshold {
            Some(snapped)
        } else {
            None
        }
    }
}

/// Snap a position to the nearest grid point
fn snap_to_grid(pos: Position) -> Position {
    Position {
        x: ((pos.x as f32 / GRID_SIZE as f32).round() as i32) * GRID_SIZE,
        y: ((pos.y as f32 / GRID_SIZE as f32).round() as i32) * GRID_SIZE,
    }
}
