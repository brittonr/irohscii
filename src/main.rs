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
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, KeyModifiers,
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

    /// Sync documents to an aspen cluster node (amsync1... ticket)
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
    let args = Args::parse();
    let mut session_manager = session::SessionManager::new()?;

    if args.list_sessions {
        handle_list_sessions_command(&session_manager)?;
        return Ok(());
    }

    let sync_config = build_sync_config(&args);
    let mut terminal = setup_terminal()?;
    let mut app = initialize_app(&mut terminal)?;
    
    determine_and_load_session(&args, &mut session_manager, &mut app)?;
    load_file_if_specified(&args, &mut app);
    app.init_active_layer();
    
    let sync_handle = start_sync_if_enabled(sync_config, &mut app, &mut session_manager)?;

    let result = run_app(
        &mut terminal,
        &mut app,
        sync_handle.as_ref(),
        &mut session_manager,
    );

    cleanup_on_shutdown(&mut app, &mut session_manager, sync_handle);
    cleanup_terminal(terminal)?;

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
    }

    Ok(())
}

/// Handle --list-sessions command
fn handle_list_sessions_command(session_manager: &session::SessionManager) -> Result<()> {
    let sessions = session_manager.list_sessions()?;
    if sessions.is_empty() {
        println!("No sessions found. Create one with --new-session <name>");
    } else {
        println!("Available sessions:");
        println!("{:<30} {:<20} LAST ACCESSED", "NAME", "ID");
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
    Ok(())
}

/// Build sync configuration from CLI arguments
fn build_sync_config(args: &Args) -> SyncConfig {
    if args.offline {
        SyncConfig {
            mode: SyncMode::Disabled,
            cluster_ticket: None,
            cluster_capability: None,
            disable_discovery: false,
        }
    } else {
        SyncConfig {
            mode: SyncMode::Active {
                join_ticket: args.join.clone(),
            },
            cluster_ticket: args.cluster.clone(),
            cluster_capability: None,
            disable_discovery: false,
        }
    }
}

/// Setup terminal and enable raw mode
fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

/// Initialize app with terminal dimensions
fn initialize_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<App> {
    let size = terminal.size()?;
    debug_assert!(size.width > 0, "Terminal width must be positive");
    debug_assert!(size.height >= 2, "Terminal height must be at least 2");
    
    let canvas_height = size.height.saturating_sub(2);
    Ok(App::new(size.width, canvas_height))
}

/// Determine which session to load and load it
fn determine_and_load_session(
    args: &Args,
    session_manager: &mut session::SessionManager,
    app: &mut App,
) -> Result<()> {
    let session_to_load = determine_session_to_load(args, session_manager, app)?;
    
    if let Some(session_id) = session_to_load {
        load_session(session_id, session_manager, app);
    }
    
    Ok(())
}

/// Determine which session should be loaded
fn determine_session_to_load(
    args: &Args,
    session_manager: &mut session::SessionManager,
    app: &mut App,
) -> Result<Option<session::SessionId>> {
    if let Some(name) = &args.new_session {
        return create_new_session(name, session_manager, app);
    }
    
    if let Some(query) = &args.session {
        return find_existing_session(query, session_manager, app);
    }
    
    Ok(find_or_create_default_session(session_manager)?)
}

/// Create a new session
fn create_new_session(
    name: &str,
    session_manager: &mut session::SessionManager,
    app: &mut App,
) -> Result<Option<session::SessionId>> {
    match session_manager.create_session(name) {
        Ok(meta) => {
            app.set_status(format!("Created session: {}", meta.name));
            Ok(Some(meta.id))
        }
        Err(e) => {
            app.set_error(format!("Failed to create session: {}", e));
            Ok(None)
        }
    }
}

/// Find an existing session by query
fn find_existing_session(
    query: &str,
    session_manager: &mut session::SessionManager,
    app: &mut App,
) -> Result<Option<session::SessionId>> {
    match session_manager.find_session(query)? {
        Some(meta) => Ok(Some(meta.id)),
        None => {
            app.set_error(format!("Session not found: {}", query));
            Ok(None)
        }
    }
}

/// Find last active session or create default
fn find_or_create_default_session(
    session_manager: &mut session::SessionManager,
) -> Result<Option<session::SessionId>> {
    if let Some(id) = session_manager.last_active() {
        if session_manager.session_exists(id) {
            return Ok(Some(id.clone()));
        }
    }
    
    let sessions = session_manager.list_sessions()?;
    if sessions.is_empty() {
        match session_manager.create_session("Default") {
            Ok(meta) => Ok(Some(meta.id)),
            Err(_) => Ok(None),
        }
    } else {
        debug_assert!(!sessions.is_empty(), "Session list should not be empty");
        Ok(Some(sessions[0].id.clone()))
    }
}

/// Load a session into the app
fn load_session(
    session_id: session::SessionId,
    session_manager: &mut session::SessionManager,
    app: &mut App,
) {
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

/// Load ASCII file if specified in args
fn load_file_if_specified(args: &Args, app: &mut App) {
    if let Some(file_path) = &args.file {
        match file_io::load_ascii(file_path) {
            Ok(shapes) => {
                for kind in shapes {
                    let _ = app.doc.add_shape(kind);
                }
                if let Err(e) = app.shape_view.rebuild(&app.doc) {
                    app.set_status(format!("Error rebuilding view: {}", e));
                }
                app.file_path = Some(file_path.clone());
            }
            Err(e) => {
                app.set_status(format!("Error loading file: {}", e));
            }
        }
    }
}

/// Start sync thread if enabled
fn start_sync_if_enabled(
    sync_config: SyncConfig,
    app: &mut App,
    session_manager: &mut session::SessionManager,
) -> Result<Option<SyncHandle>> {
    if matches!(sync_config.mode, SyncMode::Disabled) {
        return Ok(None);
    }

    match sync::start_sync_thread(sync_config) {
        Ok(handle) => {
            if let Some(endpoint_id) = &handle.endpoint_id {
                app.sync_ticket = Some(endpoint_id.clone());
                save_sync_ticket_to_session(endpoint_id, app, session_manager);
                app.set_status(format!("Session: {} (T to copy)", endpoint_id));
            }
            Ok(Some(handle))
        }
        Err(e) => {
            app.set_status(format!("Sync error: {}", e));
            Ok(None)
        }
    }
}

/// Save sync ticket to session metadata
fn save_sync_ticket_to_session(
    endpoint_id: &str,
    app: &mut App,
    session_manager: &mut session::SessionManager,
) {
    if let Some(meta) = &mut app.current_session_meta {
        meta.set_ticket(endpoint_id);
        if app.current_session.is_some() {
            let _ = session_manager.save_meta(meta);
            let _ = session_manager.save_registry();
        }
    }
}

/// Save state and cleanup on shutdown
fn cleanup_on_shutdown(
    app: &mut App,
    session_manager: &mut session::SessionManager,
    sync_handle: Option<SyncHandle>,
) {
    if let (Some(session_id), Some(meta)) = (&app.current_session, &mut app.current_session_meta) {
        meta.touch();
        let _ = session_manager.save_session(session_id, &mut app.doc, meta);
    }

    if let Err(e) = app.recent_files.save() {
        eprintln!("Failed to save recent files: {}", e);
    }

    if let Some(handle) = sync_handle {
        let _ = handle.send_command(sync::SyncCommand::Shutdown);
    }
}

/// Cleanup terminal state
fn cleanup_terminal(mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
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

/// Sync debounce interval — coalesce rapid edits into a single sync push.
/// 50ms is fast enough to feel real-time while avoiding per-keystroke overhead.
const SYNC_DEBOUNCE: Duration = Duration::from_millis(50);

/// Disk save debounce interval — coalesce rapid edits into a single disk write.
/// 2 seconds is frequent enough for crash recovery without blocking the event loop.
const DISK_SAVE_DEBOUNCE: Duration = Duration::from_secs(2);

/// Maximum iterations per event loop cycle (safety bound)
const MAX_EVENT_LOOP_ITERATIONS: u32 = 1_000_000;

/// Maximum sync events to process per cycle
const MAX_SYNC_EVENTS_PER_CYCLE: u32 = 100;

// Compile-time assertions for reasonable constants
const _: () = assert!(PRESENCE_BROADCAST_INTERVAL.as_millis() >= 10, "Presence broadcast too frequent");
const _: () = assert!(SYNC_DEBOUNCE.as_millis() >= 10, "Sync debounce too short");
const _: () = assert!(DISK_SAVE_DEBOUNCE.as_secs() >= 1, "Disk save debounce too short");
const _: () = assert!(MAX_EVENT_LOOP_ITERATIONS > 0, "Event loop bound must be positive");
const _: () = assert!(MAX_SYNC_EVENTS_PER_CYCLE > 0, "Sync event bound must be positive");

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    sync_handle: Option<&SyncHandle>,
    session_manager: &mut session::SessionManager,
) -> Result<()> {
    let mut last_presence_broadcast = Instant::now();
    let mut last_stale_prune = Instant::now();
    let mut sync_pending = false;
    let mut last_sync_sent = Instant::now();
    let mut disk_save_pending = false;
    let mut last_disk_save = Instant::now();
    let mut iteration_count: u32 = 0;

    while app.running {
        debug_assert!(iteration_count < MAX_EVENT_LOOP_ITERATIONS, "Event loop exceeded maximum iterations");
        iteration_count = iteration_count.saturating_add(1);
        
        if iteration_count >= MAX_EVENT_LOOP_ITERATIONS {
            app.set_error("Event loop iteration limit reached".to_string());
            break;
        }

        flush_pending_sync_if_due(&mut sync_pending, &mut last_sync_sent, app, sync_handle);
        flush_pending_disk_save_if_due(&mut disk_save_pending, &mut last_disk_save, app, session_manager);
        terminal.draw(|frame| ui::render(frame, &mut *app))?;
        prune_stale_peers_if_due(&mut last_stale_prune, app);
        process_sync_events(sync_handle, app);

        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key_event(key, app, &mut sync_pending, &mut disk_save_pending, sync_handle, session_manager);
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(mouse, app, &mut sync_pending, &mut disk_save_pending, sync_handle);
                }
                Event::Resize(w, h) => {
                    debug_assert!(w > 0 && h >= 2, "Invalid terminal resize dimensions");
                    app.viewport.resize(w, h.saturating_sub(2));
                }
                _ => {}
            }
        }

        broadcast_presence_if_due(&mut last_presence_broadcast, app, sync_handle);
    }

    Ok(())
}

