//! Layer rename mode handler.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{LayerRenameState, ModeContext, ModeHandler, ModeTransition};

impl ModeHandler for LayerRenameState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.text.len() <= 256, "Layer name should be reasonable length");
        match key.code {
            KeyCode::Enter => {
                ctx.app.commit_layer_rename();
                ModeTransition::Normal
            }
            KeyCode::Esc => {
                ctx.app.cancel_layer_rename();
                ModeTransition::Normal
            }
            KeyCode::Backspace => {
                ctx.app.backspace_layer_rename();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                ctx.app.add_layer_rename_char(c);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "RENAME"
    }

    fn mode_color(&self) -> Color {
        Color::Green
    }

    fn help_text(&self) -> &'static str {
        "Type new name, Enter to save, Esc to cancel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use irohscii_core::LayerId;
    use uuid::Uuid;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_state() -> LayerRenameState {
        LayerRenameState {
            layer_id: LayerId(Uuid::new_v4()),
            text: "Layer 1".to_string(),
        }
    }

    #[test]
    fn test_enter_commits() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
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
    fn test_char_input_stays() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('x')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_backspace_stays() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Backspace));
        assert!(matches!(result, ModeTransition::Stay));
    }
}
