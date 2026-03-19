//! Leader menu configuration using rat-leaderkey.
//!
//! Implements MenuContributor to register all irohscii's built-in leader key bindings.
//! This replaces the direct key handling logic in the old leader.rs module.

use rat_leaderkey::{
    LeaderAction, MenuContribution, MenuContributor, MenuPlacement, PRIORITY_BUILTIN,
};

use crate::actions::Action;

/// Builtin leader key menu contributions for irohscii.
pub struct IrohsciiBuiltins;

impl MenuContributor<Action> for IrohsciiBuiltins {
    fn menu_items(&self) -> Vec<MenuContribution<Action>> {
        vec![
            // Direct tool selection (matches ' ' in leader.rs)
            MenuContribution {
                key: ' ',
                label: "select".into(),
                action: LeaderAction::Action(Action::SelectTool),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            // Popup commands (matches 't', 'c', 'b' in leader.rs)
            MenuContribution {
                key: 't',
                label: "tool".into(),
                action: LeaderAction::Action(Action::ToolPopup),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'c',
                label: "color".into(),
                action: LeaderAction::Action(Action::ColorPopup),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'b',
                label: "brush".into(),
                action: LeaderAction::Action(Action::BrushPopup),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            // File operations (matches 's', 'o', 'e', 'n' in leader.rs - all direct root bindings)
            MenuContribution {
                key: 's',
                label: "save".into(),
                action: LeaderAction::Action(Action::FileSave),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'o',
                label: "open".into(),
                action: LeaderAction::Action(Action::FileOpen),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'e',
                label: "export".into(),
                action: LeaderAction::Action(Action::SvgExport),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'n',
                label: "new".into(),
                action: LeaderAction::Action(Action::NewDocument),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            // Sync/collaboration operations (matches 'T', 'Q', 'D', 'K', 'J' in leader.rs)
            MenuContribution {
                key: 'T',
                label: "ticket".into(),
                action: LeaderAction::Action(Action::CopyTicket),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'Q',
                label: "qr".into(),
                action: LeaderAction::Action(Action::ShowQrCode),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'D',
                label: "decode".into(),
                action: LeaderAction::Action(Action::DecodeQr),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'K',
                label: "cluster".into(),
                action: LeaderAction::Action(Action::ClusterConnect),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'J',
                label: "join".into(),
                action: LeaderAction::Action(Action::JoinSession),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            // View operations (matches 'g', 'l', 'p' in leader.rs)
            MenuContribution {
                key: 'g',
                label: "grid".into(),
                action: LeaderAction::Action(Action::ToggleGrid),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'l',
                label: "layers".into(),
                action: LeaderAction::Action(Action::ToggleLayers),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'p',
                label: "peers".into(),
                action: LeaderAction::Action(Action::ToggleParticipants),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            // App operations (matches '?', 'h', 'q' in leader.rs)
            MenuContribution {
                key: '?',
                label: "help".into(),
                action: LeaderAction::Action(Action::ShowHelp),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'h',
                label: "help".into(),
                action: LeaderAction::Action(Action::ShowHelp),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
            MenuContribution {
                key: 'q',
                label: "quit".into(),
                action: LeaderAction::Action(Action::Quit),
                placement: MenuPlacement::Root,
                priority: PRIORITY_BUILTIN,
                source: "builtin".into(),
            },
        ]
    }
}