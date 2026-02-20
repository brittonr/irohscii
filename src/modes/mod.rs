//! Mode state machine for irohscii.
//!
//! This module provides a trait-based state machine for application modes,
//! enabling better testability, maintainability, and extensibility.

mod confirm;
mod help;
mod keyboard_shape;
mod label_input;
mod layer_rename;
mod leader;
mod normal;
mod path_input;
mod popup;
mod qr_code;
mod recent_files;
mod session;
mod text_input;

pub use normal::NormalModeState;

use crossterm::event::KeyEvent;
use ratatui::style::Color;

use irohscii_core::{LayerId, Position, ShapeId};
use irohscii_session::SessionId;

use crate::app::{App, KeyboardShapeField, PendingAction, PopupKind, Tool};

/// Result of handling a key event in a mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeTransition {
    /// Remain in the current mode
    Stay,
    /// Return to Normal mode
    Normal,
    /// Transition to a specific mode
    To(Box<Mode>),
    /// Perform an external action (quit, session operations)
    Action(ModeAction),
}

impl ModeTransition {
    /// Create a transition to a specific mode
    pub fn to(mode: Mode) -> Self {
        ModeTransition::To(Box::new(mode))
    }
}

/// External actions that require handling outside the mode system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeAction {
    /// Quit the application
    Quit,
    /// Open the session browser (needs SessionManager to list sessions)
    OpenSessionBrowser,
    /// Switch to a different session
    SwitchSession(SessionId),
    /// Create a new session with the given name
    CreateSession(String),
    /// Delete a session
    DeleteSession(SessionId),
    /// Toggle pin status for a session (needs SessionManager)
    ToggleSessionPin(SessionId),
}

/// Mutable context available to mode handlers.
///
/// This provides access to the App state and optional session manager
/// for modes that need to modify application state.
pub struct ModeContext<'a> {
    pub app: &'a mut App,
}

/// Trait for handling key events in a mode.
#[allow(dead_code)]
pub trait ModeHandler {
    /// Handle a key event and return the transition result.
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition;

    /// Get the display name for this mode (shown in status bar).
    fn mode_name(&self) -> &'static str;

    /// Get the color for this mode (used in status bar).
    fn mode_color(&self) -> Color;

    /// Get help text for this mode.
    fn help_text(&self) -> &'static str;
}

/// Text input mode state - for typing text at a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputState {
    pub start_pos: Position,
    pub text: String,
}

/// Label input mode state - for editing a shape's label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelInputState {
    pub shape_id: ShapeId,
    pub text: String,
    pub cursor: usize,
}

/// Layer rename mode state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRenameState {
    pub layer_id: LayerId,
    pub text: String,
}

/// Path input mode state - shared by file/doc save/open modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathInputState {
    pub path: String,
    pub kind: PathInputKind,
}

/// Kind of path input operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathInputKind {
    FileSave,
    FileOpen,
    DocSave,
    DocOpen,
    SvgExport,
    ClusterConnect,
    QrDecode,
}

impl PathInputKind {
    pub fn mode_name(&self) -> &'static str {
        match self {
            PathInputKind::FileSave => "SAVE",
            PathInputKind::FileOpen => "OPEN",
            PathInputKind::DocSave => "SAVE DOC",
            PathInputKind::DocOpen => "OPEN DOC",
            PathInputKind::SvgExport => "SVG EXPORT",
            PathInputKind::ClusterConnect => "CLUSTER",
            PathInputKind::QrDecode => "QR DECODE",
        }
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            PathInputKind::FileSave => "Save file:",
            PathInputKind::FileOpen => "Open file:",
            PathInputKind::DocSave => "Save document:",
            PathInputKind::DocOpen => "Open document:",
            PathInputKind::SvgExport => "Export SVG:",
            PathInputKind::ClusterConnect => "Cluster ticket:",
            PathInputKind::QrDecode => "QR image path:",
        }
    }
}

/// Recent files browser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFilesState {
    pub selected: usize,
}

