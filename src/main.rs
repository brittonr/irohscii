mod app;
mod canvas;
mod document;
mod file_io;
mod presence;
mod recent_files;
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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::{App, Mode, Tool};
use document::default_storage_path;
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

    /// File to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse CLI args
    let args = Args::parse();

    // Determine sync configuration - always active unless --offline
    let sync_config = if args.offline {
        SyncConfig {
            mode: SyncMode::Disabled,
            storage_path: None,
        }
    } else {
        SyncConfig {
            mode: SyncMode::Active { join_ticket: args.join },
            storage_path: None,
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

    // Get the default storage path for persistence
    let storage_path = default_storage_path();

    // Load file if specified (ASCII export file - for viewing/editing)
    if let Some(file_path) = args.file {
        match file_io::load_ascii(&file_path) {
            Ok(shapes) => {
                // Add shapes to document
                for kind in shapes {
                    let _ = app.doc.add_shape(kind);
                }
                // Rebuild view
                if let Err(e) = app.shape_view.rebuild(&app.doc) {
                    app.set_status(format!("Error rebuilding view: {}", e));
                }
                app.file_path = Some(file_path);
            }
            Err(e) => {
                app.set_status(format!("Error loading file: {}", e));
            }
        }
        // Set storage path for autosave
        app.doc.set_storage_path(storage_path);
    } else if storage_path.exists() {
        // Load from persisted automerge document if it exists
        match document::Document::load(&storage_path) {
            Ok(doc) => {
                app.doc = doc;
                // Rebuild view from loaded document
                if let Err(e) = app.shape_view.rebuild(&app.doc) {
                    app.set_status(format!("Error rebuilding view: {}", e));
                } else {
                    app.set_status("Loaded previous session");
                }
            }
            Err(e) => {
                app.set_status(format!("Error loading saved document: {}", e));
                // Continue with fresh document but set storage path
                app.doc.set_storage_path(storage_path);
            }
        }
    } else {
        // Fresh document - set storage path for autosave
        app.doc.set_storage_path(storage_path);
    }

    // Start sync if enabled
    let sync_handle = if !matches!(sync_config.mode, SyncMode::Disabled) {
        match sync::start_sync_thread(sync_config) {
            Ok(handle) => {
                if let Some(ref endpoint_id) = handle.endpoint_id {
                    app.sync_ticket = Some(endpoint_id.clone());
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
    let result = run_app(&mut terminal, &mut app, sync_handle.as_ref());

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

/// Presence broadcast interval (50ms = 20 Hz)
const PRESENCE_BROADCAST_INTERVAL: Duration = Duration::from_millis(50);

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    sync_handle: Option<&SyncHandle>,
) -> Result<()> {
    let mut last_presence_broadcast = Instant::now();
    let mut last_stale_prune = Instant::now();
    // Track which key triggered the current popup (for release-to-confirm)
    let mut popup_trigger_key: Option<KeyCode> = None;

    while app.running {
        terminal.draw(|frame| ui::render(frame, app))?;

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
                    sync::SyncEvent::Ready { endpoint_id, local_peer_id } => {
                        app.init_presence(local_peer_id);
                        app.set_status(format!("Session ready: {}", endpoint_id));
                    }
                    sync::SyncEvent::RemoteChanges { doc } => {
                        let mut remote_doc = doc;
                        app.merge_remote(&mut remote_doc);
                    }
                    sync::SyncEvent::PeerStatus { peer_count, connected } => {
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
                                        KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
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
                                        Mode::Normal => handle_normal_mode(app, key),
                                        Mode::TextInput { .. } => handle_text_input_mode(app, key),
                                        Mode::LabelInput { .. } => handle_label_input_mode(app, key),
                                        Mode::FileSave { .. } => handle_file_save_mode(app, key),
                                        Mode::FileOpen { .. } => handle_file_open_mode(app, key),
                                        Mode::SvgExport { .. } => handle_svg_export_mode(app, key),
                                        Mode::RecentFiles { .. } => handle_recent_files_mode(app, key),
                                        Mode::SelectionPopup { .. } => {} // Handled above
                                    }
                                }
                            }

                            // After any change in Normal mode, sync if enabled and autosave
                            if matches!(app.mode, Mode::Normal) {
                                if let Some(handle) = sync_handle {
                                    let doc = app.clone_automerge();
                                    let _ = handle.send_command(sync::SyncCommand::SyncDoc { doc });
                                }
                                // Autosave after changes
                                app.autosave();
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

                    if matches!(app.mode, Mode::Normal) {
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
                                let _ = handle.send_command(sync::SyncCommand::BroadcastPresence(presence));
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

fn handle_normal_mode(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.running = false;
        }
        KeyCode::Esc => {
            app.cancel_shape();
            app.selected = None; // Deselect
        }

        // Tool selection
        KeyCode::Char('s') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_tool(Tool::Select)
        }
        KeyCode::Char('f') => app.set_tool(Tool::Freehand),
        KeyCode::Char('t') => app.set_tool(Tool::Text),
        KeyCode::Char('l') => app.set_tool(Tool::Line),
        KeyCode::Char('a') => app.set_tool(Tool::Arrow),
        KeyCode::Char('r') => app.set_tool(Tool::Rectangle),
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

        // Delete selected shape
        KeyCode::Delete | KeyCode::Backspace => {
            if app.current_tool == Tool::Select {
                app.delete_selected();
            }
        }

        // Edit label of selected shape
        KeyCode::Enter => {
            if app.current_tool == Tool::Select && app.selected.is_some() {
                if app.start_label_input() {
                    app.set_status("Editing label - type text, Enter/Esc to finish");
                }
            }
        }

        // File operations
        KeyCode::Char('S') | KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.start_save()
        }
        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => app.start_open(),

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

        // New document (capital N)
        KeyCode::Char('N') => app.new_document(),

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
        KeyCode::Char(c) => {
            app.add_label_char(c);
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
                        app.set_status("Saved!");
                    }
                    Err(e) => {
                        app.set_status(format!("Error: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
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
                            app.set_status(format!("Error rebuilding view: {}", e));
                        } else {
                            app.recent_files.add(path.clone());
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
                        app.set_status(format!("Error: {}", e));
                    }
                }
            }
            app.mode = Mode::Normal;
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
