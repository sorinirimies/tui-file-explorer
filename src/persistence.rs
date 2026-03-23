//! Persist application state between sessions.
//!
//! State is stored at `$XDG_CONFIG_HOME/tfe/state` (falling back to
//! `~/.config/tfe/state`) as a plain `KEY=VALUE` text file:
//!
//! ```text
//! # tfe state — do not edit manually
//! theme=grape
//! last_dir=/home/user/projects
//! sort_mode=name
//! show_hidden=false
//! single_pane=false
//! ```
//!
//! Unknown keys are silently ignored so that older versions of the binary can
//! read state files written by newer ones without errors.  Malformed lines
//! (no `=` separator, blank, or comment) are also skipped gracefully.
//!
//! # Backward compatibility
//!
//! Older versions of `tfe` stored only the theme name in a separate
//! `$XDG_CONFIG_HOME/tfe/theme` file.  [`load_state`] falls back to that file
//! when the new `state` file is absent, so upgrades from older versions are
//! transparent.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{SortMode, Theme};

// ── Key constants ─────────────────────────────────────────────────────────────

const KEY_THEME: &str = "theme";
const KEY_LAST_DIR: &str = "last_dir";
const KEY_LAST_DIR_RIGHT: &str = "last_dir_right";
const KEY_SORT_MODE: &str = "sort_mode";
const KEY_SHOW_HIDDEN: &str = "show_hidden";
const KEY_SINGLE_PANE: &str = "single_pane";
const KEY_CD_ON_EXIT: &str = "cd_on_exit";
const KEY_EDITOR: &str = "editor";

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

/// Path of the unified state file (`$XDG_CONFIG_HOME/tfe/state`).
pub(crate) fn state_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("state"))
}

/// Path of the legacy theme-only file (`$XDG_CONFIG_HOME/tfe/theme`).
///
/// This file was written by older versions of `tfe`. It is only ever *read*
/// (as a fallback) by the current version; all writes go to `state`.
pub(crate) fn legacy_theme_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("theme"))
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

// ── Low-level I/O ─────────────────────────────────────────────────────────────

/// Parse a `KEY=VALUE` state file at `path` into an [`AppState`].
///
/// * Any I/O error (e.g. missing file) yields a default empty state.
/// * Blank lines and lines starting with `#` are skipped.
/// * Lines without a `=` separator are skipped.
/// * Unknown keys are silently ignored (forward-compatibility).
/// * The `last_dir` field is only populated when the path is an existing
///   directory — stale entries are discarded rather than propagated.
pub(crate) fn load_state_from(path: &Path) -> AppState {
    let Ok(content) = fs::read_to_string(path) else {
        return AppState::default();
    };

    let mut state = AppState::default();

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // Skip comments and blank lines.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on the *first* `=` only so that paths containing `=` are safe.
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());

        match key {
            KEY_THEME if !value.is_empty() => {
                state.theme = Some(value.to_string());
            }
            KEY_LAST_DIR if !value.is_empty() => {
                let p = PathBuf::from(value);
                // Only restore if the directory still exists on disk.
                if p.is_dir() {
                    state.last_dir = Some(p);
                }
            }
            KEY_LAST_DIR_RIGHT if !value.is_empty() => {
                let p = PathBuf::from(value);
                // Only restore if the directory still exists on disk.
                if p.is_dir() {
                    state.last_dir_right = Some(p);
                }
            }
            KEY_SORT_MODE => {
                state.sort_mode = sort_mode_from_key(value);
            }
            KEY_SHOW_HIDDEN => {
                state.show_hidden = value.parse::<bool>().ok();
            }
            KEY_SINGLE_PANE => {
                state.single_pane = value.parse::<bool>().ok();
            }
            KEY_CD_ON_EXIT => {
                state.cd_on_exit = value.parse::<bool>().ok();
            }
            KEY_EDITOR if !value.is_empty() => {
                state.editor = Some(value.to_string());
            }
            _ => {
                // Forward-compatible: unknown keys are silently ignored.
            }
        }
    }

    state
}

