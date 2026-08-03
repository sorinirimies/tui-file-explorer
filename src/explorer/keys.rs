use super::*;

impl FileExplorer {
    pub fn handle_key(&mut self, key: KeyEvent) -> ExplorerOutcome {
        // Only react to key-press events.  On Windows (and terminals that
        // negotiate the kitty keyboard protocol) crossterm delivers both
        // Press *and* Release events for every physical key-press.  Without
        // this guard the handler runs twice per key — which double-toggles
        // marks, double-navigates, etc.
        if key.kind != crossterm::event::KeyEventKind::Press {
            return ExplorerOutcome::Pending;
        }

        // ── Rename-mode interception ──────────────────────────────────────────
        // When rename mode is active every printable character feeds the new
        // name.  Enter confirms the rename; Esc cancels.
        handle_input_mode!(self, key, rename_active, rename_input, {
            let new_name = self.rename_input.trim().to_string();
            self.rename_active = false;
            self.rename_input.clear();
            if new_name.is_empty() {
                return ExplorerOutcome::Pending;
            }
            // Grab the source path before we reload.
            let src = match self.entries.get(self.cursor) {
                Some(e) => e.path.clone(),
                None => return ExplorerOutcome::Pending,
            };
            let dst = self.current_dir.join(&new_name);
            match std::fs::rename(&src, &dst) {
                Ok(()) => {
                    self.reload();
                    // Move cursor to the renamed entry.
                    if let Some(idx) = self.entries.iter().position(|e| e.path == dst) {
                        self.cursor = idx;
                    }
                    return ExplorerOutcome::RenameCompleted(dst);
                }
                Err(e) => {
                    self.status = format!("rename failed: {e}");
                    return ExplorerOutcome::Pending;
                }
            }
        });

        // ── Touch-mode interception ───────────────────────────────────────────
        // When touch mode is active every printable character feeds the new
        // file name.  Enter confirms creation; Esc cancels.
        handle_input_mode!(self, key, touch_active, touch_input, {
            let name = self.touch_input.trim().to_string();
            self.touch_active = false;
            self.touch_input.clear();
            if name.is_empty() {
                return ExplorerOutcome::Pending;
            }
            let new_file = self.current_dir.join(&name);
            // Create parent dirs if the name contains path separators,
            // then create (or truncate-to-zero) the file itself.
            let create_result = (|| -> std::io::Result<()> {
                if let Some(parent) = new_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // OpenOptions::create(true) + write(true) creates the
                // file if absent and leaves an existing one untouched.
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&new_file)?;
                Ok(())
            })();
            match create_result {
                Ok(()) => {
                    self.reload();
                    // Move cursor to the newly created file.
                    if let Some(idx) = self.entries.iter().position(|e| e.path == new_file) {
                        self.cursor = idx;
                    }
                    return ExplorerOutcome::TouchCreated(new_file);
                }
                Err(e) => {
                    self.status = format!("touch failed: {e}");
                    return ExplorerOutcome::Pending;
                }
            }
        });

        // ── Mkdir-mode interception ───────────────────────────────────────────
        // When mkdir mode is active every printable character feeds the new
        // folder name.  Enter confirms creation; Esc cancels.
        handle_input_mode!(self, key, mkdir_active, mkdir_input, {
            let name = self.mkdir_input.trim().to_string();
            self.mkdir_active = false;
            self.mkdir_input.clear();
            if name.is_empty() {
                return ExplorerOutcome::Pending;
            }
            let new_dir = self.current_dir.join(&name);
            match std::fs::create_dir_all(&new_dir) {
                Ok(()) => {
                    self.reload();
                    // Move cursor to the newly created directory.
                    if let Some(idx) = self.entries.iter().position(|e| e.path == new_dir) {
                        self.cursor = idx;
                    }
                    return ExplorerOutcome::MkdirCreated(new_dir);
                }
                Err(e) => {
                    self.status = format!("mkdir failed: {e}");
                    return ExplorerOutcome::Pending;
                }
            }
        });

