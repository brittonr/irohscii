use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Handle mouse events for select tool
pub fn handle_select_event(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);

            // First check if we're clicking on a resize handle of the selected shape
            if app.try_start_resize(pos) {
                return;
            }

            // Try to select a shape at this position
            if app.try_select(pos) {
                // Check again for resize handle on newly selected shape
                if !app.try_start_resize(pos) {
                    // Start dragging if not on a resize handle
                    app.start_drag(pos);
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            // Handle resize or drag depending on current state
            if app.resize_state.is_some() {
                app.continue_resize(pos);
            } else {
                app.continue_drag(pos);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.finish_resize();
            app.finish_drag();
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
