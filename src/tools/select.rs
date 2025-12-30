use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Handle mouse events for select tool
pub fn handle_select_event(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            let shift = event.modifiers.contains(KeyModifiers::SHIFT);

            // First check if we're clicking on a resize handle (single selection only)
            if app.try_start_resize(pos) {
                return;
            }

            // Check if clicking on a shape
            if let Some(id) = app.shape_view.shape_at(pos) {
                if shift {
                    // Shift+click: toggle shape in selection
                    app.toggle_selection(id);
                } else if !app.is_selected(id) {
                    // Click on unselected shape: select it (replacing selection)
                    app.select_single(id);
                }
                // Start dragging if clicking on a selected shape
                if app.is_selected(id) {
                    app.start_drag(pos);
                }
            } else {
                // Clicking on empty space - start marquee selection
                if !shift {
                    app.clear_selection();
                }
                app.start_marquee(pos);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            // Handle marquee, resize, or drag depending on current state
            if app.marquee_state.is_some() {
                app.continue_marquee(pos);
            } else if app.resize_state.is_some() {
                app.continue_resize(pos);
            } else {
                app.continue_drag(pos);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.marquee_state.is_some() {
                app.finish_marquee();
            } else {
                app.finish_resize();
                app.finish_drag();
            }
        }
        MouseEventKind::ScrollUp => {
            app.viewport.pan(0, -3);
        }
        MouseEventKind::ScrollDown => {
            app.viewport.pan(0, 3);
        }
        MouseEventKind::ScrollLeft => {
            app.viewport.pan(-3, 0);
        }
        MouseEventKind::ScrollRight => {
            app.viewport.pan(3, 0);
        }
        _ => {}
    }
}