        // ── Search-mode interception ──────────────────────────────────────────
        // When search is active, printable characters feed the query rather than
        // triggering navigation shortcuts.  Navigation keys (arrows, Enter, etc.)
        // fall through to the normal handler below so the list remains usable
        // while filtering.
        if self.search_active {
            match key.code {
                KeyCode::Char(c) if key.modifiers.is_empty() => {
                    self.search_query.push(c);
                    self.cursor = 0;
                    self.scroll_offset = 0;
                    self.reload();
                    return ExplorerOutcome::Pending;
                }
                KeyCode::Backspace => {
                    if self.search_query.is_empty() {
                        // Nothing left to erase — deactivate search.
                        self.search_active = false;
                    } else {
                        self.search_query.pop();
                        self.cursor = 0;
                        self.scroll_offset = 0;
                        self.reload();
                    }
                    return ExplorerOutcome::Pending;
                }
                KeyCode::Esc => {
                    // First Esc cancels search; second Esc (when already
                    // inactive) dismisses the explorer entirely.
                    self.search_active = false;
                    self.search_query.clear();
                    self.cursor = 0;
                    self.scroll_offset = 0;
                    self.reload();
                    return ExplorerOutcome::Pending;
                }
                _ => {} // navigation keys fall through
            }
        }

        match key.code {
            // ── Dismiss ──────────────────────────────────────────────────────
            KeyCode::Esc => ExplorerOutcome::Dismissed,

            // ── Vim-style quit ───────────────────────────────────────────────
            KeyCode::Char('q') if key.modifiers.is_empty() => ExplorerOutcome::Dismissed,

            // ── Move up ──────────────────────────────────────────────────────
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                ExplorerOutcome::Pending
            }

