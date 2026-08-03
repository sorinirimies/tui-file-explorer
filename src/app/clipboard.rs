//! Clipboard and file operations: yank, paste, and delete.
//!
//! Split out of `app.rs`; owns the copy/cut/paste/delete workflows and
//! their interaction with [`super::Modal`] confirmation dialogs and
//! [`super::CopyProgress`] tracking.

use super::*;

impl App {
    // ── File operations ───────────────────────────────────────────────────────

    /// Yank (copy or cut) into the clipboard.
    ///
    /// Marks are checked in priority order:
    /// 1. Active pane marks — the normal single-pane workflow.
    /// 2. Inactive pane marks — handles the common dual-pane workflow where
    ///    the user marks files in the source pane, tabs to the destination
    ///    pane, and then presses `p` or `x`.
    /// 3. Active pane cursor entry — fallback when nothing is marked.
    ///
    /// Marks on whichever pane was used are cleared after the yank.
    ///
    /// Note: for copy operations, `paste()` calls this automatically when
    /// marks exist but the clipboard is empty — the user can simply mark
    /// files with Space and press `p` directly.
    pub fn yank(&mut self, op: ClipOp) {
        let active_marks: Vec<PathBuf> = self.active_pane().marked.iter().cloned().collect();
        // Search other panes (in order) for the first one with marks.
        let inactive_pane_idx = self
            .panes
            .iter()
            .enumerate()
            .find(|(i, p)| *i != self.active_idx && !p.marked.is_empty())
            .map(|(i, _)| i);
        let inactive_marks: Vec<PathBuf> = inactive_pane_idx
            .map(|i| self.panes[i].marked.iter().cloned().collect())
            .unwrap_or_default();

        // Determine which set of paths to use and which pane to clear marks from.
        enum Source {
            ActiveMarks,
            InactiveMarks,
            Cursor,
        }

        let source = if !active_marks.is_empty() {
            Source::ActiveMarks
        } else if !inactive_marks.is_empty() {
            Source::InactiveMarks
        } else {
            Source::Cursor
        };

        let paths: Vec<PathBuf> = match source {
            Source::ActiveMarks => {
                let mut sorted = active_marks;
                sorted.sort();
                sorted
            }
            Source::InactiveMarks => {
                let mut sorted = inactive_marks;
                sorted.sort();
                sorted
            }
            Source::Cursor => {
                if let Some(entry) = self.active_pane().current_entry() {
                    vec![entry.path.clone()]
                } else {
                    return;
                }
            }
        };

        let count = paths.len();
        let (verb, hint) = if op == ClipOp::Copy {
            ("Copied", "paste a copy")
        } else {
            ("Cut", "move")
        };

        let label = if count == 1 {
            format!(
                "'{}'",
                paths[0].file_name().unwrap_or_default().to_string_lossy()
            )
        } else {
            format!("{count} items")
        };

        self.clipboard = Some(ClipboardItem { paths, op });

        // Clear marks from whichever pane was the source.
        match source {
            Source::ActiveMarks | Source::Cursor => self.active_pane_mut().clear_marks(),
            Source::InactiveMarks => {
                if let Some(i) = inactive_pane_idx {
                    self.panes[i].clear_marks();
                }
            }
        }

        self.status_msg = format!("{verb} {label} — press p to {hint}");
    }

    /// Paste the clipboard item into the active pane's current directory.
    ///
    /// If the destination already exists, a [`Modal::Overwrite`] is
    /// raised instead of overwriting silently.
    pub fn paste(&mut self) {
        // If no clipboard exists but files are marked (Space), automatically
        // treat the marks as a copy source — the user can mark with Space
        // and paste with p directly, without a separate y step.
        if self.clipboard.is_none() {
            let has_marks = self.panes.iter().any(|p| !p.marked.is_empty());
            if has_marks {
                self.yank(ClipOp::Copy);
            }
        }

        let Some(clip) = self.clipboard.clone() else {
            self.status_msg = "Nothing in clipboard — mark files with Space first.".into();
            return;
        };

        let dst_dir = self.active_pane().current_dir.clone();

        // For a single-item clipboard check for same-dir cut and overwrite modal.
        if clip.paths.len() == 1 {
            let src = &clip.paths[0];
            let file_name = match src.file_name() {
                Some(n) => n.to_owned(),
                None => {
                    self.status_msg = "Cannot paste: clipboard path has no filename.".into();
                    return;
                }
            };
            let dst = dst_dir.join(&file_name);

            if clip.op == ClipOp::Cut && src.parent() == Some(&dst_dir) {
                self.status_msg = "Source and destination are the same — skipped.".into();
                return;
            }

            if dst.exists() {
                self.modal = Some(Modal::Overwrite {
                    src: src.clone(),
                    dst,
                    is_cut: clip.op == ClipOp::Cut,
                });
                return;
            }
        }

        // Multi-item (or single with no conflict): paste all paths.
        self.do_paste_all(&clip.paths.clone(), &dst_dir, clip.op == ClipOp::Cut);
    }

