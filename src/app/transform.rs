//! Transform methods for App (flip, rotate)

use crate::canvas::Position;

use super::App;

impl App {
    /// Flip selected shapes horizontally (mirror across vertical axis)
    pub fn flip_horizontal(&mut self) {
        if self.selected.is_empty() {
            self.set_status("Select shapes to flip");
            return;
        }
        let Some((min_x, _, max_x, _)) = self.get_selection_bounds() else {
            return;
        };
        let center_x = (min_x + max_x) / 2;
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Ok(Some(kind)) = self.doc.read_shape(id) {
                let flipped = crate::shapes::flip_horizontal(&kind, center_x);
                let _ = self.doc.update_shape(id, flipped);
            }
        }
        self.rebuild_view();
        self.set_status("Flipped horizontal");
    }

    /// Flip selected shapes vertically (mirror across horizontal axis)
    pub fn flip_vertical(&mut self) {
        if self.selected.is_empty() {
            self.set_status("Select shapes to flip");
            return;
        }
        let Some((_, min_y, _, max_y)) = self.get_selection_bounds() else {
            return;
        };
        let center_y = (min_y + max_y) / 2;
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Ok(Some(kind)) = self.doc.read_shape(id) {
                let flipped = crate::shapes::flip_vertical(&kind, center_y);
                let _ = self.doc.update_shape(id, flipped);
            }
        }
        self.rebuild_view();
        self.set_status("Flipped vertical");
    }

    /// Rotate selected shapes 90 degrees clockwise
    pub fn rotate_90_cw(&mut self) {
        if self.selected.is_empty() {
            self.set_status("Select shapes to rotate");
            return;
        }
        let Some((min_x, min_y, max_x, max_y)) = self.get_selection_bounds() else {
            return;
        };
        let center = Position::new((min_x + max_x) / 2, (min_y + max_y) / 2);
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Ok(Some(kind)) = self.doc.read_shape(id) {
                let rotated = crate::shapes::rotate_90_cw(&kind, center);
                let _ = self.doc.update_shape(id, rotated);
            }
        }
        self.rebuild_view();
        self.set_status("Rotated 90 CW");
    }

    /// Rotate selected shapes 90 degrees counter-clockwise
    pub fn rotate_90_ccw(&mut self) {
        if self.selected.is_empty() {
            self.set_status("Select shapes to rotate");
            return;
        }
        let Some((min_x, min_y, max_x, max_y)) = self.get_selection_bounds() else {
            return;
        };
        let center = Position::new((min_x + max_x) / 2, (min_y + max_y) / 2);
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Ok(Some(kind)) = self.doc.read_shape(id) {
                let rotated = crate::shapes::rotate_90_ccw(&kind, center);
                let _ = self.doc.update_shape(id, rotated);
            }
        }
        self.rebuild_view();
        self.set_status("Rotated 90 CCW");
    }
}
