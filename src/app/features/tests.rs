//! Unit tests for [`super`]: undo, bookmarks, fuzzy finder, bulk rename,
//! archive extraction, git column.

use super::*;
use crate::{filesystem::StdFileSystem, AppOptions};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::{fs, time::Duration};
use tempfile::{tempdir, TempDir};

// ── Helpers ───────────────────────────────────────────────────────────────

fn app_in(dir: &Path) -> App {
    App::new(AppOptions {
        pane_dirs: vec![dir.to_path_buf(), dir.to_path_buf()],
        use_trash: false,
        ..AppOptions::default()
    })
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .unwrap();
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        let mods = if c.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        app.handle_key(KeyEvent::new(KeyCode::Char(c), mods))
            .unwrap();
    }
}

fn wait_idle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.has_background_work() && Instant::now() < deadline {
        app.poll_background();
        std::thread::sleep(Duration::from_millis(5));
    }
    app.poll_background();
}

fn names(app: &App) -> Vec<String> {
    app.active_pane()
        .entries
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

fn tree() -> TempDir {
    let tmp = tempdir().unwrap();
    let p = tmp.path();
    fs::create_dir_all(p.join("src/deep")).unwrap();
    fs::create_dir_all(p.join(".git/objects")).unwrap();
    fs::create_dir_all(p.join("node_modules/pkg")).unwrap();
    fs::create_dir_all(p.join("target/debug")).unwrap();
    fs::write(p.join("README.md"), "r").unwrap();
    fs::write(p.join(".hidden"), "h").unwrap();
    fs::write(p.join("src/main.rs"), "m").unwrap();
    fs::write(p.join("src/deep/lib.rs"), "l").unwrap();
    fs::write(p.join(".git/HEAD"), "x").unwrap();
    fs::write(p.join("node_modules/pkg/index.js"), "x").unwrap();
    fs::write(p.join("target/debug/out"), "x").unwrap();
    tmp
}

fn collect_walk(root: &Path, show_hidden: bool) -> Vec<String> {
    let mut labels = Vec::new();
    walk_tree(&StdFileSystem, root, show_hidden, &mut |batch| {
        labels.extend(batch.into_iter().map(|i| i.label));
        true
    });
    labels
}

// ── walk_tree ─────────────────────────────────────────────────────────────

#[test]
fn walk_lists_relative_paths_and_skips_noise() {
    let tmp = tree();
    let labels = collect_walk(tmp.path(), false);
    assert!(labels.contains(&"README.md".to_string()));
    assert!(labels.contains(&"src/".to_string()));
    assert!(labels.contains(&"src/main.rs".to_string()));
    assert!(labels.contains(&"src/deep/lib.rs".to_string()));
    for noisy in [".hidden", ".git", "node_modules", "target"] {
        assert!(
            !labels.iter().any(|l| l.starts_with(noisy)),
            "{noisy} must be skipped: {labels:?}"
        );
    }
}

#[test]
fn walk_with_hidden_includes_dotfiles_and_build_dirs_but_never_vcs() {
    let tmp = tree();
    let labels = collect_walk(tmp.path(), true);
    assert!(labels.contains(&".hidden".to_string()));
    assert!(labels.contains(&"target/debug/out".to_string()));
    assert!(labels.contains(&"node_modules/pkg/index.js".to_string()));
    assert!(
        !labels.iter().any(|l| l.starts_with(".git")),
        "VCS dirs always skipped"
    );
}

#[test]
fn walk_is_breadth_first() {
    let tmp = tree();
    let labels = collect_walk(tmp.path(), false);
    let shallow = labels.iter().position(|l| l == "src/main.rs").unwrap();
    let deep = labels.iter().position(|l| l == "src/deep/lib.rs").unwrap();
    assert!(shallow < deep);
}

#[test]
fn walk_stops_when_the_receiver_goes_away() {
    let tmp = tempdir().unwrap();
    for i in 0..1200 {
        fs::write(tmp.path().join(format!("f{i}")), "").unwrap();
    }
    let mut batches = 0;
    walk_tree(&StdFileSystem, tmp.path(), false, &mut |_| {
        batches += 1;
        false // owner dropped the task
    });
    assert_eq!(batches, 1, "walk must stop right after a refused send");
}

// ── Bookmarks ─────────────────────────────────────────────────────────────

#[test]
fn b_toggles_a_bookmark_for_the_current_folder() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('b'));
    assert_eq!(app.bookmarks, vec![tmp.path().to_path_buf()]);
    press(&mut app, KeyCode::Char('b'));
    assert!(app.bookmarks.is_empty());
}

