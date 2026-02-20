mod alignment;
mod clipboard;
mod transform;
mod zorder;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

use crate::canvas::{LineStyle, Position, Viewport};

// Re-export Mode and state types from the modes module
#[allow(unused_imports)]
pub use crate::modes::{
    ConfirmDialogState, HelpScreenState, KeyboardShapeState, LabelInputState, LayerRenameState,
    Mode, PathInputKind, PathInputState, SelectionPopupState, SessionBrowserState, TextInputState,
};
use crate::document::{Document, GroupId, ShapeId, default_storage_path};
use crate::layers::{Layer, LayerId};
use crate::presence::{CursorActivity, PeerId, PeerPresence, PresenceManager, ToolKind};
use crate::recent_files::RecentFiles;
use crate::shapes::{ResizeHandle, ShapeColor, ShapeKind, ShapeView, SnapPoint, resize_shape};

/// Snap distance threshold (in characters)
pub const SNAP_THRESHOLD: i32 = 3;

/// Grid size for snap-to-grid (in characters)
pub const GRID_SIZE: i32 = 5;

/// Available brush characters for freehand drawing
pub const BRUSHES: &[char] = &[
    '*', '#', '@', '+', '.', 'o', 'x', 'O', '~', // Full and shade blocks
    '█', '░', '▒', '▓', // Half blocks
    '▀', '▄', '▌', '▐', // Quadrant blocks
    '▖', '▗', '▘', '▝', // Shapes
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

/// Field focus for keyboard shape creation dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardShapeField {
    Width,
    Height,
}

/// Kind of popup selection window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    Tool,
    Color,
    Brush,
}

/// Pending action awaiting user confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    DeleteLayer(LayerId),
    NewDocument,
    DeleteSession(String), // Session ID
}

impl PendingAction {
    /// Get the confirmation dialog title
    pub fn title(&self) -> &'static str {
        match self {
            PendingAction::DeleteLayer(_) => "Delete Layer",
            PendingAction::NewDocument => "New Document",
            PendingAction::DeleteSession(_) => "Delete Session",
        }
    }

    /// Get the confirmation dialog message
    pub fn message(&self) -> &'static str {
        match self {
            PendingAction::DeleteLayer(_) => "Delete this layer and all its shapes?",
            PendingAction::NewDocument => "Discard unsaved changes and start new document?",
            PendingAction::DeleteSession(_) => "Permanently delete this session?",
        }
    }
}

/// Severity level for status messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    /// Normal info message - clears on next action
    Info,
    /// Warning message - persists until dismissed
    Warning,
    /// Error message - persists until dismissed
    Error,
}

impl MessageSeverity {
    /// Whether this message should persist (not auto-clear)
    pub fn is_persistent(&self) -> bool {
        matches!(self, MessageSeverity::Warning | MessageSeverity::Error)
    }
}

/// State for shape drawing (line, rectangle)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeState {
    pub start: Position,
    pub current: Position,
    pub start_snap: Option<Position>,     // Snapped start position
    pub current_snap: Option<Position>,   // Snapped current position
    pub start_snap_id: Option<ShapeId>,   // Shape ID snapped to at start
    pub current_snap_id: Option<ShapeId>, // Shape ID snapped to at current
}

/// State for dragging/moving a shape
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragState {
    pub shape_id: ShapeId,
    pub last_mouse: Position,
    pub total_dx: i32,
    pub total_dy: i32,
    pub modified_shapes: Vec<ShapeId>,
    /// When the drag started (epoch ms) for soft lock ordering
    pub started_at_ms: u64,
}

/// State for resizing a shape
#[derive(Debug, Clone)]
pub struct ResizeState {
    pub shape_id: ShapeId,
    pub handle: ResizeHandle,
    pub original_kind: ShapeKind,
    pub modified_shapes: Vec<ShapeId>,
    /// When the resize started (epoch ms) for soft lock ordering
    pub started_at_ms: u64,
    /// Current preview bounds (min, max) for remote ghost rendering
    pub preview_bounds: Option<(Position, Position)>,
}

/// Orientation of a snap guide line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapOrientation {
    Vertical,   // Vertical line (shapes aligned on same x)
    Horizontal, // Horizontal line (shapes aligned on same y)
}