/// Selection popup state (tool, color, brush selection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPopupState {
    pub kind: PopupKind,
    pub selected: usize,
    /// The key that triggered this popup (for release-to-confirm).
    pub trigger_key: Option<crossterm::event::KeyCode>,
}

/// Confirm dialog state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmDialogState {
    pub action: PendingAction,
}

/// Help screen state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpScreenState {
    pub scroll: usize,
}

/// Leader menu state - Helix-style space/colon menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderMenuState;

/// Session browser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBrowserState {
    pub selected: usize,
    pub filter: String,
    pub show_pinned_only: bool,
}

/// Session create state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCreateState {
    pub name: String,
}

/// Keyboard shape creation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardShapeState {
    pub tool: Tool,
    pub width: String,
    pub height: String,
    pub focus: KeyboardShapeField,
}

/// QR code display state - shows a sync ticket as a QR code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrCodeDisplayState {
    /// The ticket string being displayed
    pub ticket: String,
}

/// Application mode state machine.
///
/// Each variant contains the state needed for that mode.
/// The enum wrapper approach (vs Box<dyn>) provides:
/// - No heap allocation on mode transitions
/// - Exhaustive pattern matching
/// - Better type inference
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    TextInput(TextInputState),
    LabelInput(LabelInputState),
    LayerRename(LayerRenameState),
    PathInput(PathInputState),
    RecentFiles(RecentFilesState),
    SelectionPopup(SelectionPopupState),
    ConfirmDialog(ConfirmDialogState),
    HelpScreen(HelpScreenState),
    LeaderMenu(LeaderMenuState),
    SessionBrowser(SessionBrowserState),
    SessionCreate(SessionCreateState),
    KeyboardShapeCreate(KeyboardShapeState),
    QrCodeDisplay(QrCodeDisplayState),
}

// Query methods for Mode - kept for future UI use (status bar display)
#[allow(dead_code)]
impl Mode {
    /// Get the display name for this mode.
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::TextInput(_) => "TEXT",
            Mode::LabelInput(_) => "LABEL",
            Mode::LayerRename(_) => "RENAME",
            Mode::PathInput(state) => state.kind.mode_name(),
            Mode::RecentFiles(_) => "RECENT",
            Mode::SelectionPopup(state) => match state.kind {
                PopupKind::Tool => "TOOL",
                PopupKind::Color => "COLOR",
                PopupKind::Brush => "BRUSH",
            },
            Mode::ConfirmDialog(_) => "CONFIRM",
            Mode::HelpScreen(_) => "HELP",
            Mode::LeaderMenu(_) => "SPACE",
            Mode::SessionBrowser(_) => "SESSIONS",
            Mode::SessionCreate(_) => "NEW SESSION",
            Mode::KeyboardShapeCreate(_) => "CREATE",
            Mode::QrCodeDisplay(_) => "QR CODE",
        }
    }

    /// Get the status bar color for this mode.
    pub fn color(&self) -> Color {
        match self {
            Mode::Normal => Color::Blue,
            Mode::TextInput(_) => Color::Green,
            Mode::LabelInput(_) => Color::Green,
            Mode::LayerRename(_) => Color::Green,
            Mode::PathInput(_) => Color::Yellow,
            Mode::RecentFiles(_) => Color::Cyan,
            Mode::SelectionPopup(_) => Color::Magenta,
            Mode::ConfirmDialog(_) => Color::Red,
            Mode::HelpScreen(_) => Color::Cyan,
            Mode::LeaderMenu(_) => Color::Cyan,
            Mode::SessionBrowser(_) => Color::Cyan,
            Mode::SessionCreate(_) => Color::Green,
            Mode::KeyboardShapeCreate(_) => Color::Yellow,
            Mode::QrCodeDisplay(_) => Color::Magenta,
        }
    }

    /// Check if this mode is Normal mode.
    pub fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal)
    }

    /// Check if this mode is a text input mode (has an active cursor).
    pub fn is_text_input(&self) -> bool {
        matches!(
            self,
            Mode::TextInput(_)
                | Mode::LabelInput(_)
                | Mode::LayerRename(_)
                | Mode::PathInput(_)
                | Mode::SessionCreate(_)
                | Mode::KeyboardShapeCreate(_)
        )
    }

    /// Check if this is a popup mode that uses release-to-confirm.
    pub fn is_popup(&self) -> bool {
        matches!(self, Mode::SelectionPopup(_))
    }

    /// Get the trigger key if this is a popup with release-to-confirm.
    pub fn popup_trigger_key(&self) -> Option<crossterm::event::KeyCode> {
        match self {
            Mode::SelectionPopup(state) => state.trigger_key,
            _ => None,
        }
    }

    /// Handle a key event and return the transition result.
    ///
    /// This dispatches to the appropriate mode handler based on the current mode.
    pub fn handle_key(&mut self, app: &mut App, key: KeyEvent) -> ModeTransition {
        let mut ctx = ModeContext { app };
        match self {
            Mode::Normal => {
                let mut handler = NormalModeState;
                handler.handle_key(&mut ctx, key)
            }
            Mode::TextInput(state) => state.handle_key(&mut ctx, key),
            Mode::LabelInput(state) => state.handle_key(&mut ctx, key),
            Mode::LayerRename(state) => state.handle_key(&mut ctx, key),
            Mode::PathInput(state) => state.handle_key(&mut ctx, key),
            Mode::RecentFiles(state) => state.handle_key(&mut ctx, key),
            Mode::SelectionPopup(state) => state.handle_key(&mut ctx, key),
            Mode::ConfirmDialog(state) => state.handle_key(&mut ctx, key),
            Mode::HelpScreen(state) => state.handle_key(&mut ctx, key),
            Mode::LeaderMenu(state) => state.handle_key(&mut ctx, key),
            Mode::SessionBrowser(state) => state.handle_key(&mut ctx, key),
            Mode::SessionCreate(state) => state.handle_key(&mut ctx, key),
            Mode::KeyboardShapeCreate(state) => state.handle_key(&mut ctx, key),
            Mode::QrCodeDisplay(state) => state.handle_key(&mut ctx, key),
        }
    }
}