#[test]
fn opening_empty_bookmarks_only_notifies() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    type_text(&mut app, "B");
    assert!(app.picker.is_none());
    assert!(app.snackbar.is_some());
}

#[test]
fn bookmark_picker_jumps_the_active_pane() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    app.bookmarks = vec![tmp.path().join("src")];
    type_text(&mut app, "B");
    assert!(app.picker.is_some());
    press(&mut app, KeyCode::Enter);
    assert!(app.picker.is_none());
    assert_eq!(app.active_pane().current_dir, tmp.path().join("src"));
    assert!(names(&app).contains(&"main.rs".to_string()));
}

#[test]
fn deleting_in_the_bookmark_picker_removes_it() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    app.bookmarks = vec![tmp.path().join("src"), tmp.path().join("target")];
    type_text(&mut app, "B");
    press(&mut app, KeyCode::Delete);
    assert_eq!(app.bookmarks, vec![tmp.path().join("target")]);
    assert!(app.picker.is_some());
    press(&mut app, KeyCode::Delete);
    assert!(app.bookmarks.is_empty());
    assert!(app.picker.is_none(), "empty list closes the picker");
}

#[test]
fn jumping_to_a_vanished_bookmark_errors_and_stays_put() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    app.jump_to(&tmp.path().join("gone"));
    assert_eq!(app.active_pane().current_dir, tmp.path());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
}

#[test]
fn escape_closes_the_picker_without_jumping() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    app.bookmarks = vec![tmp.path().join("src")];
    type_text(&mut app, "B");
    press(&mut app, KeyCode::Esc);
    assert!(app.picker.is_none());
    assert_eq!(app.active_pane().current_dir, tmp.path());
}

// ── Fuzzy finder ──────────────────────────────────────────────────────────

#[test]
fn fuzzy_finder_finds_a_deep_file_and_selects_it() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('f'));
    assert!(app.picker.as_ref().unwrap().loading);
    wait_idle(&mut app);
    assert!(!app.picker.as_ref().unwrap().loading);

    type_text(&mut app, "deeplib");
    assert_eq!(
        app.picker.as_ref().unwrap().selected().unwrap().label,
        "src/deep/lib.rs"
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.active_pane().current_dir, tmp.path().join("src/deep"));
    assert_eq!(app.active_pane().current_entry().unwrap().name, "lib.rs");
}

#[test]
fn closing_the_fuzzy_finder_cancels_the_walk() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('f'));
    press(&mut app, KeyCode::Esc);
    assert!(app.picker.is_none());
    assert!(app.picker_task.is_none());
}

#[test]
fn picker_swallows_global_shortcuts_while_open() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "dxq"); // 'd' / 'q' must be typed into the query
    assert_eq!(app.picker.as_ref().unwrap().query, "dxq");
    assert!(app.modal.is_none());
}

// ── Undo ──────────────────────────────────────────────────────────────────

#[test]
fn undo_with_empty_history_just_notifies() {
    let tmp = tree();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('u'));
    assert!(app
        .snackbar
        .as_ref()
        .is_some_and(|s| s.message.contains("Nothing to undo")));
}

#[test]
fn undo_removes_a_just_created_folder_and_file() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "newdir");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('N'));
    type_text(&mut app, "new.txt");
    press(&mut app, KeyCode::Enter);
    assert!(tmp.path().join("newdir").is_dir() && tmp.path().join("new.txt").is_file());

    press(&mut app, KeyCode::Char('u'));
    assert!(!tmp.path().join("new.txt").exists());
    assert!(
        tmp.path().join("newdir").exists(),
        "only the newest action is undone"
    );
    press(&mut app, KeyCode::Char('u'));
    assert!(!tmp.path().join("newdir").exists());
    assert!(
        !names(&app).contains(&"newdir".to_string()),
        "panes reloaded"
    );
}

