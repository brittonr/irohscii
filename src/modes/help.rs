//! Help screen mode handler.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{HelpScreenState, ModeContext, ModeHandler, ModeTransition};

impl ModeHandler for HelpScreenState {
    fn handle_key(&mut self, _ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // Close help
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::F(1) => {
                ModeTransition::Normal
            }
            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                ModeTransition::Stay
            }
            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                ModeTransition::Stay
            }
            // Page down
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll = self.scroll.saturating_add(10);
                ModeTransition::Stay
            }
            // Page up
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "HELP"
    }

    fn mode_color(&self) -> Color {
        Color::Cyan
    }

    fn help_text(&self) -> &'static str {
        "Press Esc or q to close help"
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
    fn test_escape_closes_help() {
        let mut state = HelpScreenState { scroll: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_q_closes_help() {
        let mut state = HelpScreenState { scroll: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('q')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_scroll_down() {
        let mut state = HelpScreenState { scroll: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('j')));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.scroll, 1);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = HelpScreenState { scroll: 5 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('k')));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.scroll, 4);
    }

    #[test]
    fn test_scroll_up_at_zero() {
        let mut state = HelpScreenState { scroll: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Up));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.scroll, 0); // Can't go below 0
    }

    #[test]
    fn test_page_down() {
        let mut state = HelpScreenState { scroll: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::PageDown));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.scroll, 10);
    }
}
