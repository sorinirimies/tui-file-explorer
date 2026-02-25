//! Public data types exposed by the `tui-file-explorer` crate.

use std::path::PathBuf;

// ── SortMode ──────────────────────────────────────────────────────────────────

/// Controls the order in which directory entries are listed.
///
/// The sort order can be changed at runtime with the `s` key (which cycles
/// through all variants) or programmatically via
/// [`crate::FileExplorer::set_sort_mode`].
///
/// Directories are always shown before files regardless of the active mode.
///
/// ```
/// use tui_file_explorer::SortMode;
///
/// let mode = SortMode::default(); // SortMode::Name
/// let next = mode.next();         // SortMode::SizeDesc
/// println!("{}", mode.label());   // "name"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Alphabetical by file name, A → Z. This is the default.
    #[default]
    Name,
    /// By file size, largest first. Directories sort alphabetically among
    /// themselves (they have no meaningful size).
    SizeDesc,
    /// By file extension (A → Z), then by name within each extension group.
    Extension,
}

impl SortMode {
    /// Return the next mode in the cycle: `Name → SizeDesc → Extension → Name`.
    ///
    /// Intended for the `s` key binding that cycles sort modes at runtime.
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::SizeDesc,
            Self::SizeDesc => Self::Extension,
            Self::Extension => Self::Name,
        }
    }

    /// A short human-readable label for display in the footer status bar.
    ///
    /// ```
    /// use tui_file_explorer::SortMode;
    ///
    /// assert_eq!(SortMode::Name.label(),      "name");
    /// assert_eq!(SortMode::SizeDesc.label(),  "size ↓");
    /// assert_eq!(SortMode::Extension.label(), "ext");
    /// ```
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::SizeDesc => "size ↓",
            Self::Extension => "ext",
        }
    }
}

// ── FsEntry ───────────────────────────────────────────────────────────────────

/// A single entry shown in the file-explorer list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsEntry {
    /// Display name (filename only, not full path).
    pub name: String,
    /// Absolute path to the entry.
    pub path: PathBuf,
    /// `true` if the entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (`None` for directories or when unavailable).
    pub size: Option<u64>,
    /// File extension in lower-case (empty string for directories / no ext).
    pub extension: String,
}

// ── ExplorerOutcome ───────────────────────────────────────────────────────────

/// Outcome returned by [`crate::FileExplorer::handle_key`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerOutcome {
    /// The user confirmed a file selection — contains the chosen path.
    Selected(PathBuf),
    /// The user dismissed the explorer (`Esc` / `q`) without selecting anything.
    Dismissed,
    /// A key was consumed but produced no navigational outcome yet.
    Pending,
    /// The key was not recognised / consumed by the explorer.
    Unhandled,
}
