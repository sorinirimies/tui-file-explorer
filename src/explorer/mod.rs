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
    fmt, io,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::keymap::Keymap;
use crate::types::{
    DiskUsage, ExplorerCommand, ExplorerOutcome, FsEntry, Prompt, PromptKind, SelectionMode,
    SortMode,
};

/// Default number of entries scrolled by Page Up / Page Down.
pub const PAGE_SIZE: usize = 10;

/// Error returned by checked explorer filesystem operations.
#[derive(Debug)]
pub struct ExplorerError {
    pub path: PathBuf,
    pub source: io::Error,
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.source)
    }
}

impl std::error::Error for ExplorerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

// ── FileExplorer ──────────────────────────────────────────────────────────────

/// State for the file-explorer widget.
///
/// Keep one instance in your application state and pass a mutable reference
/// to [`crate::render()`] and [`FileExplorer::handle_key`] on every frame /
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
    /// Backend used for directory and mutation operations.
    pub(crate) filesystem: crate::SharedFileSystem,
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
    /// Number of entries scrolled by Page Up / Page Down (default: 10).
    pub page_size: usize,
    /// Which entry kinds can be returned as selections.
    pub selection_mode: SelectionMode,
    /// Current incremental-search query (empty = no search active).
    pub search_query: String,
    /// Whether the explorer is currently capturing keystrokes for search input.
    pub search_active: bool,
    /// Paths that have been space-marked for a multi-item operation.
    pub marked: HashSet<PathBuf>,
    /// The open rename / new-file / new-folder prompt, if any. While `Some`,
    /// the prompt owns the keyboard.
    pub prompt: Option<Prompt>,
    /// Key bindings used by [`handle_key`](Self::handle_key) and
    /// [`handle_command`](Self::handle_command).
    pub keymap: Keymap,
    /// Cache of computed recursive directory sizes, keyed by absolute path.
    ///
    /// Each entry stores `(total_bytes, is_partial, item_count_at_computation)`.
    /// A cached value is reused as long as the directory's shallow
    /// [`FsEntry::item_count`] hasn't changed since it was computed — a cheap
    /// heuristic that invalidates the cache when files are added, removed, or
    /// renamed directly inside that directory, without requiring a full
    /// recursive re-walk on every render or keystroke. See
    /// [`Self::ensure_dir_size_before`] / [`Self::clear_dir_size_cache`].
    /// Lazily resolved symlink targets for visible rows (`None` = not a link).
    /// Cleared on every reload.
    /// Read directories on a worker thread instead of blocking the caller.
    ///
    /// Off by default (so `reload` is deterministic for embedders and tests);
    /// the `tfe` binary turns it on. A read that finishes within a few
    /// milliseconds is applied synchronously; slower ones show
    /// [`is_loading`](Self::is_loading) until they complete.
    pub async_loading: bool,
    /// Off-thread work in flight (folder sizes, `git status`).
    pub(crate) bg: background::Background,
    /// Whether the git change column is shown (see [`Self::set_show_git`]).
    pub show_git: bool,
    /// Latest `git status` for the current directory, when enabled.
    pub(crate) git: Option<crate::git::GitStatus>,
    pub(crate) link_cache: std::collections::HashMap<PathBuf, Option<PathBuf>>,
    pub(crate) dir_size_cache: std::collections::HashMap<PathBuf, (u64, bool, usize)>,
    /// Total/free space for the storage device backing `current_dir`.
    ///
    /// Refreshed on every [`Self::reload`] (i.e. on every navigation), since
    /// crossing a mount point changes which device applies. `None` when the
    /// underlying OS query fails or isn't supported on this platform.
    pub disk_usage: Option<DiskUsage>,
    /// Whether file/folder sizes are shown in the entry list.
    ///
    /// When `false`, the (potentially expensive) recursive directory-size
    /// walk (`ensure_dir_size_before`) is skipped entirely and only the
    /// cheap shallow item count is shown for directories — useful for
    /// snappier browsing of huge trees (network mounts, `node_modules`,
    /// build output, ...). Toggle with `z`. Defaults to `true`.
    pub show_sizes: bool,
    /// Whether directories show their *recursive* byte size.
    ///
    /// Only meaningful while [`show_sizes`](Self::show_sizes) is `true`. When
    /// `false`, files still show their size but directories show only the
    /// cheap shallow item count and the recursive walk never runs. Toggle
    /// with `Shift+Z`. Defaults to `true`.
    pub show_dir_sizes: bool,
}

