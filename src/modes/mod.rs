//! Mode state machine for irohscii.
//!
//! This module provides a trait-based state machine for application modes,
//! enabling better testability, maintainability, and extensibility.

mod help;
mod keyboard_shape;
mod normal;
mod path_input;
mod qr_code;
mod text_input;

pub use normal::NormalModeState;

use crossterm::event::KeyEvent;
use ratatui::style::Color;
use rat_leaderkey::LeaderAction;

use irohscii_core::{LayerId, Position, ShapeId};
use irohscii_session::SessionId;


use crossterm::event::KeyCode;

use crate::app::{App, KeyboardShapeField, PendingAction, PopupKind, Tool, BRUSHES, COLORS, TOOLS};
use crate::dispatch::dispatch_action;

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
    pub cursor: u32,
}

const _: () = assert!(u32::MAX as usize >= 65536, "u32 must fit reasonable text lengths");

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
    JoinSession,
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
            PathInputKind::JoinSession => "JOIN",
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
            PathInputKind::JoinSession => "Join ticket:",
        }
    }
}

/// Recent files browser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFilesState {
    pub selected: u32,
}

/// Selection popup state (tool, color, brush selection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPopupState {
    pub kind: PopupKind,
    pub selected: u32,
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
    pub scroll: u32,
}

/// Leader menu state - Helix-style space/colon menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderMenuState;

/// Session browser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBrowserState {
    pub selected: u32,
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

