//! Selection popup mode handler for tool, color, and brush selection.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, SelectionPopupState};
use crate::app::PopupKind;

impl ModeHandler for SelectionPopupState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // hjkl navigation
            KeyCode::Char('h') | KeyCode::Left => {
                ctx.app.popup_navigate(-1, 0);
                ModeTransition::Stay
            }
            KeyCode::Char('l') | KeyCode::Right => {
                ctx.app.popup_navigate(1, 0);
                ModeTransition::Stay
            }
            KeyCode::Char('j') | KeyCode::Down => {
                ctx.app.popup_navigate(0, 1);
                ModeTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                ctx.app.popup_navigate(0, -1);
                ModeTransition::Stay
            }
            // Enter to confirm
            KeyCode::Enter => {
                ctx.app.confirm_popup_selection();
                ModeTransition::Normal
            }
            // Escape to cancel
            KeyCode::Esc => {
                ctx.app.cancel_popup();
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
