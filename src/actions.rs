//! Action enum for leader key menu commands.
//!
//! This enum covers all commands that can be triggered from the leader menu,
//! replacing the direct key handling logic in the old leader.rs module.

use crate::layers::LayerId;

/// Actions that can be triggered from the leader key menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Direct tool selection
    /// Set the current tool to Select
    SelectTool,
    /// Set the current tool to Freehand
    FreehandTool,
    /// Set the current tool to Text
    TextTool,
    /// Set the current tool to Line
    LineTool,
    /// Set the current tool to Arrow
    ArrowTool,
    /// Set the current tool to Rectangle
    RectangleTool,
    /// Set the current tool to DoubleBox
    DoubleBoxTool,
    /// Set the current tool to Diamond
    DiamondTool,
    /// Set the current tool to Ellipse
    EllipseTool,
    /// Set the current tool to Triangle
    TriangleTool,
    /// Set the current tool to Parallelogram
    ParallelogramTool,
    /// Set the current tool to Hexagon
    HexagonTool,
    /// Set the current tool to Trapezoid
    TrapezoidTool,
    /// Set the current tool to RoundedRect
    RoundedRectTool,
    /// Set the current tool to Cylinder
    CylinderTool,
    /// Set the current tool to Cloud
    CloudTool,
    /// Set the current tool to Star
    StarTool,

    // Popup commands
    /// Open tool selection popup
    ToolPopup,
    /// Open color selection popup
    ColorPopup,
    /// Open brush selection popup
    BrushPopup,

    // File operations
    /// Save file (prompts for path)
    FileSave,
    /// Open file (prompts for path)
    FileOpen,
    /// Export to SVG (prompts for path)
    SvgExport,
    /// Create new document
    NewDocument,

    // Sync/collaboration operations
    /// Copy sync ticket to clipboard
    CopyTicket,
    /// Show QR code for current ticket
    ShowQrCode,
    /// Decode QR code from image (prompts for path)
    DecodeQr,
    /// Connect to cluster (prompts for ticket)
    ClusterConnect,
    /// Join session (prompts for ticket)
    JoinSession,

    // View operations
    /// Toggle grid display
    ToggleGrid,
    /// Toggle layer panel visibility
    ToggleLayers,
    /// Toggle participants panel visibility
    ToggleParticipants,

    // App operations
    /// Show help screen
    ShowHelp,
    /// Quit application
    Quit,

    // Edit operations (for future submenus)
    /// Undo last action
    Undo,
    /// Redo last undone action
    Redo,
    /// Copy selected shapes
    Copy,
    /// Paste from clipboard
    Paste,
    /// Delete selected shapes
    DeleteSelected,
    /// Select all shapes
    SelectAll,
    /// Clear selection and cancel shape
    ClearSelection,
    /// Duplicate selected shapes
    DuplicateSelected,
    /// Group selected shapes
    GroupSelection,
    /// Ungroup selected shapes
    UngroupSelection,
    /// Cycle line style
    CycleLineStyle,

    // Z-order operations
    /// Bring selected shapes forward
    BringForward,
    /// Send selected shapes backward
    SendBackward,
    /// Bring selected shapes to front
    BringToFront,
    /// Send selected shapes to back
    SendToBack,

    // Alignment operations
    /// Align selected shapes left
    AlignLeft,
    /// Align selected shapes right
    AlignRight,
    /// Align selected shapes top
    AlignTop,
    /// Align selected shapes bottom
    AlignBottom,
    /// Center selected shapes horizontally
    AlignCenterHorizontal,
    /// Center selected shapes vertically
    AlignCenterVertical,

    // Transform operations
    /// Flip selected shapes horizontally
    FlipHorizontal,
    /// Flip selected shapes vertically
    FlipVertical,
    /// Rotate selected shapes 90° clockwise
    Rotate90Clockwise,
    /// Rotate selected shapes 90° counter-clockwise
    Rotate90CounterClockwise,
    /// Distribute selected shapes horizontally
    DistributeHorizontal,
    /// Distribute selected shapes vertically
    DistributeVertical,

    // Keyboard shape creation
    /// Create rectangle using keyboard input
    CreateKeyboardRectangle,
    /// Create diamond using keyboard input
    CreateKeyboardDiamond,
    /// Create ellipse using keyboard input
    CreateKeyboardEllipse,
    /// Create line using keyboard input
    CreateKeyboardLine,
    /// Create arrow using keyboard input
    CreateKeyboardArrow,
    /// Create double box using keyboard input
    CreateKeyboardDoubleBox,

    // Layer operations (for future submenus)
    /// Create new layer
    NewLayer,
    /// Delete current layer
    DeleteLayer(LayerId),
    /// Toggle layer visibility
    ToggleLayerVisible(LayerId),
    /// Toggle layer lock
    ToggleLayerLock(LayerId),
    /// Rename layer
    RenameLayer(LayerId),
    /// Select layer above current
    SelectLayerUp,
    /// Select layer below current
    SelectLayerDown,
    /// Move selection to active layer
    MoveSelectionToActiveLayer,
    /// Toggle active layer visibility
    ToggleActiveLayerVisibility,
    /// Start layer rename for active layer
    StartLayerRename,
    /// Select layer by index (1-9)
    SelectLayerByIndex(u8),

    // View operations (for future submenus)
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
    /// Reset zoom to 100%
    ZoomReset,
    /// Center view on content
    CenterView,

    // Session operations (for future submenus)
    /// Open session browser
    SessionBrowser,
    /// Open recent files
    RecentFiles,
    /// Start label input for selected shapes
    StartLabelInput,
}

