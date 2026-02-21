//! Recent files browser mode handler.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, RecentFilesState};

impl ModeHandler for RecentFilesState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.selected < 1000, "Selected index should be reasonable");
        match key.code {
            KeyCode::Esc => ModeTransition::Normal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                ModeTransition::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_usize = ctx.app.recent_files.len().saturating_sub(1);
                let max = u32::try_from(max_usize).unwrap_or(u32::MAX);
                self.selected = (self.selected + 1).min(max);
                ModeTransition::Stay
            }
            KeyCode::Enter => {
                ctx.app.open_recent_file(self.selected);
                ModeTransition::Normal
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        "RECENT"
    }

    fn mode_color(&self) -> Color {
        Color::Cyan
    }

    fn help_text(&self) -> &'static str {
        "jk/arrows: navigate, Enter: open, Esc: close"
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
    fn test_escape_closes() {
        let mut state = RecentFilesState { selected: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_navigation_stays() {
        let mut state = RecentFilesState { selected: 1 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('k')));
        assert!(matches!(result, ModeTransition::Stay));
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_enter_opens() {
        let mut state = RecentFilesState { selected: 0 };
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }
}
