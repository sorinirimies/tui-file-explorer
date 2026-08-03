//! Keyboard event dispatch: the core `handle_key` state machine.
//!
//! Split out of `app.rs`; this is the single entry point that all
//! keyboard input flows through, delegating to pane/clipboard/theme
//! helpers defined in sibling modules and in the active [`FileExplorer`]
//! pane itself.

use super::*;

impl App {
    // ── Event handling ────────────────────────────────────────────────────────

    /// Process a single [`KeyEvent`] and update application state.
    ///
    /// This is the core key-dispatch method. Library consumers that read
    /// their own events (e.g. via a shared event loop) should call this
    /// directly instead of [`App::handle_event`].
    ///
    /// Returns `true` when the event loop should exit (user confirmed a
    /// selection or dismissed the explorer).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
    /// use tui_file_explorer::{App, AppOptions};
    ///
    /// let mut app = App::new(AppOptions::default());
    ///
    /// // Read the event yourself and forward only key events.
    /// if let Event::Key(key) = event::read().unwrap() {
    ///     let should_exit = app.handle_key(key).unwrap();
    /// }
    /// ```
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> io::Result<bool> {
        // Only react to key-press events.  On Windows (and terminals that
        // negotiate the kitty keyboard protocol) crossterm delivers both
        // Press *and* Release events for every physical key-press.  Without
        // this guard the handler runs twice per key — once on press and once
        // on release — which silently clobbers multi-item clipboard state
        // (the release re-runs yank after marks have been cleared, falling
        // back to the single cursor entry).
        if key.kind != crossterm::event::KeyEventKind::Press {
            return Ok(false);
        }

        // ── Inline editor intercepts all input ───────────────────────────────
        if let Some(ref mut editor) = self.inline_editor {
            match editor.handle_key(key) {
                EditorAction::Continue => return Ok(false),
                EditorAction::Saved => {
                    let fname = editor
                        .path()
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    self.notify(format!("Saved '{fname}'"));
                    // Reload panes to reflect any on-disk changes.
                    for p in self.panes.iter_mut() {
                        p.reload();
                        p.clear_dir_size_cache();
                    }
                    self.preview_state.invalidate();
                    return Ok(false);
                }
                EditorAction::Exit => {
                    self.inline_editor = None;
                    // Reload panes in case the file was changed externally.
                    for p in self.panes.iter_mut() {
                        p.reload();
                    }
                    self.preview_state.invalidate();
                    return Ok(false);
                }
            }
        }

        // Always handle Ctrl-C.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }

        // ── Modal intercepts all input ────────────────────────────────────────
        if let Some(modal) = self.modal.take() {
            match &modal {
                Modal::Delete { path } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let p = path.clone();
                        self.confirm_delete(&p);
                    }
                    _ => self.status_msg = "Delete cancelled.".into(),
                },
                Modal::MultiDelete { paths } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let ps = paths.clone();
                        self.confirm_delete_many(&ps);
                    }
                    _ => self.status_msg = "Multi-delete cancelled.".into(),
                },
                Modal::Overwrite { src, dst, is_cut } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let (s, d, cut) = (src.clone(), dst.clone(), *is_cut);
                        self.do_paste(&s, &d, cut);
                    }
                    _ => self.status_msg = "Paste cancelled.".into(),
                },
            }
            return Ok(false);
        }

        // ── Debug-log scroll (Ctrl+Up / Ctrl+Down) ───────────────────────────
        if self.verbose {
            match key.code {
                KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let max = self.debug_log.len().saturating_sub(1);
                    self.debug_scroll = (self.debug_scroll + 1).min(max);
                    return Ok(false);
                }
                KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.debug_scroll = self.debug_scroll.saturating_sub(1);
                    return Ok(false);
                }
                _ => {}
            }
        }

        // ── Preview scroll (Ctrl+J / Ctrl+K) ─────────────────────────────────
        if self.show_preview {
            match key.code {
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview_state.scroll_down(3);
                    return Ok(false);
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.preview_state.scroll_up(3);
                    return Ok(false);
                }
                _ => {}
            }
        }

        // ── Global keys (always active) ─────────────────────────────────
        // ── Editor panel navigation (arrows / j / k steal focus when open) ───
        if self.show_editor_panel {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                    let editors = App::all_editors();
                    self.editor_panel_idx = (self.editor_panel_idx + 1) % editors.len();
                    return Ok(false);
                }
                KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
                    let editors = App::all_editors();
                    self.editor_panel_idx = self
                        .editor_panel_idx
                        .checked_sub(1)
                        .unwrap_or(editors.len() - 1);
                    return Ok(false);
                }
                KeyCode::Enter => {
                    let editors = App::all_editors();
                    self.editor = editors[self.editor_panel_idx].clone();
                    self.show_editor_panel = false;
                    return Ok(false);
                }
                KeyCode::Esc => {
                    self.show_editor_panel = false;
                    return Ok(false);
                }
                _ => {}
            }
        }

        // ── Theme panel navigation (arrows / j / k steal focus when open) ────
        if self.show_theme_panel {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                    self.next_theme();
                    return Ok(false);
                }
                KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
                    self.prev_theme();
                    return Ok(false);
                }
                _ => {}
            }
        }

        match key.code {
            // Cycle theme forward
            KeyCode::Char('t') if key.modifiers.is_empty() => {
                self.next_theme();
                return Ok(false);
            }
            // Cycle theme backward
            KeyCode::Char('[') => {
                self.prev_theme();
                return Ok(false);
            }
            // Toggle theme panel — closes options/editor panels if open
            KeyCode::Char('T') => {
                self.show_theme_panel = !self.show_theme_panel;
                if self.show_theme_panel {
                    self.show_options_panel = false;
                    self.show_editor_panel = false;
                }
                return Ok(false);
            }
            // Toggle options panel — closes theme/editor panels if open
            KeyCode::Char('O') => {
                self.show_options_panel = !self.show_options_panel;
                if self.show_options_panel {
                    self.show_theme_panel = false;
                    self.show_editor_panel = false;
                }
                return Ok(false);
            }
            // Toggle editor panel — closes theme/options panels if open
            KeyCode::Char('E') => {
                self.show_editor_panel = !self.show_editor_panel;
                if self.show_editor_panel {
                    self.show_options_panel = false;
                    self.show_theme_panel = false;
                    self.sync_editor_panel_idx();
                }
                return Ok(false);
            }
            // Toggle cd-on-exit (also available in the options panel)
            KeyCode::Char('C') => {
                self.cd_on_exit = !self.cd_on_exit;
                let state = if self.cd_on_exit { "on" } else { "off" };
                self.status_msg = format!("cd-on-exit: {state}");
                return Ok(false);
            }
            // Toggle preview panel
            KeyCode::Char('P') => {
                self.show_preview = !self.show_preview;
                if self.show_preview {
                    // Force immediate preview update.
                    self.preview_state.invalidate();
                }
                return Ok(false);
            }
            // Open inline editor for the current file
            KeyCode::Char('i') if key.modifiers.is_empty() => {
                if let Some(entry) = self.active_pane().current_entry() {
                    if !entry.is_dir {
                        match InlineEditor::open(&entry.path) {
                            Ok(ed) => {
                                self.inline_editor = Some(ed);
                            }
                            Err(e) => {
                                self.notify_error(format!("Cannot edit: {e}"));
                            }
                        }
                    }
                }
                return Ok(false);
            }
            // Switch pane (cycle forward / backward through all panes)
            KeyCode::Tab => {
                self.focus_next_pane();
                return Ok(false);
            }
            KeyCode::BackTab => {
                self.focus_prev_pane();
                return Ok(false);
            }
            // Toggle single/multi-pane
            KeyCode::Char('w') if key.modifiers.is_empty() => {
                self.single_pane = !self.single_pane;
                return Ok(false);
            }
            // Open a new pane at the active pane's current directory.
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_pane_from_active();
                return Ok(false);
            }
            // Close the active pane (at least one pane always remains).
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.close_active_pane();
                return Ok(false);
            }

            // Cut
            KeyCode::Char('x') if key.modifiers.is_empty() => {
                self.yank(ClipOp::Cut);
                return Ok(false);
            }
            // Paste
            KeyCode::Char('p') if key.modifiers.is_empty() => {
                self.paste();
                return Ok(false);
            }
            // Delete
            KeyCode::Char('d') if key.modifiers.is_empty() => {
                self.prompt_delete();
                return Ok(false);
            }
            // Open the highlighted file in the configured editor.
            KeyCode::Char('e') if key.modifiers.is_empty() => {
                if self.editor != Editor::None {
                    if let Some(entry) = self.active_pane().current_entry() {
                        if !entry.path.is_dir() {
                            self.open_with_editor = Some(entry.path.clone());
                        }
                        // Silently ignore dirs — no status message per spec.
                    }
                } else {
                    // No editor configured — tell the user how to set one.
                    self.notify_error("No editor set — open Editor picker (Shift + E) to pick one");
                }
                return Ok(false);
            }
            _ => {}
        }

        // ── Delegate to active pane explorer ─────────────────────────────────
        // Clear any previous non-error status when navigating.
        let outcome = self.active_pane_mut().handle_key(key);
        match outcome {
            ExplorerOutcome::Selected(path) => {
                if path.is_dir() {
                    // A directory selection just navigates — exit normally.
                    self.selected = Some(path);
                    return Ok(true);
                }
                // File selected: need an editor to open it.
                if self.editor != Editor::None {
                    self.open_with_editor = Some(path);
                    return Ok(false);
                }
                // No editor configured — stay in the TUI and tell the user.
                self.notify_error("No editor set — open Editor picker (Shift + E) to pick one");
                return Ok(false);
            }
            ExplorerOutcome::Dismissed => return Ok(true),
            ExplorerOutcome::MkdirCreated(path) => {
                self.reload_and_notify(&path, "Created folder");
                self.preview_state.invalidate();
            }
            ExplorerOutcome::TouchCreated(path) => {
                self.reload_and_notify(&path, "Created file");
                self.preview_state.invalidate();
            }
            ExplorerOutcome::RenameCompleted(path) => {
                self.reload_and_notify(&path, "Renamed to");
                self.preview_state.invalidate();
            }
            ExplorerOutcome::Pending => {
                if self.status_msg.starts_with("Error") || self.status_msg.starts_with("Delete") {
                    // keep error messages visible
                } else {
                    self.status_msg.clear();
                }
            }
            ExplorerOutcome::Unhandled => {}
        }

        Ok(false)
    }

    /// Dispatch a pre-read terminal [`Event`].
    ///
    /// This is the testable core of event handling.  Only [`Event::Key`]
    /// events are forwarded to [`App::handle_key`]; all other event types
    /// (mouse, resize, focus, paste) are consumed and silently ignored
    /// because the application is entirely keyboard-driven.
    ///
    /// # Why mouse capture is **not** enabled
    ///
    /// Earlier versions of `tfe` enabled `EnableMouseCapture` even though no
    /// widget or handler ever inspected mouse input.  On macOS this caused
    /// SGR mouse-tracking escape sequences (`^[[<35;…M`) to leak visually
    /// at the bottom of the terminal.  Removing mouse capture is safe on
    /// **all** platforms (Linux, macOS, Windows) because the TUI is purely
    /// keyboard-driven.
    pub fn handle_raw_event(&mut self, event: Event) -> io::Result<bool> {
        match event {
            Event::Key(key) => self.handle_key(key),
            // Mouse, Resize, FocusGained, FocusLost, Paste — consume and
            // discard.  Resize is handled automatically by ratatui's
            // `Terminal::draw` which queries the terminal size each frame.
            _ => Ok(false),
        }
    }

    /// Read one terminal event and update application state.
    ///
    /// Calls [`event::read`] internally. If your application already owns the
    /// event loop and reads events itself, call [`App::handle_key`] or
    /// [`App::handle_raw_event`] instead.
    ///
    /// Returns `true` when the event loop should exit (user confirmed a
    /// selection or dismissed the explorer).
    pub fn handle_event(&mut self) -> io::Result<bool> {
        self.handle_raw_event(event::read()?)
    }
}
