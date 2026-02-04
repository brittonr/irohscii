//! Normal mode handler - the primary drawing/editing mode.
//!
//! This is the most complex mode with 80+ keybindings for:
//! - Tool selection
//! - File operations
//! - Editing operations (undo/redo/copy/paste)
//! - Z-order and grouping
//! - Layer management
//! - Alignment and transforms
//! - Viewport controls
//! - Session management

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;

use super::{Mode, ModeAction, ModeContext, ModeHandler, ModeTransition};
use crate::app::Tool;

/// Normal mode state - empty since normal mode has no state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NormalModeState;

impl ModeHandler for NormalModeState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        // Route based on key + modifiers
        match key.code {
            // =========================================================
            // Exit/Cancel
            // =========================================================
            KeyCode::Char('q') => {
                return ModeTransition::Action(ModeAction::Quit);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return ModeTransition::Action(ModeAction::Quit);
            }
            KeyCode::Esc => {
                ctx.app.cancel_shape();
                ctx.app.clear_selection();
            }

            // =========================================================
            // Session browser (Tab) - requires SessionManager
            // =========================================================
            KeyCode::Tab => {
                // Return action to let main loop handle with SessionManager
                return ModeTransition::Action(ModeAction::OpenSessionBrowser);
            }

            // =========================================================
            // Tool selection
            // =========================================================
            KeyCode::Char('s') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.set_tool(Tool::Select);
            }
            KeyCode::Char('f') => ctx.app.set_tool(Tool::Freehand),
            KeyCode::Char('t') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.set_tool(Tool::Text);
            }
            KeyCode::Char('l') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.set_tool(Tool::Line);
            }
            KeyCode::Char('a')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                ctx.app.set_tool(Tool::Arrow);
            }
            KeyCode::Char('r')
                if !ctx.app.show_layers && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                ctx.app.set_tool(Tool::Rectangle);
            }
            KeyCode::Char('b') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.set_tool(Tool::DoubleBox);
            }
            KeyCode::Char('d')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                ctx.app.set_tool(Tool::Diamond);
            }
            KeyCode::Char('e') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.set_tool(Tool::Ellipse);
            }

            // =========================================================
            // Line style cycling
            // =========================================================
            KeyCode::Char('v') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.cycle_line_style();
            }

            // =========================================================
            // Undo/Redo
            // =========================================================
            KeyCode::Char('u') => ctx.app.undo(),
            KeyCode::Char('U') => ctx.app.redo(),

            // =========================================================
            // Copy/Paste
            // =========================================================
            KeyCode::Char('y') => ctx.app.yank(),
            KeyCode::Char('p') => ctx.app.paste(),

            // =========================================================
            // Z-order control
            // =========================================================
            KeyCode::Char(']') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.bring_forward();
            }
            KeyCode::Char('[') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.send_backward();
            }
            KeyCode::Char('}') => ctx.app.bring_to_front(),
            KeyCode::Char('{') => ctx.app.send_to_back(),

            // =========================================================
            // Grouping
            // =========================================================
            KeyCode::Char('G') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.group_selection();
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.group_selection();
            }
            KeyCode::Char('G') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.ungroup_selection();
            }

            // =========================================================
            // Layer management
            // =========================================================
            KeyCode::Char('L') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.toggle_layer_panel();
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.create_layer();
            }
            KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.request_delete_layer();
            }
            KeyCode::F(2) if ctx.app.show_layers => {
                ctx.app.start_layer_rename();
            }
            KeyCode::Char('r') if ctx.app.show_layers && ctx.app.active_layer.is_some() => {
                ctx.app.start_layer_rename();
            }
            // Layer selection by number (Alt+1 through Alt+9)
            KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(1);
            }
            KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(2);
            }
            KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(3);
            }
            KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(4);
            }
            KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(5);
            }
            KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(6);
            }
            KeyCode::Char('7') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(7);
            }
            KeyCode::Char('8') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(8);
            }
            KeyCode::Char('9') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.select_layer_by_index(9);
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.move_selection_to_active_layer();
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.toggle_active_layer_visibility();
            }

            // =========================================================
            // Selection
            // =========================================================
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.select_all();
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.duplicate_selection();
            }

            // =========================================================
            // Alignment (Alt + key)
            // =========================================================
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_left();
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_right();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_top();
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_bottom();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_center_h();
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.align_center_v();
            }

            // =========================================================
            // Transform (Alt + key)
            // =========================================================
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.flip_horizontal();
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.flip_vertical();
            }
            KeyCode::Char('.') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.rotate_90_cw();
            }
            KeyCode::Char(',') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.rotate_90_ccw();
            }

            // =========================================================
            // Distribution (Alt + bracket)
            // =========================================================
            KeyCode::Char('[') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.distribute_horizontal();
            }
            KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.distribute_vertical();
            }

            // =========================================================
            // Delete selected shape
            // =========================================================
            KeyCode::Delete | KeyCode::Backspace => {
                if ctx.app.current_tool == Tool::Select {
                    ctx.app.delete_selected();
                }
            }

            // =========================================================
            // Edit label of selected shape
            // =========================================================
            KeyCode::Enter => {
                if ctx.app.current_tool == Tool::Select && !ctx.app.selected.is_empty() {
                    if ctx.app.start_label_input() {
                        ctx.app.set_status("Editing label - type text, Enter/Esc to finish");
                    }
                }
            }

            // =========================================================
            // Nudge selected shapes with hjkl (vim-style)
            // =========================================================
            KeyCode::Char('h')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && ctx.app.current_tool == Tool::Select
                    && !ctx.app.selected.is_empty() =>
            {
                ctx.app.nudge_selection(-1, 0);
            }
            KeyCode::Char('j')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && ctx.app.current_tool == Tool::Select
                    && !ctx.app.selected.is_empty() =>
            {
                ctx.app.nudge_selection(0, 1);
            }
            KeyCode::Char('k')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && ctx.app.current_tool == Tool::Select
                    && !ctx.app.selected.is_empty() =>
            {
                ctx.app.nudge_selection(0, -1);
            }
            KeyCode::Char('l')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && ctx.app.current_tool == Tool::Select
                    && !ctx.app.selected.is_empty() =>
            {
                ctx.app.nudge_selection(1, 0);
            }

            // =========================================================
            // File operations
            // =========================================================
            KeyCode::Char('s')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                ctx.app.start_save();
            }
            KeyCode::Char('o')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                ctx.app.start_open();
            }
            KeyCode::Char('S') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.start_doc_save();
            }
            KeyCode::Char('O') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.start_doc_open();
            }

            // =========================================================
            // Sync operations
            // =========================================================
            KeyCode::Char('T') => ctx.app.copy_ticket_to_clipboard(),
            KeyCode::Char('P') => {
                ctx.app.show_participants = !ctx.app.show_participants;
                let status = if ctx.app.show_participants {
                    "Participants panel shown"
                } else {
                    "Participants panel hidden"
                };
                ctx.app.set_status(status);
            }

            // =========================================================
            // SVG export
            // =========================================================
            KeyCode::Char('E') => ctx.app.start_svg_export(),

            // =========================================================
            // Grid and document
            // =========================================================
            KeyCode::Char('g') => ctx.app.toggle_grid(),
            KeyCode::Char('N') => ctx.app.request_new_document(),

            // =========================================================
            // Recent files
            // =========================================================
            KeyCode::Char('R') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !ctx.app.recent_files.is_empty() {
                    return ModeTransition::to(Mode::recent_files());
                } else {
                    ctx.app.set_status("No recent files");
                }
            }

            // =========================================================
            // Viewport panning
            // =========================================================
            KeyCode::Up => ctx.app.viewport.pan(0, -1),
            KeyCode::Down => ctx.app.viewport.pan(0, 1),
            KeyCode::Left => ctx.app.viewport.pan(-1, 0),
            KeyCode::Right => ctx.app.viewport.pan(1, 0),

            // =========================================================
            // Zoom controls
            // =========================================================
            KeyCode::Char('+') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.viewport.zoom_in();
                ctx.app
                    .set_status(format!("Zoom: {}%", (ctx.app.viewport.zoom * 100.0) as i32));
            }
            KeyCode::Char('=') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.viewport.zoom_in();
                ctx.app
                    .set_status(format!("Zoom: {}%", (ctx.app.viewport.zoom * 100.0) as i32));
            }
            KeyCode::Char('-') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.viewport.zoom_out();
                ctx.app
                    .set_status(format!("Zoom: {}%", (ctx.app.viewport.zoom * 100.0) as i32));
            }
            KeyCode::Char('0') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.viewport.reset_zoom();
                ctx.app.set_status("Zoom: 100%");
            }

            // =========================================================
            // Help screen
            // =========================================================
            KeyCode::Char('?') | KeyCode::F(1) => ctx.app.open_help(),

            // =========================================================
            // Keyboard shape creation
            // =========================================================
            KeyCode::Char('R') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.start_keyboard_shape_create(Tool::Rectangle);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.start_keyboard_shape_create(Tool::Diamond);
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.start_keyboard_shape_create(Tool::Ellipse);
            }
            KeyCode::Char('L') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.start_keyboard_shape_create(Tool::Line);
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.start_keyboard_shape_create(Tool::Arrow);
            }
            KeyCode::Char('B') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.app.start_keyboard_shape_create(Tool::DoubleBox);
            }

            _ => {}
        }

        ModeTransition::Stay
    }

    fn mode_name(&self) -> &'static str {
        "NORMAL"
    }

    fn mode_color(&self) -> Color {
        Color::Blue
    }

    fn help_text(&self) -> &'static str {
        "Normal mode - Press ? for help"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn key_alt(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::ALT)
    }

    #[test]
    fn test_quit_returns_action() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('q')));
        assert!(matches!(result, ModeTransition::Action(ModeAction::Quit)));
    }

    #[test]
    fn test_ctrl_c_returns_quit() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key_ctrl(KeyCode::Char('c')));
        assert!(matches!(result, ModeTransition::Action(ModeAction::Quit)));
    }

    #[test]
    fn test_tab_opens_session_browser() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Tab));
        assert!(matches!(
            result,
            ModeTransition::Action(ModeAction::OpenSessionBrowser)
        ));
    }

    #[test]
    fn test_tool_selection_stays() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        // s = Select tool
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('s')));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(ctx.app.current_tool, Tool::Select);

        // f = Freehand tool
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('f')));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(ctx.app.current_tool, Tool::Freehand);
    }

    #[test]
    fn test_undo_redo_stays() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('u')));
        assert!(matches!(result, ModeTransition::Stay));

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('U')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_viewport_pan_stays() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Up));
        assert!(matches!(result, ModeTransition::Stay));
        let result = state.handle_key(&mut ctx, key(KeyCode::Down));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_zoom_controls_stay() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key_ctrl(KeyCode::Char('+')));
        assert!(matches!(result, ModeTransition::Stay));

        let result = state.handle_key(&mut ctx, key_ctrl(KeyCode::Char('-')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_alignment_shortcuts() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        // Alt+l = align left
        let result = state.handle_key(&mut ctx, key_alt(KeyCode::Char('l')));
        assert!(matches!(result, ModeTransition::Stay));

        // Alt+r = align right
        let result = state.handle_key(&mut ctx, key_alt(KeyCode::Char('r')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_help_stays_in_normal() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        // Help opens via app.open_help() which changes app.mode
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('?')));
        assert!(matches!(result, ModeTransition::Stay));
    }
}