/// Flush pending sync if debounce window has elapsed
fn flush_pending_sync_if_due(
    sync_pending: &mut bool,
    last_sync_sent: &mut Instant,
    app: &App,
    sync_handle: Option<&SyncHandle>,
) {
    if *sync_pending && last_sync_sent.elapsed() >= SYNC_DEBOUNCE {
        if let Some(handle) = sync_handle {
            let doc = app.clone_automerge();
            let _ = handle.send_command(sync::SyncCommand::SyncDoc {
                doc: Box::new(doc),
            });
        }
        *sync_pending = false;
        *last_sync_sent = Instant::now();
    }
}

/// Flush pending disk save if debounce window has elapsed.
/// Coalesces rapid edits into periodic disk writes instead of per-event I/O.
fn flush_pending_disk_save_if_due(
    disk_save_pending: &mut bool,
    last_disk_save: &mut Instant,
    app: &mut App,
    session_manager: &mut session::SessionManager,
) {
    if *disk_save_pending && last_disk_save.elapsed() >= DISK_SAVE_DEBOUNCE {
        if let (Some(session_id), Some(meta)) =
            (&app.current_session, &mut app.current_session_meta)
        {
            let _ = session_manager.save_session(session_id, &mut app.doc, meta);
        }
        *disk_save_pending = false;
        *last_disk_save = Instant::now();
    }
}

