use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Handle mouse events for line drawing tool
pub fn handle_line_event(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.start_shape(pos);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.update_shape(pos);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.commit_shape();
        }
        MouseEventKind::Down(MouseButton::Right) => {
            // Cancel shape on right click
            app.cancel_shape();
        }
        MouseEventKind::Moved => {
            // Update hover snap point when moving mouse (not drawing)
            if app.shape_state.is_none() {
                let pos = app.viewport.screen_to_canvas(event.column, event.row);
                app.update_hover_snap(pos);
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
