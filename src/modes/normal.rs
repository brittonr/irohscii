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

impl NormalModeState {
    /// Handle tool selection keys
    fn handle_tool_selection(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        debug_assert!(ctx.app.doc.read_all_layers().map(|l| l.len()).unwrap_or(0) > 0);
        
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        
        match key.code {
            KeyCode::Char('s') if !has_ctrl => {
                ctx.app.set_tool(Tool::Star);
                true
            }
            KeyCode::Char('f') => {
                ctx.app.set_tool(Tool::Freehand);
                true
            }
            KeyCode::Char('t') if !has_alt => {
                ctx.app.set_tool(Tool::Text);
                true
            }
            KeyCode::Char('l') if !has_alt => {
                ctx.app.set_tool(Tool::Line);
                true
            }
            KeyCode::Char('a') if !has_ctrl && !has_alt => {
                ctx.app.set_tool(Tool::Arrow);
                true
            }
            KeyCode::Char('r') if !ctx.app.show_layers && !has_alt => {
                ctx.app.set_tool(Tool::Rectangle);
                true
            }
            KeyCode::Char('b') if !has_alt => {
                ctx.app.set_tool(Tool::DoubleBox);
                true
            }
            KeyCode::Char('d') if !has_ctrl && !has_alt => {
                ctx.app.set_tool(Tool::Diamond);
                true
            }
            KeyCode::Char('e') if !has_alt => {
                ctx.app.set_tool(Tool::Ellipse);
                true
            }
            _ => false,
        }
    }

    /// Handle layer management keys
    fn handle_layer_management(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        debug_assert!(ctx.app.doc.read_all_layers().map(|l| l.len()).unwrap_or(0) > 0);
        
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        
        match key.code {
            KeyCode::Char('L') if !has_ctrl => {
                ctx.app.toggle_layer_panel();
                true
            }
            KeyCode::Char('n') if has_ctrl => {
                ctx.app.create_layer();
                true
            }
            KeyCode::Char('D') if has_ctrl => {
                ctx.app.request_delete_layer();
                true
            }
            KeyCode::F(2) if ctx.app.show_layers => {
                ctx.app.start_layer_rename();
                true
            }
            KeyCode::Char('r') if ctx.app.show_layers && ctx.app.active_layer.is_some() => {
                ctx.app.start_layer_rename();
                true
            }
            KeyCode::Char('m') if has_ctrl => {
                ctx.app.move_selection_to_active_layer();
                true
            }
            KeyCode::Char('h') if has_ctrl => {
                ctx.app.toggle_active_layer_visibility();
                true
            }
            KeyCode::Char('1') if has_alt => {
                ctx.app.select_layer_by_index(1);
                true
            }
            KeyCode::Char('2') if has_alt => {
                ctx.app.select_layer_by_index(2);
                true
            }
            KeyCode::Char('3') if has_alt => {
                ctx.app.select_layer_by_index(3);
                true
            }
            KeyCode::Char('4') if has_alt => {
                ctx.app.select_layer_by_index(4);
                true
            }
            KeyCode::Char('5') if has_alt => {
                ctx.app.select_layer_by_index(5);
                true
            }
            KeyCode::Char('6') if has_alt => {
                ctx.app.select_layer_by_index(6);
                true
            }
            KeyCode::Char('7') if has_alt => {
                ctx.app.select_layer_by_index(7);
                true
            }
            KeyCode::Char('8') if has_alt => {
                ctx.app.select_layer_by_index(8);
                true
            }
            KeyCode::Char('9') if has_alt => {
                ctx.app.select_layer_by_index(9);
                true
            }
            _ => false,
        }
    }

    /// Handle alignment keys
    fn handle_alignment(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        if !has_alt {
            return false;
        }
        
        match key.code {
            KeyCode::Char('l') => {
                ctx.app.align_left();
                true
            }
            KeyCode::Char('r') => {
                ctx.app.align_right();
                true
            }
            KeyCode::Char('t') => {
                ctx.app.align_top();
                true
            }
            KeyCode::Char('b') => {
                ctx.app.align_bottom();
                true
            }
            KeyCode::Char('c') => {
                ctx.app.align_center_h();
                true
            }
            KeyCode::Char('m') => {
                ctx.app.align_center_v();
                true
            }
            _ => false,
        }
    }

