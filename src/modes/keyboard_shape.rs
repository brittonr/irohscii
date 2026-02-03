//! Keyboard-based shape creation mode handler.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{KeyboardShapeState, ModeContext, ModeHandler, ModeTransition};
use crate::app::KeyboardShapeField;

impl ModeHandler for KeyboardShapeState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            KeyCode::Esc => {
                ctx.app.cancel_keyboard_shape();
                ModeTransition::Normal
            }
            KeyCode::Enter => {
                ctx.app.commit_keyboard_shape();
                ModeTransition::Normal
            }
            KeyCode::Tab => {
                // Toggle focus between width and height
                self.focus = match self.focus {
                    KeyboardShapeField::Width => KeyboardShapeField::Height,
                    KeyboardShapeField::Height => KeyboardShapeField::Width,
                };
                ModeTransition::Stay
            }
            KeyCode::Backspace => {
                let field = match self.focus {
                    KeyboardShapeField::Width => &mut self.width,
                    KeyboardShapeField::Height => &mut self.height,
                };
                field.pop();
                ModeTransition::Stay
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                let field = match self.focus {
                    KeyboardShapeField::Width => &mut self.width,
                    KeyboardShapeField::Height => &mut self.height,
                };
                // Limit to reasonable length
                if field.len() < 5 {
                    field.push(c);
                }
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "CREATE"
    }

    fn mode_color(&self) -> Color {
        Color::Yellow
    }

    fn help_text(&self) -> &'static str {
        "Type dimensions, Tab to switch field, Enter to create, Esc to cancel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Tool;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_state() -> KeyboardShapeState {
        KeyboardShapeState {
            tool: Tool::Rectangle,
            width: String::new(),
            height: String::new(),
            focus: KeyboardShapeField::Width,
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
    fn test_enter_commits() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_tab_toggles_focus() {
        let mut state = test_state();
        assert_eq!(state.focus, KeyboardShapeField::Width);

        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Tab));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.focus, KeyboardShapeField::Height);

        let result = state.handle_key(&mut ctx, key(KeyCode::Tab));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.focus, KeyboardShapeField::Width);
    }

    #[test]
    fn test_digit_input() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        // Width field is focused by default
        state.handle_key(&mut ctx, key(KeyCode::Char('1')));
        state.handle_key(&mut ctx, key(KeyCode::Char('0')));
        assert_eq!(state.width, "10");
        assert_eq!(state.height, "");

        // Switch to height
        state.handle_key(&mut ctx, key(KeyCode::Tab));
        state.handle_key(&mut ctx, key(KeyCode::Char('2')));
        state.handle_key(&mut ctx, key(KeyCode::Char('0')));
        assert_eq!(state.height, "20");
    }

    #[test]
    fn test_backspace() {
        let mut state = test_state();
        state.width = "123".to_string();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        state.handle_key(&mut ctx, key(KeyCode::Backspace));
        assert_eq!(state.width, "12");
    }

    #[test]
    fn test_negative_allowed() {
        let mut state = test_state();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        state.handle_key(&mut ctx, key(KeyCode::Char('-')));
        state.handle_key(&mut ctx, key(KeyCode::Char('5')));
        assert_eq!(state.width, "-5");
    }

    #[test]
    fn test_length_limit() {
        let mut state = test_state();
        state.width = "1234".to_string();
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        // At 4 chars, can add one more
        state.handle_key(&mut ctx, key(KeyCode::Char('5')));
        assert_eq!(state.width, "12345");

        // At 5 chars, can't add more
        state.handle_key(&mut ctx, key(KeyCode::Char('6')));
        assert_eq!(state.width, "12345"); // Still 5 chars
    }
}
