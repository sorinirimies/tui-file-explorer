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
    /// A new directory was successfully created at the given path.
    MkdirCreated(PathBuf),
    /// A new empty file was successfully created at the given path.
    TouchCreated(PathBuf),
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SortMode ──────────────────────────────────────────────────────────────

    #[test]
    fn sort_mode_default_is_name() {
        assert_eq!(SortMode::default(), SortMode::Name);
    }

    #[test]
    fn sort_mode_next_name_gives_size_desc() {
        assert_eq!(SortMode::Name.next(), SortMode::SizeDesc);
    }

    #[test]
    fn sort_mode_next_size_desc_gives_extension() {
        assert_eq!(SortMode::SizeDesc.next(), SortMode::Extension);
    }

    #[test]
    fn sort_mode_next_extension_wraps_to_name() {
        assert_eq!(SortMode::Extension.next(), SortMode::Name);
    }

    #[test]
    fn sort_mode_full_cycle_returns_to_start() {
        let start = SortMode::Name;
        let cycled = start.next().next().next();
        assert_eq!(cycled, start);
    }

    #[test]
    fn sort_mode_label_name() {
        assert_eq!(SortMode::Name.label(), "name");
    }

    #[test]
    fn sort_mode_label_size_desc() {
        assert_eq!(SortMode::SizeDesc.label(), "size ↓");
    }

    #[test]
    fn sort_mode_label_extension() {
        assert_eq!(SortMode::Extension.label(), "ext");
    }

    #[test]
    fn sort_mode_is_copy() {
        let a = SortMode::Name;
        let b = a;
        assert_eq!(a, b);
    }

    // ── FsEntry ───────────────────────────────────────────────────────────────

    #[test]
    fn fs_entry_construction_file() {
        let entry = FsEntry {
            name: "main.rs".into(),
            path: PathBuf::from("/src/main.rs"),
            is_dir: false,
            size: Some(1024),
            extension: "rs".into(),
        };
        assert_eq!(entry.name, "main.rs");
        assert_eq!(entry.path, PathBuf::from("/src/main.rs"));
        assert!(!entry.is_dir);
        assert_eq!(entry.size, Some(1024));
        assert_eq!(entry.extension, "rs");
    }

    #[test]
    fn fs_entry_construction_directory() {
        let entry = FsEntry {
            name: "src".into(),
            path: PathBuf::from("/src"),
            is_dir: true,
            size: None,
            extension: String::new(),
        };
        assert!(entry.is_dir);
        assert!(entry.size.is_none());
        assert!(entry.extension.is_empty());
    }

    #[test]
    fn fs_entry_equality() {
        let a = FsEntry {
            name: "a.txt".into(),
            path: PathBuf::from("/a.txt"),
            is_dir: false,
            size: Some(42),
            extension: "txt".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn fs_entry_no_extension_is_empty_string() {
        let entry = FsEntry {
            name: "Makefile".into(),
            path: PathBuf::from("/Makefile"),
            is_dir: false,
            size: Some(256),
            extension: String::new(),
        };
        assert!(entry.extension.is_empty());
    }

    // ── ExplorerOutcome ───────────────────────────────────────────────────────

    #[test]
    fn explorer_outcome_dismissed_equals_dismissed() {
        assert_eq!(ExplorerOutcome::Dismissed, ExplorerOutcome::Dismissed);
    }

    #[test]
    fn explorer_outcome_pending_equals_pending() {
        assert_eq!(ExplorerOutcome::Pending, ExplorerOutcome::Pending);
    }

    #[test]
    fn explorer_outcome_unhandled_equals_unhandled() {
        assert_eq!(ExplorerOutcome::Unhandled, ExplorerOutcome::Unhandled);
    }

    #[test]
    fn explorer_outcome_selected_carries_path() {
        let path = PathBuf::from("/tmp/file.txt");
        let outcome = ExplorerOutcome::Selected(path.clone());
        assert_eq!(outcome, ExplorerOutcome::Selected(path));
    }

    #[test]
    fn explorer_outcome_selected_neq_dismissed() {
        let outcome = ExplorerOutcome::Selected(PathBuf::from("/tmp/x"));
        assert_ne!(outcome, ExplorerOutcome::Dismissed);
    }

    #[test]
    fn explorer_outcome_is_clone() {
        let a = ExplorerOutcome::Pending;
        let b = a.clone();
        assert_eq!(a, b);
    }
}
