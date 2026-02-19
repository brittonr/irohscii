mod app;
mod modes;
mod recent_files;
mod tools;
mod ui;

// Import workspace crates
use irohscii_sync as sync;

// Re-export commonly used types (only what main.rs actually uses)
use sync::{SyncConfig, SyncHandle, SyncMode};

// Module aliases for backwards compatibility with internal code
mod canvas {
    pub use irohscii_geometry::*;
}
mod document {
    pub use irohscii_core::{Document, GroupId, ShapeId, default_storage_path};
}
mod layers {
    pub use irohscii_core::{Layer, LayerId};
}
mod shapes {
    pub use irohscii_core::{
        ResizeHandle, ShapeColor, ShapeKind, ShapeView, SnapPoint, flip_horizontal, flip_vertical,
        resize_shape, rotate_90_ccw, rotate_90_cw,
    };
}
mod presence {
    pub use irohscii_sync::{
        CursorActivity, PeerId, PeerPresence, PresenceManager, ToolKind, peer_color,
    };
}
mod file_io {
    pub use irohscii_export::{load_ascii, save_ascii};
}
mod svg_export {
    pub use irohscii_export::save_svg;
}
mod session {
    pub use irohscii_session::{SessionId, SessionManager, SessionMeta};
}

use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use app::{App, ConfirmDialogState, Mode, PendingAction, Tool};

/// ASCII art drawing tool with real-time collaboration
#[derive(Parser, Debug)]
#[command(name = "irohscii")]
#[command(version, about, long_about = None)]
struct Args {
    /// Join an existing session using a ticket
    #[arg(long, value_name = "TICKET")]
    join: Option<String>,

    /// Sync documents to an aspen cluster node
    #[arg(long, value_name = "TICKET")]
    cluster: Option<String>,

    /// Disable sync (offline mode)
    #[arg(long)]
    offline: bool,

    /// Open a specific session by name or ID
    #[arg(long, short = 's', value_name = "SESSION")]
    session: Option<String>,

    /// Create a new session with the given name
    #[arg(long, value_name = "NAME")]
    new_session: Option<String>,

    /// List all available sessions and exit
    #[arg(long)]
    list_sessions: bool,

    /// File to open (ASCII import)
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse CLI args
    let args = Args::parse();

    // Initialize session manager
    let mut session_manager = session::SessionManager::new()?;

    // Handle --list-sessions flag
    if args.list_sessions {
        let sessions = session_manager.list_sessions()?;
        if sessions.is_empty() {
            println!("No sessions found. Create one with --new-session <name>");
        } else {
            println!("Available sessions:");
            println!("{:<30} {:<20} {}", "NAME", "ID", "LAST ACCESSED");
            println!("{}", "-".repeat(70));
            for session in sessions {
                let pinned = if session.pinned { "*" } else { " " };
                let timestamp = chrono_lite_format(session.last_accessed);
                println!(
                    "{}{:<29} {:<20} {}",
                    pinned, session.name, session.id.0, timestamp
                );
            }
        }
        return Ok(());
    }

