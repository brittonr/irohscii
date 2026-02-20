//! QR code display mode handler.
//!
//! Shows a sync ticket rendered as a QR code in the terminal.
//! Press Esc or any key to dismiss.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, QrCodeDisplayState};

impl ModeHandler for QrCodeDisplayState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // Copy ticket to clipboard, then dismiss
            KeyCode::Char('y') => {
                ctx.app.copy_ticket_to_clipboard();
                ModeTransition::Normal
            }
            // Any other key dismisses the QR code display
            _ => ModeTransition::Normal,
        }
    }

    fn mode_name(&self) -> &'static str {
        "QR CODE"
    }

    fn mode_color(&self) -> Color {
        Color::Magenta
    }

    fn help_text(&self) -> &'static str {
        "y: copy ticket, any key: dismiss"
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
    fn test_escape_dismisses() {
        let mut state = QrCodeDisplayState {
            ticket: "irohscii1TEST".to_string(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_enter_dismisses() {
        let mut state = QrCodeDisplayState {
            ticket: "irohscii1TEST".to_string(),
        };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_y_copies_and_dismisses() {
        let mut state = QrCodeDisplayState {
            ticket: "irohscii1TEST".to_string(),
        };
        let mut app = crate::app::App::new(80, 24);
        app.sync_ticket = Some("irohscii1TEST".to_string());
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('y')));
        assert!(matches!(result, ModeTransition::Normal));
    }
}