/// Prune stale peers if interval has elapsed
fn prune_stale_peers_if_due(last_stale_prune: &mut Instant, app: &mut App) {
    if last_stale_prune.elapsed() >= Duration::from_secs(1) {
        if let Some(ref mut presence) = app.presence {
            presence.prune_stale();
        }
        *last_stale_prune = Instant::now();
    }
}

/// Poll and process sync events (bounded)
fn process_sync_events(sync_handle: Option<&SyncHandle>, app: &mut App) {
    if let Some(handle) = sync_handle {
        let mut event_count: u32 = 0;
        
        while let Some(event) = handle.poll_event() {
            debug_assert!(event_count < MAX_SYNC_EVENTS_PER_CYCLE, "Sync event processing exceeded limit");
            event_count = event_count.saturating_add(1);
            
            if event_count >= MAX_SYNC_EVENTS_PER_CYCLE {
                app.set_status("Warning: Too many sync events, throttling".to_string());
                break;
            }

            match event {
                sync::SyncEvent::Ready {
                    endpoint_id,
                    local_peer_id,
                } => {
                    app.init_presence(local_peer_id);
                    app.set_status(format!("Session ready: {}", endpoint_id));
                }
                sync::SyncEvent::RemoteChanges { mut doc } => {
                    app.merge_remote(&mut doc);
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
}

/// Handle keyboard events
fn handle_key_event(
    key: event::KeyEvent,
    app: &mut App,
    sync_pending: &mut bool,
    disk_save_pending: &mut bool,
    sync_handle: Option<&SyncHandle>,
    session_manager: &mut session::SessionManager,
) {
    match key.kind {
        KeyEventKind::Press => {
            app.clear_status();
            process_key_press(key, app, session_manager);
            mark_sync_and_save_pending_if_dirty(app, sync_pending, disk_save_pending, sync_handle);
            handle_session_operations(app, session_manager);
            handle_sync_operations(app, sync_handle);
        }
        KeyEventKind::Release | KeyEventKind::Repeat => {}
    }
}

/// Process key press through mode system
fn process_key_press(
    key: event::KeyEvent,
    app: &mut App,
    session_manager: &mut session::SessionManager,
) {
    use crate::modes::ModeTransition;
    
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
            handle_mode_action(action, app, session_manager);
        }
    }
}

/// Handle mode action
fn handle_mode_action(
    action: modes::ModeAction,
    app: &mut App,
    session_manager: &mut session::SessionManager,
) {
    use crate::modes::ModeAction;
    
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

/// Mark sync and disk-save pending if the document is dirty.
/// Does NOT perform I/O — just sets flags for debounced flush later.
fn mark_sync_and_save_pending_if_dirty(
    app: &App,
    sync_pending: &mut bool,
    disk_save_pending: &mut bool,
    sync_handle: Option<&SyncHandle>,
) {
    if app.is_dirty() {
        if sync_handle.is_some() {
            *sync_pending = true;
        }
        *disk_save_pending = true;
    }
}

/// Handle session switch/create/delete operations
fn handle_session_operations(app: &mut App, session_manager: &mut session::SessionManager) {
    handle_session_switch(app, session_manager);
    handle_session_creation(app, session_manager);
    handle_session_deletion(app, session_manager);
}

/// Handle session switching
fn handle_session_switch(app: &mut App, session_manager: &mut session::SessionManager) {
    if let Some(session_id) = app.session_to_switch.take() {
        save_current_session(app, session_manager);
        
        match session_manager.open_session(&session_id) {
            Ok((doc, meta)) => {
                app.doc = doc;
                app.current_session = Some(session_id);
                app.current_session_meta = Some(meta.clone());
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
}

/// Handle session creation
fn handle_session_creation(app: &mut App, session_manager: &mut session::SessionManager) {
    if let Some(name) = app.session_to_create.take() {
        save_current_session(app, session_manager);
        
        match session_manager.create_session(&name) {
            Ok(meta) => match session_manager.open_session(&meta.id) {
                Ok((doc, meta)) => {
                    app.doc = doc;
                    app.current_session = Some(meta.id.clone());
                    app.current_session_meta = Some(meta.clone());
                    app.reset_session_ui_state();
                    
                    if let Err(e) = app.shape_view.rebuild(&app.doc) {
                        app.set_error(format!("Error: {}", e));
                    } else {
                        app.set_status(format!("Created: {}", meta.name));
                    }
                }
                Err(e) => {
                    app.set_error(format!("Failed to open new session: {}", e));
                }
            },
            Err(e) => {
                app.set_error(format!("Failed to create session: {}", e));
            }
        }
    }
}

/// Handle session deletion
fn handle_session_deletion(app: &mut App, session_manager: &mut session::SessionManager) {
    if let Some(session_id_str) = app.session_to_delete.take() {
        let session_id = session::SessionId(session_id_str);
        
        match session_manager.delete_session(&session_id) {
            Ok(()) => {
                app.set_status("Session deleted");
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

/// Save current session before switching
fn save_current_session(app: &mut App, session_manager: &mut session::SessionManager) {
    if let (Some(cur_id), Some(meta)) = (&app.current_session, &mut app.current_session_meta) {
        meta.touch();
        let _ = session_manager.save_session(cur_id, &mut app.doc, meta);
    }
}

/// Handle cluster/peer connection operations
fn handle_sync_operations(app: &mut App, sync_handle: Option<&SyncHandle>) {
    if let Some(ticket) = app.pending_cluster_ticket.take() {
        if let Some(handle) = sync_handle {
            let _ = handle.send_command(sync::SyncCommand::ConnectCluster {
                ticket,
                capability: None,
            });
        } else {
            app.set_error("Sync is disabled (--offline)");
        }
    }

    if let Some(ticket) = app.pending_join_ticket.take() {
        if let Some(handle) = sync_handle {
            let _ = handle.send_command(sync::SyncCommand::ConnectPeer { ticket });
        } else {
            app.set_error("Sync is disabled (--offline)");
        }
    }

    if let Some(ticket) = app.pending_qr_decoded_ticket.take() {
        app.sync_ticket = Some(ticket);
        app.copy_ticket_to_clipboard();
    }
}

/// Handle mouse events
fn handle_mouse_event(
    mouse: event::MouseEvent,
    app: &mut App,
    sync_pending: &mut bool,
    disk_save_pending: &mut bool,
    sync_handle: Option<&SyncHandle>,
) {
    update_cursor_position(app, &mouse);
    
    if handle_zoom_scroll(&mouse, app) {
        return;
    }

    if handle_layer_panel_click(&mouse, app) {
        return;
    }

    handle_canvas_tool_event(mouse, app, sync_pending, disk_save_pending, sync_handle);
}

/// Update cursor position for presence
fn update_cursor_position(app: &mut App, mouse: &event::MouseEvent) {
    let cursor_pos = app.viewport.screen_to_canvas(mouse.column, mouse.row);
    app.last_cursor_pos = cursor_pos;
}

/// Handle Ctrl+scroll for zooming
fn handle_zoom_scroll(mouse: &event::MouseEvent, app: &mut App) -> bool {
    if !mouse.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.viewport.zoom_in();
            app.set_status(format!("Zoom: {}%", (app.viewport.zoom * 100.0) as i32));
            true
        }
        MouseEventKind::ScrollDown => {
            app.viewport.zoom_out();
            app.set_status(format!("Zoom: {}%", (app.viewport.zoom * 100.0) as i32));
            true
        }
        _ => false,
    }
}

/// Handle clicks in layer panel
fn handle_layer_panel_click(mouse: &event::MouseEvent, app: &mut App) -> bool {
    let in_layer_panel = app.layer_panel_area.is_some_and(|area| {
        mouse.column >= area.x
            && mouse.column < area.x.saturating_add(area.width)
            && mouse.row >= area.y
            && mouse.row < area.y.saturating_add(area.height)
    });

    if !in_layer_panel {
        return false;
    }

    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
        if let Some(area) = app.layer_panel_area {
            let row_in_panel = mouse.row.saturating_sub(area.y.saturating_add(1));
            let layers = app.get_layers();
            let layer_index = row_in_panel as u32;
            
            debug_assert!(layers.len() <= u32::MAX as usize, "Too many layers");
            
            if (layer_index as usize) < layers.len() {
                let clicked_layer = &layers[layer_index as usize];

                if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                    app.toggle_layer_visibility(clicked_layer.id);
                } else if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                    app.toggle_layer_locked(clicked_layer.id);
                } else {
                    app.active_layer = Some(clicked_layer.id);
                    app.set_status(format!("Active layer: {}", clicked_layer.name));
                }
            }
        }
    }
    
    true
}

/// Handle tool events on canvas
fn handle_canvas_tool_event(
    mouse: event::MouseEvent,
    app: &mut App,
    sync_pending: &mut bool,
    disk_save_pending: &mut bool,
    sync_handle: Option<&SyncHandle>,
) {
    if !matches!(app.mode, Mode::Normal) {
        return;
    }

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

    // Only set flags — actual I/O is debounced in the main loop
    if app.is_dirty() {
        if sync_handle.is_some() {
            *sync_pending = true;
        }
        *disk_save_pending = true;
    }
}

/// Broadcast presence if interval has elapsed
fn broadcast_presence_if_due(
    last_presence_broadcast: &mut Instant,
    app: &mut App,
    sync_handle: Option<&SyncHandle>,
) {
    if let Some(handle) = sync_handle {
        if last_presence_broadcast.elapsed() >= PRESENCE_BROADCAST_INTERVAL {
            if let Some(presence) = app.build_presence(app.last_cursor_pos) {
                let _ = handle.send_command(sync::SyncCommand::BroadcastPresence(presence));
            }
            *last_presence_broadcast = Instant::now();
        }
    }
}
