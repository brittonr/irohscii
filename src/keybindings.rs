//! Keymap configuration for irohscii normal mode.
//!
//! This module builds a keymap using rat-keymap that handles all the command-like
//! keybindings from normal mode. Movement keys and stateful actions remain as
//! direct matches in normal.rs.

use std::collections::HashMap;

use ratatui::crossterm::event::KeyCode;
use rat_keymap::{KeyCombo, Keymap};

use crate::actions::Action;

/// Input modes for the keymap
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputMode {
    /// Normal mode - the primary drawing/editing mode
    Normal,
}

/// Parse an action name string for overrides
pub fn parse_action_name(s: &str) -> Option<Action> {
    match s {
        // Tool selection
        "select_tool" => Some(Action::SelectTool),
        "freehand_tool" => Some(Action::FreehandTool),
        "text_tool" => Some(Action::TextTool),
        "line_tool" => Some(Action::LineTool),
        "arrow_tool" => Some(Action::ArrowTool),
        "rectangle_tool" => Some(Action::RectangleTool),
        "doublebox_tool" => Some(Action::DoubleBoxTool),
        "diamond_tool" => Some(Action::DiamondTool),
        "ellipse_tool" => Some(Action::EllipseTool),
        "triangle_tool" => Some(Action::TriangleTool),
        "parallelogram_tool" => Some(Action::ParallelogramTool),
        "hexagon_tool" => Some(Action::HexagonTool),
        "trapezoid_tool" => Some(Action::TrapezoidTool),
        "rounded_rect_tool" => Some(Action::RoundedRectTool),
        "cylinder_tool" => Some(Action::CylinderTool),
        "cloud_tool" => Some(Action::CloudTool),
        "star_tool" => Some(Action::StarTool),
        
        // Popup commands
        "tool_popup" => Some(Action::ToolPopup),
        "color_popup" => Some(Action::ColorPopup),
        "brush_popup" => Some(Action::BrushPopup),
        
        // File operations
        "file_save" => Some(Action::FileSave),
        "file_open" => Some(Action::FileOpen),
        "svg_export" => Some(Action::SvgExport),
        "new_document" => Some(Action::NewDocument),
        
        // View operations
        "toggle_grid" => Some(Action::ToggleGrid),
        "toggle_layers" => Some(Action::ToggleLayers),
        
        // App operations
        "show_help" => Some(Action::ShowHelp),
        "quit" => Some(Action::Quit),
        
        // Edit operations
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "copy" => Some(Action::Copy),
        "paste" => Some(Action::Paste),
        "delete_selected" => Some(Action::DeleteSelected),
        "select_all" => Some(Action::SelectAll),
        "clear_selection" => Some(Action::ClearSelection),
        "duplicate_selected" => Some(Action::DuplicateSelected),
        "group_selection" => Some(Action::GroupSelection),
        "ungroup_selection" => Some(Action::UngroupSelection),
        "cycle_line_style" => Some(Action::CycleLineStyle),
        "start_label_input" => Some(Action::StartLabelInput),
        
        // Z-order operations
        "bring_forward" => Some(Action::BringForward),
        "send_backward" => Some(Action::SendBackward),
        "bring_to_front" => Some(Action::BringToFront),
        "send_to_back" => Some(Action::SendToBack),
        
        // Alignment operations
        "align_left" => Some(Action::AlignLeft),
        "align_right" => Some(Action::AlignRight),
        "align_top" => Some(Action::AlignTop),
        "align_bottom" => Some(Action::AlignBottom),
        "align_center_horizontal" => Some(Action::AlignCenterHorizontal),
        "align_center_vertical" => Some(Action::AlignCenterVertical),
        
        // Transform operations
        "flip_horizontal" => Some(Action::FlipHorizontal),
        "flip_vertical" => Some(Action::FlipVertical),
        "rotate_90_clockwise" => Some(Action::Rotate90Clockwise),
        "rotate_90_counterclockwise" => Some(Action::Rotate90CounterClockwise),
        "distribute_horizontal" => Some(Action::DistributeHorizontal),
        "distribute_vertical" => Some(Action::DistributeVertical),
        
        // Keyboard shape creation
        "create_keyboard_rectangle" => Some(Action::CreateKeyboardRectangle),
        "create_keyboard_diamond" => Some(Action::CreateKeyboardDiamond),
        "create_keyboard_ellipse" => Some(Action::CreateKeyboardEllipse),
        "create_keyboard_line" => Some(Action::CreateKeyboardLine),
        "create_keyboard_arrow" => Some(Action::CreateKeyboardArrow),
        "create_keyboard_doublebox" => Some(Action::CreateKeyboardDoubleBox),
        
        // Layer operations
        "new_layer" => Some(Action::NewLayer),
        "move_selection_to_active_layer" => Some(Action::MoveSelectionToActiveLayer),
        "toggle_active_layer_visibility" => Some(Action::ToggleActiveLayerVisibility),
        "start_layer_rename" => Some(Action::StartLayerRename),
        "select_layer_1" => Some(Action::SelectLayerByIndex(1)),
        "select_layer_2" => Some(Action::SelectLayerByIndex(2)),
        "select_layer_3" => Some(Action::SelectLayerByIndex(3)),
        "select_layer_4" => Some(Action::SelectLayerByIndex(4)),
        "select_layer_5" => Some(Action::SelectLayerByIndex(5)),
        "select_layer_6" => Some(Action::SelectLayerByIndex(6)),
        "select_layer_7" => Some(Action::SelectLayerByIndex(7)),
        "select_layer_8" => Some(Action::SelectLayerByIndex(8)),
        "select_layer_9" => Some(Action::SelectLayerByIndex(9)),
        
        // Session operations
        "session_browser" => Some(Action::SessionBrowser),
        "recent_files" => Some(Action::RecentFiles),
        
        _ => None,
    }
}