/// Visual snap guide line shown during drag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapGuide {
    pub orientation: SnapOrientation,
    pub position: i32, // x for vertical, y for horizontal
    pub start: i32,    // start of line extent
    pub end: i32,      // end of line extent
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
    pub shape_state: Option<ShapeState>,
    pub drag_state: Option<DragState>,
    pub resize_state: Option<ResizeState>,
    pub freehand_state: Option<FreehandState>,
    /// State for marquee selection
    pub marquee_state: Option<MarqueeState>,
    /// Status message with severity level
    pub status_message: Option<(String, MessageSeverity)>,
    pub hover_snap: Option<SnapPoint>,
    pub hover_grid_snap: Option<Position>,
    /// Clipboard for copy/paste (supports multiple shapes)
    clipboard: Vec<ShapeKind>,
    /// Sync session ticket for sharing
    pub sync_ticket: Option<String>,
    /// Pending cluster connection (set by UI, consumed by main loop)
    pub pending_cluster_ticket: Option<String>,
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
    /// Layer panel area (for mouse click detection)
    pub layer_panel_area: Option<Rect>,
    /// Snap guide lines shown during drag
    pub shape_snap_guides: Vec<SnapGuide>,
    /// Current session ID (if using session management)
    pub current_session: Option<crate::session::SessionId>,
    /// Current session metadata (cached for display)
    pub current_session_meta: Option<crate::session::SessionMeta>,
    /// Cached session list for browser (refreshed on open)
    pub session_list: Vec<crate::session::SessionMeta>,
    /// Session to switch to (set by UI, handled by main loop)
    pub session_to_switch: Option<crate::session::SessionId>,
    /// Session to delete (set by confirm dialog, handled by main loop)
    pub session_to_delete: Option<String>,
    /// New session to create (set by UI, handled by main loop)
    pub session_to_create: Option<String>,
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
            pending_cluster_ticket: None,
            presence: None,
            local_peer_id: None,
            last_cursor_pos: Position::new(0, 0),
            show_participants: false,
            grid_enabled: false,
            recent_files: RecentFiles::load(),
            active_layer: None,
            show_layers: false,
            layer_panel_area: None,
            shape_snap_guides: Vec::new(),
            current_session: None,
            current_session_meta: None,
            session_list: Vec::new(),
            session_to_switch: None,
            session_to_delete: None,
            session_to_create: None,
        }
    }

    /// Initialize active layer from document (call after document is loaded)
    pub fn init_active_layer(&mut self) {
        if let Ok(layer_id) = self.doc.get_default_layer() {
            self.active_layer = Some(layer_id);
        }
    }

    /// Reset UI state when switching sessions (viewport, selection, etc.)
    pub fn reset_session_ui_state(&mut self) {
        // Reset viewport to origin
        self.viewport.offset_x = 0;
        self.viewport.offset_y = 0;
        // Clear selection
        self.selected.clear();
        // Reset tool state
        self.freehand_state = None;
        self.shape_state = None;
        self.marquee_state = None;
        self.drag_state = None;
        self.resize_state = None;
        // Initialize layer from new document
        self.init_active_layer();
        // Reset mode to normal
        self.mode = Mode::Normal;
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
            // Use actual selected shape (drag_state.shape_id is ShapeId::default() for multi-select)
            let shape_id = self
                .selected
                .iter()
                .next()
                .copied()
                .unwrap_or(drag_state.shape_id);
            CursorActivity::Dragging {
                shape_id,
                delta: (drag_state.total_dx, drag_state.total_dy),
            }
        } else if let Some(ref resize_state) = self.resize_state {
            CursorActivity::Resizing {
                shape_id: resize_state.shape_id,
                preview_bounds: resize_state.preview_bounds,
            }
        } else if let Mode::TextInput(state) = &self.mode {
            CursorActivity::Typing {
                position: state.start_pos,
            }
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
        // Include drag/resize start time for soft lock ordering
        let drag_start_ms = self
            .drag_state
            .as_ref()
            .map(|d| d.started_at_ms)
            .or_else(|| self.resize_state.as_ref().map(|r| r.started_at_ms));
        Some(PeerPresence::new(
            peer_id,
            cursor_pos,
            self.current_activity(),
            self.active_layer,
            drag_start_ms,
        ))
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
        if self.doc.is_dirty()
            && let Err(e) = self.doc.save()
        {
            self.set_status(format!("Autosave error: {}", e));
        }
    }

    /// Set a status message to display (Info severity - clears on next action)
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), MessageSeverity::Info));
    }

    /// Set a warning message (persists until dismissed)
    pub fn set_warning(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), MessageSeverity::Warning));
    }

    /// Set an error message (persists until dismissed)
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), MessageSeverity::Error));
    }

    /// Clear the status message (only if it's an Info message)
    pub fn clear_status(&mut self) {
        if let Some((_, severity)) = &self.status_message
            && !severity.is_persistent()
        {
            self.status_message = None;
        }
    }

    /// Dismiss any status message (including persistent ones)
    #[allow(dead_code)]
    pub fn dismiss_status(&mut self) {
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

    /// Add a shape and assign it to the active layer
    /// Returns None if the active layer is locked
    fn add_shape_to_active_layer(&mut self, kind: ShapeKind) -> anyhow::Result<ShapeId> {
        // Check if active layer is locked - prevent creation
        if self.check_active_layer_locked() {
            return Err(anyhow::anyhow!("Layer is locked"));
        }

        // Warn if active layer is hidden (but still allow creation)
        self.check_active_layer_hidden();

        let id = self.doc.add_shape(kind)?;
        if let Some(layer_id) = self.active_layer {
            let _ = self.doc.set_shape_layer(id, layer_id);
        }
        Ok(id)
    }

    /// Switch to a different tool
    pub fn set_tool(&mut self, tool: Tool) {
        // If we're in text input mode, commit the text first
        if let Mode::TextInput(_) = &self.mode {
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
        let text_data = if let Mode::TextInput(state) = &self.mode {
            if !state.text.is_empty() {
                Some((state.start_pos, state.text.clone()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((pos, content)) = text_data {
            self.save_undo_state();
            if self
                .add_shape_to_active_layer(ShapeKind::Text {
                    pos,
                    content,
                    color: self.current_color,
                })
                .is_ok()
            {
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
        if let Some(state) = self.freehand_state.take()
            && !state.points.is_empty()
        {
            self.save_undo_state();
            if self
                .add_shape_to_active_layer(ShapeKind::Freehand {
                    points: state.points,
                    char: self.brush_char,
                    label: None,
                    color: self.current_color,
                })
                .is_ok()
            {
                self.rebuild_view();
                self.doc.mark_dirty();
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
                Tool::Line => self.add_shape_to_active_layer(ShapeKind::Line {
                    start,
                    end,
                    style: self.line_style,
                    start_connection: start_conn,
                    end_connection: current_conn,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Arrow => self.add_shape_to_active_layer(ShapeKind::Arrow {
                    start,
                    end,
                    style: self.line_style,
                    start_connection: start_conn,
                    end_connection: current_conn,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Rectangle => self.add_shape_to_active_layer(ShapeKind::Rectangle {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::DoubleBox => self.add_shape_to_active_layer(ShapeKind::DoubleBox {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Diamond => {
                    // Diamond uses center + half dimensions
                    let center = start;
                    let half_width = (end.x - start.x).abs().max(1);
                    let half_height = (end.y - start.y).abs().max(1);
                    self.add_shape_to_active_layer(ShapeKind::Diamond {
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
                    self.add_shape_to_active_layer(ShapeKind::Ellipse {
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
                    self.add_shape_to_active_layer(ShapeKind::Triangle {
                        p1: start,
                        p2: end,
                        p3,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Parallelogram => self.add_shape_to_active_layer(ShapeKind::Parallelogram {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Hexagon => {
                    let center = start;
                    let radius_x = (end.x - start.x).abs().max(2);
                    let radius_y = (end.y - start.y).abs().max(1);
                    self.add_shape_to_active_layer(ShapeKind::Hexagon {
                        center,
                        radius_x,
                        radius_y,
                        label: None,
                        color: self.current_color,
                    })
                }
                Tool::Trapezoid => self.add_shape_to_active_layer(ShapeKind::Trapezoid {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::RoundedRect => self.add_shape_to_active_layer(ShapeKind::RoundedRect {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Cylinder => self.add_shape_to_active_layer(ShapeKind::Cylinder {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Cloud => self.add_shape_to_active_layer(ShapeKind::Cloud {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }),
                Tool::Star => {
                    let center = start;
                    let outer_radius = (end.x - start.x).abs().max((end.y - start.y).abs()).max(2);
                    let inner_radius = outer_radius / 2;
                    self.add_shape_to_active_layer(ShapeKind::Star {
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
    #[allow(dead_code)]
    pub fn clear_hover_snap(&mut self) {
        self.hover_snap = None;
        self.hover_grid_snap = None;
    }

    /// Start keyboard-based shape creation for the given tool
    pub fn start_keyboard_shape_create(&mut self, tool: Tool) {
        // Only allow for tools that make sense with dimensions
        match tool {
            Tool::Rectangle
            | Tool::DoubleBox
            | Tool::Diamond
            | Tool::Ellipse
            | Tool::Triangle
            | Tool::Parallelogram
            | Tool::Hexagon
            | Tool::Trapezoid
            | Tool::RoundedRect
            | Tool::Cylinder
            | Tool::Cloud
            | Tool::Star => {
                self.mode = Mode::KeyboardShapeCreate(KeyboardShapeState {
                    tool,
                    width: "10".to_string(),
                    height: "5".to_string(),
                    focus: KeyboardShapeField::Width,
                });
            }
            Tool::Line | Tool::Arrow => {
                // Lines use length instead of width/height
                self.mode = Mode::KeyboardShapeCreate(KeyboardShapeState {
                    tool,
                    width: "20".to_string(), // length
                    height: "0".to_string(), // angle offset (0 = horizontal)
                    focus: KeyboardShapeField::Width,
                });
            }
            _ => {
                self.set_warning("This tool doesn't support keyboard creation");
            }
        }
    }

    /// Commit keyboard shape creation - create the shape at viewport center
    pub fn commit_keyboard_shape(&mut self) {
        // Extract values from mode to avoid borrow issues
        let (tool, w, h) = if let Mode::KeyboardShapeCreate(state) = &self.mode {
            let w: i32 = state.width.parse().unwrap_or(10);
            let h: i32 = state.height.parse().unwrap_or(5);
            (state.tool, w, h)
        } else {
            return;
        };

        if w <= 0 || h < 0 {
            self.set_error("Invalid dimensions");
            return;
        }

        // Calculate center of viewport
        let center_x = self.viewport.offset_x + (self.viewport.width as i32 / 2);
        let center_y = self.viewport.offset_y + (self.viewport.height as i32 / 2);

        self.save_undo_state();

        let shape = match tool {
            Tool::Line | Tool::Arrow => {
                // w = length, h = vertical offset
                let start = Position::new(center_x - w / 2, center_y);
                let end = Position::new(center_x + w / 2, center_y + h);
                if tool == Tool::Arrow {
                    ShapeKind::Arrow {
                        start,
                        end,
                        style: self.line_style,
                        start_connection: None,
                        end_connection: None,
                        label: None,
                        color: self.current_color,
                    }
                } else {
                    ShapeKind::Line {
                        start,
                        end,
                        style: self.line_style,
                        start_connection: None,
                        end_connection: None,
                        label: None,
                        color: self.current_color,
                    }
                }
            }
            Tool::Rectangle => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::Rectangle {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::DoubleBox => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::DoubleBox {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Diamond => ShapeKind::Diamond {
                center: Position::new(center_x, center_y),
                half_width: w / 2,
                half_height: h / 2,
                label: None,
                color: self.current_color,
            },
            Tool::Ellipse => ShapeKind::Ellipse {
                center: Position::new(center_x, center_y),
                radius_x: w / 2,
                radius_y: h / 2,
                label: None,
                color: self.current_color,
            },
            Tool::Triangle => {
                // Create isoceles triangle
                let p1 = Position::new(center_x, center_y - h / 2); // top
                let p2 = Position::new(center_x - w / 2, center_y + h / 2); // bottom left
                let p3 = Position::new(center_x + w / 2, center_y + h / 2); // bottom right
                ShapeKind::Triangle {
                    p1,
                    p2,
                    p3,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Parallelogram => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::Parallelogram {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Hexagon => ShapeKind::Hexagon {
                center: Position::new(center_x, center_y),
                radius_x: w / 2,
                radius_y: h / 2,
                label: None,
                color: self.current_color,
            },
            Tool::Trapezoid => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::Trapezoid {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::RoundedRect => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::RoundedRect {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Cylinder => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::Cylinder {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Cloud => {
                let start = Position::new(center_x - w / 2, center_y - h / 2);
                let end = Position::new(center_x + w / 2, center_y + h / 2);
                ShapeKind::Cloud {
                    start,
                    end,
                    label: None,
                    color: self.current_color,
                }
            }
            Tool::Star => ShapeKind::Star {
                center: Position::new(center_x, center_y),
                outer_radius: w / 2,
                inner_radius: h / 2,
                label: None,
                color: self.current_color,
            },
            _ => {
                self.mode = Mode::Normal;
                return;
            }
        };

        if self.add_shape_to_active_layer(shape).is_ok() {
            self.rebuild_view();
            self.doc.mark_dirty();
            self.set_status(format!("Created {} ({}x{})", tool.name(), w, h));
        }
        self.mode = Mode::Normal;
    }

    /// Cancel keyboard shape creation
    pub fn cancel_keyboard_shape(&mut self) {
        if matches!(self.mode, Mode::KeyboardShapeCreate(_)) {
            self.mode = Mode::Normal;
        }
    }

    // ========== Selection Methods ==========

    /// Select a single shape (replaces current selection)
    /// If the shape is part of a group, selects all shapes in the group
    pub fn select_single(&mut self, id: ShapeId) {
        self.selected.clear();
        self.selected.insert(id);

        // Expand selection to include all shapes in the same group
        if let Ok(Some(group_id)) = self.doc.get_shape_group(id)
            && let Ok(all_shapes) = self.doc.get_all_group_shapes(group_id)
        {
            for shape_id in all_shapes {
                self.selected.insert(shape_id);
            }
        }

        // Check if shape is on a locked layer and warn
        if let Some((layer_name, _visible, locked)) = self.get_shape_layer_info(id)
            && locked
        {
            self.set_warning(format!(
                "Shape on locked layer '{}' (read-only)",
                layer_name
            ));
            return;
        }

        let count = self.selected.len();
        if count == 1 {
            self.set_status("Selected shape - drag to move, [Del] to delete");
        } else {
            self.set_status(format!(
                "Selected group ({} shapes) - drag to move, [Del] to delete",
                count
            ));
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

    /// Select all shapes on the active layer (or all visible if no active layer)
    pub fn select_all(&mut self) {
        self.selected.clear();
        for shape in self.shape_view.iter() {
            // Only select shapes on visible layers
            if let Some(active) = self.active_layer {
                if shape.layer_id == Some(active) {
                    self.selected.insert(shape.id);
                }
            } else {
                self.selected.insert(shape.id);
            }
        }
        let count = self.selected.len();
        if count > 0 {
            self.set_status(format!("Selected {} shapes", count));
        } else {
            self.set_status("No shapes to select");
        }
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
    #[allow(dead_code)]
    pub fn try_select(&mut self, pos: Position) -> bool {
        if let Some(id) = self.shape_view.shape_at(pos) {
            self.select_single(id);
            true
        } else {
            self.clear_selection();
            false
        }
    }

    // ========== Alignment Methods ==========

    /// Get bounds of all selected shapes combined
    fn get_selection_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        if self.selected.is_empty() {
            return None;
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                let (sx_min, sy_min, sx_max, sy_max) = shape.bounds();
                min_x = min_x.min(sx_min);
                min_y = min_y.min(sy_min);
                max_x = max_x.max(sx_max);
                max_y = max_y.max(sy_max);
            }
        }
        Some((min_x, min_y, max_x, max_y))
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
        self.set_status(format!(
            "Ungrouped {} group{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    /// Expand selection to include all shapes in the same groups
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
            self.set_status(format!(
                "Moved {} shape{} to active layer",
                count,
                if count == 1 { "" } else { "s" }
            ));
        }
    }

    /// Check if a shape is on a locked layer
    pub fn is_shape_locked(&self, id: ShapeId) -> bool {
        if let Ok(Some(layer_id)) = self.doc.get_shape_layer(id)
            && let Ok(Some(layer)) = self.doc.read_layer(layer_id)
        {
            return layer.locked;
        }
        false
    }

    /// Check if the active layer is hidden and show warning if so
    pub fn check_active_layer_hidden(&mut self) -> bool {
        if let Some(layer_id) = self.active_layer
            && let Ok(Some(layer)) = self.doc.read_layer(layer_id)
            && !layer.visible
        {
            self.set_warning(format!("Active layer '{}' is hidden", layer.name));
            return true;
        }
        false
    }

    /// Check if the active layer is locked and show error if so
    pub fn check_active_layer_locked(&mut self) -> bool {
        if let Some(layer_id) = self.active_layer
            && let Ok(Some(layer)) = self.doc.read_layer(layer_id)
            && layer.locked
        {
            self.set_error(format!("Cannot draw on locked layer '{}'", layer.name));
            return true;
        }
        false
    }

    /// Get info about the layer a shape is on (for warnings)
    pub fn get_shape_layer_info(&self, id: ShapeId) -> Option<(String, bool, bool)> {
        if let Ok(Some(layer_id)) = self.doc.get_shape_layer(id)
            && let Ok(Some(layer)) = self.doc.read_layer(layer_id)
        {
            return Some((layer.name.clone(), layer.visible, layer.locked));
        }
        None
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

    /// Start renaming the active layer
    pub fn start_layer_rename(&mut self) {
        if let Some(layer_id) = self.active_layer
            && let Ok(Some(layer)) = self.doc.read_layer(layer_id)
        {
            self.mode = Mode::LayerRename(LayerRenameState {
                layer_id,
                text: layer.name.clone(),
            });
        }
    }

    /// Add a character to layer rename input
    pub fn add_layer_rename_char(&mut self, ch: char) {
        if let Mode::LayerRename(state) = &mut self.mode {
            state.text.push(ch);
        }
    }

    /// Remove last character from layer rename input
    pub fn backspace_layer_rename(&mut self) {
        if let Mode::LayerRename(state) = &mut self.mode {
            state.text.pop();
        }
    }

    /// Commit layer rename
    pub fn commit_layer_rename(&mut self) {
        let rename_data = if let Mode::LayerRename(state) = &self.mode {
            if !state.text.is_empty() {
                Some((state.layer_id, state.text.clone()))
            } else {
                None
            }
        } else {
            None
        };

        self.mode = Mode::Normal;

        if let Some((layer_id, new_name)) = rename_data {
            if let Err(e) = self.doc.rename_layer(layer_id, &new_name) {
                self.set_status(format!("Error: {}", e));
            } else {
                self.set_status(format!("Renamed layer to '{}'", new_name));
            }
        }
    }

    /// Cancel layer rename
    pub fn cancel_layer_rename(&mut self) {
        self.mode = Mode::Normal;
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
        if self.selected.is_empty() {
            return;
        }

        // Check if any selected shapes are on locked layers
        let all_locked = self.selected.iter().all(|&id| self.is_shape_locked(id));
        if all_locked {
            self.set_error("Cannot move - shapes are on locked layers");
            return;
        }

        // Get current timestamp for soft lock ordering
        let started_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Check if any remote peer is already dragging any of our selected shapes (soft lock)
        if let Some(ref presence_mgr) = self.presence {
            for &id in &self.selected {
                if let Some(peer) = presence_mgr.get_dragger_for_shape(id) {
                    // Another peer is dragging this shape - warn but allow
                    self.set_warning(format!(
                        "{} is already moving this shape",
                        peer.display_name()
                    ));
                    break;
                }
            }
        }

        self.save_undo_state();
        self.drag_state = Some(DragState {
            shape_id: ShapeId::default(), // Not used for multi-select
            last_mouse: pos,
            total_dx: 0,
            total_dy: 0,
            modified_shapes: Vec::new(),
            started_at_ms,
        });
    }

    /// Find shape-to-shape snap adjustments during drag
    /// Returns adjusted (dx, dy) and populates shape_snap_guides
    fn find_shape_snap(&mut self, raw_dx: i32, raw_dy: i32) -> (i32, i32) {
        self.shape_snap_guides.clear();

        if self.selected.is_empty() {
            return (raw_dx, raw_dy);
        }

        // Get combined bounds of all selected shapes
        let mut sel_min_x = i32::MAX;
        let mut sel_min_y = i32::MAX;
        let mut sel_max_x = i32::MIN;
        let mut sel_max_y = i32::MIN;

        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                let (min_x, min_y, max_x, max_y) = shape.bounds();
                sel_min_x = sel_min_x.min(min_x);
                sel_min_y = sel_min_y.min(min_y);
                sel_max_x = sel_max_x.max(max_x);
                sel_max_y = sel_max_y.max(max_y);
            }
        }

        if sel_min_x == i32::MAX {
            return (raw_dx, raw_dy);
        }

        // Proposed new bounds after drag
        let new_min_x = sel_min_x + raw_dx;
        let new_max_x = sel_max_x + raw_dx;
        let new_min_y = sel_min_y + raw_dy;
        let new_max_y = sel_max_y + raw_dy;
        let new_center_x = (new_min_x + new_max_x) / 2;
        let new_center_y = (new_min_y + new_max_y) / 2;

        let mut snap_dx: Option<i32> = None;
        let mut snap_dy: Option<i32> = None;
        let mut best_dist_x = SNAP_THRESHOLD + 1;
        let mut best_dist_y = SNAP_THRESHOLD + 1;

        // Check against all non-selected shapes
        for shape in self.shape_view.iter() {
            if self.selected.contains(&shape.id) {
                continue;
            }

            let (other_min_x, other_min_y, other_max_x, other_max_y) = shape.bounds();
            let other_center_x = (other_min_x + other_max_x) / 2;
            let other_center_y = (other_min_y + other_max_y) / 2;

            // Check horizontal alignments (x-axis snapping)
            let x_checks = [
                (new_min_x, other_min_x, "left-left"),
                (new_min_x, other_max_x, "left-right"),
                (new_max_x, other_min_x, "right-left"),
                (new_max_x, other_max_x, "right-right"),
                (new_center_x, other_center_x, "center-center"),
            ];

            for (new_x, other_x, _label) in x_checks {
                let dist = (new_x - other_x).abs();
                if dist <= SNAP_THRESHOLD && dist < best_dist_x {
                    best_dist_x = dist;
                    snap_dx = Some(other_x - new_x + raw_dx);
                }
            }

            // Check vertical alignments (y-axis snapping)
            let y_checks = [
                (new_min_y, other_min_y, "top-top"),
                (new_min_y, other_max_y, "top-bottom"),
                (new_max_y, other_min_y, "bottom-top"),
                (new_max_y, other_max_y, "bottom-bottom"),
                (new_center_y, other_center_y, "center-center"),
            ];

            for (new_y, other_y, _label) in y_checks {
                let dist = (new_y - other_y).abs();
                if dist <= SNAP_THRESHOLD && dist < best_dist_y {
                    best_dist_y = dist;
                    snap_dy = Some(other_y - new_y + raw_dy);
                }
            }
        }

        let final_dx = snap_dx.unwrap_or(raw_dx);
        let final_dy = snap_dy.unwrap_or(raw_dy);

        // Generate snap guides for visual feedback
        if snap_dx.is_some() {
            let snapped_x = sel_min_x + final_dx;
            // Find the vertical line position (could be left, right, or center)
            let guide_x = if (snapped_x - sel_min_x - raw_dx).abs() <= SNAP_THRESHOLD {
                snapped_x // left edge snapped
            } else if ((sel_max_x + final_dx) - sel_max_x - raw_dx).abs() <= SNAP_THRESHOLD {
                sel_max_x + final_dx // right edge snapped
            } else {
                (sel_min_x + sel_max_x) / 2 + final_dx // center snapped
            };

            self.shape_snap_guides.push(SnapGuide {
                orientation: SnapOrientation::Vertical,
                position: guide_x,
                start: sel_min_y + final_dy - 2,
                end: sel_max_y + final_dy + 2,
            });
        }

        if snap_dy.is_some() {
            let snapped_y = sel_min_y + final_dy;
            let guide_y = if (snapped_y - sel_min_y - raw_dy).abs() <= SNAP_THRESHOLD {
                snapped_y
            } else if ((sel_max_y + final_dy) - sel_max_y - raw_dy).abs() <= SNAP_THRESHOLD {
                sel_max_y + final_dy
            } else {
                (sel_min_y + sel_max_y) / 2 + final_dy
            };

            self.shape_snap_guides.push(SnapGuide {
                orientation: SnapOrientation::Horizontal,
                position: guide_y,
                start: sel_min_x + final_dx - 2,
                end: sel_max_x + final_dx + 2,
            });
        }

        (final_dx, final_dy)
    }

    /// Continue dragging all selected shapes
    pub fn continue_drag(&mut self, pos: Position) {
        let Some(drag) = self.drag_state.as_ref() else {
            return;
        };

        let raw_dx = pos.x - drag.last_mouse.x;
        let raw_dy = pos.y - drag.last_mouse.y;

        if raw_dx == 0 && raw_dy == 0 {
            return;
        }

        // Apply shape-to-shape snapping
        let (dx, dy) = self.find_shape_snap(raw_dx, raw_dy);

        // Collect all updates from cache (no document reads)
        let selected_ids: Vec<_> = self.selected.iter().copied().collect();

        // Get translated shapes and connected shape updates from cache
        let mut all_updates: Vec<(ShapeId, ShapeKind)> = Vec::new();
        for &id in &selected_ids {
            // Get translated version of selected shape from cache
            if let Some(shape) = self.shape_view.get(id) {
                let new_kind = shape.kind.translated(dx, dy);
                all_updates.push((id, new_kind));
            }
            // Get connected shape updates
            all_updates.extend(self.shape_view.find_connected_updates(id, dx, dy));
        }

        // Collect modified shape IDs for later document update
        let modified_ids: Vec<ShapeId> = all_updates.iter().map(|(id, _)| *id).collect();

        // Update ONLY the cache during drag (no document writes - they're slow!)
        for (id, new_kind) in all_updates {
            self.shape_view.update_shape_kind(id, new_kind);
        }

        // Track cumulative delta, modified shapes, and update mouse position
        // IMPORTANT: Only update last_mouse by the amount actually moved (dx, dy)
        // not by the raw mouse delta. This prevents getting "stuck" on snap points
        // when the snap adjustment reduces movement to 0.
        if let Some(ref mut drag) = self.drag_state {
            drag.total_dx += dx;
            drag.total_dy += dy;
            // Only advance last_mouse by the actual movement, not raw mouse position
            // This allows accumulated mouse movement to eventually escape snap zones
            drag.last_mouse.x += dx;
            drag.last_mouse.y += dy;
            // Track all modified shapes (dedupe happens at finish)
            drag.modified_shapes.extend(modified_ids);
        }
    }

    /// Finish dragging - write final positions to document
    pub fn finish_drag(&mut self) {
        if let Some(drag) = self.drag_state.take()
            && (drag.total_dx != 0 || drag.total_dy != 0)
        {
            // Dedupe modified shapes and write final positions to document
            let mut written = std::collections::HashSet::new();
            for id in drag.modified_shapes {
                if written.insert(id) && let Some(shape) = self.shape_view.get(id) {
                    let _ = self.doc.update_shape(id, shape.kind.clone());
                }
            }
            self.doc.mark_dirty();
        }
        self.shape_snap_guides.clear();
    }

    // ========== Resize Methods ==========

    /// Try to start resizing at a position (only works with single selection)
    pub fn try_start_resize(&mut self, pos: Position) -> bool {
        // Only allow resize with single selection
        if self.selected.len() != 1 {
            return false;
        }
        let id = *self.selected.iter().next().unwrap();

        // Check if shape is on a locked layer
        if self.is_shape_locked(id) {
            self.set_error("Cannot resize - shape is on a locked layer");
            return false;
        }

        if let Some(handle) = self.shape_view.find_resize_handle(id, pos, SNAP_THRESHOLD) {
            // Capture the original kind before resize starts
            let original_kind = self.shape_view.get(id).map(|s| s.kind.clone());
            if let Some(original_kind) = original_kind {
                // Get current timestamp for soft lock ordering
                let started_at_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                // Check if any remote peer is already resizing this shape (soft lock)
                if let Some(ref presence_mgr) = self.presence
                    && let Some(peer) = presence_mgr.get_dragger_for_shape(id)
                {
                    self.set_warning(format!(
                        "{} is already manipulating this shape",
                        peer.display_name()
                    ));
                }

                // Get initial preview bounds
                let (min_x, min_y, max_x, max_y) = original_kind.bounds();
                let preview_bounds =
                    Some((Position::new(min_x, min_y), Position::new(max_x, max_y)));

                self.save_undo_state();
                self.resize_state = Some(ResizeState {
                    shape_id: id,
                    handle,
                    original_kind,
                    modified_shapes: vec![id],
                    started_at_ms,
                    preview_bounds,
                });
                self.set_status("Resizing shape");
                return true;
            }
        }
        false
    }

    /// Continue resizing - updates only cache during resize for performance
    pub fn continue_resize(&mut self, pos: Position) {
        let Some(ref resize) = self.resize_state else {
            return;
        };

        let shape_id = resize.shape_id;
        let handle = resize.handle;
        let original_kind = resize.original_kind.clone();

        // Get current shape from cache to compute new state
        let Some(shape) = self.shape_view.get(shape_id) else {
            return;
        };

        let old_kind = shape.kind.clone();
        let new_kind = resize_shape(&old_kind, handle, pos);

        // Update ONLY the cache during resize (no document writes - they're slow!)
        self.shape_view
            .update_shape_kind(shape_id, new_kind.clone());

        // Find and update connected shapes in cache
        let connected_updates =
            self.shape_view
                .find_connected_updates_for_resize(shape_id, &original_kind, &new_kind);

        let mut modified_ids = vec![shape_id];
        for (id, kind) in connected_updates {
            self.shape_view.update_shape_kind(id, kind);
            modified_ids.push(id);
        }

        // Track all modified shapes and update preview bounds (dedupe happens at finish)
        if let Some(ref mut resize) = self.resize_state {
            resize.modified_shapes.extend(modified_ids);
            // Update preview bounds for presence broadcast
            let (min_x, min_y, max_x, max_y) = new_kind.bounds();
            resize.preview_bounds =
                Some((Position::new(min_x, min_y), Position::new(max_x, max_y)));
        }
    }

    /// Finish resizing - write final positions to document
    pub fn finish_resize(&mut self) {
        if let Some(resize) = self.resize_state.take() {
            // Dedupe modified shapes and write final positions to document
            let mut written = std::collections::HashSet::new();
            for id in resize.modified_shapes {
                if written.insert(id) && let Some(shape) = self.shape_view.get(id) {
                    let _ = self.doc.update_shape(id, shape.kind.clone());
                }
            }
            self.doc.mark_dirty();
        }
    }

    /// Delete all selected shapes
    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }

        // Check if any selected shapes are on locked layers
        let locked_count = self
            .selected
            .iter()
            .filter(|&&id| self.is_shape_locked(id))
            .count();

        if locked_count > 0 {
            if locked_count == self.selected.len() {
                self.set_error("Cannot delete - all selected shapes are on locked layers");
                return;
            } else {
                self.set_warning(format!(
                    "{} shape(s) on locked layers will be skipped",
                    locked_count
                ));
            }
        }

        self.save_undo_state();
        let ids: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|&id| !self.is_shape_locked(id))
            .collect();
        let delete_count = ids.len();
        for id in ids {
            let _ = self.doc.delete_shape(id);
        }
        self.selected.clear();
        self.rebuild_view();
        self.doc.mark_dirty();
        self.set_status(format!(
            "Deleted {} shape{}",
            delete_count,
            if delete_count == 1 { "" } else { "s" }
        ));
    }

    /// Nudge selected shapes by (dx, dy)
    pub fn nudge_selection(&mut self, dx: i32, dy: i32) {
        if self.selected.is_empty() {
            return;
        }

        // Check if any selected shapes are on locked layers
        let locked_count = self
            .selected
            .iter()
            .filter(|&&id| self.is_shape_locked(id))
            .count();

        if locked_count > 0 && locked_count == self.selected.len() {
            self.set_error("Cannot move - all selected shapes are on locked layers");
            return;
        }

        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if !self.is_shape_locked(id) {
                let _ = self.doc.translate_shape(id, dx, dy);
            }
        }
        self.rebuild_view();
        self.doc.mark_dirty();
    }

    /// Start text input at a position
    pub fn start_text_input(&mut self, pos: Position) {
        self.mode = Mode::TextInput(TextInputState {
            start_pos: pos,
            text: String::new(),
        });
    }

    /// Add a character to current text input
    pub fn add_text_char(&mut self, ch: char) {
        if let Mode::TextInput(state) = &mut self.mode {
            state.text.push(ch);
        }
    }

    /// Remove last character from text input
    pub fn backspace_text(&mut self) {
        if let Mode::TextInput(state) = &mut self.mode {
            state.text.pop();
        }
    }

    /// Enter file save mode
    pub fn start_save(&mut self) {
        let initial_path = self
            .file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "drawing.txt".to_string());
        self.mode = Mode::PathInput(PathInputState {
            path: initial_path,
            kind: PathInputKind::FileSave,
        });
    }

    /// Enter file open mode
    pub fn start_open(&mut self) {
        self.mode = Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::FileOpen,
        });
    }

    /// Enter document save mode (saves full .automerge document)
    pub fn start_doc_save(&mut self) {
        let initial_path = self
            .doc
            .storage_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "document.automerge".to_string());
        self.mode = Mode::PathInput(PathInputState {
            path: initial_path,
            kind: PathInputKind::DocSave,
        });
    }

    /// Enter document open mode (opens .automerge document)
    pub fn start_doc_open(&mut self) {
        self.mode = Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::DocOpen,
        });
    }

    /// Add character to current path input
    pub fn add_path_char(&mut self, ch: char) {
        if let Mode::PathInput(state) = &mut self.mode {
            state.path.push(ch);
        }
    }

    /// Remove last character from path input
    pub fn backspace_path(&mut self) {
        if let Mode::PathInput(state) = &mut self.mode {
            state.path.pop();
        }
    }

    /// Tab completion for file paths
    pub fn complete_path(&mut self) {
        let current_path = match &self.mode {
            Mode::PathInput(state) => state.path.clone(),
            _ => return,
        };

        // Parse the current path
        let path = std::path::Path::new(&current_path);

        // Get the directory and prefix to complete
        let (dir, prefix) =
            if current_path.ends_with('/') || current_path.ends_with(std::path::MAIN_SEPARATOR) {
                (std::path::PathBuf::from(&current_path), String::new())
            } else if let Some(parent) = path.parent() {
                let parent_path = if parent.as_os_str().is_empty() {
                    std::path::PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
                let file_name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                (parent_path, file_name)
            } else {
                (std::path::PathBuf::from("."), current_path.clone())
            };

        // List directory and find matches
        let matches: Vec<String> = match std::fs::read_dir(&dir) {
            Ok(entries) => {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                            let full_path = if dir.as_os_str() == "." {
                                name.clone()
                            } else {
                                dir.join(&name).to_string_lossy().to_string()
                            };
                            // Add trailing slash for directories
                            if e.path().is_dir() {
                                Some(full_path + "/")
                            } else {
                                Some(full_path)
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Err(_) => return,
        };

        // If exactly one match, use it
        // If multiple matches, find common prefix
        if matches.is_empty() {
            self.set_status("No matches");
            return;
        }

        let new_path = if matches.len() == 1 {
            matches[0].clone()
        } else {
            // Find longest common prefix
            let first = &matches[0];
            let common_len = matches.iter().skip(1).fold(first.len(), |len, s| {
                first
                    .chars()
                    .zip(s.chars())
                    .take(len)
                    .take_while(|(a, b)| a == b)
                    .count()
            });
            let common: String = first.chars().take(common_len).collect();
            self.set_status(format!("{} matches", matches.len()));
            common
        };

        // Update the path in the current mode
        if let Mode::PathInput(state) = &mut self.mode {
            state.path = new_path;
        }
    }

    /// Cycle through line styles
    pub fn cycle_line_style(&mut self) {
        self.line_style = self.line_style.next();
        self.set_status(format!("Line style: {}", self.line_style.name()));
    }

    /// Start label input for the selected shape (only works with single selection)
    pub fn start_label_input(&mut self) -> bool {
        if self.selected.len() == 1 {
            let id = *self.selected.iter().next().unwrap();
            if let Some(shape) = self.shape_view.get(id)
                && shape.supports_label()
            {
                let existing_label = shape.label().unwrap_or("").to_string();
                let cursor = existing_label.chars().count();
                self.mode = Mode::LabelInput(LabelInputState {
                    shape_id: id,
                    text: existing_label,
                    cursor,
                });
                return true;
            }
        }
        false
    }

    /// Add a character to current label input at cursor position
    pub fn add_label_char(&mut self, ch: char) {
        if let Mode::LabelInput(state) = &mut self.mode {
            // Convert to Vec<char> for proper Unicode handling
            let mut chars: Vec<char> = state.text.chars().collect();
            if state.cursor <= chars.len() {
                chars.insert(state.cursor, ch);
                state.text = chars.into_iter().collect();
                state.cursor += 1;
            }
        }
    }

    /// Remove character before cursor in label input
    pub fn backspace_label(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode
            && state.cursor > 0
        {
            let mut chars: Vec<char> = state.text.chars().collect();
            chars.remove(state.cursor - 1);
            state.text = chars.into_iter().collect();
            state.cursor -= 1;
        }
    }

    /// Move label cursor left
    pub fn move_label_cursor_left(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode
            && state.cursor > 0
        {
            state.cursor -= 1;
        }
    }

    /// Move label cursor right
    pub fn move_label_cursor_right(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode {
            let len = state.text.chars().count();
            if state.cursor < len {
                state.cursor += 1;
            }
        }
    }

    /// Move label cursor to start
    pub fn move_label_cursor_home(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode {
            state.cursor = 0;
        }
    }

    /// Move label cursor to end
    pub fn move_label_cursor_end(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode {
            state.cursor = state.text.chars().count();
        }
    }

    /// Delete character at cursor position (forward delete)
    pub fn delete_label_char(&mut self) {
        if let Mode::LabelInput(state) = &mut self.mode {
            let chars: Vec<char> = state.text.chars().collect();
            if state.cursor < chars.len() {
                let mut chars = chars;
                chars.remove(state.cursor);
                state.text = chars.into_iter().collect();
            }
        }
    }

    /// Commit the label to the shape
    pub fn commit_label(&mut self) {
        // Extract values before borrowing self mutably
        let label_data = if let Mode::LabelInput(state) = &self.mode {
            let label = if state.text.is_empty() {
                None
            } else {
                Some(state.text.clone())
            };
            Some((state.shape_id, label))
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
    #[allow(dead_code)]
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
        self.set_status(if self.grid_enabled {
            "Grid: ON"
        } else {
            "Grid: OFF"
        });
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

    /// Confirm popup selection with explicit kind and index
    pub fn confirm_popup_selection_with_index(&mut self, kind: PopupKind, selected: usize) {
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
                    // Also apply color to selected shapes
                    if !self.selected.is_empty() {
                        let count = self.apply_color_to_selected(color);
                        if count > 0 {
                            self.set_status(format!(
                                "Changed color of {} shape(s) to {}",
                                count,
                                color.name()
                            ));
                        } else {
                            self.set_status(format!("Color: {}", color.name()));
                        }
                    } else {
                        self.set_status(format!("Color: {}", color.name()));
                    }
                }
            }
            PopupKind::Brush => {
                if let Some(&brush) = BRUSHES.get(selected) {
                    self.brush_char = brush;
                    self.set_status(format!("Brush: '{}'", brush));
                }
            }
        }
        self.mode = Mode::Normal;
    }

    /// Apply a color to all selected shapes
    pub fn apply_color_to_selected(&mut self, color: ShapeColor) -> usize {
        if self.selected.is_empty() {
            return 0;
        }

        self.save_undo_state();
        let mut count = 0;

        // Collect shape IDs to avoid borrow issues
        let selected_ids: Vec<ShapeId> = self.selected.iter().copied().collect();

        for id in selected_ids {
            if let Some(shape) = self.shape_view.get(id) {
                let new_kind = shape.kind.clone().with_color(color);
                if self.doc.update_shape(id, new_kind).is_ok() {
                    count += 1;
                }
            }
        }

        if count > 0 {
            self.rebuild_view();
            self.doc.mark_dirty();
        }

        count
    }

    /// Get a description of the selected shape for the status bar
    /// Returns None if no single shape is selected
    pub fn get_selected_shape_info(&self) -> Option<String> {
        if self.selected.len() != 1 {
            return None;
        }

        let id = *self.selected.iter().next()?;
        let shape = self.shape_view.get(id)?;

        let shape_type = shape.kind.type_name();
        let color = shape.kind.color().name();

        // Get layer name
        let layer_info = if let Some(layer_id) = shape.layer_id {
            self.doc
                .read_layer(layer_id)
                .ok()
                .flatten()
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Default".to_string()
        };

        // Check if shape is grouped
        let group_info = self
            .doc
            .get_shape_group(id)
            .ok()
            .flatten()
            .map(|_| " | Grouped".to_string());

        Some(format!(
            "{} | {} | Layer: {}{}",
            shape_type,
            color,
            layer_info,
            group_info.unwrap_or_default()
        ))
    }

    // ========== Confirmation Dialog Methods ==========

    /// Request to delete the active layer (shows confirmation dialog)
    pub fn request_delete_layer(&mut self) {
        if let Some(layer_id) = self.active_layer {
            self.mode = Mode::ConfirmDialog(ConfirmDialogState {
                action: PendingAction::DeleteLayer(layer_id),
            });
        }
    }

    /// Request a new document (shows confirmation if dirty)
    pub fn request_new_document(&mut self) {
        if self.is_dirty() {
            self.mode = Mode::ConfirmDialog(ConfirmDialogState {
                action: PendingAction::NewDocument,
            });
        } else {
            // Not dirty, just create new document directly
            self.new_document();
        }
    }

    /// Confirm and execute the pending action
    pub fn confirm_pending_action(&mut self) {
        let action = if let Mode::ConfirmDialog(state) = &self.mode {
            state.action.clone()
        } else {
            return;
        };

        self.mode = Mode::Normal;

        match action {
            PendingAction::DeleteLayer(layer_id) => {
                // Set active layer to the one being deleted (in case it changed)
                self.active_layer = Some(layer_id);
                self.delete_active_layer();
            }
            PendingAction::NewDocument => {
                self.new_document();
            }
            PendingAction::DeleteSession(session_id) => {
                // Session deletion is handled by main.rs via session_to_delete
                self.session_to_delete = Some(session_id);
            }
        }
    }

    /// Cancel the pending action
    pub fn cancel_pending_action(&mut self) {
        self.mode = Mode::Normal;
        self.set_status("Cancelled");
    }

    // ========== Session Browser Methods ==========

    /// Open the session browser
    pub fn open_session_browser(&mut self, sessions: Vec<crate::session::SessionMeta>) {
        self.session_list = sessions;
        self.mode = Mode::SessionBrowser(SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        });
    }

    /// Get filtered sessions for display
    pub fn get_filtered_sessions(
        &self,
        filter: &str,
        pinned_only: bool,
    ) -> Vec<&crate::session::SessionMeta> {
        let filter_lower = filter.to_lowercase();
        self.session_list
            .iter()
            .filter(|s| {
                if pinned_only && !s.pinned {
                    return false;
                }
                if filter.is_empty() {
                    return true;
                }
                // Simple contains match for display filtering
                s.name.to_lowercase().contains(&filter_lower)
                    || s.id.0.contains(&filter_lower)
                    || s.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&filter_lower))
            })
            .collect()
    }

    /// Refresh session list and clamp selection to valid bounds
    pub fn refresh_session_list(&mut self, sessions: Vec<crate::session::SessionMeta>) {
        self.session_list = sessions;
        // Clamp selection to valid bounds
        // Extract filter values first to avoid borrow checker issues
        let (filter, show_pinned_only) = if let Mode::SessionBrowser(state) = &self.mode {
            (state.filter.clone(), state.show_pinned_only)
        } else {
            return;
        };

        let filtered_len = self.get_filtered_sessions(&filter, show_pinned_only).len();
        if let Mode::SessionBrowser(state) = &mut self.mode {
            if filtered_len == 0 {
                state.selected = 0;
            } else {
                state.selected = state.selected.min(filtered_len - 1);
            }
        }
    }

    /// Get the current session name for display
    #[allow(dead_code)]
    pub fn current_session_name(&self) -> Option<&str> {
        self.current_session_meta.as_ref().map(|m| m.name.as_str())
    }

    // =========================================================================
    // File Operation Methods (used by modes/path_input.rs)
    // =========================================================================

    /// Execute file save (ASCII export)
    pub fn execute_file_save(&mut self, path: &str) {
        use crate::file_io;
        let path_buf = std::path::PathBuf::from(path);
        match file_io::save_ascii(&self.shape_view, &path_buf) {
            Ok(()) => {
                self.recent_files.add(path_buf.clone());
                self.file_path = Some(path_buf);
                self.set_status("Exported!");
            }
            Err(e) => {
                self.set_error(format!("Export error: {}", e));
            }
        }
    }

    /// Execute file open (ASCII import)
    pub fn execute_file_open(&mut self, path: &str) {
        use crate::file_io;
        let path_buf = std::path::PathBuf::from(path);
        match file_io::load_ascii(&path_buf) {
            Ok(shapes) => {
                // Create new document and add shapes
                self.doc = Document::new();
                for kind in shapes {
                    let _ = self.doc.add_shape(kind);
                }
                // Rebuild view
                if let Err(e) = self.shape_view.rebuild(&self.doc) {
                    self.set_error(format!("Error rebuilding view: {}", e));
                } else {
                    self.recent_files.add(path_buf.clone());
                    self.file_path = Some(path_buf);
                    self.set_status("Imported!");
                }
            }
            Err(e) => {
                self.set_error(format!("Import error: {}", e));
            }
        }
    }

    /// Execute document save (native format)
    pub fn execute_doc_save(&mut self, path: &str) {
        let path_buf = std::path::PathBuf::from(path);
        match self.doc.save_to(&path_buf) {
            Ok(()) => {
                self.set_status(format!("Document saved to {}", path_buf.display()));
            }
            Err(e) => {
                self.set_error(format!("Failed to save: {}", e));
            }
        }
    }

    /// Execute document open (native format)
    pub fn execute_doc_open(&mut self, path: &str) {
        let path_buf = std::path::PathBuf::from(path);
        match Document::load(&path_buf) {
            Ok(doc) => {
                self.doc = doc;
                if let Err(e) = self.shape_view.rebuild(&self.doc) {
                    self.set_error(format!("Error rebuilding view: {}", e));
                } else {
                    self.set_status(format!("Document loaded from {}", path_buf.display()));
                }
            }
            Err(e) => {
                self.set_error(format!("Failed to load: {}", e));
            }
        }
    }

    /// Execute SVG export
    pub fn execute_svg_export(&mut self, path: &str) {
        use crate::svg_export;
        let path_buf = std::path::PathBuf::from(path);
        match svg_export::save_svg(&self.shape_view, &path_buf) {
            Ok(()) => {
                self.set_status(format!("Exported to {}", path_buf.display()));
            }
            Err(e) => {
                self.set_error(format!("SVG export error: {}", e));
            }
        }
    }

    pub fn execute_cluster_connect(&mut self, ticket: &str) {
        let ticket = ticket.trim().to_string();
        if ticket.is_empty() {
            self.set_error("No cluster ticket provided");
            return;
        }
        self.pending_cluster_ticket = Some(ticket.clone());
        self.set_status("Connecting to cluster...");
    }

    /// Open a file from the recent files list by index
    pub fn open_recent_file(&mut self, index: usize) {
        use crate::file_io;
        if let Some(file) = self.recent_files.get(index) {
            let path = file.path.clone();
            match file_io::load_ascii(&path) {
                Ok(shapes) => {
                    self.doc = Document::new();
                    for kind in shapes {
                        let _ = self.doc.add_shape(kind);
                    }
                    if let Err(e) = self.shape_view.rebuild(&self.doc) {
                        self.set_error(format!("Error rebuilding view: {}", e));
                    } else {
                        self.file_path = Some(path);
                        self.set_status("Loaded!");
                    }
                }
                Err(e) => {
                    self.set_error(format!("Load error: {}", e));
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        App::new(80, 24)
    }

    // ========== Tool Selection Tests ==========

    #[test]
    fn app_new_defaults() {
        let app = create_test_app();
        assert_eq!(app.current_tool, Tool::Select);
        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.running);
        assert!(app.selected.is_empty());
        assert_eq!(app.brush_char, '*');
        assert!(!app.grid_enabled);
    }

    #[test]
    fn app_set_tool_changes_current_tool() {
        let mut app = create_test_app();
        assert_eq!(app.current_tool, Tool::Select);

        app.set_tool(Tool::Rectangle);
        assert_eq!(app.current_tool, Tool::Rectangle);

        app.set_tool(Tool::Line);
        assert_eq!(app.current_tool, Tool::Line);

        app.set_tool(Tool::Freehand);
        assert_eq!(app.current_tool, Tool::Freehand);
    }

    #[test]
    fn app_set_tool_clears_shape_state() {
        let mut app = create_test_app();
        app.shape_state = Some(ShapeState {
            start: Position::new(0, 0),
            current: Position::new(10, 10),
            start_snap: None,
            current_snap: None,
            start_snap_id: None,
            current_snap_id: None,
        });

        app.set_tool(Tool::Rectangle);
        assert!(app.shape_state.is_none());
    }

    // ========== Selection Tests ==========

    #[test]
    fn app_select_single() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();

        app.select_single(id);
        assert!(app.selected.contains(&id));
        assert_eq!(app.selected.len(), 1);
    }

    #[test]
    fn app_clear_selection() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_single(id);

        app.clear_selection();
        assert!(app.selected.is_empty());
    }

    #[test]
    fn app_toggle_selection() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();

        assert!(!app.selected.contains(&id));
        app.toggle_selection(id);
        assert!(app.selected.contains(&id));
        app.toggle_selection(id);
        assert!(!app.selected.contains(&id));
    }

    #[test]
    fn app_select_all() {
        let mut app = create_test_app();
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(20, 20),
                end: Position::new(30, 30),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();

        app.select_all();
        assert_eq!(app.selected.len(), 2);
    }

    #[test]
    fn app_is_selected() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();

        assert!(!app.is_selected(id));
        app.select_single(id);
        assert!(app.is_selected(id));
    }

    // ========== Status Message Tests ==========

    #[test]
    fn app_set_status() {
        let mut app = create_test_app();
        app.set_status("Test message");
        assert!(app.status_message.is_some());
        let (msg, severity) = app.status_message.as_ref().unwrap();
        assert_eq!(msg, "Test message");
        assert!(matches!(severity, MessageSeverity::Info));
    }

    #[test]
    fn app_set_warning() {
        let mut app = create_test_app();
        app.set_warning("Warning message");
        assert!(app.status_message.is_some());
        let (msg, severity) = app.status_message.as_ref().unwrap();
        assert_eq!(msg, "Warning message");
        assert!(matches!(severity, MessageSeverity::Warning));
    }

    #[test]
    fn app_set_error() {
        let mut app = create_test_app();
        app.set_error("Error message");
        assert!(app.status_message.is_some());
        let (msg, severity) = app.status_message.as_ref().unwrap();
        assert_eq!(msg, "Error message");
        assert!(matches!(severity, MessageSeverity::Error));
    }

    #[test]
    fn app_clear_status() {
        let mut app = create_test_app();
        app.set_status("Test message");
        app.clear_status();
        assert!(app.status_message.is_none());
    }

    #[test]
    fn app_clear_status_does_not_clear_persistent() {
        let mut app = create_test_app();
        app.set_error("Persistent error");
        app.clear_status();
        // Error messages are persistent
        assert!(app.status_message.is_some());
    }

    // ========== Grid Toggle Tests ==========

    #[test]
    fn app_toggle_grid() {
        let mut app = create_test_app();
        assert!(!app.grid_enabled);
        app.toggle_grid();
        assert!(app.grid_enabled);
        app.toggle_grid();
        assert!(!app.grid_enabled);
    }

    // ========== Line Style Tests ==========

    #[test]
    fn app_cycle_line_style() {
        let mut app = create_test_app();
        let initial = app.line_style;
        app.cycle_line_style();
        assert_ne!(app.line_style, initial);
    }

    // ========== Undo/Redo Tests ==========

    #[test]
    fn app_undo_with_no_history() {
        let mut app = create_test_app();
        app.undo(); // Should not panic
        assert!(app.status_message.is_some());
    }

    #[test]
    fn app_redo_with_no_history() {
        let mut app = create_test_app();
        app.redo(); // Should not panic
        assert!(app.status_message.is_some());
    }

    // ========== Clipboard Tests ==========

    #[test]
    fn app_yank_with_no_selection() {
        let mut app = create_test_app();
        app.yank();
        // Should do nothing (no selection)
    }

    #[test]
    fn app_yank_copies_selected_shape() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_single(id);

        app.yank();
        assert!(app.status_message.is_some());
        // Clipboard is private, but the status message confirms yank
    }

    #[test]
    fn app_paste_with_empty_clipboard() {
        let mut app = create_test_app();
        let initial_count = app.shape_view.shape_count();
        app.paste();
        // Paste does nothing with empty clipboard
        assert_eq!(app.shape_view.shape_count(), initial_count);
    }

    // ========== Delete Tests ==========

    #[test]
    fn app_delete_with_no_selection() {
        let mut app = create_test_app();
        app.delete_selected(); // Should not panic
    }

    #[test]
    fn app_delete_removes_selected_shape() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_single(id);

        let initial_count = app.shape_view.shape_count();
        app.delete_selected();
        assert_eq!(app.shape_view.shape_count(), initial_count - 1);
        assert!(app.selected.is_empty());
    }

    // ========== Mode Tests ==========

    #[test]
    fn app_enter_and_exit_mode() {
        let mut app = create_test_app();
        assert!(matches!(app.mode, Mode::Normal));

        // Enter help screen mode
        app.mode = Mode::HelpScreen(HelpScreenState { scroll: 0 });
        assert!(matches!(app.mode, Mode::HelpScreen(_)));

        // Exit back to normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    // ========== Viewport Tests ==========

    #[test]
    fn app_viewport_resize() {
        let mut app = create_test_app();
        app.viewport.resize(100, 50);
        assert_eq!(app.viewport.width, 100);
        assert_eq!(app.viewport.height, 50);
    }

    // ========== Brush/Color Selection Tests ==========

    #[test]
    fn app_brush_char_assignment() {
        let mut app = create_test_app();
        app.brush_char = '#';
        assert_eq!(app.brush_char, '#');
    }

    #[test]
    fn app_current_color_assignment() {
        let mut app = create_test_app();
        app.current_color = ShapeColor::Red;
        assert_eq!(app.current_color, ShapeColor::Red);
    }

    // ========== Grid Snap Tests ==========

    #[test]
    fn snap_to_grid_rounds_correctly() {
        // Test grid snapping (GRID_SIZE is 5)
        // 5/5 = 1.0 rounds to 1.0, * 5 = 5
        // 7/5 = 1.4 rounds to 1.0, * 5 = 5
        let pos = Position::new(5, 7);
        let snapped = snap_to_grid(pos);
        assert_eq!(snapped.x, 5);
        assert_eq!(snapped.y, 5);
    }

    #[test]
    fn snap_to_grid_at_boundary() {
        // 8/5 = 1.6 rounds to 2.0, * 5 = 10
        let pos = Position::new(8, 8);
        let snapped = snap_to_grid(pos);
        assert_eq!(snapped.x, 10);
        assert_eq!(snapped.y, 10);
    }

    #[test]
    fn snap_to_grid_negative() {
        // -5/5 = -1.0 rounds to -1.0, * 5 = -5
        // -7/5 = -1.4 rounds to -1.0, * 5 = -5
        let pos = Position::new(-5, -7);
        let snapped = snap_to_grid(pos);
        assert_eq!(snapped.x, -5);
        assert_eq!(snapped.y, -5);
    }

    // ========== New Document Tests ==========

    #[test]
    fn app_new_document_clears_state() {
        let mut app = create_test_app();

        // Add a shape
        let _id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        // Create new document
        app.new_document();

        assert!(app.selected.is_empty());
        assert_eq!(app.shape_view.shape_count(), 0);
    }

    // ========== Duplicate Selection Tests ==========

    #[test]
    fn app_duplicate_with_no_selection() {
        let mut app = create_test_app();
        app.duplicate_selection();
        // Should show status message
        assert!(app.status_message.is_some());
    }

    #[test]
    fn app_duplicate_creates_new_shape() {
        let mut app = create_test_app();
        let id = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_single(id);

        let initial_count = app.shape_view.shape_count();
        app.duplicate_selection();

        // Should have one more shape
        assert_eq!(app.shape_view.shape_count(), initial_count + 1);
        // Selection should now be the new shape (not the original)
        assert_eq!(app.selected.len(), 1);
        assert!(!app.selected.contains(&id));
    }

    #[test]
    fn app_duplicate_multiple_shapes() {
        let mut app = create_test_app();
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(20, 20),
                end: Position::new(30, 30),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        let initial_count = app.shape_view.shape_count();
        app.duplicate_selection();

        // Should have doubled the shapes
        assert_eq!(app.shape_view.shape_count(), initial_count * 2);
        // Selection should be the 2 new shapes
        assert_eq!(app.selected.len(), 2);
    }

    // ========== Distribution Tests ==========

    #[test]
    fn app_distribute_horizontal_needs_three_shapes() {
        let mut app = create_test_app();
        // Only 2 shapes - should show message
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(50, 0),
                end: Position::new(60, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        app.distribute_horizontal();
        // Should show status message about needing 3 shapes
        assert!(app.status_message.is_some());
    }

    #[test]
    fn app_distribute_vertical_needs_three_shapes() {
        let mut app = create_test_app();
        // Only 2 shapes - should show message
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 50),
                end: Position::new(10, 60),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        app.distribute_vertical();
        // Should show status message about needing 3 shapes
        assert!(app.status_message.is_some());
    }

    #[test]
    fn app_distribute_horizontal_three_shapes() {
        let mut app = create_test_app();
        // Create 3 shapes with uneven horizontal spacing
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(15, 0), // Middle shape, not centered
                end: Position::new(25, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id3 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(50, 0),
                end: Position::new(60, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        app.distribute_horizontal();

        // Middle shape should be moved to be evenly spaced
        app.rebuild_view();
        if let Some(shape) = app.shape_view.get(id2) {
            let (min_x, _, max_x, _) = shape.bounds();
            let center_x = (min_x + max_x) / 2;
            // First center is at 5, last is at 55, so middle should be at 30
            assert_eq!(center_x, 30);
        }
    }

    #[test]
    fn app_distribute_vertical_three_shapes() {
        let mut app = create_test_app();
        // Create 3 shapes with uneven vertical spacing
        let _id1 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 0),
                end: Position::new(10, 10),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let id2 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 15), // Middle shape, not centered
                end: Position::new(10, 25),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        let _id3 = app
            .doc
            .add_shape(ShapeKind::Rectangle {
                start: Position::new(0, 50),
                end: Position::new(10, 60),
                label: None,
                color: ShapeColor::default(),
            })
            .unwrap();
        app.rebuild_view();
        app.select_all();

        app.distribute_vertical();

        // Middle shape should be moved to be evenly spaced
        app.rebuild_view();
        if let Some(shape) = app.shape_view.get(id2) {
            let (_, min_y, _, max_y) = shape.bounds();
            let center_y = (min_y + max_y) / 2;
            // First center is at 5, last is at 55, so middle should be at 30
            assert_eq!(center_y, 30);
        }
    }
}
