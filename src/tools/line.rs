use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Scroll amount per tick
const SCROLL_AMOUNT: i32 = 3;

// Compile-time assertion: scroll amount must be positive
const _: () = assert!(SCROLL_AMOUNT > 0, "SCROLL_AMOUNT must be positive");

/// Handle mouse events for line drawing tool
pub fn handle_line_event(app: &mut App, event: MouseEvent) {
    debug_assert!(event.column < u16::MAX, "Event column coordinate out of valid range");
    debug_assert!(event.row < u16::MAX, "Event row coordinate out of valid range");
    
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            debug_assert!(app.shape_state.is_none(), "Shape state should be None before starting new shape");
            app.start_shape(pos);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            debug_assert!(app.shape_state.is_some(), "Shape state should exist during drag");
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
            app.viewport.pan(0, -SCROLL_AMOUNT);
        }
        MouseEventKind::ScrollDown => {
            app.viewport.pan(0, SCROLL_AMOUNT);
        }
        MouseEventKind::ScrollLeft => {
            app.viewport.pan(-SCROLL_AMOUNT, 0);
        }
        MouseEventKind::ScrollRight => {
            app.viewport.pan(SCROLL_AMOUNT, 0);
        }
        _ => {}
    }
}
