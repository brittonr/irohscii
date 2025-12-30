use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Handle mouse events for freehand drawing tool
pub fn handle_freehand_event(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.start_freehand(pos);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.continue_freehand(pos);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.finish_freehand();
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