// Convenience constructors for common mode transitions - kept for API completeness
#[allow(dead_code)]
impl Mode {
    /// Create a new text input mode at the given position.
    pub fn text_input(start_pos: Position) -> Self {
        Mode::TextInput(TextInputState {
            start_pos,
            text: String::new(),
        })
    }

    /// Create a new label input mode for the given shape.
    pub fn label_input(shape_id: ShapeId, initial_text: String) -> Self {
        let cursor = initial_text.len();
        Mode::LabelInput(LabelInputState {
            shape_id,
            text: initial_text,
            cursor,
        })
    }

    /// Create a new layer rename mode.
    pub fn layer_rename(layer_id: LayerId, initial_text: String) -> Self {
        Mode::LayerRename(LayerRenameState {
            layer_id,
            text: initial_text,
        })
    }

    /// Create a file save mode.
    pub fn file_save(path: String) -> Self {
        Mode::PathInput(PathInputState {
            path,
            kind: PathInputKind::FileSave,
        })
    }

    /// Create a file open mode.
    pub fn file_open(path: String) -> Self {
        Mode::PathInput(PathInputState {
            path,
            kind: PathInputKind::FileOpen,
        })
    }

    /// Create a document save mode.
    pub fn doc_save(path: String) -> Self {
        Mode::PathInput(PathInputState {
            path,
            kind: PathInputKind::DocSave,
        })
    }

    /// Create a document open mode.
    pub fn doc_open(path: String) -> Self {
        Mode::PathInput(PathInputState {
            path,
            kind: PathInputKind::DocOpen,
        })
    }

    /// Create an SVG export mode.
    pub fn svg_export(path: String) -> Self {
        Mode::PathInput(PathInputState {
            path,
            kind: PathInputKind::SvgExport,
        })
    }

    /// Create a recent files browser mode.
    pub fn recent_files() -> Self {
        Mode::RecentFiles(RecentFilesState { selected: 0 })
    }