/// Grid navigation for selection popups (tool/color/brush).
fn popup_navigate(state: &mut SelectionPopupState, dx: i32, dy: i32) {
    let (cols, total) = match state.kind {
        PopupKind::Tool => (3u32, TOOLS.len() as u32),
        PopupKind::Color => (4u32, COLORS.len() as u32),
        PopupKind::Brush => (6u32, BRUSHES.len() as u32),
    };
    let row = state.selected / cols;
    let col = state.selected % cols;
    let rows = total.div_ceil(cols);
    let new_col = (col as i32 + dx).clamp(0, cols as i32 - 1) as u32;
    let new_row = (row as i32 + dy).clamp(0, rows as i32 - 1) as u32;
    state.selected = (new_row * cols + new_col).min(total - 1);
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
        match self {
            Mode::LeaderMenu(_) => {
                // Handle leader menu separately to avoid borrow checker issues
                let action_result = app.leader_menu.handle_key(&key);
                if let Some(action) = action_result {
                    match action {
                        LeaderAction::Action(action) => {
                            let mut ctx = ModeContext { app };
                            dispatch_action(action, &mut ctx)
                        }
                        LeaderAction::Command(_) | LeaderAction::Submenu(_) => {
                            // These are handled internally by rat-leaderkey
                            ModeTransition::Normal
                        }
                    }
                } else {
                    // Menu is closed or key was consumed internally
                    if !app.leader_menu.visible {
                        ModeTransition::Normal
                    } else {
                        ModeTransition::Stay
                    }
                }
            }
            _ => {
                let mut ctx = ModeContext { app };
                match self {
                    Mode::Normal => {
                        let mut handler = NormalModeState;
                        handler.handle_key(&mut ctx, key)
                    }
                    Mode::TextInput(state) => state.handle_key(&mut ctx, key),
                    Mode::PathInput(state) => state.handle_key(&mut ctx, key),
                    Mode::HelpScreen(state) => state.handle_key(&mut ctx, key),
                    Mode::KeyboardShapeCreate(state) => state.handle_key(&mut ctx, key),
                    Mode::QrCodeDisplay(state) => state.handle_key(&mut ctx, key),

                    // Confirm: y/n/enter/esc
                    Mode::ConfirmDialog(_state) => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            ctx.app.confirm_pending_action();
                            ModeTransition::Normal
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            ctx.app.cancel_pending_action();
                            ModeTransition::Normal
                        }
                        _ => ModeTransition::Stay,
                    },

                    // Label input: char/backspace/delete/arrows/enter/esc
                    Mode::LabelInput(_state) => match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            ctx.app.commit_label();
                            ModeTransition::Normal
                        }
                        KeyCode::Backspace => { ctx.app.backspace_label(); ModeTransition::Stay }
                        KeyCode::Delete => { ctx.app.delete_label_char(); ModeTransition::Stay }
                        KeyCode::Left => { ctx.app.move_label_cursor_left(); ModeTransition::Stay }
                        KeyCode::Right => { ctx.app.move_label_cursor_right(); ModeTransition::Stay }
                        KeyCode::Home => { ctx.app.move_label_cursor_home(); ModeTransition::Stay }
                        KeyCode::End => { ctx.app.move_label_cursor_end(); ModeTransition::Stay }
                        KeyCode::Char(c) => { ctx.app.add_label_char(c); ModeTransition::Stay }
                        _ => ModeTransition::Stay,
                    },

                    // Layer rename: char/backspace/enter/esc
                    Mode::LayerRename(_state) => match key.code {
                        KeyCode::Enter => { ctx.app.commit_layer_rename(); ModeTransition::Normal }
                        KeyCode::Esc => { ctx.app.cancel_layer_rename(); ModeTransition::Normal }
                        KeyCode::Backspace => { ctx.app.backspace_layer_rename(); ModeTransition::Stay }
                        KeyCode::Char(c) => { ctx.app.add_layer_rename_char(c); ModeTransition::Stay }
                        _ => ModeTransition::Stay,
                    },

                    // Selection popup: hjkl/arrows grid navigation
                    Mode::SelectionPopup(state) => {
                        match key.code {
                            KeyCode::Char('h') | KeyCode::Left => {
                                popup_navigate(state, -1, 0);
                                ModeTransition::Stay
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                popup_navigate(state, 1, 0);
                                ModeTransition::Stay
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                popup_navigate(state, 0, 1);
                                ModeTransition::Stay
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                popup_navigate(state, 0, -1);
                                ModeTransition::Stay
                            }
                            KeyCode::Enter => {
                                ctx.app.confirm_popup_selection_with_index(state.kind, state.selected);
                                ModeTransition::Normal
                            }
                            KeyCode::Esc => ModeTransition::Normal,
                            _ => ModeTransition::Stay,
                        }
                    },

                    // Recent files: jk/arrows list navigation
                    Mode::RecentFiles(state) => match key.code {
                        KeyCode::Esc => ModeTransition::Normal,
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            ModeTransition::Stay
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max = ctx.app.recent_files.len().saturating_sub(1);
                            state.selected = (state.selected + 1).min(u32::try_from(max).unwrap_or(u32::MAX));
                            ModeTransition::Stay
                        }
                        KeyCode::Enter => {
                            ctx.app.open_recent_file(state.selected);
                            ModeTransition::Normal
                        }
                        _ => ModeTransition::Stay,
                    },

                    // Session browser: navigation + filter + actions
                    Mode::SessionBrowser(state) => {
                        match key.code {
                            KeyCode::Char('j') | KeyCode::Down => {
                                let len = ctx.app.get_filtered_sessions(&state.filter, state.show_pinned_only).len() as u32;
                                if len > 0 { state.selected = (state.selected + 1).min(len - 1); }
                                ModeTransition::Stay
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                state.selected = state.selected.saturating_sub(1);
                                ModeTransition::Stay
                            }
                            KeyCode::Enter => {
                                let filtered = ctx.app.get_filtered_sessions(&state.filter, state.show_pinned_only);
                                if let Some(session) = filtered.get(state.selected as usize) {
                                    ModeTransition::Action(ModeAction::SwitchSession(session.id.clone()))
                                } else {
                                    ModeTransition::Normal
                                }
                            }
                            KeyCode::Char('n') => ModeTransition::to(Mode::SessionCreate(SessionCreateState { name: String::new() })),
                            KeyCode::Char('d') | KeyCode::Delete => {
                                let filtered = ctx.app.get_filtered_sessions(&state.filter, state.show_pinned_only);
                                if let Some(session) = filtered.get(state.selected as usize) {
                                    if ctx.app.current_session.as_ref() == Some(&session.id) {
                                        ctx.app.set_error("Cannot delete the active session");
                                        ModeTransition::Stay
                                    } else {
                                        ModeTransition::Action(ModeAction::DeleteSession(session.id.clone()))
                                    }
                                } else {
                                    ModeTransition::Stay
                                }
                            }
                            KeyCode::Char('p') => {
                                let filtered = ctx.app.get_filtered_sessions(&state.filter, state.show_pinned_only);
                                if let Some(session) = filtered.get(state.selected as usize) {
                                    ModeTransition::Action(ModeAction::ToggleSessionPin(session.id.clone()))
                                } else {
                                    ModeTransition::Stay
                                }
                            }
                            KeyCode::Esc | KeyCode::Tab => ModeTransition::Normal,
                            KeyCode::Char('*') => {
                                state.show_pinned_only = !state.show_pinned_only;
                                state.selected = 0;
                                ModeTransition::Stay
                            }
                            KeyCode::Backspace => { state.filter.pop(); state.selected = 0; ModeTransition::Stay }
                            KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                                state.filter.push(c);
                                state.selected = 0;
                                ModeTransition::Stay
                            }
                            _ => ModeTransition::Stay,
                        }
                    },

                    // Session create: type name + enter/esc
                    Mode::SessionCreate(state) => match key.code {
                        KeyCode::Esc => ModeTransition::Normal,
                        KeyCode::Enter => {
                            let trimmed = state.name.trim();
                            if trimmed.len() >= 2 {
                                ModeTransition::Action(ModeAction::CreateSession(trimmed.to_string()))
                            } else {
                                ctx.app.set_error("Session name must be at least 2 characters");
                                ModeTransition::Stay
                            }
                        }
                        KeyCode::Backspace => { state.name.pop(); ModeTransition::Stay }
                        KeyCode::Char(c) => { state.name.push(c); ModeTransition::Stay }
                        _ => ModeTransition::Stay,
                    },

                    Mode::LeaderMenu(_) => unreachable!(),
                }
            }
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
        let cursor = initial_text.len() as u32;
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
    pub fn tool_popup(current_tool_index: u32, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
        Mode::SelectionPopup(SelectionPopupState {
            kind: PopupKind::Tool,
            selected: current_tool_index,
            trigger_key,
        })
    }

    /// Create a color selection popup.
    pub fn color_popup(current_color_index: u32, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
        Mode::SelectionPopup(SelectionPopupState {
            kind: PopupKind::Color,
            selected: current_color_index,
            trigger_key,
        })
    }

    /// Create a brush selection popup.
    pub fn brush_popup(current_brush_index: u32, trigger_key: Option<crossterm::event::KeyCode>) -> Self {
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
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

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

    // --- confirm dialog ---

    #[test]
    fn confirm_y_confirms() {
        let mut mode = Mode::confirm_dialog(PendingAction::NewDocument);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('y'))), ModeTransition::Normal));
    }

    #[test]
    fn confirm_n_cancels() {
        let mut mode = Mode::confirm_dialog(PendingAction::NewDocument);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('n'))), ModeTransition::Normal));
    }

    #[test]
    fn confirm_esc_cancels() {
        let mut mode = Mode::confirm_dialog(PendingAction::NewDocument);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn confirm_enter_confirms() {
        let mut mode = Mode::confirm_dialog(PendingAction::NewDocument);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Enter)), ModeTransition::Normal));
    }

    #[test]
    fn confirm_unrecognized_stays() {
        let mut mode = Mode::confirm_dialog(PendingAction::NewDocument);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('x'))), ModeTransition::Stay));
    }

    // --- label input ---

    #[test]
    fn label_escape_commits() {
        let mut mode = Mode::label_input(ShapeId(uuid::Uuid::new_v4()), "test".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn label_char_stays() {
        let mut mode = Mode::label_input(ShapeId(uuid::Uuid::new_v4()), "test".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('x'))), ModeTransition::Stay));
    }

    #[test]
    fn label_arrows_stay() {
        let mut mode = Mode::label_input(ShapeId(uuid::Uuid::new_v4()), "test".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Left)), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Right)), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Home)), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::End)), ModeTransition::Stay));
    }

    // --- layer rename ---

    #[test]
    fn rename_enter_commits() {
        let mut mode = Mode::layer_rename(LayerId(uuid::Uuid::new_v4()), "Layer 1".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Enter)), ModeTransition::Normal));
    }

    #[test]
    fn rename_esc_cancels() {
        let mut mode = Mode::layer_rename(LayerId(uuid::Uuid::new_v4()), "Layer 1".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn rename_char_stays() {
        let mut mode = Mode::layer_rename(LayerId(uuid::Uuid::new_v4()), "Layer 1".into());
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('x'))), ModeTransition::Stay));
    }

    // --- selection popup ---

    #[test]
    fn popup_esc_closes() {
        let mut mode = Mode::tool_popup(0, None);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn popup_enter_confirms() {
        let mut mode = Mode::tool_popup(0, None);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Enter)), ModeTransition::Normal));
    }

    #[test]
    fn popup_navigation_stays() {
        let mut mode = Mode::tool_popup(0, None);
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('j'))), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('k'))), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('h'))), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('l'))), ModeTransition::Stay));
    }

    // --- recent files ---

    #[test]
    fn recent_esc_closes() {
        let mut mode = Mode::recent_files();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn recent_navigation_stays() {
        let mut mode = Mode::RecentFiles(RecentFilesState { selected: 1 });
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('k'))), ModeTransition::Stay));
    }

    #[test]
    fn recent_enter_opens() {
        let mut mode = Mode::recent_files();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Enter)), ModeTransition::Normal));
    }

    // --- session browser ---

    #[test]
    fn browser_esc_closes() {
        let mut mode = Mode::session_browser();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn browser_navigation_stays() {
        let mut mode = Mode::session_browser();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('j'))), ModeTransition::Stay));
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('k'))), ModeTransition::Stay));
    }

    #[test]
    fn browser_n_opens_create() {
        let mut mode = Mode::session_browser();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('n'))), ModeTransition::To(_)));
    }

    // --- session create ---

    #[test]
    fn create_esc_cancels() {
        let mut mode = Mode::session_create();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Esc)), ModeTransition::Normal));
    }

    #[test]
    fn create_enter_with_name_confirms() {
        let mut mode = Mode::SessionCreate(SessionCreateState { name: "test".into() });
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Enter)), ModeTransition::Action(ModeAction::CreateSession(_))));
    }

    #[test]
    fn create_char_stays() {
        let mut mode = Mode::session_create();
        let mut app = crate::app::App::new(80, 24);
        assert!(matches!(mode.handle_key(&mut app, key(KeyCode::Char('a'))), ModeTransition::Stay));
    }
}
