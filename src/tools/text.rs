use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, Mode};

/// Handle mouse events for text tool
pub fn handle_text_event(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // If already in text input mode, commit current text first
            if matches!(app.mode, Mode::TextInput { .. }) {
                app.commit_text();
            }
            // Start new text input at click position
            let pos = app.viewport.screen_to_canvas(event.column, event.row);
            app.start_text_input(pos);
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
