//! [`FileExplorer`] state machine, [`FileExplorerBuilder`], and filesystem helpers.
//!
//! ## Convenience methods
//!
//! Beyond [`FileExplorer::handle_key`] and [`FileExplorer::reload`], several
//! small helpers make common patterns more ergonomic:
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, SortMode};
//!
//! let mut explorer = FileExplorer::builder(std::env::current_dir().unwrap())
//!     .allow_extension("rs")
//!     .sort_mode(SortMode::SizeDesc)
//!     .build();
//!
//! // Inspect state without touching the raw fields
//! println!("entries : {}", explorer.entry_count());
//! println!("at root : {}", explorer.is_at_root());
//! println!("status  : {}", explorer.status());
//! println!("sort    : {}", explorer.sort_mode().label());
//! println!("search  : {}", explorer.search_query());
//!
//! // Mutate configuration — both calls automatically reload the listing
//! explorer.set_show_hidden(true);
//! explorer.set_extension_filter(["rs", "toml"]);
//! explorer.set_sort_mode(SortMode::Extension);
//!
//! // Navigate accepts anything path-like
//! explorer.navigate_to("/tmp");
//! ```

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::types::{ExplorerOutcome, FsEntry, SortMode};

// ── FileExplorer ──────────────────────────────────────────────────────────────

/// State for the file-explorer widget.
///
/// Keep one instance in your application state and pass a mutable reference
/// to [`crate::render`] and [`FileExplorer::handle_key`] on every frame /
/// key event.
///
/// # Example
///
/// ```no_run
/// use tui_file_explorer::{FileExplorer, ExplorerOutcome};
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
///
/// let mut explorer = FileExplorer::new(
///     std::env::current_dir().unwrap(),
///     vec!["iso".into(), "img".into()],
/// );
///
/// # let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
/// match explorer.handle_key(key) {
///     ExplorerOutcome::Selected(path) => println!("chosen: {}", path.display()),
///     ExplorerOutcome::Dismissed      => println!("closed"),
///     _                               => {}
/// }
/// ```
#[derive(Debug)]
pub struct FileExplorer {
    /// The directory currently being browsed.
    pub current_dir: PathBuf,
    /// The name of the currently active theme (used in the header display).
    pub theme_name: String,
    /// The label of the currently configured editor (used in the header display).
    pub editor_name: String,
    /// Sorted, search-filtered list of visible entries (dirs first, then files).
    pub entries: Vec<FsEntry>,
    /// Index of the highlighted entry.
    pub cursor: usize,
    /// Index of the first visible entry (for scrolling).
    pub(crate) scroll_offset: usize,
    /// Only files whose extension is in this list are selectable.
    /// Directories are always shown and always navigable.
    /// An empty `Vec` means *all* files are selectable.
    pub extension_filter: Vec<String>,
    /// Whether to show dotfiles / hidden entries.
    pub show_hidden: bool,
    /// Human-readable status message (shown in the footer).
    pub(crate) status: String,
    /// Current sort order for directory entries.
    pub sort_mode: SortMode,
    /// Current incremental-search query (empty = no search active).
    pub search_query: String,
    /// Whether the explorer is currently capturing keystrokes for search input.
    pub search_active: bool,
    /// Paths that have been space-marked for a multi-item operation.
    pub marked: HashSet<PathBuf>,
    /// Whether the explorer is currently capturing keystrokes for a new folder name.
    pub mkdir_active: bool,
    /// The folder name being typed when `mkdir_active` is true.
    pub mkdir_input: String,
    /// Whether the explorer is currently capturing keystrokes for a new file name.
    pub touch_active: bool,
    /// The file name being typed when `touch_active` is true.
    pub touch_input: String,
    /// Whether the explorer is currently capturing keystrokes for a rename operation.
    pub rename_active: bool,
    /// The new name being typed when `rename_active` is true.
    pub rename_input: String,
}

// ── handle_input_mode! ────────────────────────────────────────────────────────
//
// De-duplicates the character-input boilerplate shared by rename_active,
// touch_active, and mkdir_active.
//
// Parameters
// ----------
// $self     – the `&mut self` receiver (ident)
// $key      – the `KeyEvent` local (ident, taken by value in handle_key)
// $active   – the boolean field name (e.g. `rename_active`)
// $input    – the String field name  (e.g. `rename_input`)
// $on_enter – an expression that is spliced in as the `KeyCode::Enter` arm
//             body.  It must arrange for `$active` to be set to false,
//             `$input` to be cleared, and for the function to return.
//
// The macro wraps the whole match in `if $self.$active { … }` so execution
// falls through when the mode is inactive.

macro_rules! handle_input_mode {
    ($self:ident, $key:ident, $active:ident, $input:ident, $on_enter:expr) => {
        if $self.$active {
            match $key.code {
                // Printable character (no modifiers, or Shift only) → append.
                KeyCode::Char(c)
                    if $key.modifiers.is_empty()
                        || $key.modifiers == crossterm::event::KeyModifiers::SHIFT =>
                {
                    $self.$input.push(c);
                    return ExplorerOutcome::Pending;
                }
                // Backspace → pop last char.
                KeyCode::Backspace => {
                    $self.$input.pop();
                    return ExplorerOutcome::Pending;
                }
                // Enter → caller-supplied logic.
                KeyCode::Enter => $on_enter,
                // Esc → cancel without committing.
                KeyCode::Esc => {
                    $self.$active = false;
                    $self.$input.clear();
                    return ExplorerOutcome::Pending;
                }
                // Any other key → stay in mode, consume the event.
                _ => return ExplorerOutcome::Pending,
            }
        }
    };
}

impl FileExplorer {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a new explorer starting at `initial_dir`.
    ///
    /// `extension_filter` is a list of lower-case extensions *without* the
    /// leading dot (e.g. `vec!["iso".into(), "img".into()]`).
    /// Pass an empty `Vec` to allow all files.
    ///
    /// For more configuration options use [`FileExplorer::builder`] instead.
    pub fn new(initial_dir: PathBuf, extension_filter: Vec<String>) -> Self {
        let mut explorer = Self {
            current_dir: initial_dir,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            extension_filter,
            show_hidden: false,
            status: String::new(),
            sort_mode: SortMode::default(),
            search_query: String::new(),
            search_active: false,
            marked: HashSet::new(),
            mkdir_active: false,
            mkdir_input: String::new(),
            touch_active: false,
            touch_input: String::new(),
            rename_active: false,
            rename_input: String::new(),
            theme_name: String::new(),
            editor_name: String::new(),
        };
        explorer.reload();
        explorer
    }

    /// Return a [`FileExplorerBuilder`] for constructing an explorer with
    /// fine-grained configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tui_file_explorer::{FileExplorer, SortMode};
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["rs".into(), "toml".into()])
    ///     .show_hidden(true)
    ///     .sort_mode(SortMode::SizeDesc)
    ///     .build();
    /// ```
    pub fn builder(initial_dir: PathBuf) -> FileExplorerBuilder {
        FileExplorerBuilder::new(initial_dir)
    }

    /// Navigate to `path`, resetting cursor, scroll, and any active search.
    ///
    /// Accepts anything that converts into a [`PathBuf`] — a [`PathBuf`],
    /// `&Path`, `&str`, or `String` all work.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
    /// explorer.navigate_to("/tmp");
    /// explorer.navigate_to(std::path::Path::new("/home"));
    /// ```
    pub fn navigate_to(&mut self, path: impl Into<PathBuf>) {
        self.current_dir = path.into();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.reload();
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    /// Process a single keyboard event and return the [`ExplorerOutcome`].
    ///
    /// Call this from your application's key-handling function and act on
    /// [`ExplorerOutcome::Selected`] / [`ExplorerOutcome::Dismissed`].
    /// Return the set of currently marked paths (for multi-item operations).
    pub fn marked_paths(&self) -> &HashSet<PathBuf> {
        &self.marked
    }

    /// Toggle the space-mark on the currently highlighted entry and move
    /// the cursor down by one.
    pub fn toggle_mark(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor) {
            let path = entry.path.clone();
            if self.marked.contains(&path) {
                self.marked.remove(&path);
            } else {
                self.marked.insert(path);
            }
        }
        self.move_down();
    }

    /// Clear all space-marks (called after a multi-delete or on navigation).
    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

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
                for _ in 0..10 {
                    self.move_up();
                }
                ExplorerOutcome::Pending
            }

            // ── Page down ────────────────────────────────────────────────────
            KeyCode::PageDown => {
                for _ in 0..10 {
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

            // ── Toggle hidden files ───────────────────────────────────────────
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                let was = self.cursor;
                self.reload();
                self.cursor = was.min(self.entries.len().saturating_sub(1));
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

    // ── Queries ───────────────────────────────────────────────────────────────

    /// The currently highlighted [`FsEntry`], or `None` if the list is empty.
    pub fn current_entry(&self) -> Option<&FsEntry> {
        self.entries.get(self.cursor)
    }

    /// Whether the explorer is in mkdir (new-folder input) mode.
    pub fn is_mkdir_active(&self) -> bool {
        self.mkdir_active
    }

    /// The folder name being typed when mkdir mode is active.
    pub fn mkdir_input(&self) -> &str {
        &self.mkdir_input
    }

    /// Whether the explorer is in touch (new-file input) mode.
    pub fn is_touch_active(&self) -> bool {
        self.touch_active
    }

    /// The file name being typed when touch mode is active.
    pub fn touch_input(&self) -> &str {
        &self.touch_input
    }

    /// Whether the explorer is in rename (entry-rename input) mode.
    pub fn is_rename_active(&self) -> bool {
        self.rename_active
    }

    /// The new name being typed when rename mode is active.
    pub fn rename_input(&self) -> &str {
        &self.rename_input
    }

    // ── Inspectors ────────────────────────────────────────────────────────────

    /// Returns `true` when the explorer is at the filesystem root and cannot
    /// ascend any further.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let mut explorer = FileExplorer::new(std::path::PathBuf::from("/"), vec![]);
    /// assert!(explorer.is_at_root());
    /// ```
    pub fn is_at_root(&self) -> bool {
        self.current_dir.parent().is_none()
    }

    /// Returns `true` when the current directory contains no visible entries.
    ///
    /// This reflects the *filtered, visible* set — hidden files are excluded
    /// unless `show_hidden` is `true`, and an active search query narrows
    /// the set further.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of visible entries in the current directory.
    ///
    /// Equivalent to `explorer.entries.len()` but reads more naturally in
    /// condition checks.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// The current human-readable status message.
    ///
    /// The status is set by the widget when an error occurs (e.g. attempting
    /// to select a file that does not match the extension filter) and is
    /// cleared on the next successful navigation.  Returns an empty string
    /// when there is nothing to report.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The current sort mode.
    ///
    /// ```
    /// use tui_file_explorer::{FileExplorer, SortMode};
    ///
    /// let explorer = FileExplorer::new(std::path::PathBuf::from("/tmp"), vec![]);
    /// assert_eq!(explorer.sort_mode(), SortMode::Name);
    /// ```
    pub fn sort_mode(&self) -> SortMode {
        self.sort_mode
    }

    /// The current incremental-search query string.
    ///
    /// Returns an empty string when no search is active.
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Returns `true` when the explorer is actively capturing keystrokes for
    /// incremental search input.
    pub fn is_searching(&self) -> bool {
        self.search_active
    }

    // ── Mutating setters ──────────────────────────────────────────────────────

    /// Set whether hidden (dot-file) entries are visible and reload the
    /// directory listing immediately.
    ///
    /// The user can also toggle this at runtime with the `.` key.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
    /// explorer.set_show_hidden(true);
    /// assert!(explorer.show_hidden);
    /// ```
    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
        self.reload();
    }

    /// Replace the extension filter and reload the directory listing
    /// immediately.
    ///
    /// Accepts any iterable of values that convert to [`String`] — plain
    /// `&str` slices, `String` values, and arrays all work:
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
    ///
    /// // Array of &str — no .into() needed
    /// explorer.set_extension_filter(["rs", "toml"]);
    ///
    /// // Vec<String>
    /// explorer.set_extension_filter(vec!["iso".to_string(), "img".to_string()]);
    ///
    /// // Empty — allow all files
    /// explorer.set_extension_filter([] as [&str; 0]);
    /// ```
    pub fn set_extension_filter<I, S>(&mut self, filter: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extension_filter = filter.into_iter().map(Into::into).collect();
        self.reload();
    }

    /// Change the sort mode and reload the directory listing immediately.
    ///
    /// The user can also cycle through modes at runtime with the `s` key.
    ///
    /// ```no_run
    /// use tui_file_explorer::{FileExplorer, SortMode};
    ///
    /// let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
    /// explorer.set_sort_mode(SortMode::SizeDesc);
    /// assert_eq!(explorer.sort_mode(), SortMode::SizeDesc);
    /// ```
    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.reload();
    }

    // ── Internal navigation helpers ───────────────────────────────────────────

    fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        // If entries shrank (e.g. external deletion) clamp to valid range.
        self.clamp_cursor();
    }

    fn move_down(&mut self) {
        let last = self.entries.len().saturating_sub(1);
        if !self.entries.is_empty() && self.cursor < last {
            self.cursor += 1;
        }
        self.clamp_cursor();
    }

    /// Clamp `cursor` and `scroll_offset` so they never exceed the current
    /// entries length.  Safe to call at any time — a no-op when everything is
    /// already in range.
    fn clamp_cursor(&mut self) {
        let max = self.entries.len().saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }
        if self.scroll_offset > self.cursor {
            self.scroll_offset = self.cursor;
        }
    }

    fn ascend(&mut self) {
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

    // ── Directory loading ─────────────────────────────────────────────────────

    /// Re-read the current directory from the filesystem.
    ///
    /// Called automatically after every navigation action or configuration
    /// change.  Callers can invoke it manually after external filesystem
    /// mutations (e.g. a file was created or deleted in the watched directory).
    pub fn reload(&mut self) {
        self.status.clear();
        self.entries = load_entries(
            &self.current_dir,
            self.show_hidden,
            &self.extension_filter,
            self.sort_mode,
            &self.search_query,
        );
        // After every reload the entry count may have shrunk (filter change,
        // external deletion, empty directory).  Clamp so cursor and
        // scroll_offset never point past the end of the new list.
        self.clamp_cursor();
    }
}

// ── FileExplorerBuilder ───────────────────────────────────────────────────────

/// Builder for [`FileExplorer`].
///
/// Obtain one via [`FileExplorer::builder`].
///
/// # Example
///
/// ```no_run
/// use tui_file_explorer::{FileExplorer, SortMode};
///
/// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
///     .allow_extension("iso")
///     .allow_extension("img")
///     .show_hidden(false)
///     .sort_mode(SortMode::SizeDesc)
///     .build();
/// ```
pub struct FileExplorerBuilder {
    initial_dir: PathBuf,
    extension_filter: Vec<String>,
    show_hidden: bool,
    sort_mode: SortMode,
}

impl FileExplorerBuilder {
    /// Create a builder rooted at `initial_dir`.
    pub fn new(initial_dir: PathBuf) -> Self {
        Self {
            initial_dir,
            extension_filter: Vec::new(),
            show_hidden: false,
            sort_mode: SortMode::default(),
        }
    }

    /// Set the full extension filter list at once.
    ///
    /// Replaces any extensions added with [`allow_extension`](Self::allow_extension).
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["iso".into(), "img".into()])
    ///     .build();
    /// ```
    pub fn extension_filter(mut self, filter: Vec<String>) -> Self {
        self.extension_filter = filter;
        self
    }

    /// Append a single allowed extension.
    ///
    /// Call multiple times to build up the filter:
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .allow_extension("iso")
    ///     .allow_extension("img")
    ///     .build();
    /// ```
    pub fn allow_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension_filter.push(ext.into());
        self
    }

    /// Set whether hidden (dot-file) entries are shown on startup.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .show_hidden(true)
    ///     .build();
    /// ```
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set the initial sort mode.
    ///
    /// ```no_run
    /// use tui_file_explorer::{FileExplorer, SortMode};
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .sort_mode(SortMode::SizeDesc)
    ///     .build();
    /// ```
    pub fn sort_mode(mut self, mode: SortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Consume the builder and return a fully initialised [`FileExplorer`].
    pub fn build(self) -> FileExplorer {
        let mut explorer = FileExplorer {
            current_dir: self.initial_dir,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            extension_filter: self.extension_filter,
            show_hidden: self.show_hidden,
            status: String::new(),
            sort_mode: self.sort_mode,
            search_query: String::new(),
            search_active: false,
            marked: HashSet::new(),
            mkdir_active: false,
            mkdir_input: String::new(),
            touch_active: false,
            touch_input: String::new(),
            rename_active: false,
            rename_input: String::new(),
            theme_name: String::new(),
            editor_name: String::new(),
        };
        explorer.reload();
        explorer
    }
}

// ── Directory loader ──────────────────────────────────────────────────────────

/// Read `dir`, apply all active filters, sort entries, and return the result.
///
/// * Hidden entries are excluded unless `show_hidden` is `true`.
/// * When `ext_filter` is non-empty only files whose extension is in the list
///   are included (directories are always included).
/// * When `search_query` is non-empty only entries whose name contains the
///   query (case-insensitive) are included.
/// * Entries are sorted according to `sort_mode`; directories are always
///   placed before files regardless of the sort mode.
pub(crate) fn load_entries(
    dir: &Path,
    show_hidden: bool,
    ext_filter: &[String],
    sort_mode: SortMode,
    search_query: &str,
) -> Vec<FsEntry> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut dirs: Vec<FsEntry> = Vec::new();
    let mut files: Vec<FsEntry> = Vec::new();

    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };

        // Extension filter — applied to files only; directories always pass.
        if !is_dir && !ext_filter.is_empty() {
            let matches = ext_filter
                .iter()
                .any(|f| f.eq_ignore_ascii_case(&extension));
            if !matches {
                continue;
            }
        }

        // Search query filter — applied to both files and directories.
        if !search_query.is_empty() {
            let q = search_query.to_lowercase();
            if !name.to_lowercase().contains(&q) {
                continue;
            }
        }

        let size = if is_dir {
            None
        } else {
            entry.metadata().ok().map(|m| m.len())
        };

        let fs_entry = FsEntry {
            name,
            path,
            is_dir,
            size,
            extension,
        };

        if is_dir {
            dirs.push(fs_entry);
        } else {
            files.push(fs_entry);
        }
    }

    // Sort each group according to the active mode.
    // Directories always sort alphabetically among themselves.
    dirs.sort_by_key(|a| a.name.to_lowercase());

    match sort_mode {
        SortMode::Name => {
            files.sort_by_key(|a| a.name.to_lowercase());
        }
        SortMode::SizeDesc => {
            // Largest first; treat missing size as 0.
            files.sort_by_key(|b| std::cmp::Reverse(b.size.unwrap_or(0)));
        }
        SortMode::Extension => {
            // By extension first, then by name within each extension group.
            files.sort_by(|a, b| {
                a.extension
                    .cmp(&b.extension)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }
    }

    // Dirs first, then sorted files.
    dirs.extend(files);
    dirs
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Choose a Unicode icon for a directory entry.
///
/// Exposed as a public helper so that custom renderers can reuse the same
/// icon mapping without duplicating the match table.
pub fn entry_icon(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        return "📁";
    }
    match entry.extension.as_str() {
        // Disk images
        "iso" | "dmg" => "💿",
        "img" => "🖼 ",
        // Archives
        "zip" | "gz" | "xz" | "zst" | "bz2" | "tar" | "7z" | "rar" | "tgz" | "tbz2" => "📦",
        // Documents
        "pdf" => "📕",
        "txt" | "log" | "rst" => "📄",
        "md" | "mdx" | "markdown" => "📝",
        // Config / data
        "toml" | "yaml" | "yml" | "json" | "xml" | "ini" | "cfg" | "conf" | "env" => "⚙ ",
        "lock" => "🔒",
        // Source — languages
        "rs" => "🦀",
        "py" | "pyw" => "🐍",
        "js" | "mjs" | "cjs" => "📜",
        "ts" | "mts" | "cts" => "📜",
        "jsx" | "tsx" => "📜",
        "go" => "📜",
        "c" | "h" => "📜",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "📜",
        "java" | "kt" | "kts" => "📜",
        "rb" | "erb" => "📜",
        "php" => "📜",
        "swift" => "📜",
        "cs" => "📜",
        "lua" => "📜",
        "zig" => "📜",
        "ex" | "exs" => "📜",
        "hs" | "lhs" => "📜",
        "ml" | "mli" => "📜",
        // Shell scripts
        "sh" | "bash" | "zsh" | "fish" | "nu" => "📜",
        "bat" | "cmd" | "ps1" => "📜",
        // Web
        "html" | "htm" | "xhtml" => "🌐",
        "css" | "scss" | "sass" | "less" => "🎨",
        "svg" => "🎨",
        // Images (raster)
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tiff" | "tif" | "avif"
        | "heic" | "heif" => "🖼 ",
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" => "🎬",
        // Audio
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "opus" | "wma" => "🎵",
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" => "🔤",
        // Executables / binaries
        "exe" | "msi" | "deb" | "rpm" | "appimage" | "apk" => "⚙ ",
        _ => "📄",
    }
}

/// Format a byte count as a human-readable size string.
///
/// Exposed as a public helper so that custom renderers can reuse the same
/// formatting logic without reimplementing it.
///
/// ```
/// use tui_file_explorer::fmt_size;
///
/// assert_eq!(fmt_size(512),           "512 B");
/// assert_eq!(fmt_size(1_536),         "1.5 KB");
/// assert_eq!(fmt_size(2_097_152),     "2.0 MB");
/// assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
/// ```
pub fn fmt_size(bytes: u64) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = 1_024 * KB;
    const GB: u64 = 1_024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use std::fs;
    use tempfile::{tempdir, TempDir};

    // ── Fixtures ──────────────────────────────────────────────────────────────

    fn temp_dir_with_files() -> TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("ubuntu.iso"), b"fake iso content").unwrap();
        fs::write(dir.path().join("debian.img"), b"fake img content").unwrap();
        fs::write(dir.path().join("readme.txt"), b"some text").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        dir
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // ── Existing tests ────────────────────────────────────────────────────────

    #[test]
    fn new_loads_entries() {
        let tmp = temp_dir_with_files();
        let explorer =
            FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into(), "img".into()]);
        assert!(explorer
            .entries
            .iter()
            .any(|e| e.name == "subdir" && e.is_dir));
        assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
        assert!(explorer.entries.iter().any(|e| e.name == "debian.img"));
        // .txt excluded by filter
        assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
    }

    #[test]
    fn no_filter_shows_all_files() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(explorer.entries.iter().any(|e| e.name == "readme.txt"));
    }

    #[test]
    fn dirs_listed_before_files() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let first_file_idx = explorer
            .entries
            .iter()
            .position(|e| !e.is_dir)
            .unwrap_or(usize::MAX);
        let last_dir_idx = explorer.entries.iter().rposition(|e| e.is_dir).unwrap_or(0);
        assert!(
            last_dir_idx < first_file_idx,
            "all dirs must appear before any file"
        );
    }

    #[test]
    fn move_down_increments_cursor() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.move_down();
        assert_eq!(explorer.cursor, 1);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.move_up();
        assert_eq!(explorer.cursor, 0);
    }

    #[test]
    fn move_down_clamps_at_last() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let last = explorer.entries.len() - 1;
        explorer.cursor = last;
        explorer.move_down();
        assert_eq!(explorer.cursor, last);
    }

    #[test]
    fn handle_key_down_moves_cursor() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let before = explorer.cursor;
        explorer.handle_key(key(KeyCode::Down));
        assert_eq!(explorer.cursor, before + 1);
    }

    #[test]
    fn handle_key_esc_dismisses() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(
            explorer.handle_key(key(KeyCode::Esc)),
            ExplorerOutcome::Dismissed
        );
    }

    #[test]
    fn handle_key_enter_on_dir_descends() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Place cursor on the directory (dirs sort first).
        let dir_idx = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("no dir in fixture");
        explorer.cursor = dir_idx;
        let expected_path = explorer.entries[dir_idx].path.clone();
        let outcome = explorer.handle_key(key(KeyCode::Enter));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.current_dir, expected_path);
    }

    #[test]
    fn handle_key_enter_on_valid_file_selects() {
        let tmp = temp_dir_with_files();
        let mut explorer =
            FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into(), "img".into()]);
        let file_idx = explorer
            .entries
            .iter()
            .position(|e| !e.is_dir)
            .expect("no file in fixture");
        explorer.cursor = file_idx;
        let expected = explorer.entries[file_idx].path.clone();
        let outcome = explorer.handle_key(key(KeyCode::Enter));
        assert_eq!(outcome, ExplorerOutcome::Selected(expected));
    }

    #[test]
    fn handle_key_backspace_ascends() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);
        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.current_dir, tmp.path());
    }

    #[test]
    fn toggle_hidden_changes_visibility() {
        let tmp = temp_dir_with_files();
        fs::write(tmp.path().join(".hidden_file"), b"").unwrap();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.entries.iter().any(|e| e.name == ".hidden_file"));
        explorer.set_show_hidden(true);
        assert!(explorer.entries.iter().any(|e| e.name == ".hidden_file"));
    }

    #[test]
    fn fmt_size_formats_bytes() {
        assert_eq!(fmt_size(512), "512 B");
        assert_eq!(fmt_size(1_536), "1.5 KB");
        assert_eq!(fmt_size(2_097_152), "2.0 MB");
        assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn extension_filter_only_shows_matching_files() {
        // The real selectability contract lives in load_entries: only files
        // whose extension matches the filter appear in entries at all.
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into()]);

        // Matching file is present.
        assert!(
            explorer.entries.iter().any(|e| e.name == "ubuntu.iso"),
            "iso file should appear in entries"
        );
        // Non-matching file is absent.
        assert!(
            !explorer.entries.iter().any(|e| e.name == "debian.img"),
            "img file should be excluded by filter"
        );
        // Directories are always present regardless of the filter.
        assert!(
            explorer.entries.iter().any(|e| e.is_dir),
            "directories should always be visible"
        );
        // Every visible non-directory entry has the expected extension.
        assert!(
            explorer
                .entries
                .iter()
                .filter(|e| !e.is_dir)
                .all(|e| e.extension == "iso"),
            "all visible files must match the active filter"
        );
    }

    #[test]
    fn navigate_to_resets_cursor_and_scroll() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 2;
        explorer.scroll_offset = 1;
        explorer.navigate_to(tmp.path().to_path_buf());
        assert_eq!(explorer.cursor, 0);
        assert_eq!(explorer.scroll_offset, 0);
    }

    #[test]
    fn current_entry_returns_highlighted() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        let entry = explorer.current_entry().expect("should have entry");
        assert_eq!(entry, explorer.entries.first().unwrap());
    }

    #[test]
    fn unrecognised_key_returns_unhandled() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(
            explorer.handle_key(key(KeyCode::F(5))),
            ExplorerOutcome::Unhandled
        );
    }

    // ── Search tests ──────────────────────────────────────────────────────────

    #[test]
    fn slash_activates_search_mode() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.search_active);
        explorer.handle_key(key(KeyCode::Char('/')));
        assert!(explorer.search_active);
        assert_eq!(explorer.search_query(), "");
    }

    #[test]
    fn search_active_chars_append_to_query() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        explorer.handle_key(key(KeyCode::Char('u')));
        explorer.handle_key(key(KeyCode::Char('b')));
        explorer.handle_key(key(KeyCode::Char('u')));
        assert_eq!(explorer.search_query(), "ubu");
        assert!(explorer.search_active);
    }

    #[test]
    fn search_filters_entries_by_name() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Activate search and type a query that matches only ubuntu.iso
        explorer.handle_key(key(KeyCode::Char('/')));
        for c in "ubu".chars() {
            explorer.handle_key(key(KeyCode::Char(c)));
        }
        // Only ubuntu.iso (and nothing else) should be visible.
        assert_eq!(explorer.entries.len(), 1);
        assert_eq!(explorer.entries[0].name, "ubuntu.iso");
    }

    #[test]
    fn search_backspace_pops_last_char() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        explorer.handle_key(key(KeyCode::Char('u')));
        explorer.handle_key(key(KeyCode::Char('b')));
        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.search_query(), "u");
        assert!(explorer.search_active);
    }

    #[test]
    fn search_backspace_on_empty_deactivates() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        assert!(explorer.search_active);
        // Backspace on an empty query deactivates search.
        explorer.handle_key(key(KeyCode::Backspace));
        assert!(!explorer.search_active);
        assert_eq!(explorer.search_query(), "");
    }

    #[test]
    fn search_esc_clears_and_deactivates_returns_pending() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        explorer.handle_key(key(KeyCode::Char('u')));
        let outcome = explorer.handle_key(key(KeyCode::Esc));
        assert_eq!(
            outcome,
            ExplorerOutcome::Pending,
            "Esc should clear search, not dismiss"
        );
        assert!(!explorer.search_active);
        assert_eq!(explorer.search_query(), "");
    }

    #[test]
    fn esc_when_not_searching_dismisses() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.search_active);
        assert_eq!(
            explorer.handle_key(key(KeyCode::Esc)),
            ExplorerOutcome::Dismissed
        );
    }

    #[test]
    fn search_clears_on_directory_descend() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.search_active = true;
        explorer.search_query = "sub".into();
        // Navigate into subdir
        explorer.cursor = explorer.entries.iter().position(|e| e.is_dir).unwrap();
        explorer.handle_key(key(KeyCode::Enter));
        assert!(!explorer.search_active);
        assert_eq!(explorer.search_query(), "");
    }

    #[test]
    fn search_clears_on_ascend() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);

        // Manually inject search state (simulates user having typed a query
        // while already inside subdir, then pressing the ascend key).
        // When search_active is true, ALL KeyCode::Char(_) keys are consumed
        // by the search interception block — they append to the query rather
        // than triggering navigation.  Backspace pops the query.  The only
        // way to ascend while search is active is via the non-char ascend
        // keys, but those aren't exposed through handle_key without going
        // through the search block first.  Call ascend() directly: this is
        // the correct unit test for the ascend() logic itself, independent
        // of key dispatch.
        explorer.search_active = true;
        explorer.search_query = "foo".into();

        // Call ascend() directly — ascend() clears search state unconditionally.
        explorer.ascend();

        assert!(
            !explorer.search_active,
            "search must be deactivated after ascend"
        );
        assert_eq!(
            explorer.search_query(),
            "",
            "query must be cleared after ascend"
        );
        assert_eq!(
            explorer.current_dir,
            tmp.path(),
            "must have ascended to parent"
        );
    }

    #[test]
    fn backspace_in_search_pops_char_not_ascend() {
        // Verify Backspace is consumed by search interception (pops the query)
        // and does NOT trigger ascend when search is active with a non-empty query.
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir.clone(), vec![]);
        explorer.search_active = true;
        explorer.search_query = "foo".into();

        explorer.handle_key(key(KeyCode::Backspace)); // should pop 'o', not ascend

        assert_eq!(explorer.current_dir, subdir, "must NOT have ascended");
        assert_eq!(
            explorer.search_query(),
            "fo",
            "Backspace should pop last char"
        );
        assert!(explorer.search_active, "search must still be active");
    }

    // ── Sort tests ────────────────────────────────────────────────────────────

    #[test]
    fn default_sort_mode_is_name() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(explorer.sort_mode(), SortMode::Name);
    }

    #[test]
    fn sort_mode_cycles_on_s_key() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(explorer.sort_mode(), SortMode::Name);
        explorer.handle_key(key(KeyCode::Char('s')));
        assert_eq!(explorer.sort_mode(), SortMode::SizeDesc);
        explorer.handle_key(key(KeyCode::Char('s')));
        assert_eq!(explorer.sort_mode(), SortMode::Extension);
        explorer.handle_key(key(KeyCode::Char('s')));
        assert_eq!(explorer.sort_mode(), SortMode::Name);
    }

    #[test]
    fn sort_size_desc_orders_largest_first() {
        let tmp = tempfile::tempdir().expect("temp dir");
        // Create files with clearly different sizes.
        fs::write(tmp.path().join("small.txt"), vec![0u8; 10]).unwrap();
        fs::write(tmp.path().join("large.txt"), vec![0u8; 10_000]).unwrap();
        fs::write(tmp.path().join("medium.txt"), vec![0u8; 1_000]).unwrap();

        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.set_sort_mode(SortMode::SizeDesc);

        let sizes: Vec<u64> = explorer.entries.iter().filter_map(|e| e.size).collect();
        let mut sorted_desc = sizes.clone();
        sorted_desc.sort_by(|a, b| b.cmp(a));
        assert_eq!(sizes, sorted_desc, "files should be sorted largest-first");
    }

    #[test]
    fn sort_extension_groups_by_ext() {
        let tmp = tempfile::tempdir().expect("temp dir");
        fs::write(tmp.path().join("b.toml"), b"").unwrap();
        fs::write(tmp.path().join("a.rs"), b"").unwrap();
        fs::write(tmp.path().join("c.toml"), b"").unwrap();
        fs::write(tmp.path().join("z.rs"), b"").unwrap();

        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.set_sort_mode(SortMode::Extension);

        let exts: Vec<&str> = explorer
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.extension.as_str())
            .collect();

        // All rs entries should appear before toml entries (r < t).
        let rs_last = exts.iter().rposition(|&e| e == "rs").unwrap_or(0);
        let toml_first = exts.iter().position(|&e| e == "toml").unwrap_or(usize::MAX);
        assert!(rs_last < toml_first, "rs group must precede toml group");
    }

    #[test]
    fn builder_sort_mode_applied() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::builder(tmp.path().to_path_buf())
            .sort_mode(SortMode::SizeDesc)
            .build();
        assert_eq!(explorer.sort_mode(), SortMode::SizeDesc);
    }

    #[test]
    fn set_sort_mode_reloads() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.set_sort_mode(SortMode::Extension);
        assert_eq!(explorer.sort_mode(), SortMode::Extension);
        // Entries should still be present after the reload triggered by set_sort_mode.
        assert!(!explorer.entries.is_empty());
    }

    // ── Vim key tests ─────────────────────────────────────────────────────────

    #[test]
    fn j_key_moves_cursor_down() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let before = explorer.cursor;
        explorer.handle_key(key(KeyCode::Char('j')));
        assert_eq!(explorer.cursor, before + 1);
    }

    #[test]
    fn k_key_moves_cursor_up() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 2;
        explorer.handle_key(key(KeyCode::Char('k')));
        assert_eq!(explorer.cursor, 1);
    }

    #[test]
    fn h_key_ascends_to_parent() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);
        explorer.handle_key(key(KeyCode::Char('h')));
        assert_eq!(explorer.current_dir, tmp.path());
    }

    #[test]
    fn l_key_descends_into_dir() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let dir_idx = explorer.entries.iter().position(|e| e.is_dir).unwrap();
        explorer.cursor = dir_idx;
        let expected = explorer.entries[dir_idx].path.clone();
        let outcome = explorer.handle_key(key(KeyCode::Char('l')));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.current_dir, expected);
    }

    #[test]
    fn right_arrow_descends_into_dir() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let dir_idx = explorer.entries.iter().position(|e| e.is_dir).unwrap();
        explorer.cursor = dir_idx;
        let expected = explorer.entries[dir_idx].path.clone();
        let outcome = explorer.handle_key(key(KeyCode::Right));
        assert_eq!(
            outcome,
            ExplorerOutcome::Pending,
            "Right arrow should descend into directory"
        );
        assert_eq!(
            explorer.current_dir, expected,
            "Right arrow should change into the selected directory"
        );
    }

    #[test]
    fn right_arrow_on_file_moves_down_not_exits() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Pick the first file entry that is not the last entry so cursor can advance.
        let file_idx = explorer.entries.iter().position(|e| !e.is_dir).unwrap();
        // Ensure there is an entry after it to move to.
        assert!(
            file_idx + 1 < explorer.entries.len(),
            "fixture must have an entry after the first file"
        );
        explorer.cursor = file_idx;
        let original_dir = explorer.current_dir.clone();
        let outcome = explorer.handle_key(key(KeyCode::Right));
        assert_eq!(
            outcome,
            ExplorerOutcome::Pending,
            "Right arrow on a file must never exit (always Pending)"
        );
        assert_eq!(
            explorer.current_dir, original_dir,
            "Right arrow on a file must not change directory"
        );
        assert_eq!(
            explorer.cursor,
            file_idx + 1,
            "Right arrow on a file must advance the cursor by one"
        );
    }

    #[test]
    fn right_arrow_on_file_at_last_entry_does_not_overflow() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let last = explorer.entries.len() - 1;
        // Force cursor onto the last entry (guaranteed to exist in the fixture).
        explorer.cursor = last;
        explorer.handle_key(key(KeyCode::Right));
        assert_eq!(
            explorer.cursor, last,
            "Right arrow at the last entry must not overflow past it"
        );
    }

    #[test]
    fn enter_on_file_still_confirms_and_exits() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let file_idx = explorer.entries.iter().position(|e| !e.is_dir).unwrap();
        explorer.cursor = file_idx;
        let expected = explorer.entries[file_idx].path.clone();
        let outcome = explorer.handle_key(key(KeyCode::Enter));
        assert_eq!(
            outcome,
            ExplorerOutcome::Selected(expected),
            "Enter on a file should confirm (select) it and exit"
        );
    }

    #[test]
    fn left_arrow_ascends_to_parent() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);
        let outcome = explorer.handle_key(key(KeyCode::Left));
        assert_eq!(
            outcome,
            ExplorerOutcome::Pending,
            "Left arrow should return Pending after ascending"
        );
        assert_eq!(
            explorer.current_dir,
            tmp.path(),
            "Left arrow should ascend to the parent directory"
        );
    }

    #[test]
    fn right_arrow_clears_search_on_dir_descend() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Activate search so we can verify navigate() clears it.
        explorer.search_active = true;
        explorer.search_query = "sub".to_string();
        explorer.reload();
        // The search should have narrowed entries to the subdir.
        let dir_idx = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("fixture subdir must match 'sub'");
        explorer.cursor = dir_idx;
        explorer.handle_key(key(KeyCode::Right));
        assert!(
            !explorer.search_active,
            "navigate() must deactivate search on directory descend"
        );
        assert!(
            explorer.search_query.is_empty(),
            "navigate() must clear search query on directory descend"
        );
    }

    #[test]
    fn right_arrow_clears_marks_on_dir_descend() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let dir_idx = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("fixture has a subdir");
        // Mark an entry before descending.
        explorer.toggle_mark();
        assert!(
            !explorer.marked.is_empty(),
            "should have a mark before descend"
        );
        // Reset cursor back to the directory entry.
        explorer.cursor = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("fixture has a subdir");
        explorer.handle_key(key(KeyCode::Right));
        assert!(
            explorer.marked.is_empty(),
            "navigate() must clear marks on directory descend"
        );
        let _ = dir_idx;
    }

    #[test]
    fn backspace_still_ascends() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);
        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.current_dir, tmp.path());
    }

    #[test]
    fn q_key_dismisses() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(
            explorer.handle_key(key(KeyCode::Char('q'))),
            ExplorerOutcome::Dismissed
        );
    }

    // ── Page / jump key tests ─────────────────────────────────────────────────

    #[test]
    fn page_down_advances_cursor_by_ten() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..15 {
            fs::write(tmp.path().join(format!("file{i:02}.txt")), b"").unwrap();
        }
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        explorer.handle_key(key(KeyCode::PageDown));
        assert_eq!(explorer.cursor, 10);
    }

    #[test]
    fn page_up_retreats_cursor_by_ten() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..15 {
            fs::write(tmp.path().join(format!("file{i:02}.txt")), b"").unwrap();
        }
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 12;
        explorer.handle_key(key(KeyCode::PageUp));
        assert_eq!(explorer.cursor, 2);
    }

    #[test]
    fn home_key_jumps_to_top() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = explorer.entries.len() - 1;
        explorer.handle_key(key(KeyCode::Home));
        assert_eq!(explorer.cursor, 0);
        assert_eq!(explorer.scroll_offset, 0);
    }

    #[test]
    fn g_key_jumps_to_top() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = explorer.entries.len() - 1;
        explorer.handle_key(key(KeyCode::Char('g')));
        assert_eq!(explorer.cursor, 0);
        assert_eq!(explorer.scroll_offset, 0);
    }

    #[test]
    fn end_key_jumps_to_bottom() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        explorer.handle_key(key(KeyCode::End));
        assert_eq!(explorer.cursor, explorer.entries.len() - 1);
    }

    #[test]
    fn capital_g_key_jumps_to_bottom() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        let key_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
        explorer.handle_key(key_g);
        assert_eq!(explorer.cursor, explorer.entries.len() - 1);
    }

    // ── Root / status tests ───────────────────────────────────────────────────

    #[test]
    fn ascend_at_root_sets_status() {
        // Use "/" as a reliable filesystem root on macOS/Linux.
        let root = std::path::PathBuf::from("/");
        let mut explorer = FileExplorer::new(root.clone(), vec![]);
        assert!(explorer.is_at_root());
        // Still at root after attempted ascend.
        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.current_dir, root);
        assert!(
            !explorer.status().is_empty(),
            "status should report already at root"
        );
    }

    #[test]
    fn is_at_root_false_for_subdir() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.is_at_root());
    }

    // ── Accessor tests ────────────────────────────────────────────────────────

    #[test]
    fn is_empty_reflects_visible_entries() {
        let empty_dir = tempfile::tempdir().unwrap();
        let explorer = FileExplorer::new(empty_dir.path().to_path_buf(), vec![]);
        assert!(explorer.is_empty());

        let tmp = temp_dir_with_files();
        let explorer2 = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer2.is_empty());
    }

    #[test]
    fn entry_count_matches_entries_len() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(explorer.entry_count(), explorer.entries.len());
        assert!(explorer.entry_count() > 0);
    }

    #[test]
    fn search_query_empty_when_not_searching() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.is_searching());
        assert_eq!(explorer.search_query(), "");
    }

    // ── Case-insensitivity tests ──────────────────────────────────────────────

    #[test]
    fn search_is_case_insensitive() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Type "UBU" in uppercase — should still match "ubuntu.iso".
        explorer.handle_key(key(KeyCode::Char('/')));
        for c in "UBU".chars() {
            explorer.handle_key(key(KeyCode::Char(c)));
        }
        assert_eq!(explorer.entries.len(), 1);
        assert_eq!(explorer.entries[0].name, "ubuntu.iso");
    }

    #[test]
    fn extension_filter_is_case_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        // File whose on-disk extension is upper-case.
        fs::write(tmp.path().join("disk.ISO"), b"data").unwrap();
        fs::write(tmp.path().join("other.txt"), b"text").unwrap();

        // Filter expressed in lower-case should still match the upper-case ext.
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into()]);
        assert!(
            explorer.entries.iter().any(|e| e.name == "disk.ISO"),
            "upper-case extension should be matched by lower-case filter"
        );
        assert!(
            !explorer.entries.iter().any(|e| e.name == "other.txt"),
            "non-matching extension should be excluded"
        );
    }

    // ── Builder tests ─────────────────────────────────────────────────────────

    #[test]
    fn builder_allow_extension_filters_entries() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::builder(tmp.path().to_path_buf())
            .allow_extension("iso")
            .build();
        assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
        assert!(!explorer.entries.iter().any(|e| e.name == "debian.img"));
        assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
    }

    #[test]
    fn builder_show_hidden_shows_dotfiles() {
        let tmp = temp_dir_with_files();
        fs::write(tmp.path().join(".dotfile"), b"").unwrap();

        let hidden_explorer = FileExplorer::builder(tmp.path().to_path_buf())
            .show_hidden(true)
            .build();
        assert!(hidden_explorer.entries.iter().any(|e| e.name == ".dotfile"));

        let normal_explorer = FileExplorer::builder(tmp.path().to_path_buf())
            .show_hidden(false)
            .build();
        assert!(!normal_explorer.entries.iter().any(|e| e.name == ".dotfile"));
    }

    #[test]
    fn set_extension_filter_updates_entries() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // All files visible with no filter.
        assert!(explorer.entries.iter().any(|e| e.name == "readme.txt"));

        explorer.set_extension_filter(["iso"]);
        assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
        assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
    }

    // ── entry_icon tests ──────────────────────────────────────────────────────

    #[test]
    fn entry_icon_directory() {
        let entry = FsEntry {
            name: "mydir".into(),
            path: std::path::PathBuf::from("/mydir"),
            is_dir: true,
            size: None,
            extension: String::new(),
        };
        assert_eq!(entry_icon(&entry), "📁");
    }

    #[test]
    fn entry_icon_recognises_known_extensions() {
        let make = |name: &str, ext: &str| FsEntry {
            name: name.into(),
            path: std::path::PathBuf::from(name),
            is_dir: false,
            size: Some(0),
            extension: ext.into(),
        };

        assert_eq!(entry_icon(&make("archive.zip", "zip")), "📦");
        assert_eq!(entry_icon(&make("doc.pdf", "pdf")), "📕");
        assert_eq!(entry_icon(&make("notes.md", "md")), "📝");
        assert_eq!(entry_icon(&make("config.toml", "toml")), "⚙ ");
        assert_eq!(entry_icon(&make("main.rs", "rs")), "🦀");
        assert_eq!(entry_icon(&make("script.py", "py")), "🐍");
        assert_eq!(entry_icon(&make("page.html", "html")), "🌐");
        assert_eq!(entry_icon(&make("image.png", "png")), "🖼 ");
        assert_eq!(entry_icon(&make("video.mp4", "mp4")), "🎬");
        assert_eq!(entry_icon(&make("song.mp3", "mp3")), "🎵");
        assert_eq!(entry_icon(&make("unknown.xyz", "xyz")), "📄");
    }

    // ── fmt_size boundary tests ───────────────────────────────────────────────

    #[test]
    fn fmt_size_exact_boundaries() {
        // Exact powers of 1024.
        assert_eq!(fmt_size(1_024), "1.0 KB");
        assert_eq!(fmt_size(1_048_576), "1.0 MB");
        assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
        // Just below each boundary stays in the lower unit.
        assert_eq!(fmt_size(1_023), "1023 B");
        assert_eq!(fmt_size(1_047_552), "1023.0 KB"); // 1023 * 1024
    }

    // ── toggle_mark / clear_marks / Space key ─────────────────────────────────

    #[test]
    fn toggle_mark_adds_entry_to_marked_set() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(!explorer.entries.is_empty(), "need at least one entry");

        explorer.toggle_mark();

        assert_eq!(explorer.marked.len(), 1, "one entry should be marked");
    }

    #[test]
    fn toggle_mark_removes_already_marked_entry() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

        explorer.toggle_mark(); // mark
        let cursor_after_first = explorer.cursor;
        explorer.cursor = 0; // reset to the same entry
        explorer.toggle_mark(); // unmark

        assert!(
            explorer.marked.is_empty(),
            "second toggle on same entry should unmark it"
        );
        let _ = cursor_after_first; // suppress unused warning
    }

    #[test]
    fn toggle_mark_advances_cursor_down() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        // Ensure there are at least two entries so the cursor can advance.
        assert!(
            explorer.entries.len() >= 2,
            "fixture must have at least 2 entries"
        );

        let before = explorer.cursor;
        explorer.toggle_mark();

        assert_eq!(
            explorer.cursor,
            before + 1,
            "cursor should advance by one after toggle_mark"
        );
    }

    #[test]
    fn toggle_mark_at_last_entry_does_not_overflow() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.cursor = explorer.entries.len() - 1;

        explorer.toggle_mark();

        assert_eq!(
            explorer.cursor,
            explorer.entries.len() - 1,
            "cursor should stay at the last entry, not overflow"
        );
    }

    #[test]
    fn clear_marks_empties_marked_set() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

        explorer.toggle_mark();
        assert!(
            !explorer.marked.is_empty(),
            "should have a mark before clear"
        );

        explorer.clear_marks();

        assert!(
            explorer.marked.is_empty(),
            "marked set should be empty after clear_marks"
        );
    }

    #[test]
    fn space_key_marks_current_entry() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(!explorer.entries.is_empty(), "need at least one entry");

        let outcome = explorer.handle_key(key(KeyCode::Char(' ')));

        assert_eq!(
            outcome,
            ExplorerOutcome::Pending,
            "Space should return Pending"
        );
        assert_eq!(
            explorer.marked.len(),
            1,
            "Space should mark the current entry"
        );
    }

    #[test]
    fn space_key_toggles_mark_off() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

        explorer.handle_key(key(KeyCode::Char(' '))); // mark → cursor moves down
        explorer.cursor = 0; // reset to entry 0
        explorer.handle_key(key(KeyCode::Char(' '))); // unmark

        assert!(
            explorer.marked.is_empty(),
            "second Space on same entry should unmark it"
        );
    }

    #[test]
    fn marks_cleared_when_ascending_to_parent() {
        let dir = temp_dir_with_files();
        // Start inside the subdir so we can ascend.
        let sub = dir.path().join("subdir");
        fs::write(sub.join("inner.txt"), b"inner").unwrap();
        let mut explorer = FileExplorer::new(sub.clone(), vec![]);

        explorer.toggle_mark();
        assert!(
            !explorer.marked.is_empty(),
            "should have a mark before ascend"
        );

        // Ascend via Backspace.
        explorer.handle_key(key(KeyCode::Backspace));

        assert!(
            explorer.marked.is_empty(),
            "marks should be cleared after ascending to parent"
        );
    }

    #[test]
    fn marks_cleared_when_descending_into_directory() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

        // Mark the subdirectory entry.
        let sub_idx = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("fixture has a subdir");
        explorer.cursor = sub_idx;
        explorer.toggle_mark();
        assert!(
            !explorer.marked.is_empty(),
            "should have a mark before descend"
        );

        // Reset cursor back to the directory entry (toggle_mark advanced it).
        explorer.cursor = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("fixture has a subdir");

        // Descend into the subdirectory — confirm() clears marks.
        explorer.handle_key(key(KeyCode::Enter));

        assert!(
            explorer.marked.is_empty(),
            "marks should be cleared after descending into a directory"
        );
    }

    #[test]
    fn can_mark_multiple_entries() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        let total = explorer.entries.len();
        assert!(total >= 2, "fixture must have at least 2 entries");

        // Mark every entry.
        for _ in 0..total {
            explorer.toggle_mark();
        }

        assert_eq!(explorer.marked.len(), total, "all entries should be marked");
    }

    // ── Cursor / scroll boundary safety ──────────────────────────────────────

    #[test]
    fn move_up_at_top_does_not_underflow() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        // Should be a no-op, not a panic.
        explorer.handle_key(key(KeyCode::Up));
        assert_eq!(explorer.cursor, 0);
    }

    #[test]
    fn move_down_at_bottom_does_not_overflow() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        let last = explorer.entries.len().saturating_sub(1);
        explorer.cursor = last;
        explorer.handle_key(key(KeyCode::Down));
        assert_eq!(explorer.cursor, last);
    }

    #[test]
    fn move_down_on_empty_dir_does_not_panic() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(explorer.entries.is_empty());
        // Must not panic.
        explorer.handle_key(key(KeyCode::Down));
        assert_eq!(explorer.cursor, 0);
    }

    #[test]
    fn move_up_on_empty_dir_does_not_panic() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(explorer.entries.is_empty());
        explorer.handle_key(key(KeyCode::Up));
        assert_eq!(explorer.cursor, 0);
    }

    #[test]
    fn page_down_at_bottom_does_not_overflow() {
        let dir = tempdir().expect("tempdir");
        for i in 0..5 {
            fs::write(dir.path().join(format!("{i}.txt")), b"x").unwrap();
        }
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        let last = explorer.entries.len().saturating_sub(1);
        explorer.cursor = last;
        explorer.handle_key(key(KeyCode::PageDown));
        assert_eq!(explorer.cursor, last);
    }

    #[test]
    fn page_up_at_top_does_not_underflow() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.cursor = 0;
        explorer.handle_key(key(KeyCode::PageUp));
        assert_eq!(explorer.cursor, 0);
    }

    #[test]
    fn ascend_at_root_does_not_panic() {
        let mut explorer = FileExplorer::new(std::path::PathBuf::from("/"), vec![]);
        // Pressing Backspace at root must not panic — it should stay put.
        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.current_dir, std::path::PathBuf::from("/"));
    }

    #[test]
    fn cursor_clamped_after_reload_with_fewer_entries() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::write(dir.path().join("b.txt"), b"b").unwrap();
        fs::write(dir.path().join("c.txt"), b"c").unwrap();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        // Move to last entry.
        explorer.cursor = explorer.entries.len() - 1;
        // Now apply a filter that shows only one file — reload happens inside.
        explorer.set_extension_filter(["a"]);
        // Cursor must be clamped to the new (smaller) list.
        assert!(
            explorer.cursor < explorer.entries.len().max(1),
            "cursor {} out of range for {} entries",
            explorer.cursor,
            explorer.entries.len()
        );
    }

    #[test]
    fn scroll_offset_clamped_after_reload_with_empty_entries() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("test.rs"), b"fn main(){}").unwrap();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.scroll_offset = 5; // artificially stale
        explorer.cursor = 0;
        // Apply a filter that matches nothing — entries becomes empty.
        explorer.set_extension_filter(["xyz"]);
        assert_eq!(explorer.cursor, 0);
        assert_eq!(explorer.scroll_offset, 0);
    }

    #[test]
    fn marked_paths_returns_reference_to_marked_set() {
        let dir = temp_dir_with_files();
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

        explorer.toggle_mark();

        assert_eq!(
            explorer.marked_paths().len(),
            explorer.marked.len(),
            "marked_paths() should reflect the same set as the field"
        );
    }

    // ── entry_icon — extended coverage ───────────────────────────────────────

    fn make_file_entry(name: &str) -> crate::types::FsEntry {
        let ext = std::path::Path::new(name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        crate::types::FsEntry {
            name: name.to_string(),
            path: std::path::PathBuf::from(name),
            is_dir: false,
            size: None,
            extension: ext,
        }
    }

    macro_rules! assert_entry_icon {
        ($( $test_name:ident : $filename:expr => $icon:expr ),+ $(,)?) => {
            $(
                #[test]
                fn $test_name() {
                    let e = make_file_entry($filename);
                    assert_eq!(entry_icon(&e), $icon);
                }
            )+
        };
    }

    assert_entry_icon! {
        entry_icon_iso_returns_disc:                      "release.iso"  => "💿",
        entry_icon_dmg_returns_disc:                      "app.dmg"      => "💿",
        entry_icon_zip_returns_package:                   "archive.zip"  => "📦",
        entry_icon_tar_returns_package:                   "src.tar"      => "📦",
        entry_icon_gz_returns_package:                    "data.gz"      => "📦",
        entry_icon_pdf_returns_book:                      "manual.pdf"   => "📕",
        entry_icon_md_returns_memo:                       "README.md"    => "📝",
        entry_icon_toml_returns_gear:                     "Cargo.toml"   => "⚙ ",
        entry_icon_json_returns_gear:                     "config.json"  => "⚙ ",
        entry_icon_lock_returns_lock:                     "Cargo.lock"   => "🔒",
        entry_icon_py_returns_snake:                      "script.py"    => "🐍",
        entry_icon_html_returns_globe:                    "index.html"   => "🌐",
        entry_icon_css_returns_palette:                   "style.css"    => "🎨",
        entry_icon_svg_returns_palette:                   "logo.svg"     => "🎨",
        entry_icon_png_returns_image:                     "photo.png"    => "🖼 ",
        entry_icon_jpg_returns_image:                     "photo.jpg"    => "🖼 ",
        entry_icon_mp4_returns_film:                      "video.mp4"    => "🎬",
        entry_icon_mp3_returns_music:                     "song.mp3"     => "🎵",
        entry_icon_ttf_returns_font:                      "font.ttf"     => "🔤",
        entry_icon_exe_returns_gear:                      "setup.exe"    => "⚙ ",
        entry_icon_unknown_extension_returns_document:    "mystery.xyz"  => "📄",
    }

    #[test]
    fn entry_icon_no_extension_returns_document() {
        let e = crate::types::FsEntry {
            name: "Makefile".into(),
            path: std::path::PathBuf::from("Makefile"),
            is_dir: false,
            size: None,
            extension: String::new(),
        };
        assert_eq!(entry_icon(&e), "📄");
    }

    // ── fmt_size — full boundary coverage ────────────────────────────────────

    #[test]
    fn fmt_size_zero_bytes() {
        assert_eq!(fmt_size(0), "0 B");
    }

    #[test]
    fn fmt_size_one_byte() {
        assert_eq!(fmt_size(1), "1 B");
    }

    #[test]
    fn fmt_size_1023_bytes_stays_bytes() {
        assert_eq!(fmt_size(1_023), "1023 B");
    }

    #[test]
    fn fmt_size_exactly_1_kb() {
        assert_eq!(fmt_size(1_024), "1.0 KB");
    }

    #[test]
    fn fmt_size_1_5_kb() {
        assert_eq!(fmt_size(1_536), "1.5 KB");
    }

    #[test]
    fn fmt_size_1_mb_boundary() {
        assert_eq!(fmt_size(1_048_576), "1.0 MB");
    }

    #[test]
    fn fmt_size_2_mb() {
        assert_eq!(fmt_size(2_097_152), "2.0 MB");
    }

    #[test]
    fn fmt_size_1_gb_boundary() {
        assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn fmt_size_large_value() {
        // 10 GB
        assert_eq!(fmt_size(10 * 1_073_741_824), "10.0 GB");
    }

    // ── navigate_to — &str and &Path inputs ──────────────────────────────────

    #[test]
    fn navigate_to_accepts_str_slice() {
        let dir = tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();

        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.navigate_to(sub.to_str().unwrap());
        assert_eq!(explorer.current_dir, sub);
    }

    #[test]
    fn navigate_to_accepts_path_ref() {
        let dir = tempdir().expect("tempdir");
        let sub = dir.path().join("sub2");
        fs::create_dir(&sub).unwrap();

        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.navigate_to(sub.as_path());
        assert_eq!(explorer.current_dir, sub);
    }

    #[test]
    fn navigate_to_resets_cursor_to_zero() {
        let dir = tempdir().expect("tempdir");
        let sub = dir.path().join("sub3");
        fs::create_dir(&sub).unwrap();

        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.cursor = 99;
        explorer.scroll_offset = 5;
        explorer.navigate_to(sub.as_path());
        assert_eq!(explorer.cursor, 0);
        assert_eq!(explorer.scroll_offset, 0);
    }

    // ── is_searching accessor ─────────────────────────────────────────────────

    #[test]
    fn is_searching_false_by_default() {
        let dir = tempdir().expect("tempdir");
        let explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(!explorer.is_searching());
    }

    #[test]
    fn is_searching_true_after_slash_key() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        assert!(explorer.is_searching());
    }

    #[test]
    fn is_searching_false_after_esc_cancels_search() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('/')));
        explorer.handle_key(key(KeyCode::Esc));
        assert!(!explorer.is_searching());
    }

    // ── status cleared on reload ──────────────────────────────────────────────

    #[test]
    fn status_is_empty_on_fresh_explorer() {
        let dir = tempdir().expect("tempdir");
        let explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(explorer.status().is_empty());
    }

    #[test]
    fn status_cleared_after_reload() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        // Manually set a stale status message.
        explorer.status = "stale message".into();
        explorer.reload();
        assert!(
            explorer.status().is_empty(),
            "reload should clear the status message"
        );
    }

    // ── load_entries with an empty directory ──────────────────────────────────

    #[test]
    fn load_entries_empty_dir_returns_empty_vec() {
        let dir = tempdir().expect("tempdir");
        let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
        assert!(
            entries.is_empty(),
            "empty directory should produce no entries"
        );
    }

    #[test]
    fn load_entries_hidden_excluded_by_default() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join(".hidden"), b"h").unwrap();
        fs::write(dir.path().join("visible.txt"), b"v").unwrap();

        let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible.txt");
    }

    #[test]
    fn load_entries_hidden_included_when_show_hidden_true() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join(".hidden"), b"h").unwrap();
        fs::write(dir.path().join("visible.txt"), b"v").unwrap();

        let entries = load_entries(dir.path(), true, &[], crate::types::SortMode::Name, "");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_entries_nonexistent_dir_returns_empty_vec() {
        let entries = load_entries(
            std::path::Path::new("/nonexistent/path/that/does/not/exist"),
            false,
            &[],
            crate::types::SortMode::Name,
            "",
        );
        assert!(entries.is_empty());
    }

    #[test]
    fn load_entries_search_query_is_case_insensitive() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("README.md"), b"r").unwrap();
        fs::write(dir.path().join("main.rs"), b"m").unwrap();

        let entries = load_entries(
            dir.path(),
            false,
            &[],
            crate::types::SortMode::Name,
            "readme",
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "README.md");
    }

    #[test]
    fn load_entries_dirs_always_precede_files() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("z_file.txt"), b"z").unwrap();
        fs::create_dir(dir.path().join("a_dir")).unwrap();

        let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
        assert!(entries[0].is_dir, "directory must come before file");
        assert!(!entries[1].is_dir);
    }

    #[test]
    fn load_entries_ext_filter_excludes_non_matching_files() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("main.rs"), b"r").unwrap();
        fs::write(dir.path().join("Cargo.toml"), b"t").unwrap();

        let filter = vec!["rs".to_string()];
        let entries = load_entries(dir.path(), false, &filter, crate::types::SortMode::Name, "");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].extension, "rs");
    }

    #[test]
    fn load_entries_ext_filter_always_includes_dirs() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("file.txt"), b"t").unwrap();

        // Filter for .rs — the dir should still appear, the .txt file should not.
        let filter = vec!["rs".to_string()];
        let entries = load_entries(dir.path(), false, &filter, crate::types::SortMode::Name, "");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_dir);
    }

    // ── Rename mode ───────────────────────────────────────────────────────────

    #[test]
    fn r_key_activates_rename_mode_with_prefilled_name() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        // Move cursor to a known file.
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "readme.txt")
            .expect("readme.txt present");
        explorer.cursor = idx;

        let outcome = explorer.handle_key(key(KeyCode::Char('r')));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(explorer.is_rename_active());
        assert_eq!(explorer.rename_input(), "readme.txt");
    }

    #[test]
    fn r_key_on_empty_dir_does_not_activate_rename() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        assert!(explorer.entries.is_empty());

        let outcome = explorer.handle_key(key(KeyCode::Char('r')));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.is_rename_active());
    }

    #[test]
    fn rename_mode_chars_append_to_input() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('r')));
        assert!(explorer.is_rename_active());

        // Clear the prefilled name and type a fresh one.
        let original_len = explorer.rename_input().len();
        for _ in 0..original_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        explorer.handle_key(key(KeyCode::Char('n')));
        explorer.handle_key(key(KeyCode::Char('e')));
        explorer.handle_key(key(KeyCode::Char('w')));

        assert_eq!(explorer.rename_input(), "new");
        assert!(explorer.is_rename_active());
    }

    #[test]
    fn rename_mode_backspace_pops_last_char() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('r')));

        // Pop all chars then type "ab".
        let original_len = explorer.rename_input().len();
        for _ in 0..original_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        explorer.handle_key(key(KeyCode::Char('a')));
        explorer.handle_key(key(KeyCode::Char('b')));
        assert_eq!(explorer.rename_input(), "ab");

        explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(explorer.rename_input(), "a");
    }

    #[test]
    fn rename_mode_esc_cancels_without_renaming() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "readme.txt")
            .expect("readme.txt present");
        explorer.cursor = idx;

        explorer.handle_key(key(KeyCode::Char('r')));
        assert!(explorer.is_rename_active());

        let outcome = explorer.handle_key(key(KeyCode::Esc));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.is_rename_active());
        assert_eq!(explorer.rename_input(), "");
        // File must still exist under the old name.
        assert!(tmp.path().join("readme.txt").exists());
    }

    #[test]
    fn rename_mode_enter_renames_file_and_returns_rename_completed() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "readme.txt")
            .expect("readme.txt present");
        explorer.cursor = idx;

        // Activate rename, clear prefill, type new name.
        explorer.handle_key(key(KeyCode::Char('r')));
        let prefill_len = explorer.rename_input().len();
        for _ in 0..prefill_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        for c in "notes.txt".chars() {
            explorer.handle_key(key(KeyCode::Char(c)));
        }

        let outcome = explorer.handle_key(key(KeyCode::Enter));

        assert!(!explorer.is_rename_active());
        assert_eq!(explorer.rename_input(), "");
        assert!(tmp.path().join("notes.txt").exists(), "new name must exist");
        assert!(
            !tmp.path().join("readme.txt").exists(),
            "old name must be gone"
        );
        assert!(
            matches!(outcome, ExplorerOutcome::RenameCompleted(p) if p.file_name().unwrap() == "notes.txt")
        );
    }

    #[test]
    fn rename_mode_cursor_moves_to_renamed_entry() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "readme.txt")
            .expect("readme.txt present");
        explorer.cursor = idx;

        explorer.handle_key(key(KeyCode::Char('r')));
        let prefill_len = explorer.rename_input().len();
        for _ in 0..prefill_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        for c in "zzz_last.txt".chars() {
            explorer.handle_key(key(KeyCode::Char(c)));
        }
        explorer.handle_key(key(KeyCode::Enter));

        let new_idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "zzz_last.txt")
            .expect("renamed entry in list");
        assert_eq!(explorer.cursor, new_idx);
    }

    #[test]
    fn rename_mode_enter_with_empty_input_is_noop() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "readme.txt")
            .expect("readme.txt present");
        explorer.cursor = idx;

        explorer.handle_key(key(KeyCode::Char('r')));
        // Erase the prefilled name entirely, then confirm.
        let prefill_len = explorer.rename_input().len();
        for _ in 0..prefill_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        assert_eq!(explorer.rename_input(), "");

        let outcome = explorer.handle_key(key(KeyCode::Enter));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.is_rename_active());
        // Original file must still exist.
        assert!(tmp.path().join("readme.txt").exists());
    }

    #[test]
    fn rename_mode_can_rename_directory() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "subdir" && e.is_dir)
            .expect("subdir present");
        explorer.cursor = idx;

        explorer.handle_key(key(KeyCode::Char('r')));
        let prefill_len = explorer.rename_input().len();
        for _ in 0..prefill_len {
            explorer.handle_key(key(KeyCode::Backspace));
        }
        for c in "renamed_dir".chars() {
            explorer.handle_key(key(KeyCode::Char(c)));
        }
        let outcome = explorer.handle_key(key(KeyCode::Enter));

        assert!(tmp.path().join("renamed_dir").exists());
        assert!(!tmp.path().join("subdir").exists());
        assert!(matches!(outcome, ExplorerOutcome::RenameCompleted(_)));
    }

    #[test]
    fn rename_mode_unrecognised_key_returns_pending_without_cancelling() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        explorer.handle_key(key(KeyCode::Char('r')));
        assert!(explorer.is_rename_active());

        // F1 is not handled inside rename mode.
        let outcome = explorer.handle_key(key(KeyCode::F(1)));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(explorer.is_rename_active(), "rename mode must stay active");
    }

    #[test]
    fn is_rename_active_false_by_default() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.is_rename_active());
    }

    #[test]
    fn rename_input_empty_by_default() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(explorer.rename_input(), "");
    }

    // ── handle_input_mode! macro tests ───────────────────────────────────────
    // These tests exercise the Char-push, Backspace-pop, Esc-cancel, and
    // unknown-key fallthrough paths that the macro generates for every mode.

    // ── mkdir_mode via macro ──────────────────────────────────────────────────

    #[test]
    fn mkdir_mode_char_pushes_to_input_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.mkdir_active = true;
        explorer.mkdir_input.clear();

        let outcome = explorer.handle_key(key(KeyCode::Char('a')));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.mkdir_input, "a");
        assert!(explorer.mkdir_active, "mode must remain active after Char");
    }

    #[test]
    fn mkdir_mode_backspace_pops_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.mkdir_active = true;
        explorer.mkdir_input = "ab".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.mkdir_input, "a");
        assert!(explorer.mkdir_active);
    }

    #[test]
    fn mkdir_mode_esc_cancels_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.mkdir_active = true;
        explorer.mkdir_input = "half".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Esc));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.mkdir_active, "mode must be deactivated by Esc");
        assert!(
            explorer.mkdir_input.is_empty(),
            "input must be cleared by Esc"
        );
    }

    #[test]
    fn mkdir_mode_unknown_key_returns_pending_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.mkdir_active = true;
        explorer.mkdir_input = "foo".to_string();

        let outcome = explorer.handle_key(key(KeyCode::F(2)));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        // Mode and input must be unchanged.
        assert!(explorer.mkdir_active);
        assert_eq!(explorer.mkdir_input, "foo");
    }

    // ── touch_mode via macro ──────────────────────────────────────────────────

    #[test]
    fn touch_mode_char_pushes_to_input_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.touch_active = true;
        explorer.touch_input.clear();

        let outcome = explorer.handle_key(key(KeyCode::Char('z')));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.touch_input, "z");
        assert!(explorer.touch_active);
    }

    #[test]
    fn touch_mode_backspace_pops_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.touch_active = true;
        explorer.touch_input = "xy".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.touch_input, "x");
        assert!(explorer.touch_active);
    }

    #[test]
    fn touch_mode_esc_cancels_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.touch_active = true;
        explorer.touch_input = "half".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Esc));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.touch_active);
        assert!(explorer.touch_input.is_empty());
    }

    #[test]
    fn touch_mode_unknown_key_returns_pending_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.touch_active = true;
        explorer.touch_input = "bar".to_string();

        let outcome = explorer.handle_key(key(KeyCode::F(3)));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(explorer.touch_active);
        assert_eq!(explorer.touch_input, "bar");
    }

    // ── rename_mode via macro ─────────────────────────────────────────────────

    #[test]
    fn rename_mode_char_pushes_to_input_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.rename_active = true;
        explorer.rename_input.clear();

        let outcome = explorer.handle_key(key(KeyCode::Char('r')));
        // NOTE: 'r' is normally the "activate rename" key, but because
        // rename_active is already true the mode interception runs first and
        // pushes 'r' to the input — it never reaches the normal key dispatch.
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.rename_input, "r");
        assert!(explorer.rename_active);
    }

    #[test]
    fn rename_mode_backspace_pops_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.rename_active = true;
        explorer.rename_input = "cd".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Backspace));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.rename_input, "c");
        assert!(explorer.rename_active);
    }

    #[test]
    fn rename_mode_esc_cancels_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.rename_active = true;
        explorer.rename_input = "draft".to_string();

        let outcome = explorer.handle_key(key(KeyCode::Esc));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(!explorer.rename_active);
        assert!(explorer.rename_input.is_empty());
    }

    #[test]
    fn rename_mode_unknown_key_returns_pending_via_macro() {
        let dir = tempdir().expect("tempdir");
        let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
        explorer.rename_active = true;
        explorer.rename_input = "baz".to_string();

        let outcome = explorer.handle_key(key(KeyCode::F(4)));
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(explorer.rename_active);
        assert_eq!(explorer.rename_input, "baz");
    }
}
