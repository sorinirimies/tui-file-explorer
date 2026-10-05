use super::{background::AfterLoad, *};

impl ExplorerCommand {
    /// The command bound to `key` in the built-in keymap, if any.
    #[must_use]
    pub fn from_key(key: KeyEvent) -> Option<Self> {
        Keymap::builtin().resolve(key)
    }
}

// ── Text prompts (rename / touch / mkdir) ─────────────────────────────────────

impl PromptKind {
    fn outcome(self, from: PathBuf, to: PathBuf) -> ExplorerOutcome {
        match self {
            Self::Rename => ExplorerOutcome::RenameCompleted { from, to },
            Self::Touch => ExplorerOutcome::TouchCreated(to),
            Self::Mkdir => ExplorerOutcome::MkdirCreated(to),
        }
    }
}

impl FileExplorer {
    /// Feed one key to the open prompt. Always consumes the event.
    fn handle_prompt_key(&mut self, key: KeyEvent) -> ExplorerOutcome {
        let Some(prompt) = self.prompt.as_mut() else {
            return ExplorerOutcome::Pending;
        };
        match key.code {
            // Printable character (no modifiers, or Shift only) → append.
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                prompt.text.push(c);
            }
            KeyCode::Backspace => {
                prompt.text.pop();
            }
            KeyCode::Enter => return self.commit_prompt(),
            // Esc cancels without committing.
            KeyCode::Esc => self.prompt = None,
            // Any other key stays in the prompt and is swallowed.
            _ => {}
        }
        ExplorerOutcome::Pending
    }

    /// Apply the prompt's text to the filesystem and close it.
    fn commit_prompt(&mut self) -> ExplorerOutcome {
        let Some(Prompt { kind, text }) = self.prompt.take() else {
            return ExplorerOutcome::Pending;
        };
        let name = text.trim();
        if name.is_empty() {
            return ExplorerOutcome::Pending;
        }

        let target = self.current_dir.join(name);
        let mut from = target.clone();
        let result = match kind {
            PromptKind::Rename => {
                // The source is whatever is under the cursor *before* reload.
                let Some(src) = self.entries.get(self.cursor).map(|e| e.path.clone()) else {
                    return ExplorerOutcome::Pending;
                };
                from = src.clone();
                self.filesystem.rename(&src, &target)
            }
            PromptKind::Touch => self.filesystem.create_file(&target),
            PromptKind::Mkdir => self.filesystem.create_dir_all(&target),
        };

        match result {
            Ok(()) => {
                self.reload_with(AfterLoad {
                    select: Some(target.clone()),
                    cursor: None,
                });
                kind.outcome(from, target)
            }
            Err(e) => {
                self.status = format!("{} failed: {e}", kind.verb());
                ExplorerOutcome::Pending
            }
        }
    }

    /// Navigate to `path`: a directory is entered; for a file, its parent
    /// directory is opened with the file highlighted.
    ///
    /// Clears search and marks (the listing changes wholesale).
    pub fn go_to(&mut self, path: &Path) {
        let (dir, select) = if self.filesystem.is_dir(path) {
            (path.to_path_buf(), None)
        } else {
            match path.parent() {
                Some(parent) => (parent.to_path_buf(), Some(path)),
                None => return,
            }
        };
        self.current_dir = dir;
        self.search_active = false;
        self.search_query.clear();
        self.marked.clear();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.reload_with(AfterLoad {
            select: select.map(Path::to_path_buf),
            cursor: None,
        });
    }

    /// Move the cursor onto `path` if it is visible.
    pub fn select_path(&mut self, path: &Path) {
        if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
            self.cursor = idx;
        }
    }

    /// Re-read the directory but keep the cursor on (or near) its old row.
    fn reload_keep_cursor(&mut self) {
        // `reload` already restores the row (clamped), also when deferred.
        self.reload();
    }

    /// Reset the list position after the visible entries changed wholesale.
    fn reload_from_top(&mut self) {
        self.cursor = 0;
        self.scroll_offset = 0;
        self.reload();
    }
}

// ── Public key / command entry points ─────────────────────────────────────────

