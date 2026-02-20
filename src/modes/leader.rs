//! Leader menu mode handler - Helix-style space/colon menu.
//!
//! Provides single-keypress access to common commands like save, open, export,
//! and popup menus for tool/color/brush selection.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{
    HelpScreenState, LeaderMenuState, Mode, ModeAction, ModeContext, ModeHandler, ModeTransition,
    PathInputKind, PathInputState, SelectionPopupState,
};
use crate::app::{PopupKind, BRUSHES, COLORS, TOOLS};

impl ModeHandler for LeaderMenuState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        match key.code {
            // Tool selection popup
            KeyCode::Char('t') | KeyCode::Char(' ') => {
                let idx = TOOLS
                    .iter()
                    .position(|&t| t == ctx.app.current_tool)
                    .unwrap_or(0);
                ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Tool,
                    selected: idx,
                    trigger_key: None,
                }))
            }

            // Color selection popup
            KeyCode::Char('c') => {
                let idx = COLORS
                    .iter()
                    .position(|&c| c == ctx.app.current_color)
                    .unwrap_or(0);
                ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Color,
                    selected: idx,
                    trigger_key: None,
                }))
            }

            // Brush selection popup
            KeyCode::Char('b') => {
                let idx = BRUSHES
                    .iter()
                    .position(|&b| b == ctx.app.brush_char)
                    .unwrap_or(0);
                ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Brush,
                    selected: idx,
                    trigger_key: None,
                }))
            }

            // Help screen
            KeyCode::Char('?') | KeyCode::Char('h') => {
                ModeTransition::to(Mode::HelpScreen(HelpScreenState { scroll: 0 }))
            }

            // Save file
            KeyCode::Char('s') => {
                let path = ctx
                    .app
                    .file_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ModeTransition::to(Mode::PathInput(PathInputState {
                    path,
                    kind: PathInputKind::FileSave,
                }))
            }

            // Open file
            KeyCode::Char('o') => ModeTransition::to(Mode::PathInput(PathInputState {
                path: String::new(),
                kind: PathInputKind::FileOpen,
            })),

            // Export SVG
            KeyCode::Char('e') => {
                let path = ctx
                    .app
                    .file_path
                    .as_ref()
                    .map(|p| p.with_extension("svg").display().to_string())
                    .unwrap_or_else(|| "export.svg".to_string());
                ModeTransition::to(Mode::PathInput(PathInputState {
                    path,
                    kind: PathInputKind::SvgExport,
                }))
            }

            // New document
            KeyCode::Char('n') => {
                ctx.app.request_new_document();
                ModeTransition::Normal
            }

            // Toggle grid
            KeyCode::Char('g') => {
                ctx.app.toggle_grid();
                ModeTransition::Normal
            }

            // Toggle layer panel
            KeyCode::Char('l') => {
                ctx.app.toggle_layer_panel();
                ModeTransition::Normal
            }

            // Toggle participants
            KeyCode::Char('p') => {
                ctx.app.show_participants = !ctx.app.show_participants;
                let msg = if ctx.app.show_participants {
                    "Participants shown"
                } else {
                    "Participants hidden"
                };
                ctx.app.set_status(msg);
                ModeTransition::Normal
            }

            // Quit
            KeyCode::Char('q') => ModeTransition::Action(ModeAction::Quit),

            // Cancel / back to normal
            KeyCode::Esc => ModeTransition::Normal,

            // Any other key dismisses the menu (Helix-style)
            _ => ModeTransition::Normal,
        }
    }

    fn mode_name(&self) -> &'static str {
        "SPACE"
    }

    fn mode_color(&self) -> Color {
        Color::Cyan
    }

    fn help_text(&self) -> &'static str {
        "t:tool c:color b:brush s:save o:open e:export g:grid l:layers ?:help q:quit"
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
    fn test_esc_returns_normal() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Esc));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_t_returns_tool_popup() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('t')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Tool,
                    ..
                }) => (),
                _ => panic!("Expected tool popup mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_space_returns_tool_popup() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char(' ')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Tool,
                    ..
                }) => (),
                _ => panic!("Expected tool popup mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_help_returns_help_screen() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('?')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::HelpScreen(_) => (),
                _ => panic!("Expected help screen mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_h_returns_help_screen() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('h')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::HelpScreen(_) => (),
                _ => panic!("Expected help screen mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_q_returns_quit_action() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('q')));
        match result {
            ModeTransition::Action(ModeAction::Quit) => (),
            _ => panic!("Expected quit action"),
        }
    }

    #[test]
    fn test_g_toggles_grid() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let initial_grid = ctx.app.grid_enabled;
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('g')));
        assert!(matches!(result, ModeTransition::Normal));
        assert_eq!(ctx.app.grid_enabled, !initial_grid);
    }

    #[test]
    fn test_l_toggles_layer_panel() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let initial_layers = ctx.app.show_layers;
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('l')));
        assert!(matches!(result, ModeTransition::Normal));
        assert_eq!(ctx.app.show_layers, !initial_layers);
    }

    #[test]
    fn test_p_toggles_participants() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let initial_participants = ctx.app.show_participants;
        let result = state.handle_key(&mut ctx, key(KeyCode::Char('p')));
        assert!(matches!(result, ModeTransition::Normal));
        assert_eq!(ctx.app.show_participants, !initial_participants);
    }

    #[test]
    fn test_unknown_key_returns_normal() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('x')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_s_returns_file_save_mode() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('s')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::FileSave,
                    ..
                }) => (),
                _ => panic!("Expected file save mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_o_returns_file_open_mode() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('o')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::FileOpen,
                    ..
                }) => (),
                _ => panic!("Expected file open mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_e_returns_svg_export_mode() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('e')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::SvgExport,
                    ..
                }) => (),
                _ => panic!("Expected SVG export mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_c_returns_color_popup() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('c')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Color,
                    ..
                }) => (),
                _ => panic!("Expected color popup mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_b_returns_brush_popup() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('b')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Brush,
                    ..
                }) => (),
                _ => panic!("Expected brush popup mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }
}
