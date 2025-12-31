use std::collections::HashSet;
use std::path::PathBuf;

use crate::canvas::{LineStyle, Position, Viewport};
use crate::document::{default_storage_path, Document, Group, GroupId, ShapeId};
use crate::layers::{Layer, LayerId};
use crate::presence::{CursorActivity, PeerId, PeerPresence, PresenceManager, ToolKind};
use crate::shapes::{resize_shape, ResizeHandle, ShapeColor, ShapeKind, ShapeView, SnapPoint};
use crate::recent_files::RecentFiles;

/// Snap distance threshold (in characters)
pub const SNAP_THRESHOLD: i32 = 3;

/// Grid size for snap-to-grid (in characters)
pub const GRID_SIZE: i32 = 5;

/// Available brush characters for freehand drawing
pub const BRUSHES: &[char] = &[
    '*', '#', '@', '+', '.', 'o', 'x', 'O', '~',
    // Full and shade blocks
    '█', '░', '▒', '▓',
    // Half blocks
    '▀', '▄', '▌', '▐',
    // Quadrant blocks
    '▖', '▗', '▘', '▝',
    // Shapes
    '●', '○', '■', '□', '◆', '◇', '▪', '▫',
];

/// All available tools in order
pub const TOOLS: &[Tool] = &[
    Tool::Select,
    Tool::Freehand,
    Tool::Text,
    Tool::Line,
    Tool::Arrow,
    Tool::Rectangle,
    Tool::DoubleBox,
    Tool::Diamond,
    Tool::Ellipse,
    Tool::Triangle,
    Tool::Parallelogram,
    Tool::Hexagon,
    Tool::Trapezoid,
    Tool::RoundedRect,
    Tool::Cylinder,
    Tool::Cloud,
    Tool::Star,
];

/// All available colors in order
pub const COLORS: &[ShapeColor] = &[
    ShapeColor::White,
    ShapeColor::Red,
    ShapeColor::Green,
    ShapeColor::Yellow,
    ShapeColor::Blue,
    ShapeColor::Magenta,
    ShapeColor::Cyan,
    ShapeColor::Gray,
    ShapeColor::DarkGray,
    ShapeColor::LightRed,
    ShapeColor::LightGreen,
    ShapeColor::LightYellow,
    ShapeColor::LightBlue,
    ShapeColor::LightMagenta,
    ShapeColor::LightCyan,
    ShapeColor::Black,
];

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
    Triangle,
    Parallelogram,
    Hexagon,
    Trapezoid,
    RoundedRect,
    Cylinder,
    Cloud,
    Star,
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
            Tool::Triangle => "Triangle",
            Tool::Parallelogram => "Parallelogram",
            Tool::Hexagon => "Hexagon",
            Tool::Trapezoid => "Trapezoid",
            Tool::RoundedRect => "RoundedRect",
            Tool::Cylinder => "Cylinder",
            Tool::Cloud => "Cloud",
            Tool::Star => "Star",
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
    SelectionPopup { kind: PopupKind, selected: usize },
}

/// Kind of popup selection window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    Tool,
    Color,
    Brush,
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

/// State for marquee selection drag
#[derive(Debug, Clone, Copy)]
pub struct MarqueeState {
    pub start: Position,
    pub current: Position,
}

/// Main application state
pub struct App {
    /// The automerge document - THE source of truth
    pub doc: Document,
    /// Cached shape view for fast rendering
    pub shape_view: ShapeView,
    /// Currently selected shapes (supports multi-select)
    pub selected: HashSet<ShapeId>,
    pub viewport: Viewport,
    pub current_tool: Tool,
    pub mode: Mode,
    pub brush_char: char,
    pub line_style: LineStyle,
    pub current_color: ShapeColor,
    pub running: bool,
    pub file_path: Option<PathBuf>,
    pub mouse: MouseState,
    pub shape_state: Option<ShapeState>,
    pub drag_state: Option<DragState>,
    pub resize_state: Option<ResizeState>,
    pub freehand_state: Option<FreehandState>,
    /// State for marquee selection
    pub marquee_state: Option<MarqueeState>,
    pub status_message: Option<String>,
    pub hover_snap: Option<SnapPoint>,
    pub hover_grid_snap: Option<Position>,
    /// Clipboard for copy/paste (supports multiple shapes)
    clipboard: Vec<ShapeKind>,
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
    /// Active layer for new shapes
    pub active_layer: Option<LayerId>,
    /// Whether to show the layer panel
    pub show_layers: bool,
}

