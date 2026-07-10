//! Persist application state between sessions.
//!
//! State is stored at `$XDG_CONFIG_HOME/tfe/settings.json` (falling back to
//! `~/.config/tfe/settings.json`) as a JSON file.
//!
//! Writes use atomic rename (write to `.tmp`, then rename) to avoid
//! corruption on crash.
//!
//! Unknown keys are silently ignored so that older versions of the binary can
//! read state files written by newer ones without errors.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{SortMode, Theme};

// ── SortMode serde helper ─────────────────────────────────────────────────────

mod sort_mode_serde {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<SortMode>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(m) => serializer.serialize_some(sort_mode_to_key(*m)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SortMode>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        Ok(opt.and_then(|s| sort_mode_from_key(&s)))
    }
}

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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    /// Colour theme name (e.g. `"grape"`, `"nord"`, `"catppuccin-mocha"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Directory that was open in the left pane when the app last exited.
    ///
    /// Only restored when the path still exists as a directory; stale entries
    /// (deleted directories) are silently ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_dir: Option<PathBuf>,

    /// Directory that was open in the right pane when the app last exited.
    ///
    /// Only restored when the path still exists as a directory; stale entries
    /// (deleted directories) are silently ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_dir_right: Option<PathBuf>,

    /// Active sort mode: `Name`, `SizeDesc`, or `Extension`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "sort_mode_serde"
    )]
    pub sort_mode: Option<SortMode>,

    /// Whether hidden (dot-prefixed) files were visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_hidden: Option<bool>,

    /// Whether single-pane mode was active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single_pane: Option<bool>,

    /// Whether the cd-on-exit feature is enabled.
    ///
    /// When `true`, `tfe` prints the active pane's current directory to stdout
    /// on dismiss so the shell wrapper can `cd` to it.  When `false` (default),
    /// dismissing without a selection prints nothing and exits with code 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cd_on_exit: Option<bool>,

    /// The editor to use when the user presses `e` on a file.
    ///
    /// Serialised as a short key string (e.g. `"helix"`, `"nvim"`,
    /// `"custom:code"`).  `None` means "use the compiled-in default" (Helix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,

    /// Which pane (left or right) had keyboard focus when the app last exited.
    /// Serialised as `"left"` or `"right"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// Path of the JSON state file (`$XDG_CONFIG_HOME/tfe/settings.json`).
pub fn state_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
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

// ── Path-based helpers (used by tests and the public API) ─────────────────────

/// Load state from a JSON file at `path`.
///
/// Returns a default empty state if the file does not exist or cannot be
/// parsed. Directory fields are validated: stale paths are set to `None`.
pub(crate) fn load_state_from(path: &Path) -> AppState {
    let Ok(data) = fs::read_to_string(path) else {
        return AppState::default();
    };
    let Ok(mut state) = serde_json::from_str::<AppState>(&data) else {
        return AppState::default();
    };

    // Validate that directory paths still exist on disk.
    if let Some(ref p) = state.last_dir {
        if !p.is_dir() {
            state.last_dir = None;
        }
    }
    if let Some(ref p) = state.last_dir_right {
        if !p.is_dir() {
            state.last_dir_right = None;
        }
    }

    state
}

/// Save state to a JSON file at `path`.
///
/// Uses atomic write (write to `.tmp`, then rename) to avoid corruption.
/// Creates parent directories if they don't exist.
pub(crate) fn save_state_to(path: &Path, state: &AppState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|e| io::Error::other(e.to_string()))?;

    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, json.as_bytes())?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load application state from the default XDG config path.
///
/// Never returns an error — any I/O problem simply yields an empty state so
/// that the app can always start with sensible defaults.
pub fn load_state() -> AppState {
    let Some(json_path) = state_path() else {
        return AppState::default();
    };
    load_state_from(&json_path)
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