#[test]
fn undo_refuses_to_delete_a_folder_that_is_no_longer_empty() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "keep");
    press(&mut app, KeyCode::Enter);
    fs::write(tmp.path().join("keep/precious.txt"), "data").unwrap();
    press(&mut app, KeyCode::Char('u'));
    assert!(tmp.path().join("keep/precious.txt").exists());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
}

#[test]
fn undo_reverses_a_rename() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("old.txt"), "x").unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('r'));
    for _ in 0..7 {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "new.txt");
    press(&mut app, KeyCode::Enter);
    assert!(tmp.path().join("new.txt").exists());

    press(&mut app, KeyCode::Char('u'));
    assert!(tmp.path().join("old.txt").exists());
    assert!(!tmp.path().join("new.txt").exists());
}

#[test]
fn undo_reverses_a_move_between_folders() {
    let tmp = tempdir().unwrap();
    fs::create_dir(tmp.path().join("dst")).unwrap();
    fs::write(tmp.path().join("a.txt"), "payload").unwrap();
    let mut app = app_in(tmp.path());

    // Cut a.txt, enter dst, paste.
    let idx = names(&app).iter().position(|n| n == "a.txt").unwrap();
    app.active_pane_mut().cursor = idx;
    press(&mut app, KeyCode::Char('x'));
    let dst = tmp.path().join("dst");
    app.active_pane_mut().go_to(&dst);
    press(&mut app, KeyCode::Char('p'));
    assert!(dst.join("a.txt").exists() && !tmp.path().join("a.txt").exists());

    press(&mut app, KeyCode::Char('u'));
    assert_eq!(
        fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
        "payload"
    );
    assert!(!dst.join("a.txt").exists());
}

#[test]
fn undo_reverses_a_copy_by_removing_the_copy_only() {
    let tmp = tempdir().unwrap();
    fs::create_dir(tmp.path().join("dst")).unwrap();
    fs::write(tmp.path().join("a.txt"), "orig").unwrap();
    let mut app = app_in(tmp.path());
    let idx = names(&app).iter().position(|n| n == "a.txt").unwrap();
    app.active_pane_mut().cursor = idx;
    app.yank(ClipOp::Copy);
    app.active_pane_mut().go_to(&tmp.path().join("dst"));
    app.paste();
    assert!(tmp.path().join("dst/a.txt").exists());

    app.undo();
    assert!(!tmp.path().join("dst/a.txt").exists());
    assert_eq!(
        fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
        "orig"
    );
}

#[test]
fn permanent_deletes_are_not_undoable() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("gone.txt"), "x").unwrap();
    let mut app = app_in(tmp.path());
    app.confirm_delete(&tmp.path().join("gone.txt"));
    assert!(app.undo_stack.is_empty());
}

#[test]
fn undoing_a_trash_delete_reports_when_the_platform_cannot_restore() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    app.record_undo(UndoEntry::Delete {
        paths: vec![tmp.path().join("x")],
    });
    app.undo();
    // StdFileSystem cannot restore from a trash.
    assert!(app
        .snackbar
        .as_ref()
        .is_some_and(|s| s.is_error && s.message.contains("restore")));
}

#[test]
fn undo_history_is_capped() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    for i in 0..(UNDO_LIMIT + 20) {
        app.record_undo(UndoEntry::Create {
            path: PathBuf::from(format!("/x{i}")),
            is_dir: false,
        });
    }
    assert_eq!(app.undo_stack.len(), UNDO_LIMIT);
    assert_eq!(
        app.undo_stack[0],
        UndoEntry::Create {
            path: PathBuf::from("/x20"),
            is_dir: false
        },
        "oldest entries are dropped first"
    );
}

