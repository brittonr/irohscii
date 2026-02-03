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
            KeyCode::Esc | KeyCode::Tab => {
                ctx.app.close_session_browser();
                ModeTransition::Normal
            }
            // Navigate
            KeyCode::Char('j') | KeyCode::Down => {
                ctx.app.session_browser_navigate(1);
                ModeTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                ctx.app.session_browser_navigate(-1);
                ModeTransition::Stay
            }
            // Select session
            KeyCode::Enter => {
                ctx.app.session_browser_select();
                // The app will set a flag for the main loop to handle session switching
                ModeTransition::Normal
            }
            // Create new session
            KeyCode::Char('n') => {
                ctx.app.open_session_create();
                ModeTransition::to(Mode::session_create())
            }
            // Delete session
            KeyCode::Char('d') | KeyCode::Delete => {
                ctx.app.session_browser_request_delete();
                ModeTransition::Stay
            }
            // Toggle pinned - returns action for main loop to handle
            KeyCode::Char('p') => {
                if let Some(session_id) = ctx.app.session_browser_toggle_pin() {
                    ModeTransition::Action(ModeAction::ToggleSessionPin(session_id))
                } else {
                    ModeTransition::Stay
                }
            }
            // Toggle pinned-only filter
            KeyCode::Char('*') => {
                ctx.app.session_browser_toggle_pinned();
                ModeTransition::Stay
            }
            // Backspace for filter
            KeyCode::Backspace => {
                ctx.app.session_browser_filter_backspace();
                ModeTransition::Stay
            }
            // Type to filter
            KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                ctx.app.session_browser_filter_char(c);
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
            KeyCode::Esc => {
                ctx.app.session_create_cancel();
                ModeTransition::Normal
            }
            KeyCode::Enter => {
                ctx.app.session_create_confirm();
                ModeTransition::Normal
            }
            KeyCode::Backspace => {
                ctx.app.session_create_backspace();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                ctx.app.session_create_char(c);
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
        assert!(matches!(result, ModeTransition::Normal));
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
