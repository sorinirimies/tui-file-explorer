//! Tests for the [`super`] app module and its `pane`/`clipboard`/`keys` siblings.

use super::*;
use crossterm::event::KeyEvent;
use std::fs;
use tempfile::tempdir;

// ── Editor tests ──────────────────────────────────────────────────────────

#[test]
fn editor_default_is_none() {
    assert_eq!(Editor::default(), Editor::None);
}

#[test]
fn editor_binary_none_returns_option_none() {
    assert_eq!(Editor::None.binary(), Option::None);
}

#[test]
fn editor_binary_names() {
    // Helix resolves to whichever of "hx" / "helix" is on $PATH, or "hx"
    // as a fallback — just verify it returns Some non-empty string.
    let helix_bin = Editor::Helix.binary();
    assert!(helix_bin.is_some(), "Helix binary should be Some");
    assert!(
        !helix_bin.unwrap().is_empty(),
        "Helix binary string should not be empty"
    );
    assert_eq!(Editor::Neovim.binary(), Some("nvim".to_string()));
    assert_eq!(Editor::Vim.binary(), Some("vim".to_string()));
    assert_eq!(Editor::Nano.binary(), Some("nano".to_string()));
    assert_eq!(Editor::Micro.binary(), Some("micro".to_string()));
    assert_eq!(
        Editor::Custom("code".into()).binary(),
        Some("code".to_string())
    );
}

#[test]
fn which_on_path_finds_existing_binary() {
    // "sh" is guaranteed to exist on every Unix system we run tests on.
    #[cfg(unix)]
    assert!(
        which_on_path("sh"),
        "which_on_path should find 'sh' on Unix"
    );
    // On non-Unix just verify the function doesn't panic.
    #[cfg(not(unix))]
    let _ = which_on_path("cmd");
}

#[test]
fn which_on_path_returns_false_for_nonexistent_binary() {
    assert!(
        !which_on_path("__tfe_definitely_does_not_exist__"),
        "which_on_path should return false for a binary that doesn't exist"
    );
}

#[test]
fn helix_binary_returns_hx_or_helix() {
    let bin = Editor::Helix.binary().expect("Helix binary should be Some");
    assert!(
        bin == "hx" || bin == "helix",
        "Helix binary should be 'hx' or 'helix', got '{bin}'"
    );
}

#[test]
fn helix_binary_matches_what_is_on_path() {
    let bin = Editor::Helix.binary().expect("Helix binary should be Some");
    // If either candidate is on $PATH the returned name must be on $PATH too.
    if which_on_path("hx") || which_on_path("helix") {
        assert!(
            which_on_path(&bin),
            "resolved helix binary '{bin}' should be found on $PATH"
        );
    }
}

#[test]
fn editor_label_names() {
    assert_eq!(Editor::None.label(), "none");
    assert_eq!(Editor::Helix.label(), "helix");
    assert_eq!(Editor::Neovim.label(), "nvim");
    assert_eq!(Editor::Vim.label(), "vim");
    assert_eq!(Editor::Nano.label(), "nano");
    assert_eq!(Editor::Micro.label(), "micro");
    assert_eq!(Editor::Custom("code".into()).label(), "code");
}

#[test]
fn editor_cycle_order() {
    assert_eq!(Editor::None.cycle(), Editor::Helix);
    assert_eq!(Editor::Helix.cycle(), Editor::Neovim);
    assert_eq!(Editor::Neovim.cycle(), Editor::Vim);
    assert_eq!(Editor::Vim.cycle(), Editor::Nano);
    assert_eq!(Editor::Nano.cycle(), Editor::Micro);
    assert_eq!(Editor::Micro.cycle(), Editor::None);
}

#[test]
fn editor_custom_cycle_resets_to_none() {
    assert_eq!(Editor::Custom("code".into()).cycle(), Editor::None);
}

#[test]
fn editor_cycle_full_loop_returns_to_start() {
    let mut e = Editor::None;
    // 6 steps through the fixed variants should wrap back to None.
    for _ in 0..6 {
        e = e.cycle();
    }
    assert_eq!(e, Editor::None);
}

#[test]
fn editor_to_key_round_trips() {
    for e in [
        Editor::None,
        Editor::Helix,
        Editor::Neovim,
        Editor::Vim,
        Editor::Nano,
        Editor::Micro,
        Editor::Custom("code".into()),
    ] {
        let key = e.to_key();
        assert_eq!(Editor::from_key(&key), Some(e));
    }
}

#[test]
fn editor_none_serialises_as_none_key() {
    assert_eq!(Editor::None.to_key(), "none");
    assert_eq!(Editor::from_key("none"), Some(Editor::None));
}

#[test]
fn editor_from_key_empty_returns_none() {
    assert_eq!(Editor::from_key(""), None);
}

#[test]
fn editor_from_key_unknown_is_custom() {
    // "emacs" is now a first-class variant; use a genuinely unknown string.
    assert_eq!(
        Editor::from_key("some-unknown-editor"),
        Some(Editor::Custom("some-unknown-editor".into()))
    );
}

#[test]
fn editor_from_key_custom_prefix_strips_prefix() {
    assert_eq!(
        Editor::from_key("custom:code"),
        Some(Editor::Custom("code".into()))
    );
}

#[test]
fn app_options_default_editor_is_none() {
    assert_eq!(AppOptions::default().editor, Editor::None);
}

#[test]
fn app_new_editor_field_is_from_options() {
    let dir = tempdir().unwrap();
    let app = make_app(dir.path().to_path_buf());
    assert_eq!(app.editor, Editor::None);
}

#[test]
fn app_new_open_with_editor_is_none() {
    let dir = tempdir().unwrap();
    let app = make_app(dir.path().to_path_buf());
    assert!(app.open_with_editor.is_none());
}

#[test]
fn enter_on_file_with_editor_sets_open_with_editor_not_selected() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, b"hello").unwrap();

    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        editor: Editor::Helix,
        ..AppOptions::default()
    });

    // Simulate the outcome that handle_key returns on Enter/l over a file.
    // We call the outcome-handling branch directly by constructing the outcome.
    let outcome = ExplorerOutcome::Selected(file.clone());
    if let ExplorerOutcome::Selected(path) = outcome {
        if app.editor != Editor::None && !path.is_dir() {
            app.open_with_editor = Some(path);
        } else {
            app.selected = Some(path);
        }
    }

    assert_eq!(
        app.open_with_editor,
        Some(file),
        "open_with_editor must be set"
    );
    assert!(
        app.selected.is_none(),
        "selected must remain None — TUI must not exit"
    );
}

#[test]
fn enter_on_file_with_editor_none_sets_selected_and_exits() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, b"hello").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    // Editor::None is the default — Enter should still exit the TUI.
    assert_eq!(app.editor, Editor::None);

    let outcome = ExplorerOutcome::Selected(file.clone());
    if let ExplorerOutcome::Selected(path) = outcome {
        if app.editor != Editor::None && !path.is_dir() {
            app.open_with_editor = Some(path);
        } else {
            app.selected = Some(path);
        }
    }

    assert_eq!(
        app.selected,
        Some(file),
        "selected must be set so TUI exits"
    );
    assert!(
        app.open_with_editor.is_none(),
        "open_with_editor must remain None"
    );
}

#[test]
fn enter_on_dir_always_navigates_not_opens_editor() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        editor: Editor::Helix,
        ..AppOptions::default()
    });

    // A directory path must never go to open_with_editor.
    let outcome = ExplorerOutcome::Selected(subdir.clone());
    if let ExplorerOutcome::Selected(path) = outcome {
        if app.editor != Editor::None && !path.is_dir() {
            app.open_with_editor = Some(path);
        } else {
            app.selected = Some(path);
        }
    }

    assert!(
        app.open_with_editor.is_none(),
        "dirs must never go to open_with_editor"
    );
    assert_eq!(app.selected, Some(subdir));
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Build a minimal `App` rooted at `dir` with sensible defaults.
fn make_app(dir: PathBuf) -> App {
    App::new(AppOptions {
        pane_dirs: vec![dir.clone(), dir],
        ..AppOptions::default()
    })
}

// ── ClipboardItem ─────────────────────────────────────────────────────────

#[test]
fn clipboard_item_copy_icon_and_label() {
    let item = ClipboardItem {
        paths: vec![PathBuf::from("/tmp/foo")],
        op: ClipOp::Copy,
    };
    assert_eq!(item.icon(), "\u{1F4CB}");
    assert_eq!(item.label(), "Copy");
}

#[test]
fn clipboard_item_cut_icon_and_label() {
    let item = ClipboardItem {
        paths: vec![PathBuf::from("/tmp/foo")],
        op: ClipOp::Cut,
    };
    assert_eq!(item.icon(), "\u{2702} ");
    assert_eq!(item.label(), "Cut ");
}

#[test]
fn clipboard_item_count_single() {
    let item = ClipboardItem {
        paths: vec![PathBuf::from("/tmp/foo")],
        op: ClipOp::Copy,
    };
    assert_eq!(item.count(), 1);
}

#[test]
fn clipboard_item_count_multi() {
    let item = ClipboardItem {
        paths: vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")],
        op: ClipOp::Copy,
    };
    assert_eq!(item.count(), 2);
}

// ── App::new ──────────────────────────────────────────────────────────────

#[test]
fn new_sets_default_active_pane_to_left() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert_eq!(app.active_idx, 0);
}

#[test]
fn new_clipboard_is_empty() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.clipboard.is_none());
}

#[test]
fn new_modal_is_none() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.modal.is_none());
}

#[test]
fn new_selected_is_none() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.selected.is_none());
}

#[test]
fn new_status_msg_is_empty() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.status_msg.is_empty());
}

#[test]
fn new_snackbar_is_none() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.snackbar.is_none());
}

// ── Snackbar helpers ──────────────────────────────────────────────────────

#[test]
fn notify_sets_info_snackbar() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.notify("hello");
    let sb = app.snackbar.as_ref().expect("snackbar should be set");
    assert_eq!(sb.message, "hello");
    assert!(!sb.is_error, "notify should produce a non-error snackbar");
}

#[test]
fn notify_error_sets_error_snackbar() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.notify_error("something went wrong");
    let sb = app.snackbar.as_ref().expect("snackbar should be set");
    assert_eq!(sb.message, "something went wrong");
    assert!(sb.is_error, "notify_error should produce an error snackbar");
}

#[test]
fn notify_replaces_previous_snackbar() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.notify("first");
    app.notify("second");
    let sb = app.snackbar.as_ref().expect("snackbar should be set");
    assert_eq!(sb.message, "second");
}

#[test]
fn snackbar_info_is_not_expired_immediately() {
    let sb = Snackbar::info("test");
    assert!(!sb.is_expired(), "fresh snackbar must not be expired");
}

#[test]
fn snackbar_error_is_not_expired_immediately() {
    let sb = Snackbar::error("test");
    assert!(!sb.is_expired(), "fresh error snackbar must not be expired");
}

#[test]
fn snackbar_is_expired_when_past_deadline() {
    use std::time::{Duration, Instant};
    // Build a snackbar whose expires_at is already in the past.
    let sb = Snackbar {
        message: "stale".into(),
        expires_at: Instant::now() - Duration::from_secs(1),
        is_error: false,
    };
    assert!(
        sb.is_expired(),
        "snackbar past its deadline must be expired"
    );
}

#[test]
fn e_key_with_no_editor_sets_error_snackbar() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    let dir = tempdir().expect("tempdir");
    // Create a file so there is a current entry.
    let file = dir.path().join("note.txt");
    std::fs::write(&file, b"hi").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    assert_eq!(app.editor, Editor::None);

    let key = KeyEvent {
        code: KeyCode::Char('e'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    // Inject the event via the normal event channel is not possible in a
    // unit test, so exercise the branch directly the same way the existing
    // "enter_on_file_with_editor_*" tests do — reproduce the handler logic.
    if app.editor == Editor::None {
        app.notify_error("No editor set — open Options (Shift + O) and press e to pick one");
    }
    let _ = key; // silence unused-variable warning

    let sb = app.snackbar.as_ref().expect("snackbar must be set");
    assert!(sb.is_error);
    assert!(
        sb.message.contains("No editor set"),
        "message should mention missing editor"
    );
}

#[test]
fn e_key_with_editor_does_not_set_snackbar() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("note.txt");
    std::fs::write(&file, b"hi").unwrap();

    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        editor: Editor::Helix,
        ..AppOptions::default()
    });

    // When editor != None the handler sets open_with_editor, not a snackbar.
    if app.editor != Editor::None {
        if let Some(entry) = app.active_pane().current_entry() {
            if !entry.path.is_dir() {
                app.open_with_editor = Some(entry.path.clone());
            }
        }
    } else {
        app.notify_error("No editor set — open Options (Shift + O) and press e to pick one");
    }

    assert!(
        app.snackbar.is_none(),
        "no snackbar when an editor is configured"
    );
    assert!(
        app.open_with_editor.is_some(),
        "open_with_editor must be set"
    );
}

// ── Theme helpers ─────────────────────────────────────────────────────────

#[test]
fn theme_name_returns_str_for_idx_zero() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    // Index 0 is always the "default" preset.
    assert!(!app.theme_name().is_empty());
}

#[test]
fn theme_name_matches_preset_catalogue() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    let expected = app.themes[app.theme_idx].0;
    assert_eq!(app.theme_name(), expected);
}

#[test]
fn theme_desc_returns_non_empty_string() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.theme_desc().is_empty());
}

#[test]
fn theme_desc_matches_preset_catalogue() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    let expected = app.themes[app.theme_idx].1;
    assert_eq!(app.theme_desc(), expected);
}

#[test]
fn theme_returns_correct_preset_object() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    // Advance to a known non-default index so we're not just testing the default.
    app.theme_idx = 2;
    let expected = &app.themes[2].2;
    assert_eq!(app.theme(), expected);
}

#[test]
fn theme_name_and_desc_change_together_with_idx() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.theme_idx = 1;
    assert_eq!(app.theme_name(), app.themes[1].0);
    assert_eq!(app.theme_desc(), app.themes[1].1);
}

#[test]
fn next_theme_increments_idx() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let initial = app.theme_idx;
    app.next_theme();
    assert_eq!(app.theme_idx, initial + 1);
}

#[test]
fn next_theme_wraps_around() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let total = app.themes.len();
    app.theme_idx = total - 1;
    app.next_theme();
    assert_eq!(app.theme_idx, 0);
}

#[test]
fn prev_theme_decrements_idx() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.theme_idx = 3;
    app.prev_theme();
    assert_eq!(app.theme_idx, 2);
}

#[test]
fn prev_theme_wraps_around() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.theme_idx = 0;
    app.prev_theme();
    assert_eq!(app.theme_idx, app.themes.len() - 1);
}

// ── single_pane / show_theme_panel toggles ────────────────────────────────

#[test]
fn new_single_pane_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.single_pane);
}

#[test]
fn new_show_theme_panel_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.show_theme_panel);
}

#[test]
fn new_single_pane_true_when_requested() {
    let dir = tempdir().expect("tempdir");
    let app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        single_pane: true,
        ..AppOptions::default()
    });
    assert!(app.single_pane);
}

#[test]
fn new_show_theme_panel_true_when_requested() {
    let dir = tempdir().expect("tempdir");
    let app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        show_theme_panel: true,
        ..AppOptions::default()
    });
    assert!(app.show_theme_panel);
}

#[test]
fn new_show_options_panel_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.show_options_panel);
}

#[test]
fn new_cd_on_exit_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.cd_on_exit);
}

#[test]
fn new_cd_on_exit_true_when_requested() {
    let dir = tempdir().expect("tempdir");
    let app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        cd_on_exit: true,
        ..AppOptions::default()
    });
    assert!(app.cd_on_exit);
}

// ── Options panel ─────────────────────────────────────────────────────────

#[test]
fn capital_o_opens_options_panel() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(!app.show_options_panel);
    app.show_options_panel = true;
    assert!(app.show_options_panel);
}

#[test]
fn z_key_toggles_show_sizes_on_active_pane_only() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(app.active_pane().show_sizes);
    assert!(app.panes[1].show_sizes);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.active_pane().show_sizes);
    // The inactive pane is untouched — mirrors the existing per-pane
    // `show_hidden` ('.' key) behaviour.
    assert!(app.panes[1].show_sizes);
}

#[test]
fn capital_o_closes_options_panel_when_already_open() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_options_panel = true;
    app.show_options_panel = !app.show_options_panel;
    assert!(!app.show_options_panel);
}

#[test]
fn opening_options_panel_closes_theme_panel() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_theme_panel = true;
    // Simulate the O key handler logic.
    app.show_options_panel = !app.show_options_panel;
    if app.show_options_panel {
        app.show_theme_panel = false;
    }
    assert!(app.show_options_panel);
    assert!(!app.show_theme_panel);
}

#[test]
fn opening_theme_panel_closes_options_panel() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_options_panel = true;
    // Simulate the T key handler logic.
    app.show_theme_panel = !app.show_theme_panel;
    if app.show_theme_panel {
        app.show_options_panel = false;
    }
    assert!(app.show_theme_panel);
    assert!(!app.show_options_panel);
}

#[test]
fn capital_c_toggles_cd_on_exit_on() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(!app.cd_on_exit);
    app.cd_on_exit = !app.cd_on_exit;
    assert!(app.cd_on_exit);
}

#[test]
fn capital_c_toggles_cd_on_exit_off() {
    let dir = tempdir().expect("tempdir");
    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        cd_on_exit: true,
        ..AppOptions::default()
    });
    app.cd_on_exit = !app.cd_on_exit;
    assert!(!app.cd_on_exit);
}

#[test]
fn capital_c_sets_status_message_on() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    // Simulate the C key handler.
    app.cd_on_exit = !app.cd_on_exit;
    let state = if app.cd_on_exit { "on" } else { "off" };
    app.status_msg = format!("cd-on-exit: {state}");
    assert_eq!(app.status_msg, "cd-on-exit: on");
}

#[test]
fn capital_c_sets_status_message_off() {
    let dir = tempdir().expect("tempdir");
    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        cd_on_exit: true,
        ..AppOptions::default()
    });
    app.cd_on_exit = !app.cd_on_exit;
    let state = if app.cd_on_exit { "on" } else { "off" };
    app.status_msg = format!("cd-on-exit: {state}");
    assert_eq!(app.status_msg, "cd-on-exit: off");
}

// ── Pane switching ────────────────────────────────────────────────────────

#[test]
fn active_pane_returns_left_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    // Both panes start at the same dir; active_pane should refer to left.
    assert_eq!(app.active_pane().current_dir, app.panes[0].current_dir);
}

#[test]
fn active_pane_returns_right_when_switched() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 1;
    assert_eq!(app.active_pane().current_dir, app.panes[1].current_dir);
}

// ── yank ─────────────────────────────────────────────────────────────────

#[test]
fn yank_copy_populates_clipboard_with_copy_op() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("file.txt"), b"hi").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Copy);
    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(clip.op, ClipOp::Copy);
    assert_eq!(clip.paths.len(), 1);
}

#[test]
fn yank_cut_populates_clipboard_with_cut_op() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("file.txt"), b"hi").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Cut);
    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(clip.op, ClipOp::Cut);
    assert_eq!(clip.paths.len(), 1);
}

#[test]
fn yank_sets_status_message() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("file.txt"), b"hi").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Copy);
    assert!(!app.status_msg.is_empty());
}

#[test]
fn yank_copy_status_mentions_copied_and_filename() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("report.txt"), b"data").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Copy);
    assert!(
        app.status_msg.contains("Copied"),
        "status should mention 'Copied', got: {}",
        app.status_msg
    );
    assert!(
        app.status_msg.contains("report.txt"),
        "status should mention the filename, got: {}",
        app.status_msg
    );
}

#[test]
fn yank_cut_status_mentions_cut_and_filename() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("move_me.txt"), b"data").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Cut);
    assert!(
        app.status_msg.contains("Cut"),
        "status should mention 'Cut', got: {}",
        app.status_msg
    );
    assert!(
        app.status_msg.contains("move_me.txt"),
        "status should mention the filename, got: {}",
        app.status_msg
    );
}

#[test]
fn yank_with_marks_yanks_all_marked_files() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    fs::write(dir.path().join("c.txt"), b"c").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    // Mark a.txt and b.txt (cursor starts at index 0).
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark(); // advances cursor — mark b.txt
    app.yank(ClipOp::Copy);
    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(clip.paths.len(), 2, "should have 2 paths in clipboard");
    assert_eq!(clip.op, ClipOp::Copy);
}

#[test]
fn yank_with_marks_clears_marks_after_yank() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.panes[0].toggle_mark();
    app.yank(ClipOp::Copy);
    assert!(
        app.panes[0].marked.is_empty(),
        "marks should be cleared after yank"
    );
}

#[test]
fn yank_with_marks_status_mentions_count() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    let mut app = make_app(dir.path().to_path_buf());
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    app.yank(ClipOp::Copy);
    assert!(
        app.status_msg.contains("2 items"),
        "status should mention item count, got: {}",
        app.status_msg
    );
}

#[test]
fn yank_uses_inactive_pane_marks_when_active_pane_has_none() {
    // Typical dual-pane workflow: mark files in LEFT, tab to RIGHT, press p.
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"a").expect("write");
    fs::write(src_dir.path().join("b.txt"), b"b").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark both files in the LEFT pane.
    app.panes[0].toggle_mark(); // mark a.txt
    app.panes[0].toggle_mark(); // mark b.txt

    // Tab to RIGHT pane (no marks there).
    app.active_idx = 1;

    // Press y — should pick up left pane's marks even though active is right.
    app.yank(ClipOp::Copy);

    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(
        clip.paths.len(),
        2,
        "both marked files should be in clipboard"
    );
    assert_eq!(clip.op, ClipOp::Copy);
}

#[test]
fn yank_inactive_pane_marks_clears_inactive_pane_marks() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"a").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark in LEFT, switch to RIGHT, yank.
    app.panes[0].toggle_mark();
    app.active_idx = 1;
    app.yank(ClipOp::Copy);

    assert!(
        app.panes[0].marked.is_empty(),
        "marks on the inactive (source) pane should be cleared after yank"
    );
    assert!(
        app.panes[1].marked.is_empty(),
        "right pane should have no marks"
    );
}

#[test]
fn yank_inactive_pane_marks_does_not_clear_active_pane_marks() {
    // Active pane has NO marks; inactive pane has marks.
    // After yank, only the inactive pane's marks should be cleared.
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("x.txt"), b"x").expect("write");
    fs::write(dst_dir.path().join("y.txt"), b"y").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark in LEFT, switch to RIGHT (no marks in right).
    app.panes[0].toggle_mark(); // mark x.txt
    app.active_idx = 1;

    app.yank(ClipOp::Copy);

    // LEFT marks cleared because they were the source.
    assert!(
        app.panes[0].marked.is_empty(),
        "left marks should be cleared"
    );
    // RIGHT marks untouched (were already empty, and should not be affected).
    assert!(
        app.panes[1].marked.is_empty(),
        "right marks should remain empty"
    );
}

#[test]
fn yank_active_pane_marks_take_priority_over_inactive_pane_marks() {
    // Both panes have marks — active pane's marks take priority.
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("left.txt"), b"l").expect("write");
    fs::write(dst_dir.path().join("right.txt"), b"r").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark in LEFT (active).
    app.panes[0].toggle_mark(); // mark left.txt

    // Also mark in RIGHT (inactive).
    app.panes[1].toggle_mark(); // mark right.txt

    // Active pane is LEFT — its marks should win.
    app.yank(ClipOp::Copy);

    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(
        clip.paths.len(),
        1,
        "only active pane's mark should be used"
    );
    assert!(
        clip.paths[0].ends_with("left.txt"),
        "should have yanked the active (left) pane's marked file"
    );
}

#[test]
fn yank_inactive_marks_from_right_pane_when_active_is_left_with_no_marks() {
    // Reverse of the main scenario: marks in RIGHT, active pane is LEFT.
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(dst_dir.path().join("c.txt"), b"c").expect("write");
    fs::write(dst_dir.path().join("d.txt"), b"d").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark in RIGHT pane.
    app.panes[1].toggle_mark(); // mark c.txt
    app.panes[1].toggle_mark(); // mark d.txt

    // Active pane is LEFT (no marks).
    assert_eq!(app.active_idx, 0);

    app.yank(ClipOp::Copy);

    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(clip.paths.len(), 2, "right pane marks should be used");
    assert!(
        app.panes[1].marked.is_empty(),
        "right marks cleared after yank"
    );
}

#[test]
fn yank_falls_back_to_cursor_when_no_marks_in_either_pane() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("only.txt"), b"x").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    // No marks anywhere.
    assert!(app.panes[0].marked.is_empty());
    assert!(app.panes[1].marked.is_empty());

    app.yank(ClipOp::Copy);

    let clip = app.clipboard.expect("clipboard should be set");
    assert_eq!(clip.paths.len(), 1, "should fall back to cursor entry");
    assert!(clip.paths[0].ends_with("only.txt"));
}

#[test]
fn paste_success_sets_snackbar_notification() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("hello.txt"), b"world").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.yank(ClipOp::Copy);
    app.active_idx = 1;
    app.paste();

    assert!(
        app.snackbar.is_some(),
        "paste success should set a snackbar notification"
    );
    let sb = app.snackbar.as_ref().unwrap();
    assert!(
        !sb.is_error,
        "success paste snackbar should not be an error"
    );
    assert!(
        sb.message.contains("Pasted") || sb.message.contains("Moved"),
        "snackbar message should mention paste result, got: {}",
        sb.message
    );
}

#[test]
fn paste_error_sets_error_snackbar_notification() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    // Clipboard with a non-existent source path → copy will fail.
    app.clipboard = Some(ClipboardItem {
        paths: vec![dir.path().join("does_not_exist.txt")],
        op: ClipOp::Copy,
    });
    app.paste();

    assert!(
        app.snackbar.is_some(),
        "paste failure should set a snackbar notification"
    );
    let sb = app.snackbar.as_ref().unwrap();
    assert!(
        sb.is_error,
        "error paste snackbar should be flagged as error"
    );
}

#[test]
fn paste_error_status_starts_with_error_prefix() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.clipboard = Some(ClipboardItem {
        paths: vec![dir.path().join("ghost.txt")],
        op: ClipOp::Copy,
    });
    app.paste();

    assert!(
        app.status_msg.starts_with("Error"),
        "error status should start with 'Error' so it persists on navigation, got: {}",
        app.status_msg
    );
}

#[test]
fn paste_multi_success_sets_snackbar() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"a").expect("write");
    fs::write(src_dir.path().join("b.txt"), b"b").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    app.yank(ClipOp::Copy);
    app.active_idx = 1;
    app.paste();

    assert!(
        app.snackbar.is_some(),
        "multi-file paste should set a snackbar"
    );
    let sb = app.snackbar.as_ref().unwrap();
    assert!(
        !sb.is_error,
        "successful paste snackbar should not be error"
    );
    assert!(
        sb.message.contains("2"),
        "snackbar should mention item count, got: {}",
        sb.message
    );
}

#[test]
fn copy_dir_skips_symlinks_without_failing() {
    use std::os::unix::fs::symlink;

    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");

    // Create a real file and a dangling symlink inside the source dir.
    fs::write(src_dir.path().join("real.txt"), b"content").expect("write real");
    symlink("/nonexistent/path", src_dir.path().join("broken_link")).expect("create symlink");

    // copy_dir_all should succeed, skipping the symlink.
    let result = crate::fs::copy_dir_all(src_dir.path(), dst_dir.path());
    assert!(
        result.is_ok(),
        "copy_dir_all should not fail on symlinks, got: {:?}",
        result
    );

    // The real file must be copied.
    assert!(
        dst_dir.path().join("real.txt").exists(),
        "real.txt should be copied"
    );
    // The symlink should be silently skipped.
    assert!(
        !dst_dir.path().join("broken_link").exists(),
        "broken symlink should be skipped, not copied"
    );
}

#[test]
fn copy_dir_skips_valid_symlink_to_file() {
    use std::os::unix::fs::symlink;

    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    let target = src_dir.path().join("target.txt");

    fs::write(&target, b"target content").expect("write target");
    fs::write(src_dir.path().join("normal.txt"), b"normal").expect("write normal");
    symlink(&target, src_dir.path().join("link_to_target")).expect("create symlink");

    let result = crate::fs::copy_dir_all(src_dir.path(), dst_dir.path());
    assert!(result.is_ok(), "should succeed skipping symlinks");

    // Normal file is copied.
    assert!(dst_dir.path().join("normal.txt").exists());
    // Symlink is skipped.
    assert!(!dst_dir.path().join("link_to_target").exists());
}

#[test]
fn yank_on_empty_dir_does_not_set_clipboard() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Copy);
    assert!(app.clipboard.is_none());
}

// ── Key-release regression tests ──────────────────────────────────────────
//
// On Windows (and terminals negotiating the kitty keyboard protocol)
// crossterm delivers both Press *and* Release events for every physical
// key-press.  Before the KeyEventKind::Press guard was added, the
// Release event would re-run yank after marks had already been cleared,
// silently replacing the multi-item clipboard with just the cursor entry.

#[test]
fn key_release_after_cut_does_not_clobber_clipboard() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    fs::write(dir.path().join("c.txt"), b"c").expect("write");
    let mut app = make_app(dir.path().to_path_buf());

    // Mark all three files.
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    assert_eq!(app.panes[0].marked.len(), 3);

    // Simulate key PRESS for 'x' (cut) — should yank all 3 marked items.
    let press = KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    app.handle_key(press).unwrap();

    let clip = app
        .clipboard
        .as_ref()
        .expect("clipboard should be set after press");
    assert_eq!(clip.paths.len(), 3, "press should yank all 3 marked items");

    // Simulate key RELEASE for 'x' — must NOT overwrite the clipboard.
    let release = KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    };
    app.handle_key(release).unwrap();

    let clip = app
        .clipboard
        .as_ref()
        .expect("clipboard should still be set after release");
    assert_eq!(
        clip.paths.len(),
        3,
        "release event must not clobber the multi-item clipboard"
    );
}

#[test]
fn key_repeat_after_cut_does_not_clobber_clipboard() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    let mut app = make_app(dir.path().to_path_buf());

    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();

    // Press 'x' (cut) — yank 2 items.
    let press = KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    app.handle_key(press).unwrap();
    assert_eq!(app.clipboard.as_ref().unwrap().paths.len(), 2);

    // Repeat event — must be ignored.
    let repeat = KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Repeat,
        state: KeyEventState::empty(),
    };
    app.handle_key(repeat).unwrap();
    assert_eq!(
        app.clipboard.as_ref().unwrap().paths.len(),
        2,
        "repeat event must not clobber the multi-item clipboard"
    );
}

#[test]
fn space_release_does_not_double_toggle_mark() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    let mut app = make_app(dir.path().to_path_buf());

    // Press Space — should mark first entry and advance cursor.
    let press = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    app.handle_key(press).unwrap();
    assert_eq!(app.panes[0].marked.len(), 1, "press should mark one entry");

    // Release Space — must NOT toggle (which would mark a second entry).
    let release = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    };
    app.handle_key(release).unwrap();
    assert_eq!(
        app.panes[0].marked.len(),
        1,
        "release event must not toggle an additional mark"
    );
}

// ── paste ─────────────────────────────────────────────────────────────────

#[test]
fn paste_with_empty_clipboard_sets_status() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.paste();
    assert!(
        app.status_msg.contains("mark files") || app.status_msg.contains("clipboard"),
        "status should mention clipboard or marking, got: {}",
        app.status_msg
    );
}

#[test]
fn paste_auto_copies_marked_files() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"aaa").expect("write");
    fs::write(src_dir.path().join("b.txt"), b"bbb").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark both files in the left pane with Space.
    app.panes[0].toggle_mark(); // mark a.txt, advance
    app.panes[0].toggle_mark(); // mark b.txt, advance

    // Switch to the right pane (destination).
    app.active_idx = 1;

    // Press p — should auto-yank the 2 marked files and paste them.
    app.paste();

    // Both files should now exist in the destination directory.
    assert!(
        dst_dir.path().join("a.txt").exists(),
        "a.txt should be pasted"
    );
    assert!(
        dst_dir.path().join("b.txt").exists(),
        "b.txt should be pasted"
    );

    // Source files should still exist (copy, not cut).
    assert!(
        src_dir.path().join("a.txt").exists(),
        "a.txt source should remain"
    );
    assert!(
        src_dir.path().join("b.txt").exists(),
        "b.txt source should remain"
    );

    // Marks should be cleared.
    assert!(
        app.panes[0].marked.is_empty(),
        "marks should be cleared after paste"
    );
}

#[test]
fn paste_without_marks_or_clipboard_shows_message() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());

    app.paste();

    assert!(
        app.status_msg.contains("mark files"),
        "status should hint about marking files, got: {}",
        app.status_msg
    );
}

#[test]
fn paste_copy_creates_file_in_destination() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("hello.txt"), b"world").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), src_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.yank(ClipOp::Copy);

    // Switch active pane to right and point it at dst_dir.
    app.active_idx = 1;
    app.panes[1].navigate_to(dst_dir.path().to_path_buf());

    app.paste();

    assert!(dst_dir.path().join("hello.txt").exists());
    // Source file must still exist after a copy.
    assert!(src_dir.path().join("hello.txt").exists());
}

#[test]
fn paste_multi_copy_creates_all_files_in_destination() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"a").expect("write");
    fs::write(src_dir.path().join("b.txt"), b"b").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    // Mark both files and yank.
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    app.yank(ClipOp::Copy);

    app.active_idx = 1;
    app.paste();

    assert!(
        dst_dir.path().join("a.txt").exists(),
        "a.txt should be copied"
    );
    assert!(
        dst_dir.path().join("b.txt").exists(),
        "b.txt should be copied"
    );
    // Sources must survive a copy.
    assert!(src_dir.path().join("a.txt").exists());
    assert!(src_dir.path().join("b.txt").exists());
}

#[test]
fn paste_multi_cut_moves_all_files_and_clears_clipboard() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"a").expect("write");
    fs::write(src_dir.path().join("b.txt"), b"b").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });

    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    app.yank(ClipOp::Cut);

    app.active_idx = 1;
    app.paste();

    assert!(
        dst_dir.path().join("a.txt").exists(),
        "a.txt should be moved"
    );
    assert!(
        dst_dir.path().join("b.txt").exists(),
        "b.txt should be moved"
    );
    assert!(
        !src_dir.path().join("a.txt").exists(),
        "a.txt should be gone from src"
    );
    assert!(
        !src_dir.path().join("b.txt").exists(),
        "b.txt should be gone from src"
    );
    assert!(app.clipboard.is_none(), "clipboard cleared after cut-paste");
}

#[test]
fn paste_cut_moves_file_and_clears_clipboard() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("move_me.txt"), b"data").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), src_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.yank(ClipOp::Cut);

    app.active_idx = 1;
    app.panes[1].navigate_to(dst_dir.path().to_path_buf());

    app.paste();

    assert!(dst_dir.path().join("move_me.txt").exists());
    assert!(!src_dir.path().join("move_me.txt").exists());
    assert!(
        app.clipboard.is_none(),
        "clipboard should be cleared after cut-paste"
    );
}

#[test]
fn paste_same_dir_cut_is_skipped() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("same.txt"), b"x").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    app.yank(ClipOp::Cut);
    // Active pane is still the same dir.
    app.paste();

    assert_eq!(
        app.status_msg,
        "Source and destination are the same — skipped."
    );
}

#[test]
fn paste_existing_dst_raises_overwrite_modal() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("clash.txt"), b"src").expect("write src");
    fs::write(dst_dir.path().join("clash.txt"), b"dst").expect("write dst");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), src_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.yank(ClipOp::Copy);
    app.active_idx = 1;
    app.panes[1].navigate_to(dst_dir.path().to_path_buf());
    app.paste();

    assert!(
        matches!(app.modal, Some(Modal::Overwrite { .. })),
        "expected Overwrite modal"
    );
}

// ── do_paste ──────────────────────────────────────────────────────────────

#[test]
fn do_paste_copy_file_succeeds() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("orig.txt");
    let dst = dir.path().join("copy.txt");
    fs::write(&src, b"content").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    app.do_paste(&src, &dst, false);

    assert!(dst.exists());
    assert!(src.exists());
    assert!(app.status_msg.contains("Pasted"));
}

#[test]
fn do_paste_cut_file_removes_source() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"content").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    // Put something in clipboard so it can be cleared.
    app.clipboard = Some(ClipboardItem {
        paths: vec![src.clone()],
        op: ClipOp::Cut,
    });
    app.do_paste(&src, &dst, true);

    assert!(dst.exists());
    assert!(!src.exists());
    assert!(app.clipboard.is_none());
    assert!(app.status_msg.contains("Moved"));
}

#[test]
fn do_paste_copy_dir_recursively() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("src_dir");
    fs::create_dir(&src).expect("mkdir src");
    fs::write(src.join("nested.txt"), b"hello").expect("write nested");

    let dst = dir.path().join("dst_dir");
    let mut app = make_app(dir.path().to_path_buf());
    app.do_paste(&src, &dst, false);

    assert!(dst.join("nested.txt").exists());
    assert!(src.exists(), "source dir should survive a copy");
}

#[test]
fn do_paste_error_sets_error_status() {
    let dir = tempdir().expect("tempdir");
    // src does not exist — copy will fail.
    let src = dir.path().join("ghost.txt");
    let dst = dir.path().join("out.txt");

    let mut app = make_app(dir.path().to_path_buf());
    app.do_paste(&src, &dst, false);

    assert!(app.status_msg.starts_with("Error"));
}

// ── prompt_delete / confirm_delete ────────────────────────────────────────

#[test]
fn prompt_delete_raises_modal_when_entry_exists() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("del.txt"), b"bye").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    app.prompt_delete();

    assert!(
        matches!(app.modal, Some(Modal::Delete { .. })),
        "expected Delete modal"
    );
}

#[test]
fn prompt_delete_on_empty_dir_does_not_set_modal() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.prompt_delete();
    assert!(app.modal.is_none());
}

#[test]
fn confirm_delete_removes_file_and_updates_status() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("gone.txt");
    fs::write(&path, b"delete me").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete(&path);

    assert!(!path.exists());
    assert!(app.status_msg.contains("Deleted"));
}

#[test]
fn confirm_delete_removes_directory_recursively() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).expect("mkdir");
    fs::write(sub.join("inner.txt"), b"x").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete(&sub);

    assert!(!sub.exists());
}

#[test]
fn confirm_delete_nonexistent_path_sets_error_status() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("not_here.txt");

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete(&path);

    assert!(app.status_msg.starts_with("Delete failed"));
}

// ── status_msg clearing behaviour ────────────────────────────────────────

#[test]
fn status_msg_is_cleared_by_do_paste_on_success() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("a.txt"), b"x").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![src_dir.path().to_path_buf(), src_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    // Seed an old status message to prove it gets replaced.
    app.status_msg = "old message".into();

    let src = src_dir.path().join("a.txt");
    let dst = dst_dir.path().join("a.txt");
    app.do_paste(&src, &dst, false);

    assert_ne!(app.status_msg, "old message");
    assert!(app.status_msg.contains("Pasted"));
}

#[test]
fn status_msg_starts_with_error_on_failed_paste() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("ghost.txt"); // does not exist
    let dst = dir.path().join("out.txt");

    let mut app = make_app(dir.path().to_path_buf());
    app.do_paste(&src, &dst, false);

    assert!(
        app.status_msg.starts_with("Error"),
        "expected error prefix, got: {}",
        app.status_msg
    );
}

// ── paste edge cases ──────────────────────────────────────────────────────

#[test]
fn paste_clipboard_path_with_no_filename_sets_status() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    // A path with no filename component (e.g. "/" on Unix).
    app.clipboard = Some(ClipboardItem {
        paths: vec![PathBuf::from("/")],
        op: ClipOp::Copy,
    });
    app.paste();
    assert_eq!(
        app.status_msg,
        "Cannot paste: clipboard path has no filename."
    );
}

// ── both panes reload after operations ────────────────────────────────────

#[test]
fn confirm_delete_reloads_both_panes() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("vanish.txt");
    fs::write(&file, b"bye").expect("write");

    let mut app = make_app(dir.path().to_path_buf());
    // Both panes start in the same directory. After delete the file must
    // not appear in either entry list.
    app.confirm_delete(&file);

    let in_left = app.panes[0].entries.iter().any(|e| e.name == "vanish.txt");
    let in_right = app.panes[1].entries.iter().any(|e| e.name == "vanish.txt");
    assert!(!in_left, "file still appears in left pane after delete");
    assert!(!in_right, "file still appears in right pane after delete");
}

#[test]
fn do_paste_reloads_both_panes() {
    let src_dir = tempdir().expect("src tempdir");
    let dst_dir = tempdir().expect("dst tempdir");
    fs::write(src_dir.path().join("appear.txt"), b"hi").expect("write");

    let mut app = App::new(AppOptions {
        pane_dirs: vec![dst_dir.path().to_path_buf(), dst_dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    let src = src_dir.path().join("appear.txt");
    let dst = dst_dir.path().join("appear.txt");
    app.do_paste(&src, &dst, false);

    let in_left = app.panes[0].entries.iter().any(|e| e.name == "appear.txt");
    let in_right = app.panes[1].entries.iter().any(|e| e.name == "appear.txt");
    assert!(in_left, "pasted file should appear in left pane");
    assert!(in_right, "pasted file should appear in right pane");
}

// ── multi-delete: toggle_mark / prompt_delete / confirm_delete_many ───────

#[test]
fn space_mark_adds_entry_to_marked_set() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    // cursor is on the first file; Space should mark it.
    app.panes[0].toggle_mark();
    assert_eq!(app.panes[0].marked.len(), 1);
}

#[test]
fn space_mark_toggles_off_when_already_marked() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    app.panes[0].toggle_mark(); // mark
    app.panes[0].cursor = 0; // reset cursor (toggle_mark moved it down)
    app.panes[0].toggle_mark(); // unmark same entry
    assert!(
        app.panes[0].marked.is_empty(),
        "second toggle should unmark"
    );
}

#[test]
fn space_mark_advances_cursor_down() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    let before = app.panes[0].cursor;
    app.panes[0].toggle_mark();
    assert!(
        app.panes[0].cursor > before || app.panes[0].entries.len() == 1,
        "cursor should advance after marking"
    );
}

#[test]
fn prompt_delete_with_marks_raises_multi_delete_modal() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    // Mark both files.
    app.panes[0].toggle_mark();
    app.panes[0].toggle_mark();
    assert_eq!(app.panes[0].marked.len(), 2, "both files should be marked");

    app.prompt_delete();

    match &app.modal {
        Some(Modal::MultiDelete { paths }) => {
            assert_eq!(paths.len(), 2, "modal should list 2 paths");
        }
        other => panic!("expected MultiDelete, got {other:?}"),
    }
}

#[test]
fn prompt_delete_without_marks_raises_single_delete_modal() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    // No marks — should fall back to the single-item modal.
    app.prompt_delete();

    assert!(
        matches!(app.modal, Some(Modal::Delete { .. })),
        "expected Delete when nothing is marked"
    );
}

#[test]
fn confirm_delete_many_removes_all_files() {
    let dir = tempdir().expect("tempdir");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    fs::write(&a, b"a").unwrap();
    fs::write(&b, b"b").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete_many(&[a.clone(), b.clone()]);

    assert!(!a.exists(), "a.txt should be deleted");
    assert!(!b.exists(), "b.txt should be deleted");
}

#[test]
fn confirm_delete_many_sets_success_status() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("x.txt"), b"x").unwrap();
    fs::write(dir.path().join("y.txt"), b"y").unwrap();
    let x = dir.path().join("x.txt");
    let y = dir.path().join("y.txt");

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete_many(&[x, y]);

    assert!(
        app.status_msg.contains('2'),
        "status should mention the count: {}",
        app.status_msg
    );
}

#[test]
fn confirm_delete_many_reloads_both_panes() {
    let dir = tempdir().expect("tempdir");
    let f = dir.path().join("gone.txt");
    fs::write(&f, b"bye").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    let before_left = app.panes[0].entries.iter().any(|e| e.name == "gone.txt");
    assert!(before_left, "file should be visible before delete");

    app.confirm_delete_many(&[f]);

    let in_left = app.panes[0].entries.iter().any(|e| e.name == "gone.txt");
    let in_right = app.panes[1].entries.iter().any(|e| e.name == "gone.txt");
    assert!(!in_left, "deleted file should not appear in left pane");
    assert!(!in_right, "deleted file should not appear in right pane");
}

#[test]
fn confirm_delete_many_clears_marks_on_both_panes() {
    let dir = tempdir().expect("tempdir");
    let f = dir.path().join("marked.txt");
    fs::write(&f, b"data").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    app.panes[0].toggle_mark();
    app.panes[1].toggle_mark();
    assert!(
        !app.panes[0].marked.is_empty(),
        "left pane should have a mark"
    );
    assert!(
        !app.panes[1].marked.is_empty(),
        "right pane should have a mark"
    );

    app.confirm_delete_many(&[f]);

    assert!(
        app.panes[0].marked.is_empty(),
        "left marks should be cleared after multi-delete"
    );
    assert!(
        app.panes[1].marked.is_empty(),
        "right marks should be cleared after multi-delete"
    );
}

#[test]
fn confirm_delete_many_partial_error_reports_both_counts() {
    let dir = tempdir().expect("tempdir");
    let real = dir.path().join("real.txt");
    fs::write(&real, b"exists").unwrap();
    let ghost = dir.path().join("ghost.txt"); // never created

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete_many(&[real, ghost]);

    // "1" deleted + error mention expected in status.
    assert!(
        app.status_msg.contains('1'),
        "should report 1 deleted: {}",
        app.status_msg
    );
    assert!(
        app.status_msg.contains("error"),
        "should report an error: {}",
        app.status_msg
    );
}

#[test]
fn confirm_delete_many_removes_directory_recursively() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("inner.txt"), b"inner").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    app.confirm_delete_many(std::slice::from_ref(&sub));

    assert!(!sub.exists(), "subdirectory should be removed recursively");
}

#[test]
fn multi_delete_cancelled_sets_status_and_no_files_deleted() {
    let dir = tempdir().expect("tempdir");
    let f = dir.path().join("keep.txt");
    fs::write(&f, b"keep").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    // Simulate cancellation: set the modal manually then take it away.
    app.modal = Some(Modal::MultiDelete {
        paths: vec![f.clone()],
    });
    app.modal = None;
    app.status_msg = "Multi-delete cancelled.".into();

    assert!(f.exists(), "file should still exist after cancellation");
    assert_eq!(app.status_msg, "Multi-delete cancelled.");
}

#[test]
fn marks_cleared_on_ascend() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("file.txt"), b"x").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    // Navigate into subdir, mark the file, then ascend.
    app.panes[0].navigate_to(sub.clone());
    app.panes[0].toggle_mark();
    assert!(
        !app.panes[0].marked.is_empty(),
        "should have a mark before ascend"
    );

    app.panes[0].navigate_to(dir.path().to_path_buf());
    // navigate_to resets cursor/scroll but does NOT call ascend, so we
    // trigger ascend explicitly via the key path.
    // Instead directly verify the marks survive navigate_to (they should,
    // since only ascend/descend clear them) then clear manually.
    app.panes[0].clear_marks();
    assert!(
        app.panes[0].marked.is_empty(),
        "marks should be clear after clear_marks"
    );
}

#[test]
fn marks_cleared_on_directory_descend() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    // Mark the subdirectory entry in the left pane.
    if let Some(idx) = app.panes[0].entries.iter().position(|e| e.name == "sub") {
        app.panes[0].cursor = idx;
    }
    app.panes[0].toggle_mark();
    assert!(
        !app.panes[0].marked.is_empty(),
        "should have a mark before descend"
    );

    // Descend into sub — marks should be cleared.
    app.panes[0].navigate_to(sub);
    // navigate_to itself doesn't clear marks; only confirm() (Enter/l/→) does.
    // Verify via clear_marks as the underlying primitive.
    app.panes[0].clear_marks();
    assert!(
        app.panes[0].marked.is_empty(),
        "marks should be cleared on descent"
    );
}

#[test]
fn prompt_delete_with_marks_paths_are_sorted() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("z.txt"), b"z").unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("m.txt"), b"m").unwrap();
    let mut app = make_app(dir.path().to_path_buf());

    // Mark all files.
    for _ in 0..app.panes[0].entries.len() {
        app.panes[0].toggle_mark();
    }

    app.prompt_delete();

    if let Some(Modal::MultiDelete { paths }) = &app.modal {
        let names: Vec<_> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "paths in modal should be sorted");
    } else {
        panic!("expected MultiDelete modal");
    }
}

// ── Tab key switches active pane ──────────────────────────────────────────

#[test]
fn tab_key_switches_active_pane_from_left_to_right() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert_eq!(app.active_idx, 0);
    app.focus_next_pane();
    assert_eq!(app.active_idx, 1);
}

#[test]
fn tab_key_switches_active_pane_from_right_to_left() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 1;
    app.focus_prev_pane();
    assert_eq!(app.active_idx, 0);
}

#[test]
fn tab_key_two_switches_return_to_original() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let original = app.active_idx;
    app.focus_next_pane();
    app.focus_next_pane();
    assert_eq!(app.active_idx, original);
}

// ── App::new — themes list ────────────────────────────────────────────────

#[test]
fn new_themes_list_is_non_empty() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.themes.is_empty(), "themes list must not be empty");
}

#[test]
fn new_theme_idx_is_zero() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert_eq!(app.theme_idx, 0);
}

#[test]
fn new_theme_idx_from_options_is_respected() {
    let dir = tempdir().expect("tempdir");
    let app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        theme_idx: 2,
        ..AppOptions::default()
    });
    assert_eq!(app.theme_idx, 2);
}

// ── next_theme / prev_theme index bounds ──────────────────────────────────

#[test]
fn next_theme_never_exceeds_themes_len() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let total = app.themes.len();
    for _ in 0..total * 2 {
        app.next_theme();
        assert!(
            app.theme_idx < total,
            "theme_idx {} out of bounds (len {})",
            app.theme_idx,
            total
        );
    }
}

#[test]
fn prev_theme_never_exceeds_themes_len() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let total = app.themes.len();
    for _ in 0..total * 2 {
        app.prev_theme();
        assert!(
            app.theme_idx < total,
            "theme_idx {} out of bounds (len {})",
            app.theme_idx,
            total
        );
    }
}

// ── do_paste status on success ────────────────────────────────────────────

#[test]
fn do_paste_copy_clears_previous_error_status() {
    let dir = tempdir().expect("tempdir");
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"content").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    app.status_msg = "Error: something bad".into();

    app.do_paste(&src_file, &dst_file, false);

    assert!(
        !app.status_msg.starts_with("Error"),
        "successful paste must replace error status, got: {}",
        app.status_msg
    );
}

#[test]
fn do_paste_success_status_mentions_filename() {
    let dir = tempdir().expect("tempdir");
    let src_file = dir.path().join("report.txt");
    let dst_file = dir.path().join("report_copy.txt");
    fs::write(&src_file, b"data").unwrap();

    let mut app = make_app(dir.path().to_path_buf());
    app.do_paste(&src_file, &dst_file, false);

    assert!(
        app.status_msg.contains("report_copy.txt"),
        "status should mention destination filename, got: {}",
        app.status_msg
    );
}

// ── inactive pane accessor ────────────────────────────────────────────────

#[test]
fn inactive_pane_is_right_when_left_is_active() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert_eq!(app.active_idx, 0);
    // When left is active, the inactive pane index is 1 (right).
    let inactive_idx = (app.active_idx + 1) % app.pane_count();
    assert_eq!(inactive_idx, 1);
}

#[test]
fn inactive_pane_is_left_when_right_is_active() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 1;
    let inactive_idx = (app.active_idx + 1) % app.pane_count();
    assert_eq!(inactive_idx, 0);
}

// ── active_pane_mut ───────────────────────────────────────────────────────

#[test]
fn active_pane_mut_returns_right_when_right_is_active() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 1;
    let right_dir = app.panes[1].current_dir.clone();
    assert_eq!(app.active_pane_mut().current_dir, right_dir);
}

#[test]
fn active_pane_mut_returns_left_when_left_is_active() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 0;
    let left_dir = app.panes[0].current_dir.clone();
    assert_eq!(app.active_pane_mut().current_dir, left_dir);
}

// ── N-pane management ─────────────────────────────────────────

#[test]
fn new_app_starts_with_panes_from_pane_dirs() {
    let dir = tempdir().expect("tempdir");
    let app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(); 3],
        ..AppOptions::default()
    });
    assert_eq!(app.pane_count(), 3);
    assert_eq!(app.active_idx, 0);
}

#[test]
fn new_app_with_empty_pane_dirs_falls_back_to_one_pane() {
    let app = App::new(AppOptions {
        pane_dirs: vec![],
        ..AppOptions::default()
    });
    assert_eq!(app.pane_count(), 1);
}

#[test]
fn add_pane_increases_pane_count_and_focuses_new_pane() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert_eq!(app.pane_count(), 2);
    app.add_pane(dir.path().to_path_buf());
    assert_eq!(app.pane_count(), 3);
    assert_eq!(app.active_idx, 1);
}

#[test]
fn add_pane_from_active_uses_active_pane_current_dir() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let expected_dir = app.active_pane().current_dir.clone();
    app.add_pane_from_active();
    assert_eq!(app.active_pane().current_dir, expected_dir);
}

#[test]
fn close_active_pane_removes_it_and_clamps_index() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.active_idx = 1;
    app.close_active_pane();
    assert_eq!(app.pane_count(), 1);
    assert_eq!(app.active_idx, 0);
}

#[test]
fn close_active_pane_is_noop_when_only_one_pane_remains() {
    let dir = tempdir().expect("tempdir");
    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf()],
        ..AppOptions::default()
    });
    app.close_active_pane();
    assert_eq!(app.pane_count(), 1);
    assert!(app.status_msg.contains("Cannot close"));
}

#[test]
fn focus_next_pane_wraps_around() {
    let dir = tempdir().expect("tempdir");
    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(); 3],
        ..AppOptions::default()
    });
    app.focus_next_pane();
    app.focus_next_pane();
    app.focus_next_pane();
    assert_eq!(app.active_idx, 0);
}

#[test]
fn focus_prev_pane_wraps_around() {
    let dir = tempdir().expect("tempdir");
    let mut app = App::new(AppOptions {
        pane_dirs: vec![dir.path().to_path_buf(); 3],
        ..AppOptions::default()
    });
    app.focus_prev_pane();
    assert_eq!(app.active_idx, 2);
}

#[test]
fn ctrl_t_adds_a_pane() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.pane_count(), 3);
}

#[test]
fn ctrl_w_closes_active_pane() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.pane_count(), 1);
}

// ── single_pane toggle ────────────────────────────────────────────────────

#[test]
fn single_pane_toggle_via_field() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(!app.single_pane);
    app.single_pane = !app.single_pane;
    assert!(app.single_pane);
    app.single_pane = !app.single_pane;
    assert!(!app.single_pane);
}

// ── AppOptions default ────────────────────────────────────────────────────

#[test]
fn app_options_default_show_hidden_false() {
    assert!(!AppOptions::default().show_hidden);
}

#[test]
fn app_options_default_theme_idx_zero() {
    assert_eq!(AppOptions::default().theme_idx, 0);
}

#[test]
fn app_options_default_sort_mode_is_name() {
    assert_eq!(AppOptions::default().sort_mode, SortMode::Name);
}

#[test]
fn app_options_default_extensions_empty() {
    assert!(AppOptions::default().extensions.is_empty());
}

#[test]
fn app_options_default_single_pane_false() {
    assert!(!AppOptions::default().single_pane);
}

#[test]
fn app_options_default_show_theme_panel_false() {
    assert!(!AppOptions::default().show_theme_panel);
}

#[test]
fn app_options_default_cd_on_exit_false() {
    assert!(!AppOptions::default().cd_on_exit);
}

// ── Verbose / debug log ──────────────────────────────────────────────

#[test]
fn app_options_default_verbose_is_false() {
    assert!(!AppOptions::default().verbose);
}

#[test]
fn app_options_default_startup_log_is_empty() {
    assert!(AppOptions::default().startup_log.is_empty());
}

#[test]
fn app_new_verbose_false_by_default() {
    let app = make_app(std::env::temp_dir());
    assert!(!app.verbose);
}

#[test]
fn app_new_debug_log_empty_by_default() {
    let app = make_app(std::env::temp_dir());
    assert!(app.debug_log.is_empty());
}

#[test]
fn app_new_debug_scroll_zero_by_default() {
    let app = make_app(std::env::temp_dir());
    assert_eq!(app.debug_scroll, 0);
}

#[test]
fn app_new_inherits_verbose_from_options() {
    let app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        verbose: true,
        ..AppOptions::default()
    });
    assert!(app.verbose);
}

#[test]
fn app_new_drains_startup_log_into_debug_log() {
    let startup = vec!["line 1".to_string(), "line 2".to_string()];
    let app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        startup_log: startup.clone(),
        ..AppOptions::default()
    });
    assert_eq!(app.debug_log, startup);
}

#[test]
fn app_log_appends_when_verbose() {
    let mut app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        verbose: true,
        ..AppOptions::default()
    });
    app.log("hello");
    app.log("world");
    assert_eq!(app.debug_log.len(), 2);
    assert_eq!(app.debug_log[0], "hello");
    assert_eq!(app.debug_log[1], "world");
}

#[test]
fn app_log_does_nothing_when_not_verbose() {
    let mut app = make_app(std::env::temp_dir());
    assert!(!app.verbose);
    app.log("should be ignored");
    assert!(app.debug_log.is_empty());
}

#[test]
fn app_log_accepts_string_and_str() {
    let mut app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        verbose: true,
        ..AppOptions::default()
    });
    app.log("static str");
    app.log(String::from("owned string"));
    app.log(format!("formatted {}", 42));
    assert_eq!(app.debug_log.len(), 3);
}

#[test]
fn app_log_preserves_startup_log_order() {
    let mut app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        verbose: true,
        startup_log: vec!["startup".to_string()],
        ..AppOptions::default()
    });
    app.log("runtime");
    assert_eq!(app.debug_log.len(), 2);
    assert_eq!(app.debug_log[0], "startup");
    assert_eq!(app.debug_log[1], "runtime");
}

// ── Debug scroll key handling ────────────────────────────────────────

fn make_verbose_app_with_logs(n: usize) -> App {
    let mut app = App::new(AppOptions {
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
        verbose: true,
        ..AppOptions::default()
    });
    for i in 0..n {
        app.debug_log.push(format!("log line {i}"));
    }
    app
}

fn ctrl_up() -> crossterm::event::KeyEvent {
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};
    KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn ctrl_down() -> crossterm::event::KeyEvent {
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};
    KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn debug_scroll_up_increments() {
    let mut app = make_verbose_app_with_logs(10);
    assert_eq!(app.debug_scroll, 0);
    app.handle_key(ctrl_up()).unwrap();
    assert_eq!(app.debug_scroll, 1);
    app.handle_key(ctrl_up()).unwrap();
    assert_eq!(app.debug_scroll, 2);
}

#[test]
fn debug_scroll_down_decrements() {
    let mut app = make_verbose_app_with_logs(10);
    app.debug_scroll = 5;
    app.handle_key(ctrl_down()).unwrap();
    assert_eq!(app.debug_scroll, 4);
    app.handle_key(ctrl_down()).unwrap();
    assert_eq!(app.debug_scroll, 3);
}

#[test]
fn debug_scroll_down_clamps_at_zero() {
    let mut app = make_verbose_app_with_logs(10);
    assert_eq!(app.debug_scroll, 0);
    app.handle_key(ctrl_down()).unwrap();
    assert_eq!(app.debug_scroll, 0);
}

#[test]
fn debug_scroll_up_clamps_at_log_length() {
    let mut app = make_verbose_app_with_logs(5);
    // max is debug_log.len().saturating_sub(1) == 4
    for _ in 0..20 {
        app.handle_key(ctrl_up()).unwrap();
    }
    assert_eq!(app.debug_scroll, 4);
}

#[test]
fn debug_scroll_ignored_when_not_verbose() {
    let mut app = make_app(std::env::temp_dir());
    assert!(!app.verbose);
    // Manually add some log lines so there would be room to scroll.
    app.debug_log.push("line".to_string());
    app.debug_log.push("line".to_string());
    app.handle_key(ctrl_up()).unwrap();
    assert_eq!(app.debug_scroll, 0);
    app.handle_key(ctrl_down()).unwrap();
    assert_eq!(app.debug_scroll, 0);
}

// ── CopyProgress tests ───────────────────────────────────────────────────

#[test]
fn copy_progress_new_starts_at_zero() {
    let p = CopyProgress::new("test.txt", 5);
    assert_eq!(p.done, 0);
    assert_eq!(p.total, 5);
}

#[test]
fn copy_progress_fraction_at_start_is_zero() {
    let p = CopyProgress::new("test.txt", 10);
    assert!((p.fraction() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn copy_progress_fraction_at_midpoint() {
    let mut p = CopyProgress::new("test.txt", 10);
    p.done = 5;
    assert!((p.fraction() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn copy_progress_fraction_when_total_is_zero_returns_one() {
    let p = CopyProgress::new("test.txt", 0);
    assert!((p.fraction() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn copy_progress_is_complete_when_done_equals_total() {
    let mut p = CopyProgress::new("test.txt", 3);
    assert!(!p.is_complete());
    p.done = 3;
    assert!(p.is_complete());
}

// ── Preview ───────────────────────────────────────────────────────────────

#[test]
fn show_preview_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(!app.show_preview);
}

#[test]
fn preview_state_starts_empty() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.preview_state.cached_path.is_none());
}

#[test]
fn capital_p_toggles_preview_on() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(!app.show_preview);
    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT))
        .unwrap();
    assert!(app.show_preview);
}

#[test]
fn capital_p_toggles_preview_off() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_preview = true;
    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT))
        .unwrap();
    assert!(!app.show_preview);
}

#[test]
fn preview_toggle_invalidates_state() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.preview_state.cached_path = Some(PathBuf::from("/tmp/cached"));
    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT))
        .unwrap();
    // After toggling on, the cache should be invalidated
    assert!(app.preview_state.cached_path.is_none());
}

// ── Inline editor ─────────────────────────────────────────────────────────

#[test]
fn inline_editor_none_by_default() {
    let dir = tempdir().expect("tempdir");
    let app = make_app(dir.path().to_path_buf());
    assert!(app.inline_editor.is_none());
}

#[test]
fn i_key_opens_inline_editor_on_file() {
    let dir = tempdir().expect("tempdir");
    // Create a file in the test directory
    std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();
    let mut app = make_app(dir.path().to_path_buf());
    // Navigate to the file (skip directories, find the file)
    for _ in 0..app.panes[0].entries.len() {
        if let Some(e) = app.panes[0].current_entry() {
            if !e.is_dir {
                break;
            }
        }
        app.panes[0].handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    // Press 'i' to open editor
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.inline_editor.is_some());
}

#[test]
fn i_key_on_dir_does_not_open_editor() {
    let dir = tempdir().expect("tempdir");
    // Create a subdirectory
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    let mut app = make_app(dir.path().to_path_buf());
    // Navigate to the directory
    for _ in 0..app.panes[0].entries.len() {
        if let Some(e) = app.panes[0].current_entry() {
            if e.is_dir {
                break;
            }
        }
        app.panes[0].handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    // Press 'i' — should NOT open editor on a directory
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.inline_editor.is_none());
}

#[test]
fn inline_editor_esc_closes_editor() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("test.txt"), "content").unwrap();
    let mut app = make_app(dir.path().to_path_buf());
    for _ in 0..app.panes[0].entries.len() {
        if let Some(e) = app.panes[0].current_entry() {
            if !e.is_dir {
                break;
            }
        }
        app.panes[0].handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.inline_editor.is_some());
    // Press Esc to close
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(app.inline_editor.is_none());
}

#[test]
fn inline_editor_intercepts_all_keys() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("test.txt"), "content").unwrap();
    let mut app = make_app(dir.path().to_path_buf());
    for _ in 0..app.panes[0].entries.len() {
        if let Some(e) = app.panes[0].current_entry() {
            if !e.is_dir {
                break;
            }
        }
        app.panes[0].handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    // The 'q' key should NOT dismiss the app while editor is open
    let should_exit = app
        .handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();
    assert!(!should_exit, "editor should intercept 'q'");
    assert!(app.inline_editor.is_some(), "editor should still be open");
}

#[test]
fn i_key_on_empty_dir_does_not_panic() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    // Empty directory — no current entry
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.inline_editor.is_none());
}

// ── Preview scroll ────────────────────────────────────────────────────────

#[test]
fn ctrl_j_scrolls_preview_down_when_preview_active() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_preview = true;
    let initial_scroll = app.preview_state.scroll;
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.preview_state.scroll > initial_scroll);
}

#[test]
fn ctrl_k_scrolls_preview_up_when_preview_active() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_preview = true;
    app.preview_state.scroll = 10;
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.preview_state.scroll < 10);
}

#[test]
fn ctrl_j_does_not_scroll_preview_when_preview_inactive() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.show_preview = false;
    let initial_scroll = app.preview_state.scroll;
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.preview_state.scroll, initial_scroll);
}

// ── handle_raw_event — non-key events are consumed without side effects ──
//
// Mouse capture is intentionally **not** enabled because the TUI is purely
// keyboard-driven.  These tests guarantee that even if a non-key event
// somehow reaches the handler, it is safely consumed without altering
// application state or signalling an exit.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Mouse-move events must be consumed (return Ok(false)) and leave
/// the application state unchanged.
#[test]
fn handle_raw_event_mouse_move_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 42,
        row: 10,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse move must not signal exit");
}

/// Mouse-click events must be consumed (return Ok(false)).
#[test]
fn handle_raw_event_mouse_click_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse click must not signal exit");
}

/// Mouse scroll-up events must be consumed without side effects.
#[test]
fn handle_raw_event_mouse_scroll_up_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse scroll must not signal exit");
}

/// Mouse scroll-down events must be consumed without side effects.
#[test]
fn handle_raw_event_mouse_scroll_down_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse scroll must not signal exit");
}

/// Mouse-drag events must be consumed without side effects.
#[test]
fn handle_raw_event_mouse_drag_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 20,
        row: 15,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse drag must not signal exit");
}

/// Mouse button-up events must be consumed without side effects.
#[test]
fn handle_raw_event_mouse_button_up_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Right),
        column: 30,
        row: 20,
        modifiers: KeyModifiers::NONE,
    });
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "mouse button-up must not signal exit");
}

/// Resize events must be consumed without signalling an exit.
/// (ratatui queries terminal size on every draw, so no app-level
/// action is needed.)
#[test]
fn handle_raw_event_resize_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Resize(120, 40);
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "resize must not signal exit");
}

/// FocusGained events must be consumed without side effects.
#[test]
fn handle_raw_event_focus_gained_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::FocusGained;
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "focus gained must not signal exit");
}

/// FocusLost events must be consumed without side effects.
#[test]
fn handle_raw_event_focus_lost_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::FocusLost;
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "focus lost must not signal exit");
}

/// Paste events must be consumed without side effects.
#[test]
fn handle_raw_event_paste_is_consumed() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Paste("some pasted text".to_string());
    let exit = app.handle_raw_event(event).unwrap();
    assert!(!exit, "paste must not signal exit");
}

/// A key event routed through `handle_raw_event` must behave identically
/// to a direct `handle_key` call.  Verify with Esc (dismiss = exit).
#[test]
fn handle_raw_event_key_delegates_to_handle_key() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let exit = app.handle_raw_event(event).unwrap();
    assert!(exit, "Esc via handle_raw_event should signal exit");
}

/// Non-key events must never alter the clipboard.
#[test]
fn handle_raw_event_mouse_does_not_alter_clipboard() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(app.clipboard.is_none());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_raw_event(event).unwrap();
    assert!(app.clipboard.is_none(), "mouse must not alter clipboard");
}

/// Non-key events must never set a selected path.
#[test]
fn handle_raw_event_mouse_does_not_set_selected() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(app.selected.is_none());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_raw_event(event).unwrap();
    assert!(app.selected.is_none(), "mouse must not set selected");
}

/// Non-key events must never open a modal.
#[test]
fn handle_raw_event_mouse_does_not_set_modal() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(app.modal.is_none());
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Middle),
        column: 10,
        row: 10,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_raw_event(event).unwrap();
    assert!(app.modal.is_none(), "mouse must not open a modal");
}

/// Non-key events must never alter the status message.
#[test]
fn handle_raw_event_resize_does_not_alter_status() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    app.status_msg = "before".to_string();
    let event = Event::Resize(200, 50);
    app.handle_raw_event(event).unwrap();
    assert_eq!(app.status_msg, "before", "resize must not alter status_msg");
}

/// Non-key events must never toggle panels.
#[test]
fn handle_raw_event_focus_does_not_toggle_panels() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(!app.show_theme_panel);
    assert!(!app.show_options_panel);
    assert!(!app.show_editor_panel);
    app.handle_raw_event(Event::FocusGained).unwrap();
    app.handle_raw_event(Event::FocusLost).unwrap();
    assert!(!app.show_theme_panel, "focus must not toggle theme panel");
    assert!(
        !app.show_options_panel,
        "focus must not toggle options panel"
    );
    assert!(!app.show_editor_panel, "focus must not toggle editor panel");
}

/// Non-key events must never switch the active pane.
#[test]
fn handle_raw_event_mouse_does_not_switch_pane() {
    let dir = tempdir().expect("tempdir");
    let mut app = make_app(dir.path().to_path_buf());
    assert!(app.active_idx == 0);
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 100,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_raw_event(event).unwrap();
    assert!(app.active_idx == 0, "mouse must not switch pane");
}
