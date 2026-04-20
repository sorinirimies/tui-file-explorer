//! Persist application state between sessions.
//!
//! State is stored at `$XDG_CONFIG_HOME/tfe/state.redb` (falling back to
//! `~/.config/tfe/state.redb`) as a [`redb`](https://crates.io/crates/redb)
//! embedded database.
//!
//! The database contains a single table `state` with `&str` keys and `&str`
//! values.  All writes go through a single ACID transaction, giving us
//! crash-safe persistence for free.
//!
//! Unknown keys are silently ignored so that older versions of the binary can
//! read state files written by newer ones without errors.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use redb::{Database, ReadableDatabase, TableDefinition};

use crate::{SortMode, Theme};

// ── redb table definition ─────────────────────────────────────────────────────

const STATE_TABLE: TableDefinition<&str, &str> = TableDefinition::new("state");

// ── Key constants ─────────────────────────────────────────────────────────────

const KEY_THEME: &str = "theme";
const KEY_LAST_DIR: &str = "last_dir";
const KEY_LAST_DIR_RIGHT: &str = "last_dir_right";
const KEY_SORT_MODE: &str = "sort_mode";
const KEY_SHOW_HIDDEN: &str = "show_hidden";
const KEY_SINGLE_PANE: &str = "single_pane";
const KEY_CD_ON_EXIT: &str = "cd_on_exit";
const KEY_EDITOR: &str = "editor";
const KEY_ACTIVE_PANE: &str = "active_pane";

// ── AppState ──────────────────────────────────────────────────────────────────

/// All application state that is persisted between sessions.
///
/// Every field is an `Option` so that absent keys are handled gracefully —
/// the caller provides a sensible default for any field that is `None`.
///
/// # Example
///
/// ```rust,ignore
/// use crate::persistence::{AppState, load_state, save_state};
/// use tui_file_explorer::SortMode;
///
/// let mut state = load_state();
/// state.theme      = Some("nord".into());
/// state.sort_mode  = Some(SortMode::SizeDesc);
/// state.show_hidden = Some(true);
/// save_state(&state);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppState {
    /// Colour theme name (e.g. `"grape"`, `"nord"`, `"catppuccin-mocha"`).
    pub theme: Option<String>,

    /// Directory that was open in the left pane when the app last exited.
    ///
    /// Only restored when the path still exists as a directory; stale entries
    /// (deleted directories) are silently ignored.
    pub last_dir: Option<PathBuf>,

    /// Directory that was open in the right pane when the app last exited.
    ///
    /// Only restored when the path still exists as a directory; stale entries
    /// (deleted directories) are silently ignored.
    pub last_dir_right: Option<PathBuf>,

    /// Active sort mode: `Name`, `SizeDesc`, or `Extension`.
    pub sort_mode: Option<SortMode>,

    /// Whether hidden (dot-prefixed) files were visible.
    pub show_hidden: Option<bool>,

    /// Whether single-pane mode was active.
    pub single_pane: Option<bool>,

    /// Whether the cd-on-exit feature is enabled.
    ///
    /// When `true`, `tfe` prints the active pane's current directory to stdout
    /// on dismiss so the shell wrapper can `cd` to it.  When `false` (default),
    /// dismissing without a selection prints nothing and exits with code 1.
    pub cd_on_exit: Option<bool>,

    /// The editor to use when the user presses `e` on a file.
    ///
    /// Serialised as a short key string (e.g. `"helix"`, `"nvim"`,
    /// `"custom:code"`).  `None` means "use the compiled-in default" (Helix).
    pub editor: Option<String>,

    /// Which pane (left or right) had keyboard focus when the app last exited.
    /// Serialised as `"left"` or `"right"`.
    pub active_pane: Option<String>,
}

// ── Config-dir helpers ────────────────────────────────────────────────────────

/// Returns the `tfe` config directory, following XDG conventions.
///
/// Priority: `$XDG_CONFIG_HOME/tfe` → `$HOME/.config/tfe` → `None`.
fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("tfe"))
}

/// Path of the redb database (`$XDG_CONFIG_HOME/tfe/state.redb`).
pub fn state_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("state.redb"))
}

// ── SortMode serialisation helpers ───────────────────────────────────────────

/// Convert a `SortMode` to its stable on-disk key string.
fn sort_mode_to_key(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Name => "name",
        SortMode::SizeDesc => "size_desc",
        SortMode::Extension => "extension",
    }
}

/// Parse a `SortMode` from its on-disk key string.
///
/// Returns `None` for unrecognised values so that the field is left as `None`
/// rather than silently defaulting to `Name`.
fn sort_mode_from_key(s: &str) -> Option<SortMode> {
    match s {
        "name" => Some(SortMode::Name),
        "size_desc" => Some(SortMode::SizeDesc),
        "extension" => Some(SortMode::Extension),
        _ => None,
    }
}

// ── Low-level redb I/O ────────────────────────────────────────────────────────

/// Read a single string value from the redb state table.
///
/// Returns `None` when the key is absent, the table does not exist, or any
/// I/O error occurs.
fn get_str(db: &Database, key: &str) -> Option<String> {
    let txn = db.begin_read().ok()?;
    let table = txn.open_table(STATE_TABLE).ok()?;
    let guard = table.get(key).ok()??;
    Some(guard.value().to_string())
}

