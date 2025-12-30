use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Handle mouse events for rounded rectangle drawing tool
pub fn handle_roundedrect_event(app: &mut App, event: MouseEvent) {
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
            app.cancel_shape();
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
