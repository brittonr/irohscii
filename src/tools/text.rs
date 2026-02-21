use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, Mode};

/// Scroll amount per tick
const SCROLL_AMOUNT: i32 = 3;

// Compile-time assertion: scroll amount must be positive
const _: () = assert!(SCROLL_AMOUNT > 0, "SCROLL_AMOUNT must be positive");

/// Handle mouse events for text tool
pub fn handle_text_event(app: &mut App, event: MouseEvent) {
    debug_assert!(event.column < u16::MAX, "Event column coordinate out of valid range");
    debug_assert!(event.row < u16::MAX, "Event row coordinate out of valid range");
    
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // If already in text input mode, commit current text first
            if matches!(app.mode, Mode::TextInput(_)) {
                app.commit_text();
            }
            // Start new text input at click position
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.start_text_input(pos);
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