/// Read a directory path from the redb state table.
///
/// Only returns `Some` when the stored path is a non-empty string that
/// points to an existing directory on disk.
fn get_dir(db: &Database, key: &str) -> Option<PathBuf> {
    let raw = get_str(db, key)?;
    if raw.is_empty() {
        return None;
    }
    let p = PathBuf::from(raw);
    if p.is_dir() {
        Some(p)
    } else {
        None
    }
}

/// Read a boolean value from the redb state table.
fn get_bool(db: &Database, key: &str) -> Option<bool> {
    get_str(db, key)?.parse::<bool>().ok()
}

/// Load an [`AppState`] from an open redb [`Database`].
pub(crate) fn load_state_from_db(db: &Database) -> AppState {
    AppState {
        theme: get_str(db, KEY_THEME),
        last_dir: get_dir(db, KEY_LAST_DIR),
        last_dir_right: get_dir(db, KEY_LAST_DIR_RIGHT),
        sort_mode: get_str(db, KEY_SORT_MODE).and_then(|s| sort_mode_from_key(&s)),
        show_hidden: get_bool(db, KEY_SHOW_HIDDEN),
        single_pane: get_bool(db, KEY_SINGLE_PANE),
        cd_on_exit: get_bool(db, KEY_CD_ON_EXIT),
        editor: get_str(db, KEY_EDITOR),
        active_pane: get_str(db, KEY_ACTIVE_PANE),
    }
}

/// Save an [`AppState`] into an open redb [`Database`].
///
/// Uses a single write transaction to atomically replace all keys.
/// Keys whose value is `None` are removed from the table.
pub(crate) fn save_state_to_db(db: &Database, state: &AppState) -> Result<(), redb::Error> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(STATE_TABLE)?;

        // Helper: insert if Some, remove if None.
        macro_rules! put {
            ($key:expr, $val:expr) => {
                match $val {
                    Some(ref v) => {
                        table.insert($key, v.as_str())?;
                    }
                    None => {
                        let _ = table.remove($key);
                    }
                }
            };
        }

        put!(KEY_THEME, &state.theme);
        put!(
            KEY_LAST_DIR,
            &state.last_dir.as_ref().map(|p| p.display().to_string())
        );
        put!(
            KEY_LAST_DIR_RIGHT,
            &state
                .last_dir_right
                .as_ref()
                .map(|p| p.display().to_string())
        );
        put!(
            KEY_SORT_MODE,
            &state.sort_mode.map(|m| sort_mode_to_key(m).to_string())
        );
        put!(KEY_SHOW_HIDDEN, &state.show_hidden.map(|b| b.to_string()));
        put!(KEY_SINGLE_PANE, &state.single_pane.map(|b| b.to_string()));
        put!(KEY_CD_ON_EXIT, &state.cd_on_exit.map(|b| b.to_string()));
        put!(KEY_EDITOR, &state.editor);
        put!(KEY_ACTIVE_PANE, &state.active_pane);
    }
    txn.commit()?;
    Ok(())
}

// ── Path-based helpers (used by tests and the public API) ─────────────────────

/// Load state from a redb database file at `path`.
///
/// Returns a default empty state if the file does not exist or cannot be
/// opened.
pub(crate) fn load_state_from(path: &Path) -> AppState {
    if !path.exists() {
        return AppState::default();
    }
    let Ok(db) = Database::open(path) else {
        return AppState::default();
    };
    load_state_from_db(&db)
}

/// Save state to a redb database file at `path`.
///
/// Creates the database (and parent directories) if they don't exist.
pub(crate) fn save_state_to(path: &Path, state: &AppState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let db = Database::create(path).map_err(|e| io::Error::other(e.to_string()))?;
    save_state_to_db(&db, state).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load application state from the default XDG config path.
///
/// Never returns an error — any I/O problem simply yields an empty state so
/// that the app can always start with sensible defaults.
pub fn load_state() -> AppState {
    if let Some(path) = state_path() {
        return load_state_from(&path);
    }
    AppState::default()
}

/// Persist `state` to the default XDG config path.
///
/// Errors are silently discarded — persistence is best-effort and must never
/// cause the application to crash or block.
pub fn save_state(state: &AppState) {
    if let Some(path) = state_path() {
        let _ = save_state_to(&path, state);
    }
}

// ── Theme resolution ──────────────────────────────────────────────────────────

/// Find the index into `themes` whose name matches `name`.
///
/// Matching is **case-insensitive** and treats **hyphens as spaces**, so
/// `"catppuccin-mocha"` matches `"Catppuccin Mocha"`.
///
/// Returns `0` (the built-in default theme) when no match is found, and
/// prints a hint to stderr suggesting `--list-themes`.
pub fn resolve_theme_idx(name: &str, themes: &[(&str, &str, Theme)]) -> usize {
    let key = name.to_lowercase().replace('-', " ");
    for (i, (n, _, _)) in themes.iter().enumerate() {
        if n.to_lowercase().replace('-', " ") == key {
            return i;
        }
    }
    eprintln!(
        "tfe: unknown theme {:?} — falling back to default. \
         Run `tfe --list-themes` to see available options.",
        name
    );
    0
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
