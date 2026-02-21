use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Scroll amount per tick
const SCROLL_AMOUNT: i32 = 3;

// Compile-time assertion: scroll amount must be positive
const _: () = assert!(SCROLL_AMOUNT > 0, "SCROLL_AMOUNT must be positive");

/// Handle mouse events for freehand drawing tool
pub fn handle_freehand_event(app: &mut App, event: MouseEvent) {
    debug_assert!(event.column < u16::MAX, "Event column coordinate out of valid range");
    debug_assert!(event.row < u16::MAX, "Event row coordinate out of valid range");
    
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            debug_assert!(app.freehand_state.is_none(), "Freehand state should be None before starting");
            app.start_freehand(pos);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            debug_assert!(app.freehand_state.is_some(), "Freehand state should exist during drag");
            app.continue_freehand(pos);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.finish_freehand();
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