mod background;
mod builder;
mod entries;
mod keys;

pub use self::builder::FileExplorerBuilder;
#[cfg(test)]
use self::entries::load_entries;
use self::entries::try_load_entries;
pub use self::entries::{entry_icon, fmt_size};

impl FileExplorer {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a new explorer starting at `initial_dir`.
    ///
    /// `extension_filter` is a list of lower-case extensions *without* the
    /// leading dot (e.g. `vec!["iso".into(), "img".into()]`).
    /// Pass an empty `Vec` to allow all files.
    ///
    /// For more configuration options use [`FileExplorer::builder`] instead.
    #[must_use]
    pub fn new(initial_dir: PathBuf, extension_filter: Vec<String>) -> Self {
        Self::builder(initial_dir)
            .extension_filter(extension_filter)
            .build()
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
    #[must_use]
    pub fn builder(initial_dir: PathBuf) -> FileExplorerBuilder {
        FileExplorerBuilder::new(initial_dir)
    }

    /// Return the configured filesystem backend.
    #[must_use]
    pub fn filesystem(&self) -> &crate::SharedFileSystem {
        &self.filesystem
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
        if let Err(error) = self.try_navigate_to(path) {
            self.entries.clear();
            self.disk_usage = None;
            self.status = error.to_string();
            self.clamp_cursor();
        }
    }

    /// Navigate to `path` while preserving filesystem errors for the host.
    pub fn try_navigate_to(&mut self, path: impl Into<PathBuf>) -> Result<(), ExplorerError> {
        let previous_dir = std::mem::replace(&mut self.current_dir, path.into());
        let previous_cursor = self.cursor;
        let previous_scroll = self.scroll_offset;
        let previous_search_active = self.search_active;
        let previous_search_query = std::mem::take(&mut self.search_query);
        self.cursor = 0;
        self.scroll_offset = 0;
        self.search_active = false;
        if let Err(error) = self.try_reload() {
            self.current_dir = previous_dir;
            self.cursor = previous_cursor;
            self.scroll_offset = previous_scroll;
            self.search_active = previous_search_active;
            self.search_query = previous_search_query;
            return Err(error);
        }
        Ok(())
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    /// Process a single keyboard event and return the [`ExplorerOutcome`].
    ///
    /// Call this from your application's key-handling function and act on
    /// [`ExplorerOutcome::Selected`] / [`ExplorerOutcome::Dismissed`].
    /// Return the set of currently marked paths (for multi-item operations).
    #[must_use]
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

    // ── Queries ───────────────────────────────────────────────────────────────

    /// The currently highlighted [`FsEntry`], or `None` if the list is empty.
    #[must_use]
    pub fn current_entry(&self) -> Option<&FsEntry> {
        self.entries.get(self.cursor)
    }

    /// The text typed into the open prompt of `kind` (empty when another
    /// prompt, or none, is open).
    fn prompt_text(&self, kind: PromptKind) -> &str {
        match &self.prompt {
            Some(p) if p.kind == kind => &p.text,
            _ => "",
        }
    }

    /// Whether a prompt of `kind` is open.
    #[must_use]
    pub fn is_prompt_active(&self, kind: PromptKind) -> bool {
        self.prompt.as_ref().is_some_and(|p| p.kind == kind)
    }

    /// Whether any text prompt (rename / new file / new folder) is open.
    #[must_use]
    pub fn has_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    /// Open a prompt of `kind` pre-filled with `text`.
    pub fn open_prompt(&mut self, kind: PromptKind, text: impl Into<String>) {
        self.prompt = Some(Prompt {
            kind,
            text: text.into(),
        });
    }

    /// Whether the explorer is in mkdir (new-folder input) mode.
    #[must_use]
    pub fn is_mkdir_active(&self) -> bool {
        self.is_prompt_active(PromptKind::Mkdir)
    }

    /// The folder name being typed when mkdir mode is active.
    #[must_use]
    pub fn mkdir_input(&self) -> &str {
        self.prompt_text(PromptKind::Mkdir)
    }

    /// Whether the explorer is in touch (new-file input) mode.
    #[must_use]
    pub fn is_touch_active(&self) -> bool {
        self.is_prompt_active(PromptKind::Touch)
    }

    /// The file name being typed when touch mode is active.
    #[must_use]
    pub fn touch_input(&self) -> &str {
        self.prompt_text(PromptKind::Touch)
    }

    /// Whether the explorer is in rename (entry-rename input) mode.
    #[must_use]
    pub fn is_rename_active(&self) -> bool {
        self.is_prompt_active(PromptKind::Rename)
    }

    /// The new name being typed when rename mode is active.
    #[must_use]
    pub fn rename_input(&self) -> &str {
        self.prompt_text(PromptKind::Rename)
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
    #[must_use]
    pub fn is_at_root(&self) -> bool {
        self.current_dir.parent().is_none()
    }

    /// Returns `true` when the current directory contains no visible entries.
    ///
    /// This reflects the *filtered, visible* set — hidden files are excluded
    /// unless `show_hidden` is `true`, and an active search query narrows
    /// the set further.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of visible entries in the current directory.
    ///
    /// Equivalent to `explorer.entries.len()` but reads more naturally in
    /// condition checks.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Replace visible entries supplied by a host-owned asynchronous loader.
    ///
    /// Entries are used as provided; hosts can sort/filter before applying.
    pub fn replace_entries(&mut self, entries: Vec<FsEntry>) {
        self.entries = entries;
        self.clamp_cursor();
    }

    /// Set a host-provided status message.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// The current human-readable status message.
    ///
    /// The status is set by the widget when an error occurs (e.g. attempting
    /// to select a file that does not match the extension filter) and is
    /// cleared on the next successful navigation.  Returns an empty string
    /// when there is nothing to report.
    #[must_use]
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
    #[must_use]
    pub fn sort_mode(&self) -> SortMode {
        self.sort_mode
    }

    /// The current incremental-search query string.
    ///
    /// Returns an empty string when no search is active.
    #[must_use]
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Returns `true` when the explorer is actively capturing keystrokes for
    /// incremental search input.
    #[must_use]
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

    // ── Directory loading ─────────────────────────────────────────────────────

    /// Re-read the current directory from the filesystem.
    ///
    /// Called automatically after every navigation action or configuration
    /// change.  Callers can invoke it manually after external filesystem
    /// mutations (e.g. a file was created or deleted in the watched directory).
    ///
    /// With [`async_loading`](Self::async_loading) enabled a slow read does
    /// not block: the list shows "loading" and fills in on a later
    /// [`poll_background`](Self::poll_background).
    pub fn reload(&mut self) {
        let cursor = self.cursor;
        self.reload_with(background::AfterLoad {
            select: None,
            cursor: Some(cursor),
        });
    }

    /// Reload directory contents and preserve filesystem failures for hosts.
    ///
    /// Always synchronous (it must return the error), and it supersedes any
    /// background read in flight.
    pub fn try_reload(&mut self) -> Result<(), ExplorerError> {
        self.bg.cancel_load();
        self.status.clear();
        self.entries = try_load_entries(
            self.filesystem.as_ref(),
            &self.current_dir,
            self.show_hidden,
            &self.extension_filter,
            self.sort_mode,
            &self.search_query,
        )
        .map_err(|source| ExplorerError {
            path: self.current_dir.clone(),
            source,
        })?;
        self.disk_usage = self.filesystem.disk_usage(&self.current_dir);
        self.link_cache.clear();
        self.bg.clear_counted();
        self.refresh_git();
        // After every reload the entry count may have shrunk (filter change,
        // external deletion, empty directory).  Clamp so cursor and
        // scroll_offset never point past the end of the new list.
        self.clamp_cursor();
        Ok(())
    }

    // ── Directory size cache ─────────────────────────────────────────────

    /// Clear every cached recursive directory size.
    ///
    /// Call this after operations that can change file sizes *without*
    /// necessarily changing any visible directory's immediate entry count
    /// (the heuristic `ensure_dir_size_before` relies on) — e.g. a
    /// multi-file paste or delete that touches nested subdirectories.
    /// The symlink target of `path`, if it is a symlink (cached until reload).
    pub(crate) fn link_target(&mut self, path: &Path) -> Option<PathBuf> {
        if !self.filesystem.is_local() {
            return None;
        }
        self.link_cache
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read_link(path).ok())
            .clone()
    }

    pub fn clear_dir_size_cache(&mut self) {
        self.dir_size_cache.clear();
        self.bg.clear_dir_sizes();
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