#[test]
fn apply_undo_rename_checks_both_ends() {
    let tmp = tempdir().unwrap();
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    let entry = UndoEntry::Rename {
        from: a.clone(),
        to: b.clone(),
    };
    assert!(
        apply_undo(&StdFileSystem, &entry).is_err(),
        "target missing"
    );
    fs::write(&b, "1").unwrap();
    fs::write(&a, "2").unwrap();
    assert!(
        apply_undo(&StdFileSystem, &entry).is_err(),
        "origin occupied"
    );
    fs::remove_file(&a).unwrap();
    assert_eq!(apply_undo(&StdFileSystem, &entry), Ok(1));
    assert!(a.exists());
}

#[test]
fn descriptions_name_the_action() {
    assert!(UndoEntry::Rename {
        from: "/a".into(),
        to: "/b".into()
    }
    .describe()
    .contains("a → b"));
    assert!(UndoEntry::Batch(vec![]).describe().contains("0 change"));
}

// ── Bulk rename ───────────────────────────────────────────────────────────

fn files(dir: &Path, names: &[&str]) {
    for n in names {
        fs::write(dir.join(n), n).unwrap();
    }
}

#[test]
fn bulk_rename_without_an_editor_uses_the_inline_editor_and_applies_on_exit() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt", "b.txt"]);
    let mut app = app_in(tmp.path());
    assert_eq!(app.editor, Editor::None);

    type_text(&mut app, "R");
    assert!(app.inline_editor.is_some());
    let session = app.bulk_rename.clone().unwrap();
    assert_eq!(fs::read_to_string(&session.file).unwrap(), "a.txt\nb.txt\n");

    // Simulate the user's edit + save, then close the editor (Esc/Ctrl+Q).
    fs::write(&session.file, "x.txt\ny.txt\n").unwrap();
    app.inline_editor = None;
    app.editor_finished(&session.file);

    assert_eq!(
        fs::read_to_string(tmp.path().join("x.txt")).unwrap(),
        "a.txt"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("y.txt")).unwrap(),
        "b.txt"
    );
    assert!(app.bulk_rename.is_none());
    assert!(!session.file.exists(), "temp file cleaned up");
}

#[test]
fn bulk_rename_with_an_editor_requests_it_externally() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt"]);
    let mut app = app_in(tmp.path());
    app.editor = Editor::Helix;
    app.start_bulk_rename();
    let (file, _) = app.take_editor_request().expect("editor requested");
    assert!(app.inline_editor.is_none());
    fs::write(&file, "z.txt\n").unwrap();
    app.editor_finished(&file);
    assert!(tmp.path().join("z.txt").exists());
}

#[test]
fn bulk_rename_only_touches_marked_entries_when_any_are_marked() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt", "b.txt", "c.txt"]);
    let mut app = app_in(tmp.path());
    let idx = names(&app).iter().position(|n| n == "b.txt").unwrap();
    app.active_pane_mut().cursor = idx;
    press(&mut app, KeyCode::Char(' '));
    app.start_bulk_rename();
    let session = app.bulk_rename.clone().unwrap();
    assert_eq!(session.sources, vec![tmp.path().join("b.txt")]);
}

#[test]
fn bulk_rename_rejects_bad_edits_and_changes_nothing() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt", "b.txt"]);
    let mut app = app_in(tmp.path());
    app.start_bulk_rename();
    let file = app.bulk_rename.clone().unwrap().file;
    fs::write(&file, "only-one-line\n").unwrap();
    app.inline_editor = None;
    app.editor_finished(&file);
    assert!(tmp.path().join("a.txt").exists() && tmp.path().join("b.txt").exists());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
    assert!(app.undo_stack.is_empty());
}

#[test]
fn bulk_rename_is_undone_as_one_step() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a", "b"]);
    let mut app = app_in(tmp.path());
    app.start_bulk_rename();
    let file = app.bulk_rename.clone().unwrap().file;
    fs::write(&file, "b\na\n").unwrap(); // swap
    app.inline_editor = None;
    app.editor_finished(&file);
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "b");

    assert_eq!(app.undo_stack.len(), 1);
    app.undo();
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "a");
    assert_eq!(fs::read_to_string(tmp.path().join("b")).unwrap(), "b");
}

#[test]
fn editor_finished_for_an_unrelated_file_is_ignored() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a"]);
    let mut app = app_in(tmp.path());
    app.start_bulk_rename();
    app.editor_finished(Path::new("/some/other/file"));
    assert!(app.bulk_rename.is_some(), "session untouched");
    let _ = fs::remove_file(app.bulk_rename.clone().unwrap().file);
}

#[test]
fn bulk_rename_of_an_empty_folder_complains() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    app.start_bulk_rename();
    assert!(app.bulk_rename.is_none());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
}

// ── Archives ──────────────────────────────────────────────────────────────

#[cfg(feature = "archive")]
fn make_zip(path: &Path) {
    use std::io::Write;
    let mut w = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file("inner/hello.txt", opts).unwrap();
    w.write_all(b"hi").unwrap();
    w.finish().unwrap();
}

#[cfg(feature = "archive")]
#[test]
fn shift_x_extracts_the_highlighted_archive_in_the_background() {
    let tmp = tempdir().unwrap();
    make_zip(&tmp.path().join("pack.zip"));
    let mut app = app_in(tmp.path());
    type_text(&mut app, "X");
    assert!(app.has_background_work() || tmp.path().join("pack").exists());
    wait_idle(&mut app);
    assert_eq!(
        fs::read_to_string(tmp.path().join("pack/inner/hello.txt")).unwrap(),
        "hi"
    );
    assert!(names(&app).contains(&"pack".to_string()), "panes reloaded");

    app.undo();
    assert!(!tmp.path().join("pack").exists());
}

#[cfg(feature = "archive")]
#[test]
fn extracting_refuses_to_overwrite_an_existing_folder() {
    let tmp = tempdir().unwrap();
    make_zip(&tmp.path().join("pack.zip"));
    fs::create_dir(tmp.path().join("pack")).unwrap();
    let mut app = app_in(tmp.path());
    let idx = names(&app).iter().position(|n| n == "pack.zip").unwrap();
    app.active_pane_mut().cursor = idx;
    type_text(&mut app, "X");
    assert!(app.extract_task.is_none());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
}

#[cfg(feature = "archive")]
#[test]
fn extracting_a_non_archive_is_an_error() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["plain.txt"]);
    let mut app = app_in(tmp.path());
    type_text(&mut app, "X");
    assert!(app.extract_task.is_none());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
}

#[cfg(feature = "archive")]
#[test]
fn a_corrupt_archive_reports_failure_and_leaves_no_folder() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("bad.zip"), "not a zip").unwrap();
    let mut app = app_in(tmp.path());
    type_text(&mut app, "X");
    wait_idle(&mut app);
    assert!(!tmp.path().join("bad").exists());
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
    assert!(app.undo_stack.is_empty());
}

// ── Git column ────────────────────────────────────────────────────────────

#[test]
fn v_toggles_git_markers_on_every_pane() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    assert!(app.panes.iter().all(|p| !p.show_git));
    press(&mut app, KeyCode::Char('v'));
    assert!(app.panes.iter().all(|p| p.show_git));
    press(&mut app, KeyCode::Char('v'));
    assert!(app.panes.iter().all(|p| !p.show_git));
}

#[test]
fn git_markers_show_untracked_files_in_a_repo() {
    let tmp = tempdir().unwrap();
    let ok = std::process::Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .args(["init", "-q"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return; // git not installed
    }
    files(tmp.path(), &["fresh.txt"]);
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('v'));
    wait_idle(&mut app);
    assert_eq!(
        app.active_pane().git_mark(&tmp.path().join("fresh.txt")),
        Some(crate::GitMark::Untracked)
    );
}

#[test]
fn git_off_clears_markers() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('v'));
    wait_idle(&mut app);
    press(&mut app, KeyCode::Char('v'));
    assert!(app.active_pane().git.is_none());
}

// ── Custom app key bindings via the event loop entry point ────────────────

#[test]
fn remapped_app_command_works_through_handle_raw_event() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt"]);
    let mut app = app_in(tmp.path());
    let chords = [crate::KeyChord::plain(KeyCode::Char('m'))];
    app.set_key_bindings(KeyBindings::new().remap(AppCommand::Cut, &chords));

    // `m` now cuts…
    let m = Event::Key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    app.handle_raw_event(m).unwrap();
    assert_eq!(app.clipboard.as_ref().map(|c| c.op), Some(ClipOp::Cut));

    // …and the old `x` no longer does.
    app.clipboard = None;
    let x = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    app.handle_raw_event(x).unwrap();
    assert!(app.clipboard.is_none());
}

#[test]
fn custom_bindings_do_not_apply_while_typing_a_name() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    let chords = [crate::KeyChord::plain(KeyCode::Char('m'))];
    app.set_key_bindings(KeyBindings::new().remap(AppCommand::NewDirectory, &chords));
    // `n` is swallowed (old key)…
    app.handle_raw_event(Event::Key(KeyEvent::new(
        KeyCode::Char('n'),
        KeyModifiers::NONE,
    )))
    .unwrap();
    assert!(!app.active_pane().has_prompt());
    app.handle_raw_event(Event::Key(KeyEvent::new(
        KeyCode::Char('m'),
        KeyModifiers::NONE,
    )))
    .unwrap();
    assert!(app.active_pane().has_prompt());
    // While the prompt is open, `m` is just a letter.
    app.handle_raw_event(Event::Key(KeyEvent::new(
        KeyCode::Char('m'),
        KeyModifiers::NONE,
    )))
    .unwrap();
    assert_eq!(app.active_pane().mkdir_input(), "m");
}

#[test]
fn idle_app_has_no_background_work() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    assert!(!app.has_background_work());
    assert!(!app.poll_background());
}

// ── Redo ──────────────────────────────────────────────────────────────────

fn ctrl_r(app: &mut App) {
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
        .unwrap();
}

#[test]
fn redo_with_nothing_undone_just_notifies() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    ctrl_r(&mut app);
    assert!(app
        .snackbar
        .as_ref()
        .is_some_and(|s| s.message.contains("Nothing to redo")));
}

#[test]
fn ctrl_r_is_not_rename() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt"]);
    let mut app = app_in(tmp.path());
    ctrl_r(&mut app);
    assert!(!app.active_pane().has_prompt());
}

#[test]
fn redo_recreates_what_undo_removed_and_undo_works_again() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "box");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('u'));
    assert!(!tmp.path().join("box").exists());
    assert_eq!(app.redo_stack.len(), 1);

    ctrl_r(&mut app);
    assert!(tmp.path().join("box").is_dir());
    assert!(app.redo_stack.is_empty());
    assert_eq!(app.undo_stack.len(), 1, "redone action is undoable again");

    press(&mut app, KeyCode::Char('u'));
    assert!(!tmp.path().join("box").exists());
}

#[test]
fn a_new_action_discards_the_redo_history() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "one");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('u'));
    assert_eq!(app.redo_stack.len(), 1);

    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "two");
    press(&mut app, KeyCode::Enter);
    assert!(app.redo_stack.is_empty());
    ctrl_r(&mut app);
    assert!(
        !tmp.path().join("one").exists(),
        "stale redo must not resurrect 'one'"
    );
}

#[test]
fn redo_reapplies_a_rename() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["old.txt"]);
    let mut app = app_in(tmp.path());
    app.record_undo(UndoEntry::Rename {
        from: tmp.path().join("old.txt"),
        to: tmp.path().join("new.txt"),
    });
    fs::rename(tmp.path().join("old.txt"), tmp.path().join("new.txt")).unwrap();
    app.undo();
    assert!(tmp.path().join("old.txt").exists());
    app.redo();
    assert!(tmp.path().join("new.txt").exists() && !tmp.path().join("old.txt").exists());
}

