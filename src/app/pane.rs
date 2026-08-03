//! Pane management: opening, closing, focusing, and accessing panes.
//!
//! Split out of `app.rs` to keep the multi-pane logic (which grew
//! considerably when `tfe` moved from a fixed 2-pane layout to an
//! arbitrary-N-pane layout) separate from clipboard/file-op and
//! key-dispatch concerns.

use super::*;

impl App {
    pub fn active_pane(&self) -> &FileExplorer {
        &self.panes[self.active_idx]
    }

    /// Return a mutable reference to the currently active pane.
    pub fn active_pane_mut(&mut self) -> &mut FileExplorer {
        &mut self.panes[self.active_idx]
    }

    /// Number of panes currently open.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Move keyboard focus to the next pane, wrapping around.
    pub fn focus_next_pane(&mut self) {
        if self.panes.len() > 1 {
            self.active_idx = (self.active_idx + 1) % self.panes.len();
        }
    }

    /// Move keyboard focus to the previous pane, wrapping around.
    pub fn focus_prev_pane(&mut self) {
        if self.panes.len() > 1 {
            self.active_idx = self
                .active_idx
                .checked_sub(1)
                .unwrap_or(self.panes.len() - 1);
        }
    }

    /// Open a new pane rooted at `dir`, cloning display settings (extension
    /// filter, hidden-file visibility, sort mode) from the active pane, and
    /// focus it. The new pane is inserted immediately after the active one.
    pub fn add_pane(&mut self, dir: PathBuf) {
        let active = self.active_pane();
        let mut builder = FileExplorer::builder(dir)
            .show_hidden(active.show_hidden)
            .show_sizes(active.show_sizes)
            .sort_mode(active.sort_mode);
        if !active.extension_filter.is_empty() {
            builder = builder.extension_filter(active.extension_filter.clone());
        }
        let new_pane = builder.build();
        let insert_at = self.active_idx + 1;
        self.panes.insert(insert_at, new_pane);
        self.active_idx = insert_at;
    }

    /// Open a new pane at the active pane's current directory.
    pub fn add_pane_from_active(&mut self) {
        let dir = self.active_pane().current_dir.clone();
        self.add_pane(dir);
    }

    /// Close the active pane. At least one pane must always remain; if only
    /// one pane is open this is a no-op (with a status message).
    pub fn close_active_pane(&mut self) {
        if self.panes.len() <= 1 {
            self.status_msg = "Cannot close the last remaining pane.".into();
            return;
        }
        self.panes.remove(self.active_idx);
        if self.active_idx >= self.panes.len() {
            self.active_idx = self.panes.len() - 1;
        }
    }
}