impl App {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            doc: Document::new(),
            shape_view: ShapeView::new(),
            selected: HashSet::new(),
            viewport: Viewport::new(width, height),
            current_tool: Tool::Select,
            mode: Mode::Normal,
            brush_char: '*',
            line_style: LineStyle::default(),
            current_color: ShapeColor::default(),
            running: true,
            file_path: None,
            mouse: MouseState::default(),
            shape_state: None,
            drag_state: None,
            resize_state: None,
            freehand_state: None,
            marquee_state: None,
            status_message: None,
            hover_snap: None,
            hover_grid_snap: None,
            clipboard: Vec::new(),
            sync_ticket: None,
            presence: None,
            local_peer_id: None,
            last_cursor_pos: Position::new(0, 0),
            show_participants: false,
            grid_enabled: false,
            recent_files: RecentFiles::load(),
            active_layer: None,
            show_layers: false,
        }
    }

    /// Initialize active layer from document (call after document is loaded)
    pub fn init_active_layer(&mut self) {
        if let Ok(layer_id) = self.doc.get_default_layer() {
            self.active_layer = Some(layer_id);
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
        } else if self.selected.len() == 1 {
            let id = *self.selected.iter().next().unwrap();
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
            Tool::Triangle => ToolKind::Triangle,
            Tool::Parallelogram => ToolKind::Parallelogram,
            Tool::Hexagon => ToolKind::Hexagon,
            Tool::Trapezoid => ToolKind::Trapezoid,
            Tool::RoundedRect => ToolKind::RoundedRect,
            Tool::Cylinder => ToolKind::Cylinder,
            Tool::Cloud => ToolKind::Cloud,
            Tool::Star => ToolKind::Star,
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

    /// Copy selected shapes to clipboard
    pub fn yank(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.clipboard.clear();
        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                self.clipboard.push(shape.kind.clone());
            }
        }
        let count = self.clipboard.len();
        self.set_status(format!(
            "Yanked {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    /// Paste shapes from clipboard
    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.save_undo_state();
        self.selected.clear();
        for kind in self.clipboard.clone() {
            let new_kind = kind.translated(2, 1);
            if let Ok(id) = self.doc.add_shape(new_kind) {
                self.selected.insert(id);
            }
        }
        self.rebuild_view();
        self.doc.mark_dirty();
        let count = self.selected.len();
        self.set_status(format!(
            "Pasted {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
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
            if self.doc.add_shape(ShapeKind::Text { pos, content, color: self.current_color }).is_ok() {
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
                    label: None,
                    color: self.current_color,
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
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Arrow => {
                    self.doc.add_shape(ShapeKind::Arrow {
                        start,
                        end,
                        style: self.line_style,
                        start_connection: start_conn,
                        end_connection: current_conn,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Rectangle => {
                    self.doc.add_shape(ShapeKind::Rectangle {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::DoubleBox => {
                    self.doc.add_shape(ShapeKind::DoubleBox {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
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
                        color: self.current_color,
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
                        color: self.current_color,
                    })
                }
                Tool::Triangle => {
                    // Triangle from start to end, with third point below center
                    let mid_x = (start.x + end.x) / 2;
                    let height = (end.y - start.y).abs().max(1);
                    let p3 = Position::new(mid_x, start.y + height);
                    self.doc.add_shape(ShapeKind::Triangle {
                        p1: start,
                        p2: end,
                        p3,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Parallelogram => {
                    self.doc.add_shape(ShapeKind::Parallelogram {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Hexagon => {
                    let center = start;
                    let radius_x = (end.x - start.x).abs().max(2);
                    let radius_y = (end.y - start.y).abs().max(1);
                    self.doc.add_shape(ShapeKind::Hexagon {
                        center,
                        radius_x,
                        radius_y,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Trapezoid => {
                    self.doc.add_shape(ShapeKind::Trapezoid {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::RoundedRect => {
                    self.doc.add_shape(ShapeKind::RoundedRect {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Cylinder => {
                    self.doc.add_shape(ShapeKind::Cylinder {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Cloud => {
                    self.doc.add_shape(ShapeKind::Cloud {
                        start,
                        end,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Star => {
                    let center = start;
                    let outer_radius = (end.x - start.x).abs().max((end.y - start.y).abs()).max(2);
                    let inner_radius = outer_radius / 2;
                    self.doc.add_shape(ShapeKind::Star {
                        center,
                        outer_radius,
                        inner_radius,
                        label: None,
                        color: self.current_color,
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

    // ========== Selection Methods ==========

    /// Select a single shape (replaces current selection)
    /// If the shape is part of a group, selects all shapes in the group
    pub fn select_single(&mut self, id: ShapeId) {
        self.selected.clear();
        self.selected.insert(id);

        // Expand selection to include all shapes in the same group
        if let Ok(Some(group_id)) = self.doc.get_shape_group(id) {
            if let Ok(all_shapes) = self.doc.get_all_group_shapes(group_id) {
                for shape_id in all_shapes {
                    self.selected.insert(shape_id);
                }
            }
        }

        let count = self.selected.len();
        if count == 1 {
            self.set_status("Selected shape - drag to move, [Del] to delete");
        } else {
            self.set_status(format!("Selected group ({} shapes) - drag to move, [Del] to delete", count));
        }
    }

    /// Toggle shape in selection (for Shift+click)
    pub fn toggle_selection(&mut self, id: ShapeId) {
        if self.selected.contains(&id) {
            self.selected.remove(&id);
        } else {
            self.selected.insert(id);
        }
    }

    /// Clear all selection
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Check if a shape is selected
    pub fn is_selected(&self, id: ShapeId) -> bool {
        self.selected.contains(&id)
    }

    /// Select all shapes within a rectangle
    pub fn select_in_rect(&mut self, min: Position, max: Position) {
        self.selected.clear();
        for shape in self.shape_view.iter() {
            let (sx_min, sy_min, sx_max, sy_max) = shape.bounds();
            // Check if shape bounds intersect with selection rect
            if sx_max >= min.x && sx_min <= max.x && sy_max >= min.y && sy_min <= max.y {
                self.selected.insert(shape.id);
            }
        }
    }

    /// Try to select a shape at position (replaces selection)
    pub fn try_select(&mut self, pos: Position) -> bool {
        if let Some(id) = self.shape_view.shape_at(pos) {
            self.select_single(id);
            true
        } else {
            self.clear_selection();
            false
        }
    }

    // ========== Z-Order Methods ==========

    /// Bring selected shapes to front (top of z-order)
    pub fn bring_to_front(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.bring_to_front(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Brought to front");
    }

    /// Send selected shapes to back (bottom of z-order)
    pub fn send_to_back(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.send_to_back(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Sent to back");
    }

    /// Bring selected shapes forward one level
    pub fn bring_forward(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.bring_forward(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Brought forward");
    }

    /// Send selected shapes backward one level
    pub fn send_backward(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.send_backward(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Sent backward");
    }

    // ========== Group Methods ==========

    /// Group the currently selected shapes
    pub fn group_selection(&mut self) {
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to group");
            return;
        }

        self.save_undo_state();
        let members: Vec<ShapeId> = self.selected.iter().copied().collect();

        match self.doc.create_group(&members, None) {
            Ok(_group_id) => {
                self.rebuild_view();
                self.set_status(format!("Created group with {} shapes", members.len()));
            }
            Err(e) => {
                self.set_status(format!("Error creating group: {}", e));
            }
        }
    }

    /// Ungroup the groups containing selected shapes
    pub fn ungroup_selection(&mut self) {
        if self.selected.is_empty() {
            return;
        }

        self.save_undo_state();

        // Find all groups that contain selected shapes
        let mut groups_to_delete = HashSet::new();
        for shape_id in &self.selected {
            if let Ok(Some(group_id)) = self.doc.get_shape_group(*shape_id) {
                groups_to_delete.insert(group_id);
            }
        }

        if groups_to_delete.is_empty() {
            self.set_status("No groups to ungroup");
            return;
        }

        let count = groups_to_delete.len();
        for group_id in groups_to_delete {
            if let Err(e) = self.doc.delete_group(group_id) {
                self.set_status(format!("Error ungrouping: {}", e));
                return;
            }
        }

        self.rebuild_view();
        self.set_status(format!("Ungrouped {} group{}", count, if count == 1 { "" } else { "s" }));
    }

    /// Expand selection to include all shapes in the same groups
    pub fn expand_selection_to_groups(&mut self) {
        let mut expanded = self.selected.clone();

        for shape_id in self.selected.iter() {
            if let Ok(Some(group_id)) = self.doc.get_shape_group(*shape_id) {
                // Get all shapes in this group (including nested)
                if let Ok(all_shapes) = self.doc.get_all_group_shapes(group_id) {
                    for id in all_shapes {
                        expanded.insert(id);
                    }
                }
            }
        }

        self.selected = expanded;
    }

    /// Get groups containing any of the selected shapes
    pub fn get_selected_groups(&self) -> Vec<GroupId> {
        let mut groups = HashSet::new();
        for shape_id in &self.selected {
            if let Ok(Some(group_id)) = self.doc.get_shape_group(*shape_id) {
                groups.insert(group_id);
            }
        }
        groups.into_iter().collect()
    }

    /// Check if a shape is part of any group
    pub fn is_grouped(&self, id: ShapeId) -> bool {
        matches!(self.doc.get_shape_group(id), Ok(Some(_)))
    }

    // ========== Layer Methods ==========

    /// Get all layers
    pub fn get_layers(&self) -> Vec<Layer> {
        self.doc.read_all_layers().unwrap_or_default()
    }

    /// Create a new layer
    pub fn create_layer(&mut self) {
        let layers = self.get_layers();
        let name = format!("Layer {}", layers.len() + 1);

        self.save_undo_state();
        match self.doc.create_layer(&name) {
            Ok(layer_id) => {
                self.active_layer = Some(layer_id);
                self.rebuild_view();
                self.set_status(format!("Created layer: {}", name));
            }
            Err(e) => {
                self.set_status(format!("Error creating layer: {}", e));
            }
        }
    }

    /// Delete the active layer
    pub fn delete_active_layer(&mut self) {
        if let Some(layer_id) = self.active_layer {
            self.save_undo_state();
            match self.doc.delete_layer(layer_id) {
                Ok(()) => {
                    // Set active layer to default
                    self.active_layer = self.doc.get_default_layer().ok();
                    self.rebuild_view();
                    self.set_status("Deleted layer");
                }
                Err(e) => {
                    self.set_status(format!("Error: {}", e));
                }
            }
        }
    }

    /// Select a layer by index (1-based for keyboard shortcuts)
    pub fn select_layer_by_index(&mut self, index: usize) {
        let layers = self.get_layers();
        if index > 0 && index <= layers.len() {
            self.active_layer = Some(layers[index - 1].id);
            self.set_status(format!("Active layer: {}", layers[index - 1].name));
        }
    }

    /// Toggle layer visibility
    pub fn toggle_layer_visibility(&mut self, layer_id: LayerId) {
        if let Ok(Some(layer)) = self.doc.read_layer(layer_id) {
            let new_visible = !layer.visible;
            if let Err(e) = self.doc.set_layer_visible(layer_id, new_visible) {
                self.set_status(format!("Error: {}", e));
                return;
            }
            self.shape_view.set_layer_visible(layer_id, new_visible);
            self.set_status(format!(
                "Layer '{}' {}",
                layer.name,
                if new_visible { "visible" } else { "hidden" }
            ));
        }
    }

    /// Toggle active layer visibility
    pub fn toggle_active_layer_visibility(&mut self) {
        if let Some(layer_id) = self.active_layer {
            self.toggle_layer_visibility(layer_id);
        }
    }

    /// Toggle layer locked state
    pub fn toggle_layer_locked(&mut self, layer_id: LayerId) {
        if let Ok(Some(layer)) = self.doc.read_layer(layer_id) {
            let new_locked = !layer.locked;
            if let Err(e) = self.doc.set_layer_locked(layer_id, new_locked) {
                self.set_status(format!("Error: {}", e));
                return;
            }
            self.set_status(format!(
                "Layer '{}' {}",
                layer.name,
                if new_locked { "locked" } else { "unlocked" }
            ));
        }
    }

    /// Move selection to active layer
    pub fn move_selection_to_active_layer(&mut self) {
        if let Some(layer_id) = self.active_layer {
            if self.selected.is_empty() {
                self.set_status("No shapes selected");
                return;
            }

            self.save_undo_state();
            let count = self.selected.len();
            for &shape_id in &self.selected {
                if let Err(e) = self.doc.set_shape_layer(shape_id, layer_id) {
                    self.set_status(format!("Error: {}", e));
                    return;
                }
            }
            self.rebuild_view();
            self.set_status(format!("Moved {} shape{} to active layer", count, if count == 1 { "" } else { "s" }));
        }
    }

    /// Check if a shape is on a locked layer
    pub fn is_shape_locked(&self, id: ShapeId) -> bool {
        if let Ok(Some(layer_id)) = self.doc.get_shape_layer(id) {
            if let Ok(Some(layer)) = self.doc.read_layer(layer_id) {
                return layer.locked;
            }
        }
        false
    }

    /// Toggle layer panel visibility
    pub fn toggle_layer_panel(&mut self) {
        self.show_layers = !self.show_layers;
        let status = if self.show_layers {
            "Layer panel shown"
        } else {
            "Layer panel hidden"
        };
        self.set_status(status);
    }

    // ========== Marquee Selection Methods ==========

    /// Start marquee selection
    pub fn start_marquee(&mut self, pos: Position) {
        self.marquee_state = Some(MarqueeState {
            start: pos,
            current: pos,
        });
    }

    /// Continue marquee selection
    pub fn continue_marquee(&mut self, pos: Position) {
        if let Some(ref mut state) = self.marquee_state {
            state.current = pos;
        }
    }

    /// Finish marquee selection
    pub fn finish_marquee(&mut self) {
        if let Some(state) = self.marquee_state.take() {
            let min_x = state.start.x.min(state.current.x);
            let max_x = state.start.x.max(state.current.x);
            let min_y = state.start.y.min(state.current.y);
            let max_y = state.start.y.max(state.current.y);
            self.select_in_rect(Position::new(min_x, min_y), Position::new(max_x, max_y));
            let count = self.selected.len();
            if count > 0 {
                self.set_status(format!(
                    "Selected {} shape{}",
                    count,
                    if count == 1 { "" } else { "s" }
                ));
            }
        }
    }

    // ========== Drag Methods ==========

    /// Start dragging the selected shapes
    pub fn start_drag(&mut self, pos: Position) {
        if !self.selected.is_empty() {
            self.save_undo_state();
            self.drag_state = Some(DragState {
                shape_id: ShapeId::default(), // Not used for multi-select
                last_mouse: pos,
            });
        }
    }

    /// Continue dragging all selected shapes
    pub fn continue_drag(&mut self, pos: Position) {
        let Some(drag) = self.drag_state.as_ref() else {
            return;
        };

        let dx = pos.x - drag.last_mouse.x;
        let dy = pos.y - drag.last_mouse.y;

        if dx == 0 && dy == 0 {
            return;
        }

        // Move all selected shapes
        let selected_ids: Vec<_> = self.selected.iter().copied().collect();
        for id in selected_ids {
            if self.doc.translate_shape(id, dx, dy).is_ok() {
                let _ = self.doc.update_connections_for_shape(id, dx, dy);
            }
        }
        self.rebuild_view();
        if let Some(ref mut drag) = self.drag_state {
            drag.last_mouse = pos;
        }
        self.doc.mark_dirty();
    }

    /// Finish dragging
    pub fn finish_drag(&mut self) {
        self.drag_state = None;
    }

    // ========== Resize Methods ==========

    /// Try to start resizing at a position (only works with single selection)
    pub fn try_start_resize(&mut self, pos: Position) -> bool {
        // Only allow resize with single selection
        if self.selected.len() != 1 {
            return false;
        }
        let id = *self.selected.iter().next().unwrap();
        if let Some(handle) = self.shape_view.find_resize_handle(id, pos, SNAP_THRESHOLD) {
            self.save_undo_state();
            self.resize_state = Some(ResizeState { shape_id: id, handle });
            self.set_status("Resizing shape");
            return true;
        }
        false
    }

    /// Continue resizing
    pub fn continue_resize(&mut self, pos: Position) {
        if let Some(ref resize) = self.resize_state {
            if let Some(shape) = self.shape_view.get(resize.shape_id) {
                let old_kind = shape.kind.clone();
                let new_kind = resize_shape(&old_kind, resize.handle, pos);
                let shape_id = resize.shape_id;
                if self.doc.update_shape(shape_id, new_kind.clone()).is_ok() {
                    // Update connected lines to follow the resized shape's snap points
                    let _ = self.doc.update_connections_for_resize(shape_id, &old_kind, &new_kind);
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

    /// Delete all selected shapes
    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<_> = self.selected.iter().copied().collect();
        for id in ids {
            let _ = self.doc.delete_shape(id);
        }
        let count = self.selected.len();
        self.selected.clear();
        self.rebuild_view();
        self.doc.mark_dirty();
        self.set_status(format!(
            "Deleted {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
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

    /// Cycle through colors
    pub fn cycle_color(&mut self) {
        self.current_color = self.current_color.next();
        self.set_status(format!("Color: {}", self.current_color.name()));
    }

    /// Start label input for the selected shape (only works with single selection)
    pub fn start_label_input(&mut self) -> bool {
        if self.selected.len() == 1 {
            let id = *self.selected.iter().next().unwrap();
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
        self.selected.clear();
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

    // ========== Popup Selection Methods ==========

    /// Open the tool selection popup
    pub fn open_tool_popup(&mut self) {
        let selected = TOOLS.iter().position(|&t| t == self.current_tool).unwrap_or(0);
        self.mode = Mode::SelectionPopup {
            kind: PopupKind::Tool,
            selected,
        };
    }

    /// Open the color selection popup
    pub fn open_color_popup(&mut self) {
        let selected = COLORS.iter().position(|&c| c == self.current_color).unwrap_or(0);
        self.mode = Mode::SelectionPopup {
            kind: PopupKind::Color,
            selected,
        };
    }

    /// Open the brush character selection popup
    pub fn open_brush_popup(&mut self) {
        let selected = BRUSHES.iter().position(|&c| c == self.brush_char).unwrap_or(0);
        self.mode = Mode::SelectionPopup {
            kind: PopupKind::Brush,
            selected,
        };
    }

    /// Confirm the current popup selection
    pub fn confirm_popup_selection(&mut self) {
        if let Mode::SelectionPopup { kind, selected } = self.mode {
            match kind {
                PopupKind::Tool => {
                    if let Some(&tool) = TOOLS.get(selected) {
                        self.current_tool = tool;
                        self.set_status(format!("Tool: {}", tool.name()));
                    }
                }
                PopupKind::Color => {
                    if let Some(&color) = COLORS.get(selected) {
                        self.current_color = color;
                        self.set_status(format!("Color: {}", color.name()));
                    }
                }
                PopupKind::Brush => {
                    if let Some(&brush) = BRUSHES.get(selected) {
                        self.brush_char = brush;
                        self.set_status(format!("Brush: '{}'", brush));
                    }
                }
            }
        }
        self.mode = Mode::Normal;
    }

    /// Cancel the popup without changing selection
    pub fn cancel_popup(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Navigate within the popup selection grid
    pub fn popup_navigate(&mut self, dx: i32, dy: i32) {
        if let Mode::SelectionPopup { kind, ref mut selected } = self.mode {
            let (cols, total) = match kind {
                PopupKind::Tool => (3, TOOLS.len()),   // 3x3 grid for 9 tools
                PopupKind::Color => (4, COLORS.len()), // 4x4 grid for 16 colors
                PopupKind::Brush => (6, BRUSHES.len()), // 6 columns for brushes
            };
            let rows = (total + cols - 1) / cols;
            let row = *selected / cols;
            let col = *selected % cols;
            let new_col = (col as i32 + dx).clamp(0, cols as i32 - 1) as usize;
            let new_row = (row as i32 + dy).clamp(0, rows as i32 - 1) as usize;
            let new_selected = new_row * cols + new_col;
            *selected = new_selected.min(total - 1);
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
