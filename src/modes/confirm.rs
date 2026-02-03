//! Confirm dialog mode handler.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ConfirmDialogState, ModeContext, ModeHandler, ModeTransition};

impl ModeHandler for ConfirmDialogState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // Yes - confirm the action
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                ctx.app.confirm_pending_action();
                ModeTransition::Normal
            }
            // No - cancel the action
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                ctx.app.cancel_pending_action();
                ModeTransition::Normal
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "CONFIRM"
    }

    fn mode_color(&self) -> Color {
        Color::Red
    }

    fn help_text(&self) -> &'static str {
        "Press Y to confirm, N or Esc to cancel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PendingAction;
    use crossterm::event::KeyModifiers;
    use irohscii_core::LayerId;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_escape_cancels() {
        let mut state = ConfirmDialogState {
            action: PendingAction::NewDocument,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_n_cancels() {
        let mut state = ConfirmDialogState {
            action: PendingAction::NewDocument,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('n')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_y_confirms() {
        let mut state = ConfirmDialogState {
            action: PendingAction::NewDocument,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('y')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_enter_confirms() {
        let mut state = ConfirmDialogState {
            action: PendingAction::NewDocument,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_unrecognized_key_stays() {
        let mut state = ConfirmDialogState {
            action: PendingAction::NewDocument,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('x')));
        assert!(matches!(result, ModeTransition::Stay));
    }
}
