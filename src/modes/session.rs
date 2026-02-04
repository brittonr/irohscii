//! Session mode handlers for browser and create modes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{
    Mode, ModeAction, ModeContext, ModeHandler, ModeTransition, SessionBrowserState,
    SessionCreateState,
};

// ============================================================================
// Session Browser Mode
// ============================================================================

impl ModeHandler for SessionBrowserState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // Close browser
            KeyCode::Esc | KeyCode::Tab => ModeTransition::Normal,
            // Navigate down
            KeyCode::Char('j') | KeyCode::Down => {
                let len = ctx
                    .app
                    .get_filtered_sessions(&self.filter, self.show_pinned_only)
                    .len();
                if len > 0 {
                    self.selected = (self.selected + 1).min(len - 1);
                }
                ModeTransition::Stay
            }
            // Navigate up
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                ModeTransition::Stay
            }
            // Select session
            KeyCode::Enter => {
                let filtered =
                    ctx.app.get_filtered_sessions(&self.filter, self.show_pinned_only);
                if let Some(session) = filtered.get(self.selected) {
                    ModeTransition::Action(ModeAction::SwitchSession(session.id.clone()))
                } else {
                    ModeTransition::Normal
                }
            }
            // Create new session
            KeyCode::Char('n') => ModeTransition::to(Mode::SessionCreate(SessionCreateState {
                name: String::new(),
            })),
            // Delete session
            KeyCode::Char('d') | KeyCode::Delete => {
                let filtered =
                    ctx.app.get_filtered_sessions(&self.filter, self.show_pinned_only);
                if let Some(session) = filtered.get(self.selected) {
                    // Don't allow deleting current session
                    if ctx.app.current_session.as_ref() == Some(&session.id) {
                        ctx.app.set_error("Cannot delete the active session");
                        ModeTransition::Stay
                    } else {
                        ModeTransition::Action(ModeAction::DeleteSession(session.id.clone()))
                    }
                } else {
                    ModeTransition::Stay
                }
            }
            // Toggle pinned - returns action for main loop to handle
            KeyCode::Char('p') => {
                let filtered =
                    ctx.app.get_filtered_sessions(&self.filter, self.show_pinned_only);
                if let Some(session) = filtered.get(self.selected) {
                    ModeTransition::Action(ModeAction::ToggleSessionPin(session.id.clone()))
                } else {
                    ModeTransition::Stay
                }
            }
            // Toggle pinned-only filter
            KeyCode::Char('*') => {
                self.show_pinned_only = !self.show_pinned_only;
                self.selected = 0;
                ModeTransition::Stay
            }
            // Backspace for filter
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected = 0;
                ModeTransition::Stay
            }
            // Type to filter
            KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                self.filter.push(c);
                self.selected = 0;
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "SESSIONS"
    }

    fn mode_color(&self) -> Color {
        Color::Cyan
    }

    fn help_text(&self) -> &'static str {
        "jk/arrows: navigate, Enter: select, n: new, p: pin, d: delete, Esc: close"
    }
}

// ============================================================================
// Session Create Mode
// ============================================================================

impl ModeHandler for SessionCreateState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            KeyCode::Esc => ModeTransition::Normal,
            KeyCode::Enter => {
                let trimmed = self.name.trim();
                if trimmed.len() >= 2 {
                    ModeTransition::Action(ModeAction::CreateSession(trimmed.to_string()))
                } else {
                    ctx.app.set_error("Session name must be at least 2 characters");
                    ModeTransition::Stay
                }
            }
            KeyCode::Backspace => {
                self.name.pop();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                self.name.push(c);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "NEW SESSION"
    }

    fn mode_color(&self) -> Color {
        Color::Green
    }

    fn help_text(&self) -> &'static str {
        "Type session name, Enter to create, Esc to cancel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    // Session Browser Tests

    #[test]
    fn test_browser_escape_closes() {
        let mut state = SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_browser_navigation_stays() {
        let mut state = SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('j'))),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('k'))),
            ModeTransition::Stay
        ));
    }

    #[test]
    fn test_browser_enter_selects() {
        let mut state = SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_browser_n_opens_create() {
        let mut state = SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('n')));
        assert!(matches!(result, ModeTransition::To(_)));
    }

    #[test]
    fn test_browser_filter_chars() {
        let mut state = SessionBrowserState {
            selected: 0,
            filter: String::new(),
            show_pinned_only: false,
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Char('a'))),
            ModeTransition::Stay
        ));
        assert!(matches!(
            state.handle_key(&mut ctx, key(KeyCode::Backspace)),
            ModeTransition::Stay
        ));
    }

    // Session Create Tests

    #[test]
    fn test_create_escape_cancels() {
        let mut state = SessionCreateState {
            name: String::new(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_create_enter_confirms() {
        let mut state = SessionCreateState {
            name: "test".to_string(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(
            result,
            ModeTransition::Action(ModeAction::CreateSession(_))
        ));
    }

    #[test]
    fn test_create_char_input_stays() {
        let mut state = SessionCreateState {
            name: String::new(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('a')));
        assert!(matches!(result, ModeTransition::Stay));
    }
}
