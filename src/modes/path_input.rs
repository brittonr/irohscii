//! Path input mode handler for file save/open dialogs.
//!
//! This handles the keyboard navigation for all path input modes:
//! FileSave, FileOpen, DocSave, DocOpen, SvgExport.
//!
//! The actual file I/O operations are handled separately after
//! the mode returns a transition.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{ModeContext, ModeHandler, ModeTransition, PathInputKind, PathInputState};

/// Result from a path input operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathInputResult {
    /// User cancelled the operation
    Cancelled,
    /// User confirmed with this path
    Confirmed(String, PathInputKind),
}

impl ModeHandler for PathInputState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            KeyCode::Esc => {
                // Cancel - just return to normal
                ModeTransition::Normal
            }
            KeyCode::Enter => {
                // Execute the path operation based on kind
                self.execute_path_operation(ctx);
                ModeTransition::Normal
            }
            KeyCode::Tab => {
                // Path completion
                ctx.app.complete_path();
                ModeTransition::Stay
            }
            KeyCode::Backspace => {
                ctx.app.backspace_path();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                ctx.app.add_path_char(c);
                ModeTransition::Stay
            }
            _ => ModeTransition::Stay,
        }
    }

    fn mode_name(&self) -> &'static str {
        self.kind.mode_name()
    }

    fn mode_color(&self) -> Color {
        Color::Yellow
    }

    fn help_text(&self) -> &'static str {
        "Type path, Tab to complete, Enter to confirm, Esc to cancel"
    }
}

impl PathInputState {
    /// Execute the path operation based on the kind.
    /// This delegates to App methods that handle the actual I/O.
    fn execute_path_operation(&self, ctx: &mut ModeContext<'_>) {
        match self.kind {
            PathInputKind::FileSave => {
                ctx.app.execute_file_save(&self.path);
            }
            PathInputKind::FileOpen => {
                ctx.app.execute_file_open(&self.path);
            }
            PathInputKind::DocSave => {
                ctx.app.execute_doc_save(&self.path);
            }
            PathInputKind::DocOpen => {
                ctx.app.execute_doc_open(&self.path);
            }
            PathInputKind::SvgExport => {
                ctx.app.execute_svg_export(&self.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn test_state(kind: PathInputKind) -> PathInputState {
        PathInputState {
            path: "/tmp/test.txt".to_string(),
            kind,
        }
    }

    #[test]
    fn test_escape_cancels() {
        let mut state = test_state(PathInputKind::FileSave);
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_enter_confirms() {
        let mut state = test_state(PathInputKind::FileSave);
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Enter));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_tab_completes_path() {
        let mut state = test_state(PathInputKind::FileOpen);
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Tab));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_char_input_stays() {
        let mut state = test_state(PathInputKind::DocSave);
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('x')));
        assert!(matches!(result, ModeTransition::Stay));
    }

    #[test]
    fn test_mode_names() {
        assert_eq!(
            test_state(PathInputKind::FileSave).mode_name(),
            "SAVE"
        );
        assert_eq!(
            test_state(PathInputKind::FileOpen).mode_name(),
            "OPEN"
        );
        assert_eq!(
            test_state(PathInputKind::DocSave).mode_name(),
            "SAVE DOC"
        );
        assert_eq!(
            test_state(PathInputKind::DocOpen).mode_name(),
            "OPEN DOC"
        );
        assert_eq!(
            test_state(PathInputKind::SvgExport).mode_name(),
            "SVG EXPORT"
        );
    }
}
