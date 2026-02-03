mod app;
mod canvas;
mod document;
mod file_io;
mod layers;
mod presence;
mod recent_files;
mod session;
mod shapes;
mod svg_export;
mod sync;
mod tools;
mod ui;
mod undo;

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

use app::{App, Mode, Tool};
use sync::{SyncConfig, SyncHandle, SyncMode};

/// ASCII art drawing tool with real-time collaboration
#[derive(Parser, Debug)]
#[command(name = "irohscii")]
#[command(version, about, long_about = None)]
struct Args {
    /// Join an existing session using a ticket
    #[arg(long, value_name = "TICKET")]
    join: Option<String>,

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
            storage_path: None,
            disable_discovery: false,
        }
    } else {
        SyncConfig {
            mode: SyncMode::Active {
                join_ticket: args.join,
            },
            storage_path: None,
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
                    sync::SyncEvent::RemoteChanges { doc } => {
                        let mut remote_doc = doc;
                        app.merge_remote(&mut remote_doc);
                    }
                    sync::SyncEvent::PeerStatus {
                        peer_count,
                        connected,
                    } => {
                        if connected {
                            app.set_status(format!("Peer connected ({})", peer_count));
                        } else {
                            app.set_status(format!("Peer disconnected ({})", peer_count));
                        }
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
                            if let Mode::SelectionPopup { .. } = &app.mode {
                                if Some(key.code) == popup_trigger_key {
                                    // Same key pressed again = confirm (fallback for terminals without release)
                                    app.confirm_popup_selection();
                                    popup_trigger_key = None;
                                } else {
                                    handle_selection_popup_mode(app, key);
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
                                    match &app.mode {
                                        Mode::Normal => {
                                            handle_normal_mode(app, key, session_manager)
                                        }
                                        Mode::TextInput { .. } => handle_text_input_mode(app, key),
                                        Mode::LabelInput { .. } => {
                                            handle_label_input_mode(app, key)
                                        }
                                        Mode::LayerRename { .. } => {
                                            handle_layer_rename_mode(app, key)
                                        }
                                        Mode::FileSave { .. } => handle_file_save_mode(app, key),
                                        Mode::FileOpen { .. } => handle_file_open_mode(app, key),
                                        Mode::DocSave { .. } => handle_doc_save_mode(app, key),
                                        Mode::DocOpen { .. } => handle_doc_open_mode(app, key),
                                        Mode::SvgExport { .. } => handle_svg_export_mode(app, key),
                                        Mode::RecentFiles { .. } => {
                                            handle_recent_files_mode(app, key)
                                        }
                                        Mode::SelectionPopup { .. } => {} // Handled above
                                        Mode::ConfirmDialog { .. } => {
                                            handle_confirm_dialog_mode(app, key)
                                        }
                                        Mode::HelpScreen { .. } => {
                                            handle_help_screen_mode(app, key)
                                        }
                                        Mode::SessionBrowser { .. } => {
                                            handle_session_browser_mode(app, key, session_manager)
                                        }
                                        Mode::SessionCreate { .. } => {
                                            handle_session_create_mode(app, key)
                                        }
                                    }
                                }
                            }

                            // After any change in Normal mode, sync if enabled and autosave
                            if matches!(app.mode, Mode::Normal) {
                                if let Some(handle) = sync_handle {
                                    let doc = app.clone_automerge();
                                    let _ = handle.send_command(sync::SyncCommand::SyncDoc { doc });
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
                        }
                        KeyEventKind::Release => {
                            // Check if this is the release of the popup trigger key
                            if let Mode::SelectionPopup { .. } = &app.mode {
                                if Some(key.code) == popup_trigger_key {
                                    app.confirm_popup_selection();
                                    popup_trigger_key = None;
                                }
                            }
                        }
                        KeyEventKind::Repeat => {
                            // Handle repeats same as press for navigation
                            if let Mode::SelectionPopup { .. } = &app.mode {
                                if Some(key.code) != popup_trigger_key {
                                    handle_selection_popup_mode(app, key);
                                }
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    // Track cursor position for presence
                    let cursor_pos = app.viewport.screen_to_canvas(mouse.column, mouse.row);
                    app.last_cursor_pos = cursor_pos;

                    // Check if click is in layer panel
                    let in_layer_panel = app.layer_panel_area.map_or(false, |area| {
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
                                let _ = handle.send_command(sync::SyncCommand::SyncDoc { doc });
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

fn handle_normal_mode(
    app: &mut App,
    key: event::KeyEvent,
    session_manager: &mut session::SessionManager,
) {
    match key.code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.running = false;
        }
        KeyCode::Esc => {
            app.cancel_shape();
            app.clear_selection(); // Deselect
        }

        // Session browser (Tab key)
        KeyCode::Tab => match session_manager.list_sessions() {
            Ok(sessions) => app.open_session_browser(sessions),
            Err(e) => app.set_error(format!("Failed to list sessions: {}", e)),
        },

        // Tool selection
        KeyCode::Char('s') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_tool(Tool::Select)
        }
        KeyCode::Char('f') => app.set_tool(Tool::Freehand),
        KeyCode::Char('t') => app.set_tool(Tool::Text),
        KeyCode::Char('l') => app.set_tool(Tool::Line),
        KeyCode::Char('a') => app.set_tool(Tool::Arrow),
        KeyCode::Char('r') if !app.show_layers => app.set_tool(Tool::Rectangle),
        KeyCode::Char('b') => app.set_tool(Tool::DoubleBox),
        KeyCode::Char('d') => app.set_tool(Tool::Diamond),
        KeyCode::Char('e') => app.set_tool(Tool::Ellipse),

        // Line style cycling (c and C are now popup triggers for brush/color)
        KeyCode::Char('v') => app.cycle_line_style(),

        // Undo/Redo (Helix/Kakoune keymaps)
        KeyCode::Char('u') => app.undo(),
        KeyCode::Char('U') => app.redo(),

        // Copy/Paste (Helix keymaps)
        KeyCode::Char('y') => app.yank(),
        KeyCode::Char('p') => app.paste(),

        // Z-order control
        KeyCode::Char(']') => app.bring_forward(),
        KeyCode::Char('[') => app.send_backward(),
        KeyCode::Char('}') => app.bring_to_front(),
        KeyCode::Char('{') => app.send_to_back(),

        // Grouping (Shift+G to group, Ctrl+Shift+G to ungroup)
        KeyCode::Char('G') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.group_selection()
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.group_selection()
        }
        KeyCode::Char('G') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.ungroup_selection()
        }

        // Layer shortcuts
        KeyCode::Char('L') => app.toggle_layer_panel(),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => app.create_layer(),
        KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.request_delete_layer()
        }
        KeyCode::F(2) if app.show_layers => app.start_layer_rename(),
        KeyCode::Char('r') if app.show_layers && app.active_layer.is_some() => {
            app.start_layer_rename()
        }
        KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(1)
        }
        KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(2)
        }
        KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(3)
        }
        KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(4)
        }
        KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(5)
        }
        KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(6)
        }
        KeyCode::Char('7') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(7)
        }
        KeyCode::Char('8') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(8)
        }
        KeyCode::Char('9') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.select_layer_by_index(9)
        }
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.move_selection_to_active_layer()
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_active_layer_visibility()
        }

        // Select all (Ctrl+A)
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => app.select_all(),

        // Alignment shortcuts (Alt + key)
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::ALT) => app.align_left(),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::ALT) => app.align_right(),
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::ALT) => app.align_top(),
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => app.align_bottom(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::ALT) => app.align_center_h(),
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => app.align_center_v(),

        // Delete selected shape
        KeyCode::Delete | KeyCode::Backspace => {
            if app.current_tool == Tool::Select {
                app.delete_selected();
            }
        }

        // Edit label of selected shape
        KeyCode::Enter => {
            if app.current_tool == Tool::Select && !app.selected.is_empty() {
                if app.start_label_input() {
                    app.set_status("Editing label - type text, Enter/Esc to finish");
                }
            }
        }

        // File operations (ASCII export/import)
        KeyCode::Char('s')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            app.start_save()
        }
        KeyCode::Char('o')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            app.start_open()
        }
        // Document save/open (with Shift)
        KeyCode::Char('S') if key.modifiers.contains(KeyModifiers::CONTROL) => app.start_doc_save(),
        KeyCode::Char('O') if key.modifiers.contains(KeyModifiers::CONTROL) => app.start_doc_open(),

        // Copy sync ticket to clipboard (capital T)
        KeyCode::Char('T') => app.copy_ticket_to_clipboard(),

        // Toggle participant panel (capital P)
        KeyCode::Char('P') => {
            app.show_participants = !app.show_participants;
            let status = if app.show_participants {
                "Participants panel shown"
            } else {
                "Participants panel hidden"
            };
            app.set_status(status);
        }

        // SVG export (capital E)
        KeyCode::Char('E') => app.start_svg_export(),

        // Toggle grid (g key)
        KeyCode::Char('g') => app.toggle_grid(),

        // New document (capital N) - shows confirmation if dirty
        KeyCode::Char('N') => app.request_new_document(),

        // Recent files (capital R)
        KeyCode::Char('R') => {
            if !app.recent_files.is_empty() {
                app.mode = Mode::RecentFiles { selected: 0 };
            } else {
                app.set_status("No recent files");
            }
        }

        // Viewport panning
        KeyCode::Up => app.viewport.pan(0, -1),
        KeyCode::Down => app.viewport.pan(0, 1),
        KeyCode::Left => app.viewport.pan(-1, 0),
        KeyCode::Right => app.viewport.pan(1, 0),

        // Help screen
        KeyCode::Char('?') | KeyCode::F(1) => app.open_help(),

        _ => {}
    }
}

