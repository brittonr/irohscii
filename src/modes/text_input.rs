//! Text input mode handler for typing text at a canvas position.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, TextInputState};

impl ModeHandler for TextInputState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.text.len() <= 4096, "Text length should be reasonable");
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                ctx.app.commit_text();
                ModeTransition::Normal
            }
            KeyCode::Backspace => {
                ctx.app.backspace_text();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                ctx.app.add_text_char(c);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "TEXT"
    }

    fn mode_color(&self) -> Color {
        Color::Green
    }

    fn help_text(&self) -> &'static str {
        "Type text, Enter/Esc to finish"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use irohscii_core::Position;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_escape_commits_text() {
        let mut state = TextInputState {
            start_pos: Position::new(0, 0),
            text: String::new(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_enter_commits_text() {
        let mut state = TextInputState {
            start_pos: Position::new(0, 0),
            text: String::new(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_char_input_stays() {
        let mut state = TextInputState {
            start_pos: Position::new(0, 0),
            text: String::new(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('a')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_backspace_stays() {
        let mut state = TextInputState {
            start_pos: Position::new(0, 0),
            text: "hello".to_string(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Backspace));
        assert!(matches!(result, ModeTransition::Stay));
    }
}