impl FileExplorer {
    /// Dispatch a semantic command without coupling host code to key bindings.
    ///
    /// Routed through [`handle_key`](Self::handle_key) so active prompts and
    /// incremental search see it exactly like the equivalent key press.
    pub fn handle_command(&mut self, command: ExplorerCommand) -> ExplorerOutcome {
        self.handle_key(self.keymap.canonical_key(command))
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ExplorerOutcome {
        press_only!(key, ExplorerOutcome::Pending);

        // An open rename / touch / mkdir prompt owns the keyboard.
        if self.prompt.is_some() {
            return self.handle_prompt_key(key);
        }

        // While searching, printable characters feed the query rather than
        // triggering shortcuts. Navigation keys fall through so the filtered
        // list stays usable.
        if self.search_active {
            match key.code {
                KeyCode::Char(c) if key.modifiers.is_empty() => {
                    self.search_query.push(c);
                    self.reload_from_top();
                    return ExplorerOutcome::Pending;
                }
                KeyCode::Backspace => {
                    if self.search_query.is_empty() {
                        // Nothing left to erase — deactivate search.
                        self.search_active = false;
                    } else {
                        self.search_query.pop();
                        self.reload_from_top();
                    }
                    return ExplorerOutcome::Pending;
                }
                // First Esc cancels search; a second (inactive) one dismisses.
                KeyCode::Esc => {
                    self.search_active = false;
                    self.search_query.clear();
                    self.reload_from_top();
                    return ExplorerOutcome::Pending;
                }
                _ => {}
            }
        }

        match self.keymap.resolve(key) {
            Some(command) => self.execute(command),
            None => ExplorerOutcome::Unhandled,
        }
    }

    /// Run `command` directly (no prompt / search interception).
    fn execute(&mut self, command: ExplorerCommand) -> ExplorerOutcome {
        use ExplorerCommand as C;
        match command {
            C::Dismiss => return ExplorerOutcome::Dismissed,
            C::MoveUp => self.move_up(),
            C::MoveDown => self.move_down(),
            C::PageUp => (0..self.page_size).for_each(|_| self.move_up()),
            C::PageDown => (0..self.page_size).for_each(|_| self.move_down()),
            C::Top => {
                self.cursor = 0;
                self.scroll_offset = 0;
            }
            C::Bottom => {
                if !self.entries.is_empty() {
                    self.cursor = self.entries.len() - 1;
                }
            }
            C::Ascend => self.ascend(),
            // Navigate / Confirm may select a file, so they report outcomes.
            C::Navigate => return self.navigate(),
            C::Confirm => return self.confirm(),
            C::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.reload_keep_cursor();
            }
            C::ToggleSizes => self.show_sizes = !self.show_sizes,
            C::ToggleDirSizes => self.show_dir_sizes = !self.show_dir_sizes,
            C::Search => self.search_active = true,
            C::CycleSort => {
                self.sort_mode = self.sort_mode.next();
                self.reload_keep_cursor();
            }
            C::ToggleMark => self.toggle_mark(),
            C::NewDirectory => self.open_prompt(PromptKind::Mkdir, ""),
            C::NewFile => self.open_prompt(PromptKind::Touch, ""),
            // Pre-fill with the current name so it can be edited in place.
            C::Rename => {
                if let Some(entry) = self.entries.get(self.cursor) {
                    let name = entry.name.clone();
                    self.open_prompt(PromptKind::Rename, name);
                }
            }
        }
        ExplorerOutcome::Pending
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
        if let Some(parent) = self.current_dir.parent().map(std::path::Path::to_path_buf) {
            let prev = self.current_dir.clone();
            self.current_dir = parent;
            self.cursor = 0;
            self.scroll_offset = 0;
            // Clear search and marks when navigating to a different directory.
            self.search_active = false;
            self.search_query.clear();
            self.marked.clear();
            // Land the cursor on the directory we just came from.
            self.reload_with(AfterLoad {
                select: Some(prev),
                cursor: None,
            });
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
            if matches!(
                self.selection_mode,
                SelectionMode::Directories | SelectionMode::Any
            ) {
                return ExplorerOutcome::Selected(entry.path.clone());
            }
            let path = entry.path.clone();
            // Clear search and marks when descending into a subdirectory.
            self.search_active = false;
            self.search_query.clear();
            self.marked.clear();
            self.navigate_to(path);
            ExplorerOutcome::Pending
        } else if self.selection_mode == SelectionMode::Directories {
            self.status = "Only directories can be selected.".into();
            ExplorerOutcome::Pending
        } else {
            // All visible files already passed the extension filter in load_entries,
            // so every non-directory entry is unconditionally selectable here.
            ExplorerOutcome::Selected(entry.path.clone())
        }
    }
}
