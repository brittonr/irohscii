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

const _: () = {
    // Ensure PathInputKind covers all expected cases
    const fn check_path_kind_coverage() {
        let _ = PathInputKind::FileSave;
        let _ = PathInputKind::FileOpen;
        let _ = PathInputKind::DocSave;
        let _ = PathInputKind::DocOpen;
        let _ = PathInputKind::SvgExport;
        let _ = PathInputKind::ClusterConnect;
        let _ = PathInputKind::QrDecode;
        let _ = PathInputKind::JoinSession;
    }
    check_path_kind_coverage();
};

impl ModeHandler for PathInputState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(self.path.len() <= 4096, "Path length should be reasonable");
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
                // Path completion — operates on self.path directly since
                // app.mode is temporarily taken during handle_key dispatch
                self.complete_path(ctx);
                ModeTransition::Stay
            }
            KeyCode::Backspace => {
                // Modify self.path directly (app.mode is taken during dispatch)
                self.path.pop();
                ModeTransition::Stay
            }
            KeyCode::Char(c) => {
                // Modify self.path directly (app.mode is taken during dispatch)
                self.path.push(c);
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
    /// Tab-complete the current path using rat-widgets path_completer.
    fn complete_path(&mut self, ctx: &mut ModeContext<'_>) {
        debug_assert!(self.path.len() <= 4096);
        
        let matches = rat_widgets::path_completer(&self.path);
        
        if matches.is_empty() {
            ctx.app.set_status("No matches");
            return;
        }

        if matches.len() == 1 {
            // Single match - use it directly
            self.path = matches[0].clone();
        } else {
            // Multiple matches - find longest common prefix
            let first = &matches[0];
            let common_len = matches.iter().skip(1).fold(first.len(), |len, s| {
                first
                    .chars()
                    .zip(s.chars())
                    .take(len)
                    .take_while(|(a, b)| a == b)
                    .count()
            });
            let common: String = first.chars().take(common_len).collect();
            if !common.is_empty() && common != self.path {
                self.path = common;
            }
            ctx.app.set_status(format!("{} matches", matches.len()));
        }
    }

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
            PathInputKind::ClusterConnect => {
                ctx.app.execute_cluster_connect(&self.path);
            }
            PathInputKind::QrDecode => {
                ctx.app.execute_qr_decode(&self.path);
            }
            PathInputKind::JoinSession => {
                ctx.app.execute_join_session(&self.path);
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
        assert_eq!(
            test_state(PathInputKind::ClusterConnect).mode_name(),
            "CLUSTER"
        );
        assert_eq!(
            test_state(PathInputKind::QrDecode).mode_name(),
            "QR DECODE"
        );
        assert_eq!(
            test_state(PathInputKind::JoinSession).mode_name(),
            "JOIN"
        );
    }
}
