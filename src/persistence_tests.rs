//! Tests for the [`super`] persistence module (redb v4).

use super::*;
use std::fs;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a temp directory and return a path inside it for the redb file.
/// The returned `TempDir` must be kept alive for the duration of the test.
fn tmp_state_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("tfe").join("state.redb");
    (dir, path)
}

// ── Field round-trip macro ────────────────────────────────────────────────────

macro_rules! assert_field_round_trips {
    ($( $test_name:ident : $field:ident = $val:expr ),+ $(,)?) => {
        $(
            #[test]
            fn $test_name() {
                let (_dir, path) = tmp_state_path();
                let state = AppState {
                    $field: Some($val),
                    ..Default::default()
                };
                save_state_to(&path, &state).unwrap();
                assert_eq!(load_state_from(&path).$field, Some($val));
            }
        )+
    };
}

assert_field_round_trips! {
    show_hidden_true_round_trips:  show_hidden  = true,
    show_hidden_false_round_trips: show_hidden  = false,
    single_pane_true_round_trips:  single_pane  = true,
    single_pane_false_round_trips: single_pane  = false,
    cd_on_exit_true_round_trips:   cd_on_exit   = true,
    cd_on_exit_false_round_trips:  cd_on_exit   = false,
    active_pane_left_round_trips:  active_pane  = "left".into(),
    active_pane_right_round_trips: active_pane  = "right".into(),
}

// ── sort_mode_to_key / sort_mode_from_key ─────────────────────────────────────

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

// ── AppState default ──────────────────────────────────────────────────────────

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
    assert!(state.editor.is_none());
    assert!(state.active_pane.is_none());
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

// ── save_state_to / load_state_from (redb round-trips) ────────────────────────

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
        active_pane: Some("left".into()),
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
    assert!(loaded.last_dir_right.is_none());
    assert!(loaded.sort_mode.is_none());
    assert!(loaded.show_hidden.is_none());
    assert!(loaded.single_pane.is_none());
    assert!(loaded.cd_on_exit.is_none());
    assert!(loaded.editor.is_none());
    assert!(loaded.active_pane.is_none());
}

#[test]
fn missing_file_returns_default_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent").join("state.redb");
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

// ── Sort mode ─────────────────────────────────────────────────────────────────

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

// ── last_dir ──────────────────────────────────────────────────────────────────

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
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table
            .insert(KEY_LAST_DIR, "/this/path/does/not/exist/tfe_test_xyz")
            .unwrap();
    }
    txn.commit().unwrap();
    drop(db);
    assert!(
        load_state_from(&path).last_dir.is_none(),
        "stale last_dir should be silently discarded"
    );
}

#[test]
fn last_dir_empty_value_loads_as_none() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_LAST_DIR, "").unwrap();
    }
    txn.commit().unwrap();
    drop(db);
    assert!(load_state_from(&path).last_dir.is_none());
}

// ── Theme field ───────────────────────────────────────────────────────────────

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

// ── Editor field ──────────────────────────────────────────────────────────────

#[test]
fn editor_round_trips() {
    let editors = ["nvim", "helix", "custom:code --wait", "emacs"];
    for ed in editors {
        let (_dir, path) = tmp_state_path();
        let state = AppState {
            editor: Some(ed.into()),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
        assert_eq!(
            load_state_from(&path).editor,
            Some(ed.to_string()),
            "round-trip failed for editor {ed:?}"
        );
    }
}

// ── resolve_theme_idx ─────────────────────────────────────────────────────────

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
    for (i, (name, _, _)) in themes.iter().enumerate() {
        let resolved = resolve_theme_idx(name, &themes);
        assert_eq!(
            resolved, i,
            "preset {name:?} resolved to wrong index {resolved} (expected {i})"
        );
    }
}

// ── Full end-to-end (all fields) ──────────────────────────────────────────────

