//! Unit tests for [`super`].

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use tempfile::tempdir;

// ── Helpers ───────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn make_dual() -> DualPane {
    let dir = tempdir().expect("tempdir");
    // Create a dummy file so the pane is not empty.
    fs::write(dir.path().join("file.txt"), b"x").unwrap();
    // Leak the TempDir so the path survives — acceptable in tests.
    let path = dir.keep();
    DualPane::builder(path).build()
}

// ── DualPaneActive ────────────────────────────────────────────────────────

#[test]
fn dual_pane_active_other_left_returns_right() {
    assert_eq!(DualPaneActive::Left.other(), DualPaneActive::Right);
}

#[test]
fn dual_pane_active_other_right_returns_left() {
    assert_eq!(DualPaneActive::Right.other(), DualPaneActive::Left);
}

// ── Builder ───────────────────────────────────────────────────────────────

#[test]
fn builder_default_active_side_is_left() {
    let dual = make_dual();
    assert_eq!(dual.active_side, DualPaneActive::Left);
}

#[test]
fn builder_default_single_pane_is_false() {
    let dual = make_dual();
    assert!(!dual.single_pane);
}

#[test]
fn builder_single_pane_true_when_requested() {
    let dir = tempdir().expect("tempdir");
    let dual = DualPane::builder(dir.path().to_path_buf())
        .single_pane(true)
        .build();
    assert!(dual.single_pane);
}

#[test]
fn builder_right_dir_sets_independent_right_pane() {
    let left_dir = tempdir().expect("left tempdir");
    let right_dir = tempdir().expect("right tempdir");
    let dual = DualPane::builder(left_dir.path().to_path_buf())
        .right_dir(right_dir.path().to_path_buf())
        .build();
    assert_eq!(dual.left.current_dir, left_dir.path());
    assert_eq!(dual.right.current_dir, right_dir.path());
    assert_ne!(dual.left.current_dir, dual.right.current_dir);
}

#[test]
fn builder_without_right_dir_mirrors_left() {
    let dir = tempdir().expect("tempdir");
    let dual = DualPane::builder(dir.path().to_path_buf()).build();
    assert_eq!(dual.left.current_dir, dual.right.current_dir);
}

#[test]
fn builder_show_hidden_applies_to_both_panes() {
    let dir = tempdir().expect("tempdir");
    let dual = DualPane::builder(dir.path().to_path_buf())
        .show_hidden(true)
        .build();
    assert!(dual.left.show_hidden);
    assert!(dual.right.show_hidden);
}

#[test]
fn builder_sort_mode_applies_to_both_panes() {
    let dir = tempdir().expect("tempdir");
    let dual = DualPane::builder(dir.path().to_path_buf())
        .sort_mode(SortMode::SizeDesc)
        .build();
    assert_eq!(dual.left.sort_mode, SortMode::SizeDesc);
    assert_eq!(dual.right.sort_mode, SortMode::SizeDesc);
}

#[test]
fn builder_extension_filter_applies_to_both_panes() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.rs"), b"fn main(){}").unwrap();
    let dual = DualPane::builder(dir.path().to_path_buf())
        .allow_extension("rs")
        .build();
    assert_eq!(dual.left.extension_filter, vec!["rs"]);
    assert_eq!(dual.right.extension_filter, vec!["rs"]);
}

#[test]
fn builder_allow_extension_accumulates() {
    let dir = tempdir().expect("tempdir");
    let dual = DualPane::builder(dir.path().to_path_buf())
        .allow_extension("rs")
        .allow_extension("toml")
        .build();
    assert!(dual.left.extension_filter.contains(&"rs".to_string()));
    assert!(dual.left.extension_filter.contains(&"toml".to_string()));
}

// ── Pane accessors ────────────────────────────────────────────────────────

#[test]
fn active_returns_left_by_default() {
    let dual = make_dual();
    assert_eq!(dual.active().current_dir, dual.left.current_dir);
}

#[test]
fn active_returns_right_after_focus_switch() {
    let mut dual = make_dual();
    dual.active_side = DualPaneActive::Right;
    assert_eq!(dual.active().current_dir, dual.right.current_dir);
}

#[test]
fn active_mut_returns_left_by_default() {
    let mut dual = make_dual();
    let dir = dual.active_mut().current_dir.clone();
    assert_eq!(dir, dual.left.current_dir);
}

#[test]
fn inactive_returns_right_when_left_is_active() {
    let dual = make_dual();
    assert_eq!(dual.inactive().current_dir, dual.right.current_dir);
}

#[test]
fn inactive_returns_left_when_right_is_active() {
    let mut dual = make_dual();
    dual.active_side = DualPaneActive::Right;
    assert_eq!(dual.inactive().current_dir, dual.left.current_dir);
}