/// Serialise `state` to a `KEY=VALUE` file at `path`.
///
/// Parent directories are created automatically.  Only fields that are `Some`
/// are written, so the file only ever contains meaningful values.
pub(crate) fn save_state_to(path: &Path, state: &AppState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut out = String::from("# tfe state — do not edit manually\n");

    if let Some(ref theme) = state.theme {
        out.push_str(&format!("{KEY_THEME}={theme}\n"));
    }
    if let Some(ref dir) = state.last_dir {
        out.push_str(&format!("{KEY_LAST_DIR}={}\n", dir.display()));
    }
    if let Some(ref dir) = state.last_dir_right {
        out.push_str(&format!("{KEY_LAST_DIR_RIGHT}={}\n", dir.display()));
    }
    if let Some(mode) = state.sort_mode {
        out.push_str(&format!("{KEY_SORT_MODE}={}\n", sort_mode_to_key(mode)));
    }
    if let Some(hidden) = state.show_hidden {
        out.push_str(&format!("{KEY_SHOW_HIDDEN}={hidden}\n"));
    }
    if let Some(single) = state.single_pane {
        out.push_str(&format!("{KEY_SINGLE_PANE}={single}\n"));
    }
    if let Some(cd) = state.cd_on_exit {
        out.push_str(&format!("{KEY_CD_ON_EXIT}={cd}\n"));
    }
    if let Some(ref editor) = state.editor {
        out.push_str(&format!("{KEY_EDITOR}={editor}\n"));
    }

    fs::write(path, out)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load application state from the default XDG config path.
///
/// Falls back to the legacy `tfe/theme` file when the new `state` file is
/// absent, providing a seamless upgrade from older `tfe` versions.
///
/// Never returns an error — any I/O problem simply yields an empty state so
/// that the app can always start with sensible defaults.
pub fn load_state() -> AppState {
    if let Some(path) = state_path() {
        if path.exists() {
            return load_state_from(&path);
        }
    }

    // ── Legacy fallback ───────────────────────────────────────────────────────
    // Older versions of `tfe` only persisted the theme name, in a file called
    // `tfe/theme`.  Read it here so the user's theme choice is preserved after
    // upgrading.
    let mut state = AppState::default();
    if let Some(legacy) = legacy_theme_path() {
        if let Ok(raw) = fs::read_to_string(&legacy) {
            let name = raw.trim().to_string();
            if !name.is_empty() {
                state.theme = Some(name);
            }
        }
    }
    state
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // ── sort_mode_to_key / sort_mode_from_key ─────────────────────────────────

    #[test]
    fn sort_mode_to_key_name() {
        assert_eq!(sort_mode_to_key(SortMode::Name), "name");
    }

    #[test]
    fn sort_mode_to_key_size_desc() {
        assert_eq!(sort_mode_to_key(SortMode::SizeDesc), "size_desc");
    }

    #[test]
    fn sort_mode_to_key_extension() {
        assert_eq!(sort_mode_to_key(SortMode::Extension), "extension");
    }

    #[test]
    fn sort_mode_from_key_name() {
        assert_eq!(sort_mode_from_key("name"), Some(SortMode::Name));
    }

    #[test]
    fn sort_mode_from_key_size_desc() {
        assert_eq!(sort_mode_from_key("size_desc"), Some(SortMode::SizeDesc));
    }

    #[test]
    fn sort_mode_from_key_extension() {
        assert_eq!(sort_mode_from_key("extension"), Some(SortMode::Extension));
    }

    #[test]
    fn sort_mode_from_key_unknown_returns_none() {
        assert_eq!(sort_mode_from_key("bogus"), None);
        assert_eq!(sort_mode_from_key(""), None);
        assert_eq!(sort_mode_from_key("SIZE_DESC"), None);
    }

    #[test]
    fn sort_mode_key_round_trips_all_variants() {
        for mode in [SortMode::Name, SortMode::SizeDesc, SortMode::Extension] {
            let key = sort_mode_to_key(mode);
            let back = sort_mode_from_key(key);
            assert_eq!(back, Some(mode), "round-trip failed for {mode:?}");
        }
    }

    // ── AppState default ──────────────────────────────────────────────────────

    #[test]
    fn app_state_default_all_fields_none() {
        let state = AppState::default();
        assert!(state.theme.is_none());
        assert!(state.last_dir.is_none());
        assert!(state.last_dir_right.is_none());
        assert!(state.sort_mode.is_none());
        assert!(state.show_hidden.is_none());
        assert!(state.single_pane.is_none());
        assert!(state.cd_on_exit.is_none());
    }

    #[test]
    fn app_state_default_equals_default() {
        assert_eq!(AppState::default(), AppState::default());
    }

    #[test]
    fn app_state_clone_equals_original() {
        let state = AppState {
            theme: Some("nord".into()),
            show_hidden: Some(true),
            ..Default::default()
        };
        assert_eq!(state.clone(), state);
    }

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Create a temp directory and return a path inside it for the state file.
    /// The returned `TempDir` must be kept alive for the duration of the test.
    fn tmp_state_path() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("tfe").join("state");
        (dir, path)
    }

    /// Create a temp directory and return a path inside it for the legacy
    /// theme file.
    fn tmp_theme_path() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("tfe").join("theme");
        (dir, path)
    }

    // ── save_state_to / load_state_from ───────────────────────────────────────

    #[test]
    fn full_state_round_trips() {
        let (_dir, path) = tmp_state_path();
        let original = AppState {
            theme: Some("grape".into()),
            last_dir: Some(std::env::temp_dir()),
            last_dir_right: Some(std::env::temp_dir()),
            sort_mode: Some(SortMode::SizeDesc),
            show_hidden: Some(true),
            single_pane: Some(false),
            cd_on_exit: Some(true),
            editor: Some("nvim".into()),
        };
        save_state_to(&path, &original).unwrap();
        let loaded = load_state_from(&path);
        assert_eq!(loaded, original);
    }

    #[test]
    fn partial_state_leaves_absent_fields_as_none() {
        let (_dir, path) = tmp_state_path();
        let partial = AppState {
            theme: Some("nord".into()),
            ..Default::default()
        };
        save_state_to(&path, &partial).unwrap();
        let loaded = load_state_from(&path);
        assert_eq!(loaded.theme, Some("nord".into()));
        assert!(loaded.last_dir.is_none());
        assert!(loaded.sort_mode.is_none());
        assert!(loaded.show_hidden.is_none());
        assert!(loaded.single_pane.is_none());
        assert!(loaded.cd_on_exit.is_none());
    }

    #[test]
    fn cd_on_exit_true_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            cd_on_exit: Some(true),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        let loaded = load_state_from(&path);
        assert_eq!(loaded.cd_on_exit, Some(true));
    }

    #[test]
    fn cd_on_exit_false_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            cd_on_exit: Some(false),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        let loaded = load_state_from(&path);
        assert_eq!(loaded.cd_on_exit, Some(false));
    }

    #[test]
    fn missing_file_returns_default_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent").join("state");
        assert_eq!(load_state_from(&path), AppState::default());
    }

    #[test]
    fn empty_file_returns_default_state() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
        assert_eq!(load_state_from(&path), AppState::default());
    }

    #[test]
    fn save_state_creates_parent_directories() {
        let (_dir, path) = tmp_state_path();
        assert!(
            !path.parent().unwrap().exists(),
            "parent should not exist yet"
        );
        save_state_to(&path, &AppState::default()).unwrap();
        assert!(path.exists(), "state file should have been created");
    }

    #[test]
    fn save_state_overwrites_previous_content() {
        let (_dir, path) = tmp_state_path();
        let first = AppState {
            theme: Some("grape".into()),
            ..Default::default()
        };
        let second = AppState {
            theme: Some("ocean".into()),
            ..Default::default()
        };
        save_state_to(&path, &first).unwrap();
        save_state_to(&path, &second).unwrap();
        assert_eq!(load_state_from(&path).theme, Some("ocean".into()));
    }

    // ── File format edge cases ────────────────────────────────────────────────

    #[test]
    fn comment_lines_are_ignored() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# tfe state\n# another comment\ntheme=dracula\n").unwrap();
        assert_eq!(load_state_from(&path).theme, Some("dracula".into()));
    }

    #[test]
    fn blank_lines_are_ignored() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "\n\ntheme=nord\n\nsort_mode=name\n\n").unwrap();
        let state = load_state_from(&path);
        assert_eq!(state.theme, Some("nord".into()));
        assert_eq!(state.sort_mode, Some(SortMode::Name));
    }

    #[test]
    fn unknown_keys_are_silently_ignored() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "theme=nord\nfuture_feature=42\nanother_new_key=xyz\n",
        )
        .unwrap();
        let state = load_state_from(&path);
        assert_eq!(state.theme, Some("nord".into()));
    }

    #[test]
    fn malformed_lines_without_equals_are_skipped() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "this_has_no_equals\ntheme=grape\njust_text\n").unwrap();
        let state = load_state_from(&path);
        assert_eq!(state.theme, Some("grape".into()));
    }

    #[test]
    fn value_containing_equals_sign_is_preserved() {
        // Paths on some systems may theoretically contain `=`; we split on
        // the *first* `=` only, so the rest of the value is intact.
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Manufacture a value with an embedded `=` via the theme field
        // (unusual but the parser must handle it without panicking).
        fs::write(&path, "theme=weird=name\n").unwrap();
        let state = load_state_from(&path);
        assert_eq!(state.theme, Some("weird=name".into()));
    }

    #[test]
    fn surrounding_whitespace_in_values_is_trimmed() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "theme=  dracula  \nshow_hidden=  true  \n").unwrap();
        let state = load_state_from(&path);
        assert_eq!(state.theme, Some("dracula".into()));
        assert_eq!(state.show_hidden, Some(true));
    }

    // ── Sort mode ─────────────────────────────────────────────────────────────

    #[test]
    fn all_sort_modes_round_trip() {
        for mode in [SortMode::Name, SortMode::SizeDesc, SortMode::Extension] {
            let (_dir, path) = tmp_state_path();
            let state = AppState {
                sort_mode: Some(mode),
                ..Default::default()
            };
            save_state_to(&path, &state).unwrap();
            let loaded = load_state_from(&path);
            assert_eq!(
                loaded.sort_mode,
                Some(mode),
                "round-trip failed for {mode:?}"
            );
        }
    }

    #[test]
    fn unknown_sort_mode_value_yields_none() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "sort_mode=bogus_value\n").unwrap();
        assert!(load_state_from(&path).sort_mode.is_none());
    }

    // ── Boolean fields ────────────────────────────────────────────────────────

    #[test]
    fn show_hidden_true_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            show_hidden: Some(true),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(load_state_from(&path).show_hidden, Some(true));
    }

    #[test]
    fn show_hidden_false_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            show_hidden: Some(false),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(load_state_from(&path).show_hidden, Some(false));
    }

    #[test]
    fn single_pane_true_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            single_pane: Some(true),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(load_state_from(&path).single_pane, Some(true));
    }

    #[test]
    fn single_pane_false_round_trips() {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            single_pane: Some(false),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(load_state_from(&path).single_pane, Some(false));
    }

    #[test]
    fn invalid_bool_value_yields_none() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "show_hidden=yes\nsingle_pane=1\n").unwrap();
        let state = load_state_from(&path);
        assert!(state.show_hidden.is_none(), "\"yes\" is not a valid bool");
        assert!(state.single_pane.is_none(), "\"1\" is not a valid bool");
    }

    // ── last_dir ──────────────────────────────────────────────────────────────

    #[test]
    fn last_dir_round_trips_for_existing_directory() {
        let (_dir, path) = tmp_state_path();
        let existing = std::env::temp_dir(); // guaranteed to exist
        let state = AppState {
            last_dir: Some(existing.clone()),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(load_state_from(&path).last_dir, Some(existing));
    }

    #[test]
    fn last_dir_for_nonexistent_path_loads_as_none() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Write a path that is extremely unlikely to exist.
        fs::write(&path, "last_dir=/this/path/does/not/exist/tfe_test_xyz\n").unwrap();
        assert!(
            load_state_from(&path).last_dir.is_none(),
            "stale last_dir should be silently discarded"
        );
    }

    #[test]
    fn last_dir_empty_value_loads_as_none() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "last_dir=\n").unwrap();
        assert!(load_state_from(&path).last_dir.is_none());
    }

    // ── Theme field ───────────────────────────────────────────────────────────

    #[test]
    fn theme_names_with_spaces_and_hyphens_round_trip() {
        let names = [
            "default",
            "grape",
            "catppuccin-mocha",
            "tokyo night",
            "Nord",
        ];
        for name in names {
            let (_dir, path) = tmp_state_path();
            let state = AppState {
                theme: Some(name.into()),
                ..Default::default()
            };
            save_state_to(&path, &state).unwrap();
            assert_eq!(
                load_state_from(&path).theme,
                Some(name.to_string()),
                "round-trip failed for theme {name:?}"
            );
        }
    }

    #[test]
    fn empty_theme_value_loads_as_none() {
        let (_dir, path) = tmp_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "theme=\n").unwrap();
        assert!(load_state_from(&path).theme.is_none());
    }

    // ── Legacy backward compatibility ─────────────────────────────────────────

    #[test]
    fn legacy_theme_file_content_is_readable() {
        let (_dir, path) = tmp_theme_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Simulate what the old `tfe` wrote: just a theme name, possibly with
        // a trailing newline added by the OS or a text editor.
        fs::write(&path, "  nord\n").unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let trimmed = raw.trim().to_string();
        assert_eq!(trimmed, "nord");
    }

    #[test]
    fn legacy_theme_file_with_trailing_whitespace_is_trimmed() {
        let (_dir, path) = tmp_theme_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "\t dracula \n\n").unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let trimmed = raw.trim().to_string();
        assert_eq!(trimmed, "dracula");
        assert!(!trimmed.is_empty());
    }

    // ── resolve_theme_idx ─────────────────────────────────────────────────────

    #[test]
    fn resolve_theme_idx_finds_default_theme_at_zero() {
        let themes = Theme::all_presets();
        assert_eq!(resolve_theme_idx("default", &themes), 0);
    }

    #[test]
    fn resolve_theme_idx_finds_named_theme() {
        let themes = Theme::all_presets();
        let idx = resolve_theme_idx("grape", &themes);
        assert_ne!(idx, 0, "grape must not collide with the default index");
        assert_eq!(themes[idx].0.to_lowercase(), "grape");
    }

    #[test]
    fn resolve_theme_idx_is_case_insensitive() {
        let themes = Theme::all_presets();
        let lower = resolve_theme_idx("grape", &themes);
        let upper = resolve_theme_idx("GRAPE", &themes);
        let mixed = resolve_theme_idx("Grape", &themes);
        assert_eq!(lower, upper, "lower vs upper");
        assert_eq!(lower, mixed, "lower vs mixed");
    }

    #[test]
    fn resolve_theme_idx_normalises_hyphens_to_spaces() {
        let themes = Theme::all_presets();
        let spaced = resolve_theme_idx("catppuccin mocha", &themes);
        let hyphen = resolve_theme_idx("catppuccin-mocha", &themes);
        assert_eq!(spaced, hyphen);
    }

    #[test]
    fn resolve_theme_idx_unknown_name_returns_zero() {
        let themes = Theme::all_presets();
        assert_eq!(resolve_theme_idx("this-theme-does-not-exist", &themes), 0);
    }

    #[test]
    fn resolve_theme_idx_persisted_name_survives_round_trip() {
        let themes = Theme::all_presets();
        let (_dir, path) = tmp_state_path();

        let original_idx = resolve_theme_idx("nord", &themes);
        let original_name = themes[original_idx].0;

        let state = AppState {
            theme: Some(original_name.into()),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();

        let loaded_name = load_state_from(&path).theme.unwrap();
        let loaded_idx = resolve_theme_idx(&loaded_name, &themes);

        assert_eq!(
            original_idx, loaded_idx,
            "theme index must survive a full save/load cycle"
        );
    }

    #[test]
    fn resolve_theme_idx_all_presets_are_found() {
        let themes = Theme::all_presets();
        // Every preset must resolve to itself (not fall back to 0) unless it
        // is legitimately the first entry.
        for (i, (name, _, _)) in themes.iter().enumerate() {
            let resolved = resolve_theme_idx(name, &themes);
            assert_eq!(
                resolved, i,
                "preset {name:?} resolved to wrong index {resolved} (expected {i})"
            );
        }
    }

    // ── Full end-to-end (all fields) ──────────────────────────────────────────

    #[test]
    fn all_fields_independent_round_trips() {
        // Verify each field persists correctly when set alone, confirming no
        // cross-field interference in the serialiser / parser.
        let existing_dir = std::env::temp_dir();

        let cases: Vec<AppState> = vec![
            AppState {
                theme: Some("dracula".into()),
                ..Default::default()
            },
            AppState {
                last_dir: Some(existing_dir.clone()),
                ..Default::default()
            },
            AppState {
                sort_mode: Some(SortMode::Extension),
                ..Default::default()
            },
            AppState {
                show_hidden: Some(true),
                ..Default::default()
            },
            AppState {
                single_pane: Some(true),
                ..Default::default()
            },
        ];

        for case in cases {
            let (_dir, path) = tmp_state_path();
            save_state_to(&path, &case).unwrap();
            let loaded = load_state_from(&path);
            assert_eq!(loaded, case, "round-trip failed for {case:?}");
        }
    }

    // ── single-pane: last_dir_right preservation ──────────────────────────────

    /// Simulates the exact save logic in main.rs:
    ///
    ///   let last_dir_right = if app.single_pane {
    ///       saved.last_dir_right.clone()   // ← preserve, don't clobber
    ///   } else {
    ///       Some(app.right.current_dir.clone())
    ///   };
    ///
    /// When single-pane mode is active the right pane is hidden and its
    /// current_dir mirrors the left pane's starting directory.  If we wrote
    /// that mirrored value we'd clobber the real right-pane path, so we
    /// preserve whatever was previously persisted.
    #[test]
    fn last_dir_right_is_preserved_when_single_pane_is_active() {
        let (_dir, path) = tmp_state_path();
        let left_dir = std::env::temp_dir();
        let right_dir = {
            // Use a sub-directory of temp so it's a different path that exists.
            let p = std::env::temp_dir().join("tfe_test_right_pane_persist");
            std::fs::create_dir_all(&p).unwrap();
            p
        };

        // First session: dual-pane, user navigated each pane to a different dir.
        let first_session = AppState {
            last_dir: Some(left_dir.clone()),
            last_dir_right: Some(right_dir.clone()),
            single_pane: Some(false),
            ..Default::default()
        };
        save_state_to(&path, &first_session).unwrap();

        // Second session starts: load state, user switches to single-pane, exits.
        let saved = load_state_from(&path);
        assert_eq!(
            saved.last_dir_right,
            Some(right_dir.clone()),
            "right pane dir should have survived the first save"
        );

        // Replicate the main.rs save logic when single_pane == true:
        // the right pane's current_dir is the same as left (it was never navigated).
        let mirrored_right = left_dir.clone(); // what app.right.current_dir would be
        let last_dir_right = if true
        /* single_pane active */
        {
            saved.last_dir_right.clone() // preserve
        } else {
            Some(mirrored_right)
        };

        let second_session = AppState {
            last_dir: Some(left_dir.clone()),
            last_dir_right,
            single_pane: Some(true),
            ..Default::default()
        };
        save_state_to(&path, &second_session).unwrap();

        let restored = load_state_from(&path);
        assert_eq!(
            restored.last_dir_right,
            Some(right_dir.clone()),
            "last_dir_right must not be clobbered by the hidden right pane's mirrored path \
             when single_pane was active on exit"
        );
        assert_ne!(
            restored.last_dir_right, restored.last_dir,
            "right and left pane dirs should remain independent after a single-pane session"
        );
    }

    /// When the app is opened for the very first time (no prior state file),
    /// last_dir_right is None and the right pane correctly mirrors the left.
    #[test]
    fn last_dir_right_is_none_on_fresh_install() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent").join("state");
        let state = load_state_from(&path);
        assert!(
            state.last_dir_right.is_none(),
            "fresh install should have no persisted right-pane dir"
        );
    }

    /// Dual-pane exit: last_dir_right IS updated (the normal case).
    #[test]
    fn last_dir_right_is_updated_when_dual_pane_is_active() {
        let (_dir, path) = tmp_state_path();
        let left_dir = std::env::temp_dir();
        let right_dir = {
            let p = std::env::temp_dir().join("tfe_test_right_dual");
            std::fs::create_dir_all(&p).unwrap();
            p
        };

        let state = AppState {
            last_dir: Some(left_dir.clone()),
            last_dir_right: Some(right_dir.clone()),
            single_pane: Some(false),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();

        let loaded = load_state_from(&path);
        assert_eq!(
            loaded.last_dir_right,
            Some(right_dir),
            "dual-pane exit should persist the right pane's actual directory"
        );
    }
}