#[test]
fn all_fields_independent_round_trips() {
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
        AppState {
            editor: Some("helix".into()),
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

// ── single-pane: last_dir_right preservation ──────────────────────────────────

#[test]
fn last_dir_right_is_preserved_when_single_pane_is_active() {
    let (_dir, path) = tmp_state_path();
    let left_dir = std::env::temp_dir();
    let right_dir = {
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
    let last_dir_right = saved.last_dir_right.clone(); // preserve

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

#[test]
fn last_dir_right_is_none_on_fresh_install() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent").join("state.redb");
    let state = load_state_from(&path);
    assert!(
        state.last_dir_right.is_none(),
        "fresh install should have no persisted right-pane dir"
    );
}

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
        last_dir: Some(left_dir),
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

// ── None fields are removed from db ───────────────────────────────────────────

#[test]
fn none_field_removes_previously_stored_key() {
    let (_dir, path) = tmp_state_path();

    let first = AppState {
        theme: Some("grape".into()),
        editor: Some("nvim".into()),
        ..Default::default()
    };
    save_state_to(&path, &first).unwrap();
    assert_eq!(load_state_from(&path).editor, Some("nvim".into()));

    // Save again with editor = None.
    let second = AppState {
        theme: Some("grape".into()),
        editor: None,
        ..Default::default()
    };
    save_state_to(&path, &second).unwrap();
    assert!(
        load_state_from(&path).editor.is_none(),
        "editor should have been removed"
    );
}

// ── redb atomicity / multiple writes ──────────────────────────────────────────

#[test]
fn multiple_saves_to_same_db_are_atomic() {
    let (_dir, path) = tmp_state_path();
    for i in 0..10 {
        let state = AppState {
            theme: Some(format!("theme_{i}")),
            ..Default::default()
        };
        save_state_to(&path, &state).unwrap();
    }
    assert_eq!(
        load_state_from(&path).theme,
        Some("theme_9".into()),
        "last write wins"
    );
}

// ── load_state_from_db / save_state_to_db directly ────────────────────────────

#[test]
fn load_from_empty_db_returns_default() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let state = load_state_from_db(&db);
    assert_eq!(state, AppState::default());
}

#[test]
fn save_and_load_via_db_handle() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();

    let state = AppState {
        theme: Some("neon".into()),
        show_hidden: Some(true),
        sort_mode: Some(SortMode::Extension),
        ..Default::default()
    };
    save_state_to_db(&db, &state).unwrap();
    let loaded = load_state_from_db(&db);
    assert_eq!(loaded, state);
}

// ── get_str / get_dir / get_bool helpers ──────────────────────────────────────

#[test]
fn get_str_returns_none_for_missing_key() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    // Create the table with at least one key so the table exists.
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_THEME, "test").unwrap();
    }
    txn.commit().unwrap();
    assert!(get_str(&db, KEY_EDITOR).is_none());
}

#[test]
fn get_str_returns_value_for_existing_key() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_THEME, "dracula").unwrap();
    }
    txn.commit().unwrap();
    assert_eq!(get_str(&db, KEY_THEME), Some("dracula".to_string()));
}

#[test]
fn get_dir_returns_none_for_non_directory_path() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        // Store a path that exists as a file, not a directory.
        table.insert(KEY_LAST_DIR, path.to_str().unwrap()).unwrap();
    }
    txn.commit().unwrap();
    // The redb file itself exists but is not a directory.
    assert!(get_dir(&db, KEY_LAST_DIR).is_none());
}

#[test]
fn get_bool_returns_none_for_non_bool_value() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_SHOW_HIDDEN, "yes").unwrap();
        table.insert(KEY_SINGLE_PANE, "1").unwrap();
    }
    txn.commit().unwrap();
    assert!(
        get_bool(&db, KEY_SHOW_HIDDEN).is_none(),
        "\"yes\" is not a valid bool"
    );
    assert!(
        get_bool(&db, KEY_SINGLE_PANE).is_none(),
        "\"1\" is not a valid bool"
    );
}

#[test]
fn get_bool_parses_true_and_false() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_SHOW_HIDDEN, "true").unwrap();
        table.insert(KEY_SINGLE_PANE, "false").unwrap();
    }
    txn.commit().unwrap();
    assert_eq!(get_bool(&db, KEY_SHOW_HIDDEN), Some(true));
    assert_eq!(get_bool(&db, KEY_SINGLE_PANE), Some(false));
}

// ── config_dir / state_path ───────────────────────────────────────────────────

#[test]
fn state_path_ends_with_redb_extension() {
    // This test may return None in environments without $HOME or
    // $XDG_CONFIG_HOME, which is fine — we only assert the shape when Some.
    if let Some(p) = state_path() {
        assert!(
            p.to_str().unwrap().ends_with("state.redb"),
            "state_path should end with state.redb, got: {p:?}"
        );
        assert!(
            p.to_str().unwrap().contains("tfe"),
            "state_path should be inside a tfe directory"
        );
    }
}

// ── Unknown keys in the db are harmlessly ignored ─────────────────────────────

#[test]
fn unknown_keys_in_db_are_ignored() {
    let (_dir, path) = tmp_state_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = Database::create(&path).unwrap();
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(STATE_TABLE).unwrap();
        table.insert(KEY_THEME, "nord").unwrap();
        table.insert("future_feature", "42").unwrap();
        table.insert("another_new_key", "xyz").unwrap();
    }
    txn.commit().unwrap();
    drop(db);

    let state = load_state_from(&path);
    assert_eq!(state.theme, Some("nord".into()));
    // Unknown keys don't cause errors or affect known fields.
}

// ── active_pane ───────────────────────────────────────────────────────────────

#[test]
fn active_pane_none_on_fresh_install() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent").join("state.redb");
    assert!(load_state_from(&path).active_pane.is_none());
}

#[test]
fn active_pane_persists_with_full_state() {
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
        active_pane: Some("right".into()),
    };
    save_state_to(&path, &original).unwrap();
    let loaded = load_state_from(&path);
    assert_eq!(loaded, original);
}