    // Determine sync configuration - always active unless --offline
    let sync_config = if args.offline {
        SyncConfig {
            mode: SyncMode::Disabled,
            cluster_ticket: None,
            disable_discovery: false,
        }
    } else {
        SyncConfig {
            mode: SyncMode::Active {
                join_ticket: args.join,
            },
            cluster_ticket: args.cluster,
            disable_discovery: false,
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with terminal size
    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height.saturating_sub(2)); // Reserve 2 rows for status/help

    // Determine which session to load
    let session_to_load: Option<session::SessionId> = if let Some(name) = args.new_session {
        // Create a new session
        match session_manager.create_session(&name) {
            Ok(meta) => {
                app.set_status(format!("Created session: {}", meta.name));
                Some(meta.id)
            }
            Err(e) => {
                app.set_error(format!("Failed to create session: {}", e));
                None
            }
        }
    } else if let Some(query) = args.session {
        // Open specified session
        match session_manager.find_session(&query)? {
            Some(meta) => Some(meta.id),
            None => {
                app.set_error(format!("Session not found: {}", query));
                None
            }
        }
    } else {
        // Try to load last active session, or create default
        match session_manager.last_active() {
            Some(id) if session_manager.session_exists(id) => Some(id.clone()),
            _ => {
                // Create default session if none exists
                let sessions = session_manager.list_sessions()?;
                if sessions.is_empty() {
                    match session_manager.create_session("Default") {
                        Ok(meta) => Some(meta.id),
                        Err(_) => None,
                    }
                } else {
                    Some(sessions[0].id.clone())
                }
            }
        }
    };

    // Load the session
    if let Some(session_id) = session_to_load {
        match session_manager.open_session(&session_id) {
            Ok((doc, meta)) => {
                app.doc = doc;
                app.current_session = Some(session_id);
                app.current_session_meta = Some(meta.clone());
                if let Err(e) = app.shape_view.rebuild(&app.doc) {
                    app.set_status(format!("Error rebuilding view: {}", e));
                } else {
                    app.set_status(format!("Session: {}", meta.name));
                }
            }
            Err(e) => {
                app.set_error(format!("Failed to load session: {}", e));
            }
        }
    }

    // Load ASCII file if specified (imports into current session)
    if let Some(file_path) = args.file {
        match file_io::load_ascii(&file_path) {
            Ok(shapes) => {
                for kind in shapes {
                    let _ = app.doc.add_shape(kind);
                }
                if let Err(e) = app.shape_view.rebuild(&app.doc) {
                    app.set_status(format!("Error rebuilding view: {}", e));
                }
                app.file_path = Some(file_path);
            }
            Err(e) => {
                app.set_status(format!("Error loading file: {}", e));
            }
        }
    }

    // Initialize active layer from document
    app.init_active_layer();

    // Start sync if enabled
    let sync_handle = if !matches!(sync_config.mode, SyncMode::Disabled) {
        match sync::start_sync_thread(sync_config) {
            Ok(handle) => {
                if let Some(endpoint_id) = &handle.endpoint_id {
                    app.sync_ticket = Some(endpoint_id.clone());
                    // Save ticket to session metadata
                    if let Some(meta) = &mut app.current_session_meta {
                        meta.set_ticket(endpoint_id);
                        if app.current_session.is_some() {
                            let _ = session_manager.save_meta(meta);
                            let _ = session_manager.save_registry();
                        }
                    }
                    app.set_status(format!("Session: {} (T to copy)", endpoint_id));
                }
                Some(handle)
            }
            Err(e) => {
                app.set_status(format!("Sync error: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Main event loop
    let result = run_app(
        &mut terminal,
        &mut app,
        sync_handle.as_ref(),
        &mut session_manager,
    );

    // Save current session on shutdown
    if let (Some(session_id), Some(meta)) = (&app.current_session, &mut app.current_session_meta) {
        meta.touch();
        let _ = session_manager.save_session(session_id, &mut app.doc, meta);
    }

    // Save recent files on shutdown
    if let Err(e) = app.recent_files.save() {
        eprintln!("Failed to save recent files: {}", e);
    }

    // Cleanup: shutdown sync
    if let Some(handle) = sync_handle {
        let _ = handle.send_command(sync::SyncCommand::Shutdown);
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
    }

    Ok(())
}

/// Simple timestamp formatter (no chrono dependency)
fn chrono_lite_format(unix_secs: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let diff = now.saturating_sub(unix_secs);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{}d ago", diff / 86400)
    } else {
        format!("{}w ago", diff / 604800)
    }
}

/// Presence broadcast interval (50ms = 20 Hz)
const PRESENCE_BROADCAST_INTERVAL: Duration = Duration::from_millis(50);

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    sync_handle: Option<&SyncHandle>,
    session_manager: &mut session::SessionManager,
) -> Result<()> {
    let mut last_presence_broadcast = Instant::now();
    let mut last_stale_prune = Instant::now();
    // Track which key triggered the current popup (for release-to-confirm)
    let mut popup_trigger_key: Option<KeyCode> = None;

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut *app))?;

        // Prune stale peers every second
        if last_stale_prune.elapsed() >= Duration::from_secs(1) {
            if let Some(ref mut presence) = app.presence {
                presence.prune_stale();
            }
            last_stale_prune = Instant::now();
        }

        // Poll for sync events (non-blocking)
        if let Some(handle) = sync_handle {
            while let Some(event) = handle.poll_event() {
                match event {
                    sync::SyncEvent::Ready {
                        endpoint_id,
                        local_peer_id,
                    } => {
                        app.init_presence(local_peer_id);
                        app.set_status(format!("Session ready: {}", endpoint_id));
                    }
                    sync::SyncEvent::RemoteChanges { mut doc } => {
                        app.merge_remote(&mut *doc);
                    }
                    sync::SyncEvent::PresenceUpdate(presence) => {
                        if let Some(ref mut mgr) = app.presence {
                            mgr.update_peer(presence);
                        }
                    }
                    sync::SyncEvent::PresenceRemoved { peer_id } => {
                        if let Some(ref mut mgr) = app.presence {
                            mgr.remove_peer(&peer_id);
                        }
                    }
                    sync::SyncEvent::Error(msg) => {
                        app.set_status(format!("Sync error: {}", msg));
                    }
                }
            }
        }

        // Use poll with timeout to allow sync event processing
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    // Handle key press vs release for popup confirmation
                    match key.kind {
                        KeyEventKind::Press => {
                            // Clear status message on any keypress
                            app.clear_status();

                            // Check if we're in a popup and this is a re-press of trigger key (fallback confirm)
                            if let Mode::SelectionPopup(_) = &app.mode {
                                if Some(key.code) == popup_trigger_key {
                                    // Same key pressed again = confirm (fallback for terminals without release)
                                    app.confirm_popup_selection();
                                    popup_trigger_key = None;
                                } else {
                                    // Use new dispatch for popup navigation
                                    // Take mode out to avoid double mutable borrow
                                    use crate::modes::ModeTransition;
                                    let mut mode = std::mem::take(&mut app.mode);
                                    let transition = mode.handle_key(app, key);
                                    match transition {
                                        ModeTransition::Normal => app.mode = Mode::Normal,
                                        ModeTransition::To(new_mode) => app.mode = *new_mode,
                                        ModeTransition::Stay | ModeTransition::Action(_) => app.mode = mode,
                                    }
                                }
                            } else {
                                // Check for popup triggers in Normal mode
                                let triggered_popup = if matches!(app.mode, Mode::Normal) {
                                    match key.code {
                                        KeyCode::Char(' ') => {
                                            app.open_tool_popup();
                                            popup_trigger_key = Some(key.code);
                                            true
                                        }
                                        KeyCode::Char('C') => {
                                            app.open_color_popup();
                                            popup_trigger_key = Some(key.code);
                                            true
                                        }
                                        KeyCode::Char('c')
                                            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
                                        {
                                            app.open_brush_popup();
                                            popup_trigger_key = Some(key.code);
                                            true
                                        }
                                        _ => false,
                                    }
                                } else {
                                    false
                                };

                                if !triggered_popup {
                                    // Use the new Mode::handle_key dispatch
                                    // Take mode out to avoid double mutable borrow
                                    use crate::modes::{ModeAction, ModeTransition};
                                    let mut mode = std::mem::take(&mut app.mode);
                                    let transition = mode.handle_key(app, key);
                                    match transition {
                                        ModeTransition::Stay => {
                                            app.mode = mode;
                                        }
                                        ModeTransition::Normal => {
                                            app.mode = Mode::Normal;
                                        }
                                        ModeTransition::To(new_mode) => {
                                            app.mode = *new_mode;
                                        }
                                        ModeTransition::Action(action) => {
                                            match action {
                                                ModeAction::Quit => {
                                                    app.running = false;
                                                }
                                                ModeAction::OpenSessionBrowser => {
                                                    if let Ok(sessions) = session_manager.list_sessions() {
                                                        app.open_session_browser(sessions);
                                                    }
                                                }
                                                ModeAction::SwitchSession(session_id) => {
                                                    app.session_to_switch = Some(session_id);
                                                    app.mode = Mode::Normal;
                                                }
                                                ModeAction::CreateSession(name) => {
                                                    app.session_to_create = Some(name);
                                                    app.mode = Mode::Normal;
                                                }
                                                ModeAction::DeleteSession(session_id) => {
                                                    // Open confirm dialog before deleting
                                                    app.mode = Mode::ConfirmDialog(ConfirmDialogState {
                                                        action: PendingAction::DeleteSession(session_id.0.clone()),
                                                    });
                                                }
                                                ModeAction::ToggleSessionPin(session_id) => {
                                                    let _ = session_manager.toggle_pinned(&session_id);
                                                    if let Ok(sessions) = session_manager.list_sessions() {
                                                        app.refresh_session_list(sessions);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // After any change in Normal mode, sync if enabled and autosave
                            if matches!(app.mode, Mode::Normal) {
                                if let Some(handle) = sync_handle {
                                    let doc = app.clone_automerge();
                                    let _ = handle.send_command(sync::SyncCommand::SyncDoc {
                                        doc: Box::new(doc),
                                    });
                                }
                                // Autosave to session
                                if let (Some(session_id), Some(meta)) =
                                    (&app.current_session, &mut app.current_session_meta)
                                {
                                    let _ = session_manager.save_session(
                                        session_id,
                                        &mut app.doc,
                                        meta,
                                    );
                                }
                            }

                            // Handle session switching requests
                            if let Some(session_id) = app.session_to_switch.take() {
                                // Save current session first
                                if let (Some(cur_id), Some(meta)) =
                                    (&app.current_session, &mut app.current_session_meta)
                                {
                                    meta.touch();
                                    let _ =
                                        session_manager.save_session(cur_id, &mut app.doc, meta);
                                }
                                // Load new session
                                match session_manager.open_session(&session_id) {
                                    Ok((doc, meta)) => {
                                        app.doc = doc;
                                        app.current_session = Some(session_id);
                                        app.current_session_meta = Some(meta.clone());
                                        // Reset all UI state for the new session
                                        app.reset_session_ui_state();
                                        if let Err(e) = app.shape_view.rebuild(&app.doc) {
                                            app.set_error(format!("Error rebuilding view: {}", e));
                                        } else {
                                            app.set_status(format!("Switched to: {}", meta.name));
                                        }
                                    }
                                    Err(e) => {
                                        app.set_error(format!("Failed to switch session: {}", e));
                                    }
                                }
                            }

                            // Handle session creation requests
                            if let Some(name) = app.session_to_create.take() {
                                // Save current session first
                                if let (Some(cur_id), Some(meta)) =
                                    (&app.current_session, &mut app.current_session_meta)
                                {
                                    meta.touch();
                                    let _ =
                                        session_manager.save_session(cur_id, &mut app.doc, meta);
                                }
                                // Create and switch to new session
                                match session_manager.create_session(&name) {
                                    Ok(meta) => match session_manager.open_session(&meta.id) {
                                        Ok((doc, meta)) => {
                                            app.doc = doc;
                                            app.current_session = Some(meta.id.clone());
                                            app.current_session_meta = Some(meta.clone());
                                            // Reset all UI state for the new session
                                            app.reset_session_ui_state();
                                            if let Err(e) = app.shape_view.rebuild(&app.doc) {
                                                app.set_error(format!("Error: {}", e));
                                            } else {
                                                app.set_status(format!("Created: {}", meta.name));
                                            }
                                        }
                                        Err(e) => {
                                            app.set_error(format!(
                                                "Failed to open new session: {}",
                                                e
                                            ));
                                        }
                                    },
                                    Err(e) => {
                                        app.set_error(format!("Failed to create session: {}", e));
                                    }
                                }
                            }

                            // Handle session deletion requests
                            if let Some(session_id_str) = app.session_to_delete.take() {
                                let session_id = session::SessionId(session_id_str);
                                match session_manager.delete_session(&session_id) {
                                    Ok(()) => {
                                        app.set_status("Session deleted");
                                        // Refresh session list with bounds checking
                                        if let Ok(sessions) = session_manager.list_sessions() {
                                            app.refresh_session_list(sessions);
                                        }
                                    }
                                    Err(e) => {
                                        app.set_error(format!("Failed to delete: {}", e));
                                    }
                                }
                            }

                            // Handle pending cluster connection
                            if let Some(ticket) = app.pending_cluster_ticket.take() {
                                if let Some(handle) = sync_handle {
                                    let _ = handle.send_command(
                                        sync::SyncCommand::ConnectCluster { ticket },
                                    );
                                } else {
                                    app.set_error("Sync is disabled (--offline)");
                                }
                            }
                        }
                        KeyEventKind::Release => {
                            // Check if this is the release of the popup trigger key
                            if let Mode::SelectionPopup(_) = &app.mode {
                                if Some(key.code) == popup_trigger_key {
                                    app.confirm_popup_selection();
                                    popup_trigger_key = None;
                                }
                            }
                        }
                        KeyEventKind::Repeat => {
                            // Handle repeats same as press for navigation
                            if let Mode::SelectionPopup(_) = &app.mode {
                                if Some(key.code) != popup_trigger_key {
                                    // Use new dispatch for popup navigation
                                    // Take mode out to avoid double mutable borrow
                                    use crate::modes::ModeTransition;
                                    let mut mode = std::mem::take(&mut app.mode);
                                    let transition = mode.handle_key(app, key);
                                    match transition {
                                        ModeTransition::Normal => app.mode = Mode::Normal,
                                        ModeTransition::To(new_mode) => app.mode = *new_mode,
                                        ModeTransition::Stay | ModeTransition::Action(_) => app.mode = mode,
                                    }
                                }
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    // Track cursor position for presence
                    let cursor_pos = app.viewport.screen_to_canvas(mouse.column, mouse.row);
                    app.last_cursor_pos = cursor_pos;

                    // Handle Ctrl+scroll for zoom
                    if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                app.viewport.zoom_in();
                                app.set_status(format!(
                                    "Zoom: {}%",
                                    (app.viewport.zoom * 100.0) as i32
                                ));
                                continue;
                            }
                            MouseEventKind::ScrollDown => {
                                app.viewport.zoom_out();
                                app.set_status(format!(
                                    "Zoom: {}%",
                                    (app.viewport.zoom * 100.0) as i32
                                ));
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Check if click is in layer panel
                    let in_layer_panel = app.layer_panel_area.is_some_and(|area| {
                        mouse.column >= area.x
                            && mouse.column < area.x + area.width
                            && mouse.row >= area.y
                            && mouse.row < area.y + area.height
                    });

                    if in_layer_panel {
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            if let Some(area) = app.layer_panel_area {
                                // Calculate which layer was clicked (accounting for border)
                                let row_in_panel = mouse.row.saturating_sub(area.y + 1); // +1 for top border
                                let layers = app.get_layers();
                                if (row_in_panel as usize) < layers.len() {
                                    let clicked_layer = &layers[row_in_panel as usize];

                                    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                                        // Shift+click: toggle visibility
                                        app.toggle_layer_visibility(clicked_layer.id);
                                    } else if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                                        // Ctrl+click: toggle locked
                                        app.toggle_layer_locked(clicked_layer.id);
                                    } else {
                                        // Normal click: select layer
                                        app.active_layer = Some(clicked_layer.id);
                                        app.set_status(format!(
                                            "Active layer: {}",
                                            clicked_layer.name
                                        ));
                                    }
                                }
                            }
                        }
                    } else if matches!(app.mode, Mode::Normal) {
                        match app.current_tool {
                            Tool::Select => tools::handle_select_event(app, mouse),
                            Tool::Freehand => tools::handle_freehand_event(app, mouse),
                            Tool::Text => tools::handle_text_event(app, mouse),
                            Tool::Line => tools::handle_line_event(app, mouse),
                            Tool::Arrow => tools::handle_arrow_event(app, mouse),
                            Tool::Rectangle => tools::handle_rectangle_event(app, mouse),
                            Tool::DoubleBox => tools::handle_doublebox_event(app, mouse),
                            Tool::Diamond => tools::handle_diamond_event(app, mouse),
                            Tool::Ellipse => tools::handle_ellipse_event(app, mouse),
                            Tool::Triangle => tools::handle_triangle_event(app, mouse),
                            Tool::Parallelogram => tools::handle_parallelogram_event(app, mouse),
                            Tool::Hexagon => tools::handle_hexagon_event(app, mouse),
                            Tool::Trapezoid => tools::handle_trapezoid_event(app, mouse),
                            Tool::RoundedRect => tools::handle_roundedrect_event(app, mouse),
                            Tool::Cylinder => tools::handle_cylinder_event(app, mouse),
                            Tool::Cloud => tools::handle_cloud_event(app, mouse),
                            Tool::Star => tools::handle_star_event(app, mouse),
                        }

                        // After mouse events, sync if enabled and dirty, then autosave
                        if app.is_dirty() {
                            if let Some(handle) = sync_handle {
                                let doc = app.clone_automerge();
                                let _ = handle.send_command(sync::SyncCommand::SyncDoc {
                                    doc: Box::new(doc),
                                });
                            }
                            // Autosave after changes
                            app.autosave();
                        }
                    }

                    // Broadcast presence (throttled)
                    if let Some(handle) = sync_handle {
                        if last_presence_broadcast.elapsed() >= PRESENCE_BROADCAST_INTERVAL {
                            if let Some(presence) = app.build_presence(cursor_pos) {
                                let _ = handle
                                    .send_command(sync::SyncCommand::BroadcastPresence(presence));
                            }
                            last_presence_broadcast = Instant::now();
                        }
                    }
                }
                Event::Resize(w, h) => {
                    app.viewport.resize(w, h.saturating_sub(2));
                }
                _ => {}
            }
        }
    }

    Ok(())
}