    /// Perform the actual copy/move for a single src→dst pair.
    ///
    /// Used by the overwrite-confirmation modal path (single file only).
    /// For multi-file paste use [`App::do_paste_all`].
    pub fn do_paste(&mut self, src: &Path, dst: &Path, is_cut: bool) {
        let result = if src.is_dir() {
            copy_dir_all(src, dst)
        } else {
            fs::copy(src, dst).map(|_| ())
        };

        match result {
            Ok(()) => {
                if is_cut {
                    let _ = if src.is_dir() {
                        fs::remove_dir_all(src)
                    } else {
                        fs::remove_file(src)
                    };
                    self.clipboard = None;
                }
                for p in self.panes.iter_mut() {
                    p.reload();
                    p.clear_dir_size_cache();
                }
                let msg = format!(
                    "{} '{}'",
                    if is_cut { "Moved" } else { "Pasted" },
                    dst.file_name().unwrap_or_default().to_string_lossy()
                );
                self.status_msg = msg.clone();
                self.notify(msg);
            }
            Err(e) => {
                let msg = format!("Paste failed: {e}");
                self.status_msg = format!("Error: {msg}");
                self.notify_error(msg);
            }
        }
    }

    /// Paste all `srcs` into `dst_dir`, performing copy or move for each.
    ///
    /// Errors are collected and reported in the status message alongside the
    /// success count.  On a fully successful cut the clipboard is cleared.
    pub fn do_paste_all(&mut self, srcs: &[PathBuf], dst_dir: &Path, is_cut: bool) {
        let mut errors: Vec<String> = Vec::new();
        let mut succeeded: usize = 0;
        let total = srcs.len();
        let verb_label = if is_cut { "Moving" } else { "Copying" };

        // Initialise progress — visible immediately on the next render.
        self.copy_progress = Some(CopyProgress::new(
            format!("{verb_label} {total} item(s)…"),
            total,
        ));

        for src in srcs {
            let file_name = match src.file_name() {
                Some(n) => n,
                None => {
                    errors.push(format!("skipped (no filename): {}", src.display()));
                    if let Some(p) = &mut self.copy_progress {
                        p.done += 1;
                    }
                    continue;
                }
            };

            // Update the "currently processing" label before the (potentially
            // slow) copy so the UI reflects what is happening right now.
            if let Some(p) = &mut self.copy_progress {
                p.current_item = file_name.to_string_lossy().into_owned();
            }

            let dst = dst_dir.join(file_name);

            // Skip same-dir cut silently.
            if is_cut && src.parent() == Some(dst_dir) {
                if let Some(p) = &mut self.copy_progress {
                    p.done += 1;
                }
                continue;
            }

            let result = if src.is_dir() {
                copy_dir_all(src, &dst)
            } else {
                fs::copy(src, &dst).map(|_| ())
            };

            match result {
                Ok(()) => {
                    if is_cut {
                        let _ = if src.is_dir() {
                            fs::remove_dir_all(src)
                        } else {
                            fs::remove_file(src)
                        };
                    }
                    succeeded += 1;
                }
                Err(e) => {
                    errors.push(format!(
                        "'{}': {e}",
                        src.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }

            if let Some(p) = &mut self.copy_progress {
                p.done += 1;
            }
        }

        // Clear progress now that the operation has finished.
        self.copy_progress = None;

        if is_cut && errors.is_empty() {
            self.clipboard = None;
        }

        for p in self.panes.iter_mut() {
            p.reload();
            p.clear_dir_size_cache();
        }

        if errors.is_empty() {
            let verb = if is_cut { "Moved" } else { "Pasted" };
            let msg = format!("{verb} {succeeded} item(s).");
            self.status_msg = msg.clone();
            self.notify(msg);
        } else {
            let verb = if is_cut { "Moved" } else { "Pasted" };
            let msg = format!(
                "{verb} {succeeded}, {} error(s): {}",
                errors.len(),
                errors.join("; ")
            );
            self.status_msg = format!("Error: {msg}");
            self.notify_error(msg);
        }
    }

    /// Raise a [`Modal::Delete`] for the currently highlighted entry,
    /// or a [`Modal::MultiDelete`] when there are space-marked entries
    /// in the active pane.
    pub fn prompt_delete(&mut self) {
        let marked: Vec<PathBuf> = self.active_pane().marked.iter().cloned().collect();
        if !marked.is_empty() {
            let mut sorted = marked;
            sorted.sort();
            self.modal = Some(Modal::MultiDelete { paths: sorted });
        } else if let Some(entry) = self.active_pane().current_entry() {
            self.modal = Some(Modal::Delete {
                path: entry.path.clone(),
            });
        }
    }

    /// Execute a confirmed multi-deletion and reload both panes.
    pub fn confirm_delete_many(&mut self, paths: &[PathBuf]) {
        let mut errors: Vec<String> = Vec::new();
        let mut deleted: usize = 0;

        for path in paths {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            match result {
                Ok(()) => deleted += 1,
                Err(e) => errors.push(format!(
                    "'{}': {e}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )),
            }
        }

        for p in self.panes.iter_mut() {
            p.clear_marks();
            p.reload();
            p.clear_dir_size_cache();
        }

        if errors.is_empty() {
            self.status_msg = format!("Deleted {deleted} item(s).");
        } else {
            self.status_msg = format!(
                "Deleted {deleted}, {} error(s): {}",
                errors.len(),
                errors.join("; ")
            );
        }
    }

    /// Execute a confirmed deletion and reload both panes.
    pub fn confirm_delete(&mut self, path: &Path) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match result {
            Ok(()) => {
                for p in self.panes.iter_mut() {
                    p.reload();
                    p.clear_dir_size_cache();
                }
                self.status_msg = format!("Deleted '{name}'");
            }
            Err(e) => {
                self.status_msg = format!("Delete failed: {e}");
            }
        }
    }
}
