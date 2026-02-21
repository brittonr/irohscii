//! Leader menu mode handler - Helix-style space/colon menu.
//!
//! Provides single-keypress access to common commands like save, open, export,
//! and popup menus for tool/color/brush selection.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

use super::{
    HelpScreenState, LeaderMenuState, Mode, ModeAction, ModeContext, ModeHandler, ModeTransition,
    PathInputKind, PathInputState, QrCodeDisplayState, SelectionPopupState,
};
use crate::app::{PopupKind, Tool, BRUSHES, COLORS, TOOLS};

impl LeaderMenuState {
    /// Handle popup selection commands (tool, color, brush)
    fn handle_popup_selection(ctx: &mut ModeContext<'_>, key: KeyEvent) -> Option<ModeTransition> {
        debug_assert!(TOOLS.len() > 0 && COLORS.len() > 0 && BRUSHES.len() > 0);
        
        match key.code {
            KeyCode::Char(' ') => {
                ctx.app.set_tool(Tool::Select);
                Some(ModeTransition::Normal)
            }
            KeyCode::Char('t') => {
                let idx = TOOLS
                    .iter()
                    .position(|&t| t == ctx.app.current_tool)
                    .unwrap_or(0) as u32;
                Some(ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Tool,
                    selected: idx,
                    trigger_key: None,
                })))
            }
            KeyCode::Char('c') => {
                let idx = COLORS
                    .iter()
                    .position(|&c| c == ctx.app.current_color)
                    .unwrap_or(0) as u32;
                Some(ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Color,
                    selected: idx,
                    trigger_key: None,
                })))
            }
            KeyCode::Char('b') => {
                let idx = BRUSHES
                    .iter()
                    .position(|&b| b == ctx.app.brush_char)
                    .unwrap_or(0) as u32;
                Some(ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                    kind: PopupKind::Brush,
                    selected: idx,
                    trigger_key: None,
                })))
            }
            _ => None,
        }
    }

    /// Handle file operations (save, open, export)
    fn handle_file_ops(ctx: &mut ModeContext<'_>, key: KeyEvent) -> Option<ModeTransition> {
        match key.code {
            KeyCode::Char('s') => {
                let path = ctx
                    .app
                    .file_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                Some(ModeTransition::to(Mode::PathInput(PathInputState {
                    path,
                    kind: PathInputKind::FileSave,
                })))
            }
            KeyCode::Char('o') => Some(ModeTransition::to(Mode::PathInput(PathInputState {
                path: String::new(),
                kind: PathInputKind::FileOpen,
            }))),
            KeyCode::Char('e') => {
                let path = ctx
                    .app
                    .file_path
                    .as_ref()
                    .map(|p| p.with_extension("svg").display().to_string())
                    .unwrap_or_else(|| "export.svg".to_string());
                Some(ModeTransition::to(Mode::PathInput(PathInputState {
                    path,
                    kind: PathInputKind::SvgExport,
                })))
            }
            _ => None,
        }
    }

    /// Handle sync/networking commands (QR, cluster, join)
    fn handle_sync_ops(ctx: &mut ModeContext<'_>, key: KeyEvent) -> Option<ModeTransition> {
        match key.code {
            KeyCode::Char('T') => {
                ctx.app.copy_ticket_to_clipboard();
                Some(ModeTransition::Normal)
            }
            KeyCode::Char('Q') => {
                if let Some(ref ticket) = ctx.app.sync_ticket {
                    Some(ModeTransition::to(Mode::QrCodeDisplay(QrCodeDisplayState {
                        ticket: ticket.clone(),
                    })))
                } else {
                    ctx.app.set_status("No sync session active");
                    Some(ModeTransition::Normal)
                }
            }
            KeyCode::Char('D') => Some(ModeTransition::to(Mode::PathInput(PathInputState {
                path: String::new(),
                kind: PathInputKind::QrDecode,
            }))),
            KeyCode::Char('K') => Some(ModeTransition::to(Mode::PathInput(PathInputState {
                path: String::new(),
                kind: PathInputKind::ClusterConnect,
            }))),
            KeyCode::Char('J') => Some(ModeTransition::to(Mode::PathInput(PathInputState {
                path: String::new(),
                kind: PathInputKind::JoinSession,
            }))),
            _ => None,
        }
    }
}

