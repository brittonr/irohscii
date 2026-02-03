//! Z-order methods for App (bring to front, send to back, etc.)

use crate::document::ShapeId;

use super::App;

impl App {
    /// Bring selected shapes to front (top of z-order)
    pub fn bring_to_front(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.bring_to_front(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Brought to front");
    }

    /// Send selected shapes to back (bottom of z-order)
    pub fn send_to_back(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.send_to_back(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Sent to back");
    }

    /// Bring selected shapes forward one level
    pub fn bring_forward(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.bring_forward(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Brought forward");
    }

    /// Send selected shapes backward one level
    pub fn send_backward(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let ids: Vec<ShapeId> = self.selected.iter().copied().collect();
        if let Err(e) = self.doc.send_backward(&ids) {
            self.set_status(format!("Error: {}", e));
            return;
        }
        self.rebuild_view();
        self.set_status("Sent backward");
    }
}
