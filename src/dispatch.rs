//! Action dispatching for the leader key menu.
//!
//! Translates Action variants into the actual app method calls and mode transitions.
//! This centralizes the mapping between actions and their implementations.



use crate::actions::Action;
use crate::app::{PopupKind, Tool, TOOLS, COLORS, BRUSHES};
use crate::modes::{
    Mode, ModeAction, ModeContext, ModeTransition, PathInputKind, PathInputState,
    HelpScreenState, QrCodeDisplayState, SelectionPopupState,
};

/// Dispatch an action to the appropriate app method and return the mode transition.
///
/// This function centralizes the mapping between Action variants and their implementations,
/// replacing the direct key handling logic from the old leader.rs module.
pub fn dispatch_action(action: Action, ctx: &mut ModeContext<'_>) -> ModeTransition {
    match action {
        // Direct tool selection
        Action::SelectTool => {
            ctx.app.set_tool(Tool::Select);
            ModeTransition::Normal
        }
        Action::FreehandTool => {
            ctx.app.set_tool(Tool::Freehand);
            ModeTransition::Normal
        }
        Action::TextTool => {
            ctx.app.set_tool(Tool::Text);
            ModeTransition::Normal
        }
        Action::LineTool => {
            ctx.app.set_tool(Tool::Line);
            ModeTransition::Normal
        }
        Action::ArrowTool => {
            ctx.app.set_tool(Tool::Arrow);
            ModeTransition::Normal
        }
        Action::RectangleTool => {
            ctx.app.set_tool(Tool::Rectangle);
            ModeTransition::Normal
        }
        Action::DoubleBoxTool => {
            ctx.app.set_tool(Tool::DoubleBox);
            ModeTransition::Normal
        }
        Action::DiamondTool => {
            ctx.app.set_tool(Tool::Diamond);
            ModeTransition::Normal
        }
        Action::EllipseTool => {
            ctx.app.set_tool(Tool::Ellipse);
            ModeTransition::Normal
        }
        Action::TriangleTool => {
            ctx.app.set_tool(Tool::Triangle);
            ModeTransition::Normal
        }
        Action::ParallelogramTool => {
            ctx.app.set_tool(Tool::Parallelogram);
            ModeTransition::Normal
        }
        Action::HexagonTool => {
            ctx.app.set_tool(Tool::Hexagon);
            ModeTransition::Normal
        }
        Action::TrapezoidTool => {
            ctx.app.set_tool(Tool::Trapezoid);
            ModeTransition::Normal
        }
        Action::RoundedRectTool => {
            ctx.app.set_tool(Tool::RoundedRect);
            ModeTransition::Normal
        }
        Action::CylinderTool => {
            ctx.app.set_tool(Tool::Cylinder);
            ModeTransition::Normal
        }
        Action::CloudTool => {
            ctx.app.set_tool(Tool::Cloud);
            ModeTransition::Normal
        }
        Action::StarTool => {
            ctx.app.set_tool(Tool::Star);
            ModeTransition::Normal
        }

        // Popup commands
        Action::ToolPopup => {
            debug_assert!(TOOLS.len() > 0, "TOOLS array must not be empty");
            let idx = TOOLS
                .iter()
                .position(|&t| t == ctx.app.current_tool)
                .unwrap_or(0) as u32;
            ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                kind: PopupKind::Tool,
                selected: idx,
                trigger_key: None,
            }))
        }
        Action::ColorPopup => {
            debug_assert!(COLORS.len() > 0, "COLORS array must not be empty");
            let idx = COLORS
                .iter()
                .position(|&c| c == ctx.app.current_color)
                .unwrap_or(0) as u32;
            ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                kind: PopupKind::Color,
                selected: idx,
                trigger_key: None,
            }))
        }
        Action::BrushPopup => {
            debug_assert!(BRUSHES.len() > 0, "BRUSHES array must not be empty");
            let idx = BRUSHES
                .iter()
                .position(|&b| b == ctx.app.brush_char)
                .unwrap_or(0) as u32;
            ModeTransition::to(Mode::SelectionPopup(SelectionPopupState {
                kind: PopupKind::Brush,
                selected: idx,
                trigger_key: None,
            }))
        }

        // File operations
        Action::FileSave => {
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
        Action::FileOpen => ModeTransition::to(Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::FileOpen,
        })),
        Action::SvgExport => {
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
        Action::NewDocument => {
            ctx.app.request_new_document();
            ModeTransition::Normal
        }

        // Sync/collaboration operations
        Action::CopyTicket => {
            ctx.app.copy_ticket_to_clipboard();
            ModeTransition::Normal
        }
        Action::ShowQrCode => {
            if let Some(ref ticket) = ctx.app.sync_ticket {
                ModeTransition::to(Mode::QrCodeDisplay(QrCodeDisplayState {
                    ticket: ticket.clone(),
                }))
            } else {
                ctx.app.set_status("No sync session active");
                ModeTransition::Normal
            }
        }
        Action::DecodeQr => ModeTransition::to(Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::QrDecode,
        })),
        Action::ClusterConnect => ModeTransition::to(Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::ClusterConnect,
        })),
        Action::JoinSession => ModeTransition::to(Mode::PathInput(PathInputState {
            path: String::new(),
            kind: PathInputKind::JoinSession,
        })),

        // View operations
        Action::ToggleGrid => {
            ctx.app.toggle_grid();
            ModeTransition::Normal
        }
        Action::ToggleLayers => {
            ctx.app.toggle_layer_panel();
            ModeTransition::Normal
        }
        Action::ToggleParticipants => {
            ctx.app.show_participants = !ctx.app.show_participants;
            let msg = if ctx.app.show_participants {
                "Participants shown"
            } else {
                "Participants hidden"
            };
            ctx.app.set_status(msg);
            ModeTransition::Normal
        }

        // App operations
        Action::ShowHelp => {
            ModeTransition::to(Mode::HelpScreen(HelpScreenState { scroll: 0 }))
        }
        Action::Quit => ModeTransition::Action(ModeAction::Quit),

        // Edit operations (for future use)
        Action::Undo => {
            ctx.app.undo();
            ModeTransition::Normal
        }
        Action::Redo => {
            ctx.app.redo();
            ModeTransition::Normal
        }
        Action::Copy => {
            ctx.app.yank();
            ModeTransition::Normal
        }
        Action::Paste => {
            ctx.app.paste();
            ModeTransition::Normal
        }
        Action::DeleteSelected => {
            ctx.app.delete_selected();
            ModeTransition::Normal
        }
        Action::SelectAll => {
            ctx.app.select_all();
            ModeTransition::Normal
        }
        Action::ClearSelection => {
            ctx.app.cancel_shape();
            ctx.app.clear_selection();
            ModeTransition::Normal
        }
        Action::DuplicateSelected => {
            ctx.app.duplicate_selection();
            ModeTransition::Normal
        }
        Action::GroupSelection => {
            ctx.app.group_selection();
            ModeTransition::Normal
        }
        Action::UngroupSelection => {
            ctx.app.ungroup_selection();
            ModeTransition::Normal
        }
        Action::CycleLineStyle => {
            ctx.app.cycle_line_style();
            ModeTransition::Normal
        }
        Action::StartLabelInput => {
            let is_select = ctx.app.current_tool == Tool::Select;
            let has_selection = !ctx.app.selected.is_empty();
            if is_select && has_selection && ctx.app.start_label_input() {
                ctx.app.set_status("Editing label - type text, Enter/Esc to finish");
            }
            ModeTransition::Normal
        }
        
        // Z-order operations
        Action::BringForward => {
            ctx.app.bring_forward();
            ModeTransition::Normal
        }
        Action::SendBackward => {
            ctx.app.send_backward();
            ModeTransition::Normal
        }
        Action::BringToFront => {
            ctx.app.bring_to_front();
            ModeTransition::Normal
        }
        Action::SendToBack => {
            ctx.app.send_to_back();
            ModeTransition::Normal
        }
        
        // Alignment operations
        Action::AlignLeft => {
            ctx.app.align_left();
            ModeTransition::Normal
        }
        Action::AlignRight => {
            ctx.app.align_right();
            ModeTransition::Normal
        }
        Action::AlignTop => {
            ctx.app.align_top();
            ModeTransition::Normal
        }
        Action::AlignBottom => {
            ctx.app.align_bottom();
            ModeTransition::Normal
        }
        Action::AlignCenterHorizontal => {
            ctx.app.align_center_h();
            ModeTransition::Normal
        }
        Action::AlignCenterVertical => {
            ctx.app.align_center_v();
            ModeTransition::Normal
        }
        
        // Transform operations
        Action::FlipHorizontal => {
            ctx.app.flip_horizontal();
            ModeTransition::Normal
        }
        Action::FlipVertical => {
            ctx.app.flip_vertical();
            ModeTransition::Normal
        }
        Action::Rotate90Clockwise => {
            ctx.app.rotate_90_cw();
            ModeTransition::Normal
        }
        Action::Rotate90CounterClockwise => {
            ctx.app.rotate_90_ccw();
            ModeTransition::Normal
        }
        Action::DistributeHorizontal => {
            debug_assert!(ctx.app.selected.len() >= 3, "Distribute requires 3+ shapes");
            ctx.app.distribute_horizontal();
            ModeTransition::Normal
        }
        Action::DistributeVertical => {
            debug_assert!(ctx.app.selected.len() >= 3, "Distribute requires 3+ shapes");
            ctx.app.distribute_vertical();
            ModeTransition::Normal
        }
        
        // Keyboard shape creation
        Action::CreateKeyboardRectangle => {
            ctx.app.start_keyboard_shape_create(Tool::Rectangle);
            ModeTransition::Normal
        }
        Action::CreateKeyboardDiamond => {
            ctx.app.start_keyboard_shape_create(Tool::Diamond);
            ModeTransition::Normal
        }
        Action::CreateKeyboardEllipse => {
            ctx.app.start_keyboard_shape_create(Tool::Ellipse);
            ModeTransition::Normal
        }
        Action::CreateKeyboardLine => {
            ctx.app.start_keyboard_shape_create(Tool::Line);
            ModeTransition::Normal
        }
        Action::CreateKeyboardArrow => {
            ctx.app.start_keyboard_shape_create(Tool::Arrow);
            ModeTransition::Normal
        }
        Action::CreateKeyboardDoubleBox => {
            ctx.app.start_keyboard_shape_create(Tool::DoubleBox);
            ModeTransition::Normal
        }

        // Layer operations
        Action::NewLayer => {
            ctx.app.create_layer();
            ModeTransition::Normal
        }
        Action::MoveSelectionToActiveLayer => {
            ctx.app.move_selection_to_active_layer();
            ModeTransition::Normal
        }
        Action::ToggleActiveLayerVisibility => {
            ctx.app.toggle_active_layer_visibility();
            ModeTransition::Normal
        }
        Action::StartLayerRename => {
            ctx.app.start_layer_rename();
            ModeTransition::Normal
        }
        Action::SelectLayerByIndex(index) => {
            ctx.app.select_layer_by_index(index.into());
            ModeTransition::Normal
        }
        Action::DeleteLayer(layer_id) => {
            let _ = ctx.app.doc.delete_layer(layer_id);
            ModeTransition::Normal
        }
        Action::ToggleLayerVisible(layer_id) => {
            ctx.app.toggle_layer_visibility(layer_id);
            ModeTransition::Normal
        }
        Action::ToggleLayerLock(layer_id) => {
            ctx.app.toggle_layer_locked(layer_id);
            ModeTransition::Normal
        }
        Action::RenameLayer(_layer_id) => {
            // Would transition to layer rename mode
            // For now, just return normal
            ModeTransition::Normal
        }
        Action::SelectLayerUp => {
            // Implementation for selecting layer up
            ModeTransition::Normal
        }
        Action::SelectLayerDown => {
            // Implementation for selecting layer down
            ModeTransition::Normal
        }

        // View operations (for future use)
        Action::ZoomIn => {
            ctx.app.viewport.zoom_in();
            ModeTransition::Normal
        }
        Action::ZoomOut => {
            ctx.app.viewport.zoom_out();
            ModeTransition::Normal
        }
        Action::ZoomReset => {
            ctx.app.viewport.reset_zoom();
            ModeTransition::Normal
        }
        Action::CenterView => {
            // Implementation for centering view
            ModeTransition::Normal
        }

        // Session operations (for future use)
        Action::SessionBrowser => {
            // Would transition to session browser mode
            // For now, just return normal
            ModeTransition::Normal
        }
        Action::RecentFiles => {
            // Would transition to recent files mode
            // For now, just return normal
            ModeTransition::Normal
        }
    }
}