    /// Handle transform operations (flip, rotate)
    fn handle_transforms(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        if !has_alt {
            return false;
        }
        
        match key.code {
            KeyCode::Char('h') => {
                ctx.app.flip_horizontal();
                true
            }
            KeyCode::Char('v') => {
                ctx.app.flip_vertical();
                true
            }
            KeyCode::Char('.') => {
                ctx.app.rotate_90_cw();
                true
            }
            KeyCode::Char(',') => {
                ctx.app.rotate_90_ccw();
                true
            }
            KeyCode::Char('[') => {
                debug_assert!(ctx.app.selected.len() >= 3, "Distribute requires 3+ shapes");
                ctx.app.distribute_horizontal();
                true
            }
            KeyCode::Char(']') => {
                debug_assert!(ctx.app.selected.len() >= 3, "Distribute requires 3+ shapes");
                ctx.app.distribute_vertical();
                true
            }
            _ => false,
        }
    }

    /// Handle keyboard-based shape creation
    fn handle_keyboard_shapes(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        
        match key.code {
            KeyCode::Char('R') if has_ctrl => {
                ctx.app.start_keyboard_shape_create(Tool::Rectangle);
                true
            }
            KeyCode::Char('d') if has_alt => {
                ctx.app.start_keyboard_shape_create(Tool::Diamond);
                true
            }
            KeyCode::Char('e') if has_alt => {
                ctx.app.start_keyboard_shape_create(Tool::Ellipse);
                true
            }
            KeyCode::Char('L') if has_ctrl => {
                ctx.app.start_keyboard_shape_create(Tool::Line);
                true
            }
            KeyCode::Char('a') if has_alt => {
                ctx.app.start_keyboard_shape_create(Tool::Arrow);
                true
            }
            KeyCode::Char('B') if has_ctrl => {
                ctx.app.start_keyboard_shape_create(Tool::DoubleBox);
                true
            }
            _ => false,
        }
    }

    /// Handle file operations (save, open, etc.)
    fn handle_file_ops(ctx: &mut ModeContext<'_>, key: KeyEvent) -> Option<ModeTransition> {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
        
        match key.code {
            KeyCode::Char('s') if has_ctrl && !has_shift => {
                ctx.app.start_save();
                Some(ModeTransition::Stay)
            }
            KeyCode::Char('o') if has_ctrl && !has_shift => {
                ctx.app.start_open();
                Some(ModeTransition::Stay)
            }
            KeyCode::Char('S') if has_ctrl => {
                ctx.app.start_doc_save();
                Some(ModeTransition::Stay)
            }
            KeyCode::Char('O') if has_ctrl => {
                ctx.app.start_doc_open();
                Some(ModeTransition::Stay)
            }
            KeyCode::Char('R') if !has_ctrl => {
                if !ctx.app.recent_files.is_empty() {
                    Some(ModeTransition::to(Mode::recent_files()))
                } else {
                    ctx.app.set_status("No recent files");
                    Some(ModeTransition::Stay)
                }
            }
            _ => None,
        }
    }

    /// Handle editing operations (undo, redo, copy, paste, group, z-order)
    fn handle_editing_ops(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        
        match key.code {
            KeyCode::Char('u') => {
                ctx.app.undo();
                true
            }
            KeyCode::Char('U') => {
                ctx.app.redo();
                true
            }
            KeyCode::Char('y') => {
                ctx.app.yank();
                true
            }
            KeyCode::Char('p') => {
                ctx.app.paste();
                true
            }
            KeyCode::Char(']') if !has_alt => {
                ctx.app.bring_forward();
                true
            }
            KeyCode::Char('[') if !has_alt => {
                ctx.app.send_backward();
                true
            }
            KeyCode::Char('}') => {
                ctx.app.bring_to_front();
                true
            }
            KeyCode::Char('{') => {
                ctx.app.send_to_back();
                true
            }
            KeyCode::Char('G') if !has_ctrl => {
                ctx.app.group_selection();
                true
            }
            KeyCode::Char('g') if has_ctrl => {
                ctx.app.group_selection();
                true
            }
            KeyCode::Char('G') if has_ctrl => {
                ctx.app.ungroup_selection();
                true
            }
            KeyCode::Char('a') if has_ctrl => {
                ctx.app.select_all();
                true
            }
            KeyCode::Char('d') if has_ctrl => {
                ctx.app.duplicate_selection();
                true
            }
            _ => false,
        }
    }