    /// Create a tool selection popup.
    pub fn tool_popup(current_tool_index: usize, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
        Mode::SelectionPopup(SelectionPopupState {
            kind: PopupKind::Tool,
            selected: current_tool_index,
            trigger_key,
        })
    }

    /// Create a color selection popup.
    pub fn color_popup(current_color_index: usize, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
        Mode::SelectionPopup(SelectionPopupState {
            kind: PopupKind::Color,
            selected: current_color_index,
            trigger_key,
        })
    }

    /// Create a brush selection popup.
    pub fn brush_popup(current_brush_index: usize, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
        Mode::SelectionPopup(SelectionPopupState {
            kind: PopupKind::Brush,
            selected: current_brush_index,
            trigger_key,
        })
    }

    /// Create a confirm dialog mode.
    pub fn confirm_dialog(action: PendingAction) -> Self {
        Mode::ConfirmDialog(ConfirmDialogState { action })
    }

    /// Create a leader menu mode (Helix-style Space/:).
    pub fn leader_menu() -> Self {
        Mode::LeaderMenu(LeaderMenuState)
    }

    /// Create a help screen mode.
    pub fn help_screen() -> Self {
        Mode::HelpScreen(HelpScreenState { scroll: 0 })
    }

    /// Create a session browser mode.
    pub fn session_browser() -> Self {
        Mode::SessionBrowser(SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        })
    }

    /// Create a session create mode.
    pub fn session_create() -> Self {
        Mode::SessionCreate(SessionCreateState {
            name: String::new(),
        })
    }

    /// Create a keyboard shape creation mode.
    pub fn keyboard_shape_create(tool: Tool) -> Self {
        Mode::KeyboardShapeCreate(KeyboardShapeState {
            tool,
            width: String::new(),
            height: String::new(),
            focus: KeyboardShapeField::Width,
        })
    }

    /// Create a QR code display mode for a ticket.
    pub fn qr_code_display(ticket: String) -> Self {
        Mode::QrCodeDisplay(QrCodeDisplayState { ticket })
    }

    /// Create a QR decode input mode.
    pub fn qr_decode() -> Self {
        Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::QrDecode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_default_is_normal() {
        assert!(matches!(Mode::default(), Mode::Normal));
    }

    #[test]
    fn test_mode_names() {
        assert_eq!(Mode::Normal.name(), "NORMAL");
        assert_eq!(Mode::text_input(Position::new(0, 0)).name(), "TEXT");
        assert_eq!(Mode::file_save(String::new()).name(), "SAVE");
        assert_eq!(Mode::help_screen().name(), "HELP");
    }

    #[test]
    fn test_is_text_input() {
        assert!(!Mode::Normal.is_text_input());
        assert!(Mode::text_input(Position::new(0, 0)).is_text_input());
        assert!(Mode::session_create().is_text_input());
        assert!(!Mode::help_screen().is_text_input());
    }

    #[test]
    fn test_is_popup() {
        assert!(!Mode::Normal.is_popup());
        assert!(Mode::tool_popup(0, None).is_popup());
        assert!(Mode::color_popup(0, None).is_popup());
        assert!(Mode::brush_popup(0, None).is_popup());
    }

    #[test]
    fn test_popup_trigger_key() {
        use crossterm::event::KeyCode;

        let mode = Mode::tool_popup(0, Some(KeyCode::Char(' ')));
        assert_eq!(mode.popup_trigger_key(), Some(KeyCode::Char(' ')));

        let mode = Mode::Normal;
        assert_eq!(mode.popup_trigger_key(), None);
    }

    #[test]
    fn test_label_input_cursor_position() {
        use uuid::Uuid;
        let shape_id = ShapeId(Uuid::new_v4());
        let mode = Mode::label_input(shape_id, "hello".to_string());
        if let Mode::LabelInput(state) = mode {
            assert_eq!(state.cursor, 5);
        } else {
            panic!("Expected LabelInput mode");
        }
    }
}