fn handle_text_input_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.commit_text();
        }
        KeyCode::Backspace => {
            app.backspace_text();
        }
        KeyCode::Char(c) => {
            app.add_text_char(c);
        }
        _ => {}
    }
}

fn handle_label_input_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.commit_label();
        }
        KeyCode::Backspace => {
            app.backspace_label();
        }
        KeyCode::Delete => {
            app.delete_label_char();
        }
        KeyCode::Left => {
            app.move_label_cursor_left();
        }
        KeyCode::Right => {
            app.move_label_cursor_right();
        }
        KeyCode::Home => {
            app.move_label_cursor_home();
        }
        KeyCode::End => {
            app.move_label_cursor_end();
        }
        KeyCode::Char(c) => {
            app.add_label_char(c);
        }
        _ => {}
    }
}

fn handle_layer_rename_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.commit_layer_rename();
        }
        KeyCode::Esc => {
            app.cancel_layer_rename();
        }
        KeyCode::Backspace => {
            app.backspace_layer_rename();
        }
        KeyCode::Char(c) => {
            app.add_layer_rename_char(c);
        }
        _ => {}
    }
}

fn handle_file_save_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Mode::FileSave { path } = &app.mode {
                let path = PathBuf::from(path.clone());
                match file_io::save_ascii(&app.shape_view, &path) {
                    Ok(()) => {
                        app.recent_files.add(path.clone());
                        app.file_path = Some(path);
                        app.set_status("Exported!");
                    }
                    Err(e) => {
                        app.set_error(format!("Export error: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.complete_path();
        }
        KeyCode::Backspace => {
            app.backspace_path();
        }
        KeyCode::Char(c) => {
            app.add_path_char(c);
        }
        _ => {}
    }
}

fn handle_file_open_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Mode::FileOpen { path } = &app.mode {
                let path = PathBuf::from(path.clone());
                match file_io::load_ascii(&path) {
                    Ok(shapes) => {
                        // Create new document and add shapes
                        app.doc = document::Document::new();
                        for kind in shapes {
                            let _ = app.doc.add_shape(kind);
                        }
                        // Rebuild view
                        if let Err(e) = app.shape_view.rebuild(&app.doc) {
                            app.set_error(format!("Error rebuilding view: {}", e));
                        } else {
                            app.recent_files.add(path.clone());
                            app.file_path = Some(path);
                            app.set_status("Imported!");
                        }
                    }
                    Err(e) => {
                        app.set_error(format!("Import error: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.complete_path();
        }
        KeyCode::Backspace => {
            app.backspace_path();
        }
        KeyCode::Char(c) => {
            app.add_path_char(c);
        }
        _ => {}
    }
}

fn handle_svg_export_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Mode::SvgExport { path } = &app.mode {
                let path = PathBuf::from(path.clone());
                match svg_export::save_svg(&app.shape_view, &path) {
                    Ok(()) => {
                        app.set_status(format!("Exported to {}", path.display()));
                    }
                    Err(e) => {
                        app.set_error(format!("SVG export error: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.complete_path();
        }
        KeyCode::Backspace => {
            app.backspace_path();
        }
        KeyCode::Char(c) => {
            app.add_path_char(c);
        }
        _ => {}
    }
}

fn handle_doc_save_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Mode::DocSave { path } = &app.mode {
                let path = PathBuf::from(path.clone());
                match app.doc.save_to(&path) {
                    Ok(()) => {
                        app.set_status(format!("Document saved to {}", path.display()));
                    }
                    Err(e) => {
                        app.set_error(format!("Failed to save: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.complete_path();
        }
        KeyCode::Backspace => {
            app.backspace_path();
        }
        KeyCode::Char(c) => {
            app.add_path_char(c);
        }
        _ => {}
    }
}

fn handle_doc_open_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Mode::DocOpen { path } = &app.mode {
                let path = PathBuf::from(path.clone());
                match document::Document::load(&path) {
                    Ok(doc) => {
                        app.doc = doc;
                        // Rebuild view
                        if let Err(e) = app.shape_view.rebuild(&app.doc) {
                            app.set_error(format!("Error rebuilding view: {}", e));
                        } else {
                            app.init_active_layer();
                            app.selected.clear();
                            app.set_status(format!("Document loaded from {}", path.display()));
                        }
                    }
                    Err(e) => {
                        app.set_error(format!("Failed to load: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.complete_path();
        }
        KeyCode::Backspace => {
            app.backspace_path();
        }
        KeyCode::Char(c) => {
            app.add_path_char(c);
        }
        _ => {}
    }
}

fn handle_recent_files_mode(app: &mut App, key: event::KeyEvent) {
    if let Mode::RecentFiles { ref mut selected } = app.mode {
        match key.code {
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                *selected = selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = app.recent_files.len().saturating_sub(1);
                *selected = (*selected + 1).min(max);
            }
            KeyCode::Enter => {
                let idx = *selected;
                if let Some(file) = app.recent_files.get(idx) {
                    let path = file.path.clone();
                    match file_io::load_ascii(&path) {
                        Ok(shapes) => {
                            app.doc = document::Document::new();
                            for kind in shapes {
                                let _ = app.doc.add_shape(kind);
                            }
                            if let Err(e) = app.shape_view.rebuild(&app.doc) {
                                app.set_status(format!("Error: {}", e));
                            } else {
                                app.file_path = Some(path);
                                app.set_status("Loaded!");
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Error: {}", e));
                        }
                    }
                }
                app.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}

fn handle_selection_popup_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        // hjkl navigation
        KeyCode::Char('h') | KeyCode::Left => app.popup_navigate(-1, 0),
        KeyCode::Char('l') | KeyCode::Right => app.popup_navigate(1, 0),
        KeyCode::Char('j') | KeyCode::Down => app.popup_navigate(0, 1),
        KeyCode::Char('k') | KeyCode::Up => app.popup_navigate(0, -1),
        // Enter to confirm
        KeyCode::Enter => app.confirm_popup_selection(),
        // Escape to cancel
        KeyCode::Esc => app.cancel_popup(),
        _ => {}
    }
}

fn handle_confirm_dialog_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        // Yes - confirm the action
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.confirm_pending_action();
        }
        // No - cancel the action
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_pending_action();
        }
        _ => {}
    }
}

fn handle_help_screen_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        // Close help
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::F(1) => {
            app.close_help();
        }
        // Scroll down
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_help(1);
        }
        // Scroll up
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_help(-1);
        }
        // Page down
        KeyCode::PageDown | KeyCode::Char(' ') => {
            app.scroll_help(10);
        }
        // Page up
        KeyCode::PageUp => {
            app.scroll_help(-10);
        }
        _ => {}
    }
}

fn handle_session_browser_mode(
    app: &mut App,
    key: event::KeyEvent,
    session_manager: &mut session::SessionManager,
) {
    match key.code {
        // Close browser
        KeyCode::Esc | KeyCode::Tab => {
            app.close_session_browser();
        }
        // Navigate
        KeyCode::Char('j') | KeyCode::Down => {
            app.session_browser_navigate(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.session_browser_navigate(-1);
        }
        // Select session
        KeyCode::Enter => {
            app.session_browser_select();
        }
        // Create new session
        KeyCode::Char('n') => {
            app.open_session_create();
        }
        // Delete session
        KeyCode::Char('d') | KeyCode::Delete => {
            app.session_browser_request_delete();
        }
        // Toggle pinned
        KeyCode::Char('p') => {
            if let Some(session_id) = app.session_browser_toggle_pin() {
                if let Ok(pinned) = session_manager.toggle_pinned(&session_id) {
                    // Refresh list with bounds checking to show updated pinned status
                    if let Ok(sessions) = session_manager.list_sessions() {
                        app.refresh_session_list(sessions);
                    }
                    let msg = if pinned { "Pinned" } else { "Unpinned" };
                    app.set_status(msg);
                }
            }
        }
        // Toggle pinned-only filter
        KeyCode::Char('*') => {
            app.session_browser_toggle_pinned();
        }
        // Backspace for filter
        KeyCode::Backspace => {
            app.session_browser_filter_backspace();
        }
        // Type to filter
        KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
            app.session_browser_filter_char(c);
        }
        _ => {}
    }
}

fn handle_session_create_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.session_create_cancel();
        }
        KeyCode::Enter => {
            app.session_create_confirm();
        }
        KeyCode::Backspace => {
            app.session_create_backspace();
        }
        KeyCode::Char(c) => {
            app.session_create_char(c);
        }
        _ => {}
    }
}