    /// Handle viewport operations (pan, zoom)
    fn handle_viewport_ops(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        
        match key.code {
            KeyCode::Up => {
                ctx.app.viewport.pan(0, -1);
                true
            }
            KeyCode::Down => {
                ctx.app.viewport.pan(0, 1);
                true
            }
            KeyCode::Left => {
                ctx.app.viewport.pan(-1, 0);
                true
            }
            KeyCode::Right => {
                ctx.app.viewport.pan(1, 0);
                true
            }
            KeyCode::Char('+') | KeyCode::Char('=') if has_ctrl => {
                ctx.app.viewport.zoom_in();
                let zoom_pct = (ctx.app.viewport.zoom * 100.0) as i32;
                ctx.app.set_status(format!("Zoom: {}%", zoom_pct));
                true
            }
            KeyCode::Char('-') if has_ctrl => {
                ctx.app.viewport.zoom_out();
                let zoom_pct = (ctx.app.viewport.zoom * 100.0) as i32;
                ctx.app.set_status(format!("Zoom: {}%", zoom_pct));
                true
            }
            KeyCode::Char('0') if has_ctrl => {
                ctx.app.viewport.reset_zoom();
                ctx.app.set_status("Zoom: 100%");
                true
            }
            _ => false,
        }
    }

    /// Handle nudge operations with hjkl
    fn handle_nudge(ctx: &mut ModeContext<'_>, key: KeyEvent) -> bool {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        let is_select = ctx.app.current_tool == Tool::Select;
        let has_selection = !ctx.app.selected.is_empty();
        
        if has_ctrl || has_alt || !is_select || !has_selection {
            return false;
        }
        
        debug_assert!(ctx.app.current_tool == Tool::Select);
        
        match key.code {
            KeyCode::Char('h') => {
                ctx.app.nudge_selection(-1, 0);
                true
            }
            KeyCode::Char('j') => {
                ctx.app.nudge_selection(0, 1);
                true
            }
            KeyCode::Char('k') => {
                ctx.app.nudge_selection(0, -1);
                true
            }
            KeyCode::Char('l') => {
                ctx.app.nudge_selection(1, 0);
                true
            }
            _ => false,
        }
    }
}

impl ModeHandler for NormalModeState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(ctx.app.doc.read_all_layers().map(|l| l.len()).unwrap_or(0) > 0);
        
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let _has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
        // Try helper functions first (they check their own conditions)
        if Self::handle_tool_selection(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_layer_management(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_alignment(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_transforms(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_keyboard_shapes(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_nudge(ctx, key) {
            return ModeTransition::Stay;
        }
        if Self::handle_editing_ops(ctx, key) {
            return ModeTransition::Stay;
        }
        if let Some(transition) = Self::handle_file_ops(ctx, key) {
            return transition;
        }
        if Self::handle_viewport_ops(ctx, key) {
            return ModeTransition::Stay;
        }
        
        // Handle remaining commands
        match key.code {
            KeyCode::Char('c') if has_ctrl => {
                return ModeTransition::Action(ModeAction::Quit);
            }
            KeyCode::Esc => {
                ctx.app.cancel_shape();
                ctx.app.clear_selection();
            }
            KeyCode::Tab => {
                return ModeTransition::Action(ModeAction::OpenSessionBrowser);
            }
            KeyCode::Char('v') if !key.modifiers.contains(KeyModifiers::ALT) => {
                ctx.app.cycle_line_style();
            }
            KeyCode::Char('g') => ctx.app.toggle_grid(),
            KeyCode::Delete | KeyCode::Backspace => {
                if ctx.app.current_tool == Tool::Select {
                    ctx.app.delete_selected();
                }
            }
            KeyCode::Enter => {
                let is_select = ctx.app.current_tool == Tool::Select;
                let has_selection = !ctx.app.selected.is_empty();
                if is_select && has_selection && ctx.app.start_label_input() {
                    ctx.app.set_status("Editing label - type text, Enter/Esc to finish");
                }
            }
            KeyCode::Char(' ') | KeyCode::Char(':') => {
                return ModeTransition::to(Mode::leader_menu());
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                return ModeTransition::to(Mode::help_screen());
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
    fn test_q_does_not_quit_directly() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        // q should NOT quit directly - quit goes through leader menu (Space/: → q)
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('q')));
        assert!(matches!(result, ModeTransition::Stay));
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
    fn test_help_transitions_to_help_screen() {
        let mut state = NormalModeState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = super::ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('?')));
        assert!(matches!(result, ModeTransition::To(_)));
    }
}
