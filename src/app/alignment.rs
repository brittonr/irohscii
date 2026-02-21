//! Alignment and distribution methods for App

use crate::document::ShapeId;

use super::App;

impl App {
    /// Align selected shapes to left edge
    pub fn align_left(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((target_x, _, _, _)) = self.get_selection_bounds() else {
            return;
        };
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (sx_min, _, _, _) = shape.bounds();
                let dx = target_x - sx_min;
                if dx != 0 {
                    let _ = self.doc.translate_shape(id, dx, 0);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned left");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Align selected shapes to right edge
    pub fn align_right(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((_, _, target_x, _)) = self.get_selection_bounds() else {
            return;
        };
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (_, _, sx_max, _) = shape.bounds();
                let dx = target_x - sx_max;
                if dx != 0 {
                    let _ = self.doc.translate_shape(id, dx, 0);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned right");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Align selected shapes to top edge
    pub fn align_top(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((_, target_y, _, _)) = self.get_selection_bounds() else {
            return;
        };
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (_, sy_min, _, _) = shape.bounds();
                let dy = target_y - sy_min;
                if dy != 0 {
                    let _ = self.doc.translate_shape(id, 0, dy);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned top");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Align selected shapes to bottom edge
    pub fn align_bottom(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((_, _, _, target_y)) = self.get_selection_bounds() else {
            return;
        };
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (_, _, _, sy_max) = shape.bounds();
                let dy = target_y - sy_max;
                if dy != 0 {
                    let _ = self.doc.translate_shape(id, 0, dy);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned bottom");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Align selected shapes to horizontal center
    pub fn align_center_h(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((min_x, _, max_x, _)) = self.get_selection_bounds() else {
            return;
        };
        let target_center = (min_x + max_x) / 2;
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (sx_min, _, sx_max, _) = shape.bounds();
                let shape_center = (sx_min + sx_max) / 2;
                let dx = target_center - shape_center;
                if dx != 0 {
                    let _ = self.doc.translate_shape(id, dx, 0);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned center (horizontal)");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Align selected shapes to vertical center
    pub fn align_center_v(&mut self) {
        
        if self.selected.len() < 2 {
            self.set_status("Select at least 2 shapes to align");
            return;
        }
        let Some((_, min_y, _, max_y)) = self.get_selection_bounds() else {
            return;
        };
        let target_center = (min_y + max_y) / 2;
        
        debug_assert!(self.selected.iter().all(|&id| self.shape_view.get(id).is_some()), 
                      "precondition: all selected shapes exist in view");
        
        self.save_undo_state();
        for &id in self.selected.clone().iter() {
            if let Some(shape) = self.shape_view.get(id) {
                let (_, sy_min, _, sy_max) = shape.bounds();
                let shape_center = (sy_min + sy_max) / 2;
                let dy = target_center - shape_center;
                if dy != 0 {
                    let _ = self.doc.translate_shape(id, 0, dy);
                }
            }
        }
        self.rebuild_view();
        self.set_status("Aligned center (vertical)");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Distribute selected shapes evenly (horizontal spacing)
    pub fn distribute_horizontal(&mut self) {
        if self.selected.len() < 3 {
            self.set_status("Select at least 3 shapes to distribute");
            return;
        }

        // Collect shapes with their centers and IDs
        let mut shapes: Vec<(ShapeId, i32, i32)> = Vec::new(); // (id, center_x, width)
        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                let (min_x, _, max_x, _) = shape.bounds();
                let center_x = (min_x + max_x) / 2;
                let width = max_x - min_x;
                shapes.push((id, center_x, width));
            }
        }

        debug_assert!(shapes.len() == self.selected.len(), 
                      "precondition: all selected shapes exist in view");

        // Sort by center position (left to right)
        shapes.sort_by_key(|(_, center, _)| *center);

        // Calculate the span from first to last shape center
        let first_center = shapes.first().map(|(_, c, _)| *c).unwrap_or(0);
        let last_center = shapes.last().map(|(_, c, _)| *c).unwrap_or(0);
        let total_span = last_center - first_center;
        let num_gaps = shapes.len() - 1;

        let has_zero_gaps = num_gaps == 0;
        let has_zero_span = total_span == 0;
        if has_zero_gaps {
            return;
        }
        if has_zero_span {
            return;
        }

        let gap = total_span / num_gaps as i32;

        self.save_undo_state();

        // Move each shape to its new position (skip first and last)
        for (i, (id, current_center, _)) in shapes.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == shapes.len() - 1;
            if is_first {
                continue; // Keep first in place
            }
            if is_last {
                continue; // Keep last in place
            }
            let target_center = first_center + (gap * i as i32);
            let dx = target_center - current_center;
            if dx != 0 {
                let _ = self.doc.translate_shape(*id, dx, 0);
            }
        }

        self.rebuild_view();
        self.set_status("Distributed horizontally");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }

    /// Distribute selected shapes evenly (vertical spacing)
    pub fn distribute_vertical(&mut self) {
        if self.selected.len() < 3 {
            self.set_status("Select at least 3 shapes to distribute");
            return;
        }

        // Collect shapes with their centers and IDs
        let mut shapes: Vec<(ShapeId, i32, i32)> = Vec::new(); // (id, center_y, height)
        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                let (_, min_y, _, max_y) = shape.bounds();
                let center_y = (min_y + max_y) / 2;
                let height = max_y - min_y;
                shapes.push((id, center_y, height));
            }
        }

        debug_assert!(shapes.len() == self.selected.len(), 
                      "precondition: all selected shapes exist in view");

        // Sort by center position (top to bottom)
        shapes.sort_by_key(|(_, center, _)| *center);

        // Calculate the span from first to last shape center
        let first_center = shapes.first().map(|(_, c, _)| *c).unwrap_or(0);
        let last_center = shapes.last().map(|(_, c, _)| *c).unwrap_or(0);
        let total_span = last_center - first_center;
        let num_gaps = shapes.len() - 1;

        let has_zero_gaps = num_gaps == 0;
        let has_zero_span = total_span == 0;
        if has_zero_gaps {
            return;
        }
        if has_zero_span {
            return;
        }

        let gap = total_span / num_gaps as i32;

        self.save_undo_state();

        // Move each shape to its new position (skip first and last)
        for (i, (id, current_center, _)) in shapes.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == shapes.len() - 1;
            if is_first {
                continue; // Keep first in place
            }
            if is_last {
                continue; // Keep last in place
            }
            let target_center = first_center + (gap * i as i32);
            let dy = target_center - current_center;
            if dy != 0 {
                let _ = self.doc.translate_shape(*id, 0, dy);
            }
        }

        self.rebuild_view();
        self.set_status("Distributed vertically");
        
        debug_assert!(self.shape_view.len() > 0, "postcondition: view contains shapes");
    }
}
