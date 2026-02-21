//! Selection popup mode handler for tool, color, and brush selection.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, SelectionPopupState};
use crate::app::{PopupKind, BRUSHES, COLORS, TOOLS};

impl SelectionPopupState {
    /// Navigate within the popup grid
    fn navigate(&mut self, dx: i32, dy: i32) {
        debug_assert!(TOOLS.len() > 0 && COLORS.len() > 0 && BRUSHES.len() > 0);
        
        let (cols, total) = match self.kind {
            PopupKind::Tool => (3u32, TOOLS.len() as u32),
            PopupKind::Color => (4u32, COLORS.len() as u32),
            PopupKind::Brush => (6u32, BRUSHES.len() as u32),
        };
        let rows = total.div_ceil(cols);
        let row = self.selected / cols;
        let col = self.selected % cols;
        let new_col = (col as i32 + dx).clamp(0, cols as i32 - 1) as u32;
        let new_row = (row as i32 + dy).clamp(0, rows as i32 - 1) as u32;
        let new_selected = new_row * cols + new_col;
        self.selected = new_selected.min(total - 1);
    }
}

const _: () = assert!(u32::MAX as usize >= 256, "u32 must fit popup selections");

impl ModeHandler for SelectionPopupState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.selected < 256, "Selection index should be reasonable");
        match key.code {
            // hjkl navigation
            KeyCode::Char('h') | KeyCode::Left => {
                self.navigate(-1, 0);
                ModeTransition::Stay
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.navigate(1, 0);
                ModeTransition::Stay
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.navigate(0, 1);
                ModeTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.navigate(0, -1);
                ModeTransition::Stay
            }
            // Enter to confirm
            KeyCode::Enter => {
                ctx.app.confirm_popup_selection_with_index(self.kind, self.selected as u32);
                ModeTransition::Normal
            }
            // Escape to cancel
            KeyCode::Esc => {
                ModeTransition::Normal
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        match self.kind {
            PopupKind::Tool => "TOOL",
            PopupKind::Color => "COLOR",
            PopupKind::Brush => "BRUSH",
        }
    }

    fn mode_color(&self) -> Color {
        Color::Magenta
    }

    fn help_text(&self) -> &'static str {
        "hjkl/arrows to navigate, Enter to select, Esc to cancel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_state() -> SelectionPopupState {
        SelectionPopupState {
            kind: PopupKind::Tool,
            selected: 0,
            trigger_key: None,
        }
    }

    #[test]
    fn test_escape_cancels() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_enter_confirms() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_navigation_stays() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('h'))),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('j'))),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('k'))),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('l'))),
            ModeTransition::Stay
        ));
    }

    #[test]
    fn test_mode_names() {
        assert_eq!(
            SelectionPopupState {
                kind: PopupKind::Tool,
                selected: 0,
                trigger_key: None,
            }
            .mode_name(),
            "TOOL"
        );
        assert_eq!(
            SelectionPopupState {
                kind: PopupKind::Color,
                selected: 0,
                trigger_key: None,
            }
            .mode_name(),
            "COLOR"
        );
        assert_eq!(
            SelectionPopupState {
                kind: PopupKind::Brush,
                selected: 0,
                trigger_key: None,
            }
            .mode_name(),
            "BRUSH"
        );
    }
}