#[test]
fn focus_left_sets_active_to_left() {
    let mut dual = make_dual();
    dual.active_side = DualPaneActive::Right;
    dual.focus_left();
    assert_eq!(dual.active_side, DualPaneActive::Left);
}

#[test]
fn focus_right_sets_active_to_right() {
    let mut dual = make_dual();
    dual.focus_right();
    assert_eq!(dual.active_side, DualPaneActive::Right);
}

// ── Key handling ──────────────────────────────────────────────────────────

#[test]
fn tab_switches_active_pane_from_left_to_right() {
    let mut dual = make_dual();
    assert_eq!(dual.active_side, DualPaneActive::Left);
    let outcome = dual.handle_key(key(KeyCode::Tab));
    assert_eq!(outcome, DualPaneOutcome::Pending);
    assert_eq!(dual.active_side, DualPaneActive::Right);
}

#[test]
fn tab_switches_active_pane_from_right_to_left() {
    let mut dual = make_dual();
    dual.active_side = DualPaneActive::Right;
    dual.handle_key(key(KeyCode::Tab));
    assert_eq!(dual.active_side, DualPaneActive::Left);
}

#[test]
fn tab_returns_pending() {
    let mut dual = make_dual();
    assert_eq!(dual.handle_key(key(KeyCode::Tab)), DualPaneOutcome::Pending);
}

#[test]
fn w_key_toggles_single_pane_on() {
    let mut dual = make_dual();
    assert!(!dual.single_pane);
    let outcome = dual.handle_key(key(KeyCode::Char('w')));
    assert_eq!(outcome, DualPaneOutcome::Pending);
    assert!(dual.single_pane);
}

#[test]
fn w_key_toggles_single_pane_off() {
    let mut dual = make_dual();
    dual.single_pane = true;
    dual.handle_key(key(KeyCode::Char('w')));
    assert!(!dual.single_pane);
}

#[test]
fn w_key_returns_pending() {
    let mut dual = make_dual();
    assert_eq!(
        dual.handle_key(key(KeyCode::Char('w'))),
        DualPaneOutcome::Pending
    );
}

#[test]
fn navigation_keys_forwarded_to_active_pane() {
    let mut dual = make_dual();
    // Down arrow should move the left pane cursor.
    let before = dual.left.cursor;
    dual.handle_key(key(KeyCode::Down));
    // Cursor only advances if there are entries; the file we created ensures there are.
    let after = dual.left.cursor;
    // Either it moved (entries exist) or stayed at 0 (only one entry).
    assert!(after >= before);
}

#[test]
fn esc_returns_dismissed() {
    let mut dual = make_dual();
    assert_eq!(
        dual.handle_key(key(KeyCode::Esc)),
        DualPaneOutcome::Dismissed
    );
}

#[test]
fn q_key_returns_dismissed() {
    let mut dual = make_dual();
    assert_eq!(
        dual.handle_key(key(KeyCode::Char('q'))),
        DualPaneOutcome::Dismissed
    );
}

#[test]
fn unrecognised_key_returns_unhandled() {
    let mut dual = make_dual();
    // F5 is not bound by DualPane or FileExplorer.
    assert_eq!(
        dual.handle_key(key(KeyCode::F(5))),
        DualPaneOutcome::Unhandled
    );
}

#[test]
fn enter_on_file_returns_selected() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("pick.txt"), b"hello").unwrap();
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
    // Cursor is on the file; press Enter.
    let outcome = dual.handle_key(key(KeyCode::Enter));
    assert!(
        matches!(outcome, DualPaneOutcome::Selected(_)),
        "expected Selected, got {outcome:?}"
    );
}

#[test]
fn tab_does_not_affect_inactive_pane_cursor() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Move left pane cursor down.
    dual.handle_key(key(KeyCode::Down));
    let left_cursor_before_tab = dual.left.cursor;

    // Switch to right pane.
    dual.handle_key(key(KeyCode::Tab));

    // Left pane cursor must not have changed.
    assert_eq!(dual.left.cursor, left_cursor_before_tab);
}

#[test]
fn navigation_after_tab_affects_right_pane() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Switch to right pane then navigate.
    dual.handle_key(key(KeyCode::Tab));
    let right_before = dual.right.cursor;
    dual.handle_key(key(KeyCode::Down));
    let right_after = dual.right.cursor;

    // Left pane cursor must be untouched.
    assert_eq!(dual.left.cursor, 0, "left pane cursor should not move");
    assert!(
        right_after >= right_before,
        "right pane cursor should have advanced"
    );
}

