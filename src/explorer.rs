//! [`FileExplorer`] state machine, [`FileExplorerBuilder`], and filesystem helpers.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::types::{ExplorerOutcome, FsEntry};

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
    /// Sorted list of visible entries (dirs first, then files).
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
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["rs".into(), "toml".into()])
    ///     .show_hidden(true)
    ///     .build();
    /// ```
    pub fn builder(initial_dir: PathBuf) -> FileExplorerBuilder {
        FileExplorerBuilder::new(initial_dir)
    }

    /// Navigate to `path`, resetting cursor and scroll to the top.
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.current_dir = path;
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

            _ => ExplorerOutcome::Unhandled,
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// The currently highlighted [`FsEntry`], or `None` if the list is empty.
    pub fn current_entry(&self) -> Option<&FsEntry> {
        self.entries.get(self.cursor)
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
    pub fn reload(&mut self) {
        self.status.clear();
        self.entries = load_entries(&self.current_dir, self.show_hidden, &self.extension_filter);
    }
}

// ── FileExplorerBuilder ───────────────────────────────────────────────────────

/// Builder for [`FileExplorer`].
///
/// Obtain one via [`FileExplorer::builder`], configure it with the chained
/// setter methods, then call [`build`](FileExplorerBuilder::build) to create
/// the explorer.
///
/// # Example
///
/// ```no_run
/// use tui_file_explorer::FileExplorer;
///
/// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
///     .extension_filter(vec!["iso".into(), "img".into()])
///     .show_hidden(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct FileExplorerBuilder {
    initial_dir: PathBuf,
    extension_filter: Vec<String>,
    show_hidden: bool,
}

impl FileExplorerBuilder {
    /// Create a new builder rooted at `initial_dir`.
    pub fn new(initial_dir: PathBuf) -> Self {
        Self {
            initial_dir,
            extension_filter: Vec::new(),
            show_hidden: false,
        }
    }

    /// Set the extension filter.
    ///
    /// Only files whose extension (lower-case, without the leading dot) appears
    /// in `filter` will be selectable. Directories are always navigable.
    /// Pass an empty `Vec` (the default) to allow all files.
    pub fn extension_filter(mut self, filter: Vec<String>) -> Self {
        self.extension_filter = filter;
        self
    }

    /// Add a single extension to the filter (lower-case, without the leading dot).
    ///
    /// Can be called multiple times to build the list incrementally.
    pub fn allow_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension_filter.push(ext.into());
        self
    }

    /// Whether to show hidden (dot-file) entries on startup.
    ///
    /// Defaults to `false`. The user can always toggle visibility with `.`
    /// while the explorer is running.
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Consume the builder and return a configured [`FileExplorer`].
    pub fn build(self) -> FileExplorer {
        let mut explorer = FileExplorer {
            current_dir: self.initial_dir,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            extension_filter: self.extension_filter,
            show_hidden: self.show_hidden,
            status: String::new(),
        };
        explorer.reload();
        explorer
    }
}

// ── Filesystem helpers ────────────────────────────────────────────────────────

pub(crate) fn load_entries(dir: &Path, show_hidden: bool, ext_filter: &[String]) -> Vec<FsEntry> {
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

        // Exclude files that don't match the filter (dirs are always shown).
        if !is_dir && !ext_filter.is_empty() {
            let matches = ext_filter
                .iter()
                .any(|f| f.eq_ignore_ascii_case(&extension));
            if !matches {
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

    // Sort each group alphabetically (case-insensitive).
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Dirs first, then matching files.
    dirs.extend(files);
    dirs
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Choose a unicode icon for a directory entry.
pub(crate) fn entry_icon(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        return "📁";
    }
    match entry.extension.as_str() {
        "iso" => "💿",
        "img" => "🖼 ",
        "zip" | "gz" | "xz" | "zst" | "bz2" | "tar" => "📦",
        "txt" | "md" | "rst" => "📄",
        "sh" | "bash" | "zsh" | "fish" => "📜",
        "toml" | "yaml" | "yml" | "json" => "⚙ ",
        _ => "📄",
    }
}

/// Format a byte count as a human-readable size string.
pub(crate) fn fmt_size(bytes: u64) -> String {
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

    fn temp_dir_with_files() -> TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("ubuntu.iso"), b"fake iso content").unwrap();
        fs::write(dir.path().join("debian.img"), b"fake img content").unwrap();
        fs::write(dir.path().join("readme.txt"), b"some text").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        dir
    }

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
            "dirs should appear before files"
        );
    }

    #[test]
    fn move_down_increments_cursor() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert_eq!(explorer.cursor, 0);
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
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let outcome = explorer.handle_key(key);
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert_eq!(explorer.cursor, 1);
    }

    #[test]
    fn handle_key_esc_dismisses() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(explorer.handle_key(key), ExplorerOutcome::Dismissed);
    }

    #[test]
    fn handle_key_enter_on_dir_descends() {
        let tmp = temp_dir_with_files();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let subdir_idx = explorer
            .entries
            .iter()
            .position(|e| e.is_dir)
            .expect("no dirs");
        explorer.cursor = subdir_idx;
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let outcome = explorer.handle_key(key);
        assert_eq!(outcome, ExplorerOutcome::Pending);
        assert!(explorer.current_dir.ends_with("subdir"));
    }

    #[test]
    fn handle_key_enter_on_valid_file_selects() {
        let tmp = temp_dir_with_files();
        let mut explorer =
            FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into(), "img".into()]);
        let iso_idx = explorer
            .entries
            .iter()
            .position(|e| e.name == "ubuntu.iso")
            .expect("ubuntu.iso not found");
        explorer.cursor = iso_idx;
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let outcome = explorer.handle_key(key);
        assert!(matches!(outcome, ExplorerOutcome::Selected(_)));
        if let ExplorerOutcome::Selected(p) = outcome {
            assert!(p.ends_with("ubuntu.iso"));
        }
    }

    #[test]
    fn handle_key_backspace_ascends() {
        let tmp = temp_dir_with_files();
        let subdir = tmp.path().join("subdir");
        let mut explorer = FileExplorer::new(subdir, vec![]);
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        explorer.handle_key(key);
        assert_eq!(explorer.current_dir, tmp.path());
    }

    #[test]
    fn toggle_hidden_changes_visibility() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".hidden"), b"x").unwrap();
        fs::write(tmp.path().join("visible.txt"), b"y").unwrap();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        assert!(!explorer.entries.iter().any(|e| e.name == ".hidden"));
        let key = KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE);
        explorer.handle_key(key);
        assert!(explorer.entries.iter().any(|e| e.name == ".hidden"));
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
        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
        assert_eq!(explorer.handle_key(key), ExplorerOutcome::Unhandled);
    }
}
