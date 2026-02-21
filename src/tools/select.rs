use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

/// Scroll amount per tick
const SCROLL_AMOUNT: i32 = 3;

// Compile-time assertion: scroll amount must be positive
const _: () = assert!(SCROLL_AMOUNT > 0, "SCROLL_AMOUNT must be positive");

/// Handle mouse events for select tool
pub fn handle_select_event(app: &mut App, event: MouseEvent) {
    debug_assert!(event.column < u16::MAX, "Event column coordinate out of valid range");
    debug_assert!(event.row < u16::MAX, "Event row coordinate out of valid range");
    
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
            debug_assert!(
                [app.marquee_state.is_some(), app.resize_state.is_some(), app.drag_state.is_some()].iter().filter(|&&x| x).count() <= 1,
                "Only one of marquee, resize, or drag should be active at a time"
            );
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