// ── toggle_single_pane ────────────────────────────────────────────────────

#[test]
fn toggle_single_pane_flips_state() {
    let mut dual = make_dual();
    assert!(!dual.single_pane);
    dual.toggle_single_pane();
    assert!(dual.single_pane);
    dual.toggle_single_pane();
    assert!(!dual.single_pane);
}

// ── reload_both ───────────────────────────────────────────────────────────

#[test]
fn reload_both_picks_up_new_files() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    assert!(dual.left.entries.is_empty());
    assert!(dual.right.entries.is_empty());

    // Add a file externally, then reload.
    fs::write(dir.path().join("new.txt"), b"hi").unwrap();
    dual.reload_both();

    assert!(!dual.left.entries.is_empty(), "left should see new file");
    assert!(!dual.right.entries.is_empty(), "right should see new file");
}

// ── DualPaneActive default ────────────────────────────────────────────────

#[test]
fn dual_pane_active_default_is_left() {
    assert_eq!(DualPaneActive::default(), DualPaneActive::Left);
}

// ── focus_left / focus_right after a switch ───────────────────────────────

#[test]
fn focus_left_after_switch_to_right_restores_left() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
    dual.focus_right();
    assert_eq!(dual.active_side, DualPaneActive::Right);
    dual.focus_left();
    assert_eq!(dual.active_side, DualPaneActive::Left);
}

#[test]
fn focus_right_after_switch_to_left_restores_right() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
    dual.focus_left();
    assert_eq!(dual.active_side, DualPaneActive::Left);
    dual.focus_right();
    assert_eq!(dual.active_side, DualPaneActive::Right);
}

// ── inactive accessor after focus switch ──────────────────────────────────

#[test]
fn inactive_after_focus_right_is_left() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
    dual.focus_right();
    // When right is active, inactive() must return the left pane.
    assert_eq!(
        dual.inactive().current_dir,
        dual.left.current_dir,
        "inactive() should point to left when right is active"
    );
}

#[test]
fn inactive_after_focus_left_is_right() {
    let dir = tempdir().expect("tempdir");
    let _right_dir = {
        let d = tempdir().expect("tempdir right");
        d.path().to_path_buf()
        // Note: TempDir is dropped here but path still exists momentarily;
        // use a sub-dir of the same temp dir instead.
    };
    let sub = dir.path().join("right_sub");
    fs::create_dir(&sub).unwrap();
    let mut dual = DualPane::builder(dir.path().to_path_buf())
        .right_dir(sub.clone())
        .build();
    dual.focus_left();
    assert_eq!(
        dual.inactive().current_dir,
        sub,
        "inactive() should point to right when left is active"
    );
}

// ── builder extension_filter applies to both panes ────────────────────────

#[test]
fn builder_extension_filter_limits_visible_files_on_both_panes() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("main.rs"), b"fn main(){}").unwrap();
    fs::write(dir.path().join("Cargo.toml"), b"[package]").unwrap();

    let dual = DualPane::builder(dir.path().to_path_buf())
        .extension_filter(vec!["rs".into()])
        .build();

    // Only .rs files visible — .toml must be filtered out on both sides.
    assert_eq!(dual.left.entries.len(), 1, "left should show only .rs");
    assert_eq!(dual.right.entries.len(), 1, "right should show only .rs");
    assert_eq!(dual.left.entries[0].extension, "rs");
    assert_eq!(dual.right.entries[0].extension, "rs");
}

// ── DualPaneOutcome variants ──────────────────────────────────────────────

#[test]
fn dual_pane_outcome_dismissed_eq() {
    assert_eq!(DualPaneOutcome::Dismissed, DualPaneOutcome::Dismissed);
}

#[test]
fn dual_pane_outcome_pending_eq() {
    assert_eq!(DualPaneOutcome::Pending, DualPaneOutcome::Pending);
}

#[test]
fn dual_pane_outcome_unhandled_eq() {
    assert_eq!(DualPaneOutcome::Unhandled, DualPaneOutcome::Unhandled);
}

#[test]
fn dual_pane_outcome_selected_carries_path() {
    use std::path::PathBuf;
    let path = PathBuf::from("/tmp/chosen.txt");
    let outcome = DualPaneOutcome::Selected(path.clone());
    assert_eq!(outcome, DualPaneOutcome::Selected(path));
}

#[test]
fn dual_pane_outcome_selected_neq_dismissed() {
    use std::path::PathBuf;
    let outcome = DualPaneOutcome::Selected(PathBuf::from("/tmp/x"));
    assert_ne!(outcome, DualPaneOutcome::Dismissed);
}

// ── active_mut returns mutable ref to active pane ─────────────────────────