/// Convenience function for creating a KeyCombo from a character
fn kc(c: char) -> KeyCombo {
    KeyCombo::new(KeyCode::Char(c), false, false, false)
}

/// Convenience function for creating a KeyCombo with ctrl
fn ctrl(c: char) -> KeyCombo {
    KeyCombo::new(KeyCode::Char(c), true, false, false)
}

/// Convenience function for creating a KeyCombo with alt
fn alt(c: char) -> KeyCombo {
    KeyCombo::new(KeyCode::Char(c), false, true, false)
}

/// Convenience function for creating a KeyCombo with specific key
fn key(code: KeyCode) -> KeyCombo {
    KeyCombo::new(code, false, false, false)
}

/// Convenience function for creating a KeyCombo with ctrl and specific key
fn ctrl_key(code: KeyCode) -> KeyCombo {
    KeyCombo::new(code, true, false, false)
}

/// Build the main keymap for irohscii
pub fn build_keymap() -> Keymap<Action, InputMode> {
    let mut normal = HashMap::new();
    
    // Basic commands that should be in keymap (not movement/stateful)
    normal.insert(ctrl('c'), Action::Quit);
    normal.insert(key(KeyCode::Esc), Action::ClearSelection);
    normal.insert(key(KeyCode::Tab), Action::SessionBrowser);
    normal.insert(kc('v'), Action::CycleLineStyle);
    normal.insert(kc('g'), Action::ToggleGrid);
    normal.insert(key(KeyCode::Delete), Action::DeleteSelected);
    normal.insert(key(KeyCode::Backspace), Action::DeleteSelected);
    normal.insert(key(KeyCode::Enter), Action::StartLabelInput);
    normal.insert(kc('?'), Action::ShowHelp);
    normal.insert(key(KeyCode::F(1)), Action::ShowHelp);
    
    // Tool selection (only unconditional ones go in keymap)
    normal.insert(kc('f'), Action::FreehandTool);
    normal.insert(kc('b'), Action::DoubleBoxTool);
    // Note: s, t, l, a, r, d, e have conditional logic and stay as direct matches
    
    // Layer management (unconditional ones only)
    normal.insert(kc('L'), Action::ToggleLayers);
    normal.insert(ctrl('n'), Action::NewLayer);
    normal.insert(ctrl('D'), Action::DeleteLayer(crate::layers::LayerId::default())); // Placeholder - actual layer in dispatch
    normal.insert(ctrl('h'), Action::ToggleActiveLayerVisibility);
    normal.insert(ctrl('m'), Action::MoveSelectionToActiveLayer);
    normal.insert(alt('1'), Action::SelectLayerByIndex(1));
    normal.insert(alt('2'), Action::SelectLayerByIndex(2));
    normal.insert(alt('3'), Action::SelectLayerByIndex(3));
    normal.insert(alt('4'), Action::SelectLayerByIndex(4));
    normal.insert(alt('5'), Action::SelectLayerByIndex(5));
    normal.insert(alt('6'), Action::SelectLayerByIndex(6));
    normal.insert(alt('7'), Action::SelectLayerByIndex(7));
    normal.insert(alt('8'), Action::SelectLayerByIndex(8));
    normal.insert(alt('9'), Action::SelectLayerByIndex(9));
    // Note: F2 and 'r' in layer mode have conditions and stay as direct matches
    
    // Alignment operations
    normal.insert(alt('l'), Action::AlignLeft);
    normal.insert(alt('r'), Action::AlignRight);
    normal.insert(alt('t'), Action::AlignTop);
    normal.insert(alt('b'), Action::AlignBottom);
    normal.insert(alt('c'), Action::AlignCenterHorizontal);
    normal.insert(alt('m'), Action::AlignCenterVertical);
    
    // Transform operations
    normal.insert(alt('h'), Action::FlipHorizontal);
    normal.insert(alt('v'), Action::FlipVertical);
    normal.insert(alt('.'), Action::Rotate90Clockwise);
    normal.insert(alt(','), Action::Rotate90CounterClockwise);
    normal.insert(alt('['), Action::DistributeHorizontal);
    normal.insert(alt(']'), Action::DistributeVertical);
    
    // Keyboard shape creation
    normal.insert(ctrl('R'), Action::CreateKeyboardRectangle);
    normal.insert(alt('d'), Action::CreateKeyboardDiamond);
    normal.insert(alt('e'), Action::CreateKeyboardEllipse);
    normal.insert(ctrl('L'), Action::CreateKeyboardLine);
    normal.insert(alt('a'), Action::CreateKeyboardArrow);
    normal.insert(ctrl('B'), Action::CreateKeyboardDoubleBox);
    
    // File operations (simplified - some have complex logic in original)
    normal.insert(ctrl('s'), Action::FileSave);
    normal.insert(ctrl('o'), Action::FileOpen);
    normal.insert(kc('R'), Action::RecentFiles);
    // Note: Ctrl+S, Ctrl+O have different behavior for doc vs file saving
    
    // Editing operations  
    normal.insert(kc('u'), Action::Undo);
    normal.insert(kc('U'), Action::Redo);
    normal.insert(kc('y'), Action::Copy);
    normal.insert(kc('p'), Action::Paste);
    normal.insert(kc(']'), Action::BringForward);
    normal.insert(kc('['), Action::SendBackward);
    normal.insert(kc('}'), Action::BringToFront);
    normal.insert(kc('{'), Action::SendToBack);
    normal.insert(kc('G'), Action::GroupSelection);
    normal.insert(ctrl('g'), Action::GroupSelection);
    normal.insert(ctrl('G'), Action::UngroupSelection);
    normal.insert(ctrl('a'), Action::SelectAll);
    normal.insert(ctrl('d'), Action::DuplicateSelected);
    
    // Note: Space and : are excluded because they open leader menu (stateful)
    // Movement keys (hjkl, arrows) are excluded because they're contextual
    // Zoom keys (Ctrl+/-, Ctrl+0) are excluded because they're movement
    // Layer 'r' key when in layer mode is excluded (stateful)
    
    let mode_bindings = vec![(InputMode::Normal, normal)];
    let overrides = vec![];
    
    Keymap::build(mode_bindings, &overrides, parse_action_name)
}