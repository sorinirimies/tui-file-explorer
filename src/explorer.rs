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
    pub fn handle_key(&mut self, key: KeyEvent) -> ExplorerOutcome {
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
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                self.ascend();
                ExplorerOutcome::Pending
            }

            // ── Confirm / descend ─────────────────────────────────────────────
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.confirm(),

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

            _ => ExplorerOutcome::Unhandled,
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// The currently highlighted [`FsEntry`], or `None` if the list is empty.
    pub fn current_entry(&self) -> Option<&FsEntry> {
        self.entries.get(self.cursor)
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
    }

    fn move_down(&mut self) {
        if !self.entries.is_empty() && self.cursor < self.entries.len() - 1 {
            self.cursor += 1;
        }
    }

    fn ascend(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            let prev = self.current_dir.clone();
            self.current_dir = parent;
            self.cursor = 0;
            self.scroll_offset = 0;
            // Clear search when navigating to a different directory.
            self.search_active = false;
            self.search_query.clear();
            self.reload();
            // Try to land the cursor on the directory we just came from.
            if let Some(idx) = self.entries.iter().position(|e| e.path == prev) {
                self.cursor = idx;
            }
        } else {
            self.status = "Already at the filesystem root.".to_string();
        }
    }

    fn confirm(&mut self) -> ExplorerOutcome {
        let Some(entry) = self.entries.get(self.cursor) else {
            return ExplorerOutcome::Pending;
        };

        if entry.is_dir {
            let path = entry.path.clone();
            // Clear search when descending into a subdirectory.
            self.search_active = false;
            self.search_query.clear();
            self.navigate_to(path);
            ExplorerOutcome::Pending
        } else if self.is_selectable(entry) {
            ExplorerOutcome::Selected(entry.path.clone())
        } else {
            self.status = format!("Not a supported file type. Allowed: {}", self.filter_hint());
            ExplorerOutcome::Pending
        }
    }

    // ── Helpers exposed to sibling modules ───────────────────────────────────

    /// Returns `true` when `entry` may be confirmed by the user.
    ///
    /// Directories are never "selectable" (they are navigated into instead).
    /// Files pass when the extension filter is empty, or when the file's
    /// extension matches one of the allowed extensions.
    pub(crate) fn is_selectable(&self, entry: &FsEntry) -> bool {
        if entry.is_dir {
            return false;
        }
        if self.extension_filter.is_empty() {
            return true;
        }
        self.extension_filter
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(&entry.extension))
    }

    fn filter_hint(&self) -> String {
        if self.extension_filter.is_empty() {
            "*".to_string()
        } else {
            self.extension_filter
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(", ")
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
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    match sort_mode {
        SortMode::Name => {
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        SortMode::SizeDesc => {
            // Largest first; treat missing size as 0.
            files.sort_by(|a, b| b.size.unwrap_or(0).cmp(&a.size.unwrap_or(0)));
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
    use tempfile::TempDir;

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
    fn is_selectable_respects_filter() {
        let tmp = temp_dir_with_files();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into()]);

        let iso_entry = FsEntry {
            name: "ubuntu.iso".into(),
            path: tmp.path().join("ubuntu.iso"),
            is_dir: false,
            size: Some(16),
            extension: "iso".into(),
        };
        let img_entry = FsEntry {
            name: "debian.img".into(),
            path: tmp.path().join("debian.img"),
            is_dir: false,
            size: Some(16),
            extension: "img".into(),
        };
        let dir_entry = FsEntry {
            name: "subdir".into(),
            path: tmp.path().join("subdir"),
            is_dir: true,
            size: None,
            extension: String::new(),
        };

        assert!(
            explorer.is_selectable(&iso_entry),
            "iso should be selectable"
        );
        assert!(
            !explorer.is_selectable(&img_entry),
            "img should not be selectable"
        );
        assert!(
            !explorer.is_selectable(&dir_entry),
            "dirs are never selectable"
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
        // We bypass handle_key so the search interception doesn't consume
        // the Backspace — instead we call ascend() via the Left arrow which
        // is only handled by the non-search branch.
        explorer.search_active = true;
        explorer.search_query = "foo".into();

        // Left arrow is not intercepted by the search block, so it reaches
        // the ascend() arm in the main match.
        explorer.handle_key(key(KeyCode::Left));

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
}