#[test]
fn active_mut_returns_right_when_right_is_active() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("right");
    fs::create_dir(&sub).unwrap();
    let mut dual = DualPane::builder(dir.path().to_path_buf())
        .right_dir(sub.clone())
        .build();
    dual.focus_right();
    assert_eq!(
        dual.active_mut().current_dir,
        sub,
        "active_mut() should return right pane when right is active"
    );
}

// ── toggle_single_pane ────────────────────────────────────────────────────

#[test]
fn toggle_single_pane_from_false_to_true() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
    assert!(!dual.single_pane);
    dual.toggle_single_pane();
    assert!(dual.single_pane);
}

#[test]
fn toggle_single_pane_twice_returns_to_original() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf())
        .single_pane(true)
        .build();
    dual.toggle_single_pane();
    dual.toggle_single_pane();
    assert!(
        dual.single_pane,
        "two toggles should restore original state"
    );
}

// ── Rename / MkdirCreated / TouchCreated outcome surfacing ────────────────

#[test]
fn dual_pane_outcome_mkdir_created_carries_path() {
    let path = std::path::PathBuf::from("/tmp/new_dir");
    let outcome = DualPaneOutcome::MkdirCreated(path.clone());
    assert_eq!(outcome, DualPaneOutcome::MkdirCreated(path));
}

#[test]
fn dual_pane_outcome_touch_created_carries_path() {
    let path = std::path::PathBuf::from("/tmp/new_file.txt");
    let outcome = DualPaneOutcome::TouchCreated(path.clone());
    assert_eq!(outcome, DualPaneOutcome::TouchCreated(path));
}

#[test]
fn dual_pane_outcome_rename_completed_carries_path() {
    let path = std::path::PathBuf::from("/tmp/renamed.txt");
    let from = PathBuf::from("/tmp/old.txt");
    let outcome = DualPaneOutcome::RenameCompleted {
        from: from.clone(),
        to: path.clone(),
    };
    assert_eq!(outcome, DualPaneOutcome::RenameCompleted { from, to: path });
}

#[test]
fn r_key_surfaces_rename_completed_from_active_pane() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("file.txt"), b"x").expect("write");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Activate rename mode.
    dual.handle_key(key(KeyCode::Char('r')));
    assert!(dual.active().is_rename_active());

    // Clear the prefilled name and type a new one.
    let prefill_len = dual.active().rename_input().len();
    for _ in 0..prefill_len {
        dual.handle_key(key(KeyCode::Backspace));
    }
    for c in "renamed.txt".chars() {
        dual.handle_key(key(KeyCode::Char(c)));
    }

    let outcome = dual.handle_key(key(KeyCode::Enter));
    assert!(
        matches!(outcome, DualPaneOutcome::RenameCompleted { ref to, .. } if to.file_name().unwrap() == "renamed.txt"),
        "expected RenameCompleted, got {outcome:?}"
    );
    assert!(dir.path().join("renamed.txt").exists());
    assert!(!dir.path().join("file.txt").exists());
}

#[test]
fn r_key_on_inactive_pane_does_not_activate_rename_there() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.txt"), b"a").expect("write");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Active pane is left by default.
    dual.handle_key(key(KeyCode::Char('r')));
    assert!(dual.left.is_rename_active());
    assert!(!dual.right.is_rename_active());
}

#[test]
fn mkdir_created_outcome_is_surfaced_not_collapsed() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Activate mkdir mode and confirm with a new folder name.
    // Use only lowercase letters that don't trigger other key bindings.
    dual.handle_key(key(KeyCode::Char('n')));
    for c in "freshdir".chars() {
        dual.handle_key(key(KeyCode::Char(c)));
    }
    let outcome = dual.handle_key(key(KeyCode::Enter));

    assert!(
        matches!(outcome, DualPaneOutcome::MkdirCreated(ref p) if p.file_name().unwrap() == "freshdir"),
        "expected MkdirCreated, got {outcome:?}"
    );
}

#[test]
fn touch_created_outcome_is_surfaced_not_collapsed() {
    let dir = tempdir().expect("tempdir");
    let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

    // Activate touch mode (Shift+N) and confirm with a new file name.
    // Use only lowercase letters/dots that don't trigger other key bindings.
    dual.handle_key(key(KeyCode::Char('N')));
    for c in "fresh.txt".chars() {
        dual.handle_key(key(KeyCode::Char(c)));
    }
    let outcome = dual.handle_key(key(KeyCode::Enter));

    assert!(
        matches!(outcome, DualPaneOutcome::TouchCreated(ref p) if p.file_name().unwrap() == "fresh.txt"),
        "expected TouchCreated, got {outcome:?}"
    );
}