impl ModeHandler for LeaderMenuState {
    fn handle_key(&mut self, ctx: &mut ModeContext<'_>, key: KeyEvent) -> ModeTransition {
        debug_assert!(TOOLS.len() > 0, "TOOLS array must not be empty");
        debug_assert!(COLORS.len() > 0, "COLORS array must not be empty");
        debug_assert!(BRUSHES.len() > 0, "BRUSHES array must not be empty");
        
        // Try helper functions
        if let Some(transition) = Self::handle_popup_selection(ctx, key) {
            return transition;
        }
        if let Some(transition) = Self::handle_file_ops(ctx, key) {
            return transition;
        }
        if let Some(transition) = Self::handle_sync_ops(ctx, key) {
            return transition;
        }
        // Handle remaining commands
        match key.code {
            KeyCode::Char('?') | KeyCode::Char('h') => {
                ModeTransition::to(Mode::HelpScreen(HelpScreenState { scroll: 0 }))
            }
            KeyCode::Char('n') => {
                ctx.app.request_new_document();
                ModeTransition::Normal
            }
            KeyCode::Char('g') => {
                ctx.app.toggle_grid();
                ModeTransition::Normal
            }
            KeyCode::Char('l') => {
                ctx.app.toggle_layer_panel();
                ModeTransition::Normal
            }
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
            KeyCode::Char('q') => ModeTransition::Action(ModeAction::Quit),
            KeyCode::Esc => ModeTransition::Normal,
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
        "Space:select t:tool c:color b:brush s:save o:open e:export n:new g:grid l:layers p:peers T:ticket Q:qr D:decode K:cluster J:join ?:help q:quit"
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
    fn test_space_sets_select_tool() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        // Set a different tool first
        ctx.app.set_tool(crate::app::Tool::Freehand);
        let result = state.handle_key(&mut ctx, key(KeyCode::Char(' ')));
        assert!(matches!(result, ModeTransition::Normal));
        assert_eq!(ctx.app.current_tool, crate::app::Tool::Select);
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
    fn test_shift_k_opens_cluster_connect() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('K')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::ClusterConnect,
                    ..
                }) => (),
                _ => panic!("Expected cluster connect mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_shift_j_opens_join_session() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('J')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::JoinSession,
                    ..
                }) => (),
                _ => panic!("Expected join session mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_shift_t_copies_ticket() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('T')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_shift_q_shows_qr_with_ticket() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        app.sync_ticket = Some("irohscii1TESTDATA".to_string());
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('Q')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::QrCodeDisplay(QrCodeDisplayState { ref ticket }) => {
                    assert_eq!(ticket, "irohscii1TESTDATA");
                }
                _ => panic!("Expected QR code display mode"),
            },
            _ => panic!("Expected To transition"),
        }
    }

    #[test]
    fn test_shift_q_no_ticket_returns_normal() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('Q')));
        assert!(matches!(result, ModeTransition::Normal));
    }

    #[test]
    fn test_shift_d_opens_qr_decode() {
        let mut state = LeaderMenuState;
        let mut app = crate::app::App::new(80, 24);
        let mut ctx = ModeContext { app: &mut app };

        let result = state.handle_key(&mut ctx, key(KeyCode::Char('D')));
        match result {
            ModeTransition::To(mode) => match *mode {
                Mode::PathInput(PathInputState {
                    kind: PathInputKind::QrDecode,
                    ..
                }) => (),
                _ => panic!("Expected QR decode path input mode"),
            },
            _ => panic!("Expected To transition"),
        }
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
