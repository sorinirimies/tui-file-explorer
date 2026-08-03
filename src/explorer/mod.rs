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

use crate::types::{DiskUsage, ExplorerOutcome, FsEntry, SortMode};

/// Default number of entries scrolled by Page Up / Page Down.
pub const PAGE_SIZE: usize = 10;

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
    /// Number of entries scrolled by Page Up / Page Down (default: 10).
    pub page_size: usize,
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
    /// Cache of computed recursive directory sizes, keyed by absolute path.
    ///
    /// Each entry stores `(total_bytes, is_partial, item_count_at_computation)`.
    /// A cached value is reused as long as the directory's shallow
    /// [`FsEntry::item_count`] hasn't changed since it was computed — a cheap
    /// heuristic that invalidates the cache when files are added, removed, or
    /// renamed directly inside that directory, without requiring a full
    /// recursive re-walk on every render or keystroke. See
    /// [`Self::ensure_dir_size_before`] / [`Self::clear_dir_size_cache`].
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

mod builder;
mod entries;
mod keys;

pub use self::builder::FileExplorerBuilder;
use self::entries::load_entries;
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
            page_size: PAGE_SIZE,
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
            disk_usage: None,
            dir_size_cache: std::collections::HashMap::new(),
            show_sizes: true,
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
        self.disk_usage = crate::fs::disk_usage(&self.current_dir);
        // After every reload the entry count may have shrunk (filter change,
        // external deletion, empty directory).  Clamp so cursor and
        // scroll_offset never point past the end of the new list.
        self.clamp_cursor();
    }

    // ── Directory size cache ─────────────────────────────────────────────

    /// Return the cached recursive size for the directory at `path`,
    /// recomputing it (bounded by [`crate::fs::DIR_SIZE_MAX_ENTRIES`] /
    /// [`crate::fs::DIR_SIZE_MAX_DURATION`], plus an optional shared
    /// `deadline`) if it is missing or stale.
    ///
    /// `item_count` should be the directory's current shallow entry count
    /// (i.e. [`FsEntry::item_count`]); it is used as a cheap staleness check
    /// — a mismatch against the cached value means something changed
    /// directly inside the directory since it was last computed.
    ///
    /// When `deadline` is `Some` and has already passed, a cache miss is left
    /// uncomputed for this call (returning `(0, false)`) instead of
    /// performing the walk — this lets callers share one wall-clock budget
    /// across many rows in a single render pass so a folder full of large
    /// subdirectories can never make a render noticeably slow; skipped
    /// directories keep showing their cheap shallow item count and get
    /// picked up on a later redraw. Pass `None` for unbounded behaviour.
    ///
    /// Returns `(total_bytes, is_partial)`. Intended for renderers that only
    /// need sizes for the currently visible rows, since this may perform a
    /// bounded filesystem walk.
    pub(crate) fn ensure_dir_size_before(
        &mut self,
        path: &Path,
        item_count: Option<usize>,
        deadline: Option<std::time::Instant>,
    ) -> (u64, bool) {
        let Some(item_count) = item_count else {
            return (0, false);
        };
        if let Some(&(bytes, partial, cached_count)) = self.dir_size_cache.get(path) {
            if cached_count == item_count {
                return (bytes, partial);
            }
        }
        if let Some(deadline) = deadline {
            if std::time::Instant::now() >= deadline {
                // Out of budget this frame — leave uncached, try again later.
                return (0, false);
            }
        }
        let (bytes, partial) = crate::fs::dir_size(path);
        self.dir_size_cache
            .insert(path.to_path_buf(), (bytes, partial, item_count));
        (bytes, partial)
    }

    /// Clear every cached recursive directory size.
    ///
    /// Call this after operations that can change file sizes *without*
    /// necessarily changing any visible directory's immediate entry count
    /// (the heuristic `ensure_dir_size_before` relies on) — e.g. a
    /// multi-file paste or delete that touches nested subdirectories.
    pub fn clear_dir_size_cache(&mut self) {
        self.dir_size_cache.clear();
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
