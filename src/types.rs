//! Public data types exposed by the `tui-file-explorer` crate.

use std::path::PathBuf;

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