#[test]
fn redo_reapplies_move_and_copy() {
    let tmp = tempdir().unwrap();
    fs::create_dir(tmp.path().join("dst")).unwrap();
    files(tmp.path(), &["m.txt", "c.txt"]);
    let dst = tmp.path().join("dst");
    let mut app = app_in(tmp.path());

    // Move m.txt.
    let i = names(&app).iter().position(|n| n == "m.txt").unwrap();
    app.active_pane_mut().cursor = i;
    app.yank(ClipOp::Cut);
    app.active_pane_mut().go_to(&dst);
    app.paste();
    // Copy c.txt.
    app.active_pane_mut().go_to(tmp.path());
    let i = names(&app).iter().position(|n| n == "c.txt").unwrap();
    app.active_pane_mut().cursor = i;
    app.yank(ClipOp::Copy);
    app.active_pane_mut().go_to(&dst);
    app.paste();
    assert!(dst.join("m.txt").exists() && dst.join("c.txt").exists());

    app.undo();
    app.undo();
    assert!(tmp.path().join("m.txt").exists() && !dst.join("c.txt").exists());

    app.redo();
    app.redo();
    assert!(dst.join("m.txt").exists() && !tmp.path().join("m.txt").exists());
    assert!(
        dst.join("c.txt").exists() && tmp.path().join("c.txt").exists(),
        "copy keeps the source"
    );
}

#[test]
fn redo_reapplies_a_bulk_swap_in_one_step() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a", "b"]);
    let mut app = app_in(tmp.path());
    app.start_bulk_rename();
    let file = app.bulk_rename.clone().unwrap().file;
    fs::write(&file, "b\na\n").unwrap();
    app.inline_editor = None;
    app.editor_finished(&file);
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "b");
    app.undo();
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "a");
    app.redo();
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "b");
    assert_eq!(fs::read_to_string(tmp.path().join("b")).unwrap(), "a");
}

#[test]
fn redo_fails_cleanly_when_the_world_changed() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "box");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('u'));
    fs::create_dir(tmp.path().join("box")).unwrap(); // someone re-created it
    ctrl_r(&mut app);
    assert!(app.snackbar.as_ref().is_some_and(|s| s.is_error));
    assert!(app.redo_stack.is_empty(), "a failed entry is dropped");
}

#[cfg(feature = "archive")]
#[test]
fn redo_reruns_an_extraction_in_the_background() {
    let tmp = tempdir().unwrap();
    make_zip(&tmp.path().join("pack.zip"));
    let mut app = app_in(tmp.path());
    type_text(&mut app, "X");
    wait_idle(&mut app);
    app.undo();
    assert!(!tmp.path().join("pack").exists());

    app.redo();
    wait_idle(&mut app);
    assert!(tmp.path().join("pack/inner/hello.txt").exists());
    assert_eq!(app.undo_stack.len(), 1);
    assert!(app.redo_stack.is_empty());
    app.undo();
    assert!(!tmp.path().join("pack").exists());
}

#[test]
fn redo_binding_can_be_remapped() {
    let tmp = tempdir().unwrap();
    let mut app = app_in(tmp.path());
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "box");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('u'));
    let chords = [crate::KeyChord::plain(KeyCode::Char('U'))];
    app.set_key_bindings(KeyBindings::new().remap(AppCommand::Redo, &chords));
    app.handle_raw_event(Event::Key(KeyEvent::new(
        KeyCode::Char('U'),
        KeyModifiers::SHIFT,
    )))
    .unwrap();
    assert!(tmp.path().join("box").exists());
}

// ── Async directory loading through the App ───────────────────────────────

#[test]
fn app_with_async_loading_behaves_like_sync_for_fast_disks() {
    let tmp = tempdir().unwrap();
    files(tmp.path(), &["a.txt"]);
    let mut app = App::new(AppOptions {
        pane_dirs: vec![tmp.path().to_path_buf()],
        async_loading: true,
        ..AppOptions::default()
    });
    assert!(app.panes.iter().all(|p| p.async_loading));
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "box");
    press(&mut app, KeyCode::Enter);
    assert!(names(&app).contains(&"box".to_string()));
    assert_eq!(
        app.active_pane().current_entry().unwrap().name,
        "box",
        "new entry selected"
    );

    // New panes inherit the loading mode.
    app.add_pane_from_active();
    assert!(app.panes.iter().all(|p| p.async_loading));
}
