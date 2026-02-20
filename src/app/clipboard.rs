//! Clipboard methods for App (copy, paste, duplicate)

use crate::shapes::ShapeKind;

use super::App;

impl App {
    /// Copy sync ticket to system clipboard
    pub fn copy_ticket_to_clipboard(&mut self) {
        if let Some(ref ticket) = self.sync_ticket {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Try wl-copy (Wayland) first - spawn and forget
            if let Ok(mut child) = Command::new("wl-copy")
                .arg(ticket)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                // Don't wait, just detach
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                self.set_status(format!("Copied: {}", ticket));
                return;
            }

            // Fall back to xsel
            if let Ok(mut child) = Command::new("xsel")
                .args(["--clipboard", "--input"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                && let Some(mut stdin) = child.stdin.take()
            {
                let ticket_clone = ticket.clone();
                std::thread::spawn(move || {
                    let _ = stdin.write_all(ticket_clone.as_bytes());
                    drop(stdin);
                    let _ = child.wait();
                });
                self.set_status(format!("Copied: {}", ticket));
                return;
            }

            self.set_status("Install wl-copy or xsel for clipboard");
        } else {
            self.set_status("No sync session active");
        }
    }

    /// Copy selected shapes to clipboard
    pub fn yank(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.clipboard.clear();
        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                self.clipboard.push(shape.kind.clone());
            }
        }
        let count = self.clipboard.len();
        self.set_status(format!(
            "Yanked {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    /// Paste shapes from clipboard
    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.save_undo_state();
        self.selected.clear();
        for kind in self.clipboard.clone() {
            let new_kind = kind.translated(2, 1);
            if let Ok(id) = self.add_shape_to_active_layer(new_kind) {
                self.selected.insert(id);
            }
        }
        self.rebuild_view();
        self.doc.mark_dirty();
        let count = self.selected.len();
        self.set_status(format!(
            "Pasted {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    /// Duplicate selected shapes (Ctrl+D)
    pub fn duplicate_selection(&mut self) {
        if self.selected.is_empty() {
            self.set_status("No shapes selected to duplicate");
            return;
        }

        self.save_undo_state();

        // Collect shapes to duplicate
        let mut shapes_to_add: Vec<ShapeKind> = Vec::new();
        for &id in &self.selected {
            if let Some(shape) = self.shape_view.get(id) {
                // Offset duplicated shapes slightly (down-right)
                let new_kind = shape.kind.translated(2, 1);
                shapes_to_add.push(new_kind);
            }
        }

        // Add duplicated shapes and select them
        self.selected.clear();
        for kind in shapes_to_add {
            if let Ok(id) = self.add_shape_to_active_layer(kind) {
                self.selected.insert(id);
            }
        }

        self.rebuild_view();
        self.doc.mark_dirty();

        let count = self.selected.len();
        self.set_status(format!(
            "Duplicated {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }
}