            // ── Move down ────────────────────────────────────────────────────
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                ExplorerOutcome::Pending
            }

            // ── Page up ──────────────────────────────────────────────────────
            KeyCode::PageUp => {
                for _ in 0..self.page_size {
                    self.move_up();
                }
                ExplorerOutcome::Pending
            }

            // ── Page down ────────────────────────────────────────────────────
            KeyCode::PageDown => {
                for _ in 0..self.page_size {
                    self.move_down();
                }
                ExplorerOutcome::Pending
            }

            // ── Jump to top ──────────────────────────────────────────────────
            KeyCode::Home | KeyCode::Char('g') => {
                self.cursor = 0;
                self.scroll_offset = 0;
                ExplorerOutcome::Pending
            }

            // ── Jump to bottom ───────────────────────────────────────────────
            KeyCode::End | KeyCode::Char('G') => {
                if !self.entries.is_empty() {
                    self.cursor = self.entries.len() - 1;
                }
                ExplorerOutcome::Pending
            }

            // ── Ascend (go to parent) ─────────────────────────────────────────
            // Left arrow / Backspace / h all ascend to the parent directory.
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
                self.ascend();
                ExplorerOutcome::Pending
            }

            // ── Navigate right (pure navigation, never exits) ─────────────────
            // Right arrow descends into a directory; on a file it just moves
            // the cursor down so the user can keep browsing.
            KeyCode::Right => self.navigate(),

            // ── Confirm / descend ─────────────────────────────────────────────
            // Enter / l descend into a directory or confirm (select) a file,
            // which signals the caller to exit the TUI.
            KeyCode::Enter | KeyCode::Char('l') => self.confirm(),

            // ── Toggle hidden files ─────────────────────────────────
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                let was = self.cursor;
                self.reload();
                self.cursor = was.min(self.entries.len().saturating_sub(1));
                ExplorerOutcome::Pending
            }

            // ── Toggle file/folder size display ───────────────────────
            KeyCode::Char('z') if key.modifiers.is_empty() => {
                self.show_sizes = !self.show_sizes;
                ExplorerOutcome::Pending
            }

            // ── Activate incremental search ───────────────────────────────────
            KeyCode::Char('/') if key.modifiers.is_empty() => {
                self.search_active = true;
                ExplorerOutcome::Pending
            }

            // ── Cycle sort mode ───────────────────────────────────────────────
            KeyCode::Char('s') if key.modifiers.is_empty() => {
                self.sort_mode = self.sort_mode.next();
                let was = self.cursor;
                self.reload();
                self.cursor = was.min(self.entries.len().saturating_sub(1));
                ExplorerOutcome::Pending
            }

            // ── Toggle space-mark on current entry ────────────────────────────
            KeyCode::Char(' ') => {
                self.toggle_mark();
                ExplorerOutcome::Pending
            }

            // ── Activate mkdir mode ───────────────────────────────────────────
            KeyCode::Char('n') if key.modifiers.is_empty() => {
                self.mkdir_active = true;
                self.mkdir_input.clear();
                ExplorerOutcome::Pending
            }

            // ── Activate touch (new file) mode ────────────────────────────────
            // Shift+N — complement to `n` (mkdir).
            KeyCode::Char('N') if key.modifiers.is_empty() => {
                self.touch_active = true;
                self.touch_input.clear();
                ExplorerOutcome::Pending
            }

            // ── Activate rename mode ──────────────────────────────────────────
            // `r` — pre-fills the input with the current entry's name so the
            // user can edit it rather than type from scratch.
            KeyCode::Char('r') if key.modifiers.is_empty() => {
                if let Some(entry) = self.entries.get(self.cursor) {
                    self.rename_input = entry.name.clone();
                    self.rename_active = true;
                }
                ExplorerOutcome::Pending
            }

            _ => ExplorerOutcome::Unhandled,
        }
    }

    // ── Internal navigation helpers ───────────────────────────────────────────

    pub(crate) fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        // If entries shrank (e.g. external deletion) clamp to valid range.
        self.clamp_cursor();
    }

    pub(crate) fn move_down(&mut self) {
        let last = self.entries.len().saturating_sub(1);
        if !self.entries.is_empty() && self.cursor < last {
            self.cursor += 1;
        }
        self.clamp_cursor();
    }

    /// Clamp `cursor` and `scroll_offset` so they never exceed the current
    /// entries length.  Safe to call at any time — a no-op when everything is
    /// already in range.
    pub(crate) fn clamp_cursor(&mut self) {
        let max = self.entries.len().saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }
        if self.scroll_offset > self.cursor {
            self.scroll_offset = self.cursor;
        }
    }

    pub(crate) fn ascend(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            let prev = self.current_dir.clone();
            self.current_dir = parent;
            self.cursor = 0;
            self.scroll_offset = 0;
            // Clear search and marks when navigating to a different directory.
            self.search_active = false;
            self.search_query.clear();
            self.marked.clear();
            self.reload();
            // Try to land the cursor on the directory we just came from.
            if let Some(idx) = self.entries.iter().position(|e| e.path == prev) {
                self.cursor = idx;
            }
            // Always clamp in case the parent is empty or shorter than expected.
            self.clamp_cursor();
        } else {
            // Already at root — stay put, do nothing.
            self.status = "Already at the filesystem root.".to_string();
        }
    }

    /// Navigate into the highlighted entry without ever exiting the TUI.
    ///
    /// - **Directory** → descend (same as `confirm` on a dir).
    /// - **File** → move the cursor down one step so the user can keep
    ///   browsing without accidentally triggering a selection/exit.
    fn navigate(&mut self) -> ExplorerOutcome {
        let Some(entry) = self.entries.get(self.cursor) else {
            return ExplorerOutcome::Pending;
        };

        if entry.is_dir {
            let path = entry.path.clone();
            self.search_active = false;
            self.search_query.clear();
            self.marked.clear();
            self.navigate_to(path);
        } else {
            self.move_down();
        }
        ExplorerOutcome::Pending
    }

    fn confirm(&mut self) -> ExplorerOutcome {
        let Some(entry) = self.entries.get(self.cursor) else {
            return ExplorerOutcome::Pending;
        };

        if entry.is_dir {
            let path = entry.path.clone();
            // Clear search and marks when descending into a subdirectory.
            self.search_active = false;
            self.search_query.clear();
            self.marked.clear();
            self.navigate_to(path);
            ExplorerOutcome::Pending
        } else {
            // All visible files already passed the extension filter in load_entries,
            // so every non-directory entry is unconditionally selectable here.
            ExplorerOutcome::Selected(entry.path.clone())
        }
    }
}
