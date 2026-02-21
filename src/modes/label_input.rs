//! Label input mode handler for editing shape labels.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{LabelInputState, ModeContext, ModeHandler, ModeTransition};

impl ModeHandler for LabelInputState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.cursor as usize <= self.text.len(), "Cursor must be within text bounds");
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                ctx.app.commit_label();
                ModeTransition::Normal
            }
            KeyCode::Backspace => {
                ctx.app.backspace_label();
                ModeTransition::Stay
            }
            KeyCode::Delete => {
                ctx.app.delete_label_char();
                ModeTransition::Stay
            }
            KeyCode::Left => {
                ctx.app.move_label_cursor_left();
                ModeTransition::Stay
            }
            KeyCode::Right => {
                ctx.app.move_label_cursor_right();
                ModeTransition::Stay
            }
            KeyCode::Home => {
                ctx.app.move_label_cursor_home();
                ModeTransition::Stay
            }
            KeyCode::End => {
                ctx.app.move_label_cursor_end();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                ctx.app.add_label_char(c);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "LABEL"
    }

    fn mode_color(&self) -> Color {
        Color::Green
    }

    fn help_text(&self) -> &'static str {
        "Edit label, arrows to move cursor, Enter/Esc to finish"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use irohscii_core::ShapeId;
    use uuid::Uuid;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_state() -> LabelInputState {
        LabelInputState {
            shape_id: ShapeId(Uuid::new_v4()),
            text: "test".to_string(),
            cursor: 4,
        }
    }

    #[test]
    fn test_escape_commits() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
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
    fn test_char_input_stays() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('x')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_cursor_movement_stays() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Left)),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Right)),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Home)),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::End)),
            ModeTransition::Stay
        ));
    }
}