impl Action {
    /// Get a human-readable description of this action.
    pub fn description(&self) -> &'static str {
        match self {
            Action::SelectTool => "Select tool",
            Action::FreehandTool => "Freehand tool",
            Action::TextTool => "Text tool",
            Action::LineTool => "Line tool",
            Action::ArrowTool => "Arrow tool",
            Action::RectangleTool => "Rectangle tool",
            Action::DoubleBoxTool => "Double box tool",
            Action::DiamondTool => "Diamond tool",
            Action::EllipseTool => "Ellipse tool",
            Action::TriangleTool => "Triangle tool",
            Action::ParallelogramTool => "Parallelogram tool",
            Action::HexagonTool => "Hexagon tool",
            Action::TrapezoidTool => "Trapezoid tool",
            Action::RoundedRectTool => "Rounded rect tool",
            Action::CylinderTool => "Cylinder tool",
            Action::CloudTool => "Cloud tool",
            Action::StarTool => "Star tool",
            Action::ToolPopup => "Tool picker",
            Action::ColorPopup => "Color picker",
            Action::BrushPopup => "Brush picker",
            Action::FileSave => "Save file",
            Action::FileOpen => "Open file",
            Action::SvgExport => "Export SVG",
            Action::NewDocument => "New document",
            Action::CopyTicket => "Copy ticket",
            Action::ShowQrCode => "Show QR code",
            Action::DecodeQr => "Decode QR",
            Action::ClusterConnect => "Connect cluster",
            Action::JoinSession => "Join session",
            Action::ToggleGrid => "Toggle grid",
            Action::ToggleLayers => "Toggle layers",
            Action::ToggleParticipants => "Toggle participants",
            Action::ShowHelp => "Show help",
            Action::Quit => "Quit",
            Action::Undo => "Undo",
            Action::Redo => "Redo",
            Action::Copy => "Copy",
            Action::Paste => "Paste",
            Action::DeleteSelected => "Delete",
            Action::SelectAll => "Select all",
            Action::ClearSelection => "Clear selection",
            Action::DuplicateSelected => "Duplicate",
            Action::GroupSelection => "Group",
            Action::UngroupSelection => "Ungroup",
            Action::CycleLineStyle => "Cycle line style",
            Action::BringForward => "Bring forward",
            Action::SendBackward => "Send backward",
            Action::BringToFront => "Bring to front",
            Action::SendToBack => "Send to back",
            Action::AlignLeft => "Align left",
            Action::AlignRight => "Align right",
            Action::AlignTop => "Align top",
            Action::AlignBottom => "Align bottom",
            Action::AlignCenterHorizontal => "Center horizontal",
            Action::AlignCenterVertical => "Center vertical",
            Action::FlipHorizontal => "Flip horizontal",
            Action::FlipVertical => "Flip vertical",
            Action::Rotate90Clockwise => "Rotate 90° CW",
            Action::Rotate90CounterClockwise => "Rotate 90° CCW",
            Action::DistributeHorizontal => "Distribute horizontal",
            Action::DistributeVertical => "Distribute vertical",
            Action::CreateKeyboardRectangle => "Create rectangle",
            Action::CreateKeyboardDiamond => "Create diamond",
            Action::CreateKeyboardEllipse => "Create ellipse",
            Action::CreateKeyboardLine => "Create line",
            Action::CreateKeyboardArrow => "Create arrow",
            Action::CreateKeyboardDoubleBox => "Create double box",
            Action::NewLayer => "New layer",
            Action::DeleteLayer(_) => "Delete layer",
            Action::ToggleLayerVisible(_) => "Toggle visible",
            Action::ToggleLayerLock(_) => "Toggle lock",
            Action::RenameLayer(_) => "Rename layer",
            Action::SelectLayerUp => "Layer up",
            Action::SelectLayerDown => "Layer down",
            Action::MoveSelectionToActiveLayer => "Move to layer",
            Action::ToggleActiveLayerVisibility => "Toggle layer visible",
            Action::StartLayerRename => "Rename layer",
            Action::SelectLayerByIndex(n) => match n {
                1 => "Layer 1",
                2 => "Layer 2", 
                3 => "Layer 3",
                4 => "Layer 4",
                5 => "Layer 5",
                6 => "Layer 6",
                7 => "Layer 7",
                8 => "Layer 8",
                9 => "Layer 9",
                _ => "Layer N",
            },
            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::ZoomReset => "Reset zoom",
            Action::CenterView => "Center view",
            Action::SessionBrowser => "Session browser",
            Action::RecentFiles => "Recent files",
            Action::StartLabelInput => "Edit label",
        }
    }
}