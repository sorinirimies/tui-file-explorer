//! Unit tests for [`super`].

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::io::Write;
use tempfile::tempdir;

/// Helper: create a press `KeyEvent`.
fn press(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Helper: create a press `KeyEvent` with modifiers.
fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Helper: write `content` into a temp file and return (`dir`, `path`).
fn temp_file(content: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, content).unwrap();
    (dir, path)
}

// ── Construction ──────────────────────────────────────────────────────

#[test]
fn open_reads_file_content() {
    let (_dir, path) = temp_file("hello\nworld");
    let ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.lines(), &["hello", "world"]);
}

#[test]
fn open_empty_file_has_one_line() {
    let (_dir, path) = temp_file("");
    let ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.lines(), &[""]);
    assert_eq!(ed.line_count(), 1);
}

#[test]
fn open_nonexistent_file_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nope.txt");
    assert!(InlineEditor::open(&path).is_err());
}

#[test]
fn open_too_large_file_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("big.txt");
    // Create a file just over the limit.
    let mut f = fs::File::create(&path).unwrap();
    let chunk = vec![b'A'; 1024];
    for _ in 0..(MAX_EDIT_FILE_SIZE / 1024 + 1) {
        f.write_all(&chunk).unwrap();
    }
    drop(f);
    assert!(InlineEditor::open(&path).is_err());
}

// ── Getters ───────────────────────────────────────────────────────────

#[test]
fn line_count_matches_file_lines() {
    let (_dir, path) = temp_file("a\nb\nc");
    let ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.line_count(), 3);
}

#[test]
fn is_modified_false_initially() {
    let (_dir, path) = temp_file("hello");
    let ed = InlineEditor::open(&path).unwrap();
    assert!(!ed.is_modified());
}

#[test]
fn path_returns_opened_path() {
    let (_dir, path) = temp_file("hello");
    let ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.path(), path);
}

#[test]
fn status_empty_initially() {
    let (_dir, path) = temp_file("hello");
    let ed = InlineEditor::open(&path).unwrap();
    assert!(ed.status().is_empty());
}

// ── Cursor movement ───────────────────────────────────────────────────

#[test]
fn move_down_increments_row() {
    let (_dir, path) = temp_file("a\nb\nc");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Down));
    assert_eq!(ed.cursor_row(), 1);
}

#[test]
fn move_up_decrements_row() {
    let (_dir, path) = temp_file("a\nb\nc");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Down));
    ed.handle_key(press(KeyCode::Down));
    ed.handle_key(press(KeyCode::Up));
    assert_eq!(ed.cursor_row(), 1);
}

#[test]
fn move_up_at_top_stays() {
    let (_dir, path) = temp_file("a\nb");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Up));
    assert_eq!(ed.cursor_row(), 0);
}

#[test]
fn move_down_at_bottom_stays() {
    let (_dir, path) = temp_file("a\nb");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Down));
    ed.handle_key(press(KeyCode::Down)); // already at last line
    assert_eq!(ed.cursor_row(), 1);
}

#[test]
fn move_right_increments_col() {
    let (_dir, path) = temp_file("abc");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Right));
    assert_eq!(ed.cursor_col(), 1);
}

#[test]
fn move_left_decrements_col() {
    let (_dir, path) = temp_file("abc");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Right));
    ed.handle_key(press(KeyCode::Right));
    ed.handle_key(press(KeyCode::Left));
    assert_eq!(ed.cursor_col(), 1);
}

#[test]
fn move_left_at_col_zero_goes_to_prev_line_end() {
    let (_dir, path) = temp_file("abc\nde");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Down)); // row 1, col 0
    ed.handle_key(press(KeyCode::Left)); // should go to row 0, col 3
    assert_eq!(ed.cursor_row(), 0);
    assert_eq!(ed.cursor_col(), 3);
}

#[test]
fn move_right_at_line_end_goes_to_next_line_start() {
    let (_dir, path) = temp_file("ab\ncd");
    let mut ed = InlineEditor::open(&path).unwrap();
    // Move to end of first line
    ed.handle_key(press(KeyCode::End));
    assert_eq!(ed.cursor_col(), 2);
    // Move right should go to row 1, col 0
    ed.handle_key(press(KeyCode::Right));
    assert_eq!(ed.cursor_row(), 1);
    assert_eq!(ed.cursor_col(), 0);
}

#[test]
fn cursor_col_clamped_on_vertical_move() {
    let (_dir, path) = temp_file("abcdef\nab");
    let mut ed = InlineEditor::open(&path).unwrap();
    // Move to col 5 on the first line
    for _ in 0..5 {
        ed.handle_key(press(KeyCode::Right));
    }
    assert_eq!(ed.cursor_col(), 5);
    // Move down to shorter line — col should clamp to 2
    ed.handle_key(press(KeyCode::Down));
    assert_eq!(ed.cursor_col(), 2);
}

#[test]
fn home_moves_to_col_zero() {
    let (_dir, path) = temp_file("hello world");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::End));
    assert!(ed.cursor_col() > 0);
    ed.handle_key(press(KeyCode::Home));
    assert_eq!(ed.cursor_col(), 0);
}

#[test]
fn end_moves_to_line_end() {
    let (_dir, path) = temp_file("hello");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::End));
    assert_eq!(ed.cursor_col(), 5);
}

// ── Text editing ──────────────────────────────────────────────────────

#[test]
fn insert_char_at_cursor() {
    let (_dir, path) = temp_file("ac");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Right)); // after 'a'
    ed.handle_key(press(KeyCode::Char('b')));
    assert_eq!(ed.lines()[0], "abc");
    assert_eq!(ed.cursor_col(), 2);
}

#[test]
fn insert_char_sets_modified() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    assert!(!ed.is_modified());
    ed.handle_key(press(KeyCode::Char('y')));
    assert!(ed.is_modified());
}

#[test]
fn backspace_deletes_char() {
    let (_dir, path) = temp_file("abc");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::End)); // col 3
    ed.handle_key(press(KeyCode::Backspace));
    assert_eq!(ed.lines()[0], "ab");
    assert_eq!(ed.cursor_col(), 2);
}

#[test]
fn backspace_at_line_start_joins_lines() {
    let (_dir, path) = temp_file("ab\ncd");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Down)); // row 1, col 0
    ed.handle_key(press(KeyCode::Backspace));
    assert_eq!(ed.line_count(), 1);
    assert_eq!(ed.lines()[0], "abcd");
    assert_eq!(ed.cursor_row(), 0);
    assert_eq!(ed.cursor_col(), 2); // end of "ab"
}

#[test]
fn delete_removes_char_at_cursor() {
    let (_dir, path) = temp_file("abc");
    let mut ed = InlineEditor::open(&path).unwrap();
    // cursor at col 0 — delete 'a'
    ed.handle_key(press(KeyCode::Delete));
    assert_eq!(ed.lines()[0], "bc");
    assert_eq!(ed.cursor_col(), 0);
}

#[test]
fn delete_at_line_end_joins_with_next() {
    let (_dir, path) = temp_file("ab\ncd");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::End)); // col 2
    ed.handle_key(press(KeyCode::Delete));
    assert_eq!(ed.line_count(), 1);
    assert_eq!(ed.lines()[0], "abcd");
}

#[test]
fn enter_splits_line() {
    let (_dir, path) = temp_file("abcd");
    let mut ed = InlineEditor::open(&path).unwrap();
    // Move to col 2 then press Enter
    ed.handle_key(press(KeyCode::Right));
    ed.handle_key(press(KeyCode::Right));
    ed.handle_key(press(KeyCode::Enter));
    assert_eq!(ed.line_count(), 2);
    assert_eq!(ed.lines()[0], "ab");
    assert_eq!(ed.lines()[1], "cd");
    assert_eq!(ed.cursor_row(), 1);
    assert_eq!(ed.cursor_col(), 0);
}

#[test]
fn tab_inserts_spaces() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Tab));
    assert_eq!(ed.lines()[0], "    x");
    assert_eq!(ed.cursor_col(), TAB_WIDTH);
}

// ── Save ──────────────────────────────────────────────────────────────

#[test]
fn save_writes_to_disk() {
    let (_dir, path) = temp_file("original");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::End));
    ed.handle_key(press(KeyCode::Char('!')));
    ed.save().unwrap();
    let on_disk = fs::read_to_string(&path).unwrap();
    assert_eq!(on_disk, "original!");
}

#[test]
fn save_clears_modified_flag() {
    let (_dir, path) = temp_file("hi");
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::Char('x')));
    assert!(ed.is_modified());
    ed.save().unwrap();
    assert!(!ed.is_modified());
}

// ── Key handling — action types ───────────────────────────────────────

#[test]
fn esc_returns_exit() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.handle_key(press(KeyCode::Esc)), EditorAction::Exit);
}

#[test]
fn ctrl_s_saves_and_returns_saved() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    let action = ed.handle_key(press_mod(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert_eq!(action, EditorAction::Saved);
}

#[test]
fn regular_char_returns_continue() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    let action = ed.handle_key(press(KeyCode::Char('a')));
    assert_eq!(action, EditorAction::Continue);
}

// ── Scroll ────────────────────────────────────────────────────────────

#[test]
fn scroll_keeps_cursor_visible() {
    // Create a file with 40 lines — more than one page (PAGE_SIZE = 20).
    let content: String = (0..40)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (_dir, path) = temp_file(&content);
    let mut ed = InlineEditor::open(&path).unwrap();
    // Move cursor down past the page boundary.
    for _ in 0..25 {
        ed.handle_key(press(KeyCode::Down));
    }
    assert_eq!(ed.cursor_row(), 25);
    // scroll_row should have adjusted so cursor is visible.
    assert!(ed.scroll_row() <= ed.cursor_row());
    assert!(ed.cursor_row() < ed.scroll_row() + PAGE_SIZE);
}

#[test]
fn page_down_advances_scroll() {
    let content: String = (0..60)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (_dir, path) = temp_file(&content);
    let mut ed = InlineEditor::open(&path).unwrap();
    ed.handle_key(press(KeyCode::PageDown));
    assert_eq!(ed.cursor_row(), PAGE_SIZE);
    assert!(ed.scroll_row() <= ed.cursor_row());
}

#[test]
fn page_up_retreats_scroll() {
    let content: String = (0..60)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (_dir, path) = temp_file(&content);
    let mut ed = InlineEditor::open(&path).unwrap();
    // Go down two pages then up one.
    ed.handle_key(press(KeyCode::PageDown));
    ed.handle_key(press(KeyCode::PageDown));
    let row_before = ed.cursor_row();
    ed.handle_key(press(KeyCode::PageUp));
    assert_eq!(ed.cursor_row(), row_before - PAGE_SIZE);
}

// ── Release / Repeat events are ignored ───────────────────────────────

#[test]
fn release_event_is_ignored() {
    let (_dir, path) = temp_file("x");
    let mut ed = InlineEditor::open(&path).unwrap();
    let release = KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: KeyEventState::NONE,
    };
    let action = ed.handle_key(release);
    assert_eq!(action, EditorAction::Continue);
    // Nothing should have been inserted.
    assert_eq!(ed.lines()[0], "x");
}

// ── UTF-8 safety ──────────────────────────────────────────────────────

#[test]
fn insert_and_delete_multibyte_chars() {
    let (_dir, path) = temp_file("aé");
    let mut ed = InlineEditor::open(&path).unwrap();
    assert_eq!(ed.lines()[0].chars().count(), 2);

    // Move to col 1 (between 'a' and 'é'), insert '→'
    ed.handle_key(press(KeyCode::Right));
    ed.handle_key(press(KeyCode::Char('→')));
    assert_eq!(ed.lines()[0], "a→é");
    assert_eq!(ed.cursor_col(), 2);

    // Backspace should remove '→'
    ed.handle_key(press(KeyCode::Backspace));
    assert_eq!(ed.lines()[0], "aé");
    assert_eq!(ed.cursor_col(), 1);
}

// ── CRLF handling ─────────────────────────────────────────────────────

#[test]
fn crlf_line_endings_are_handled() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("crlf.txt");
    fs::write(&path, "line1\r\nline2\r\n").unwrap();
    let ed = InlineEditor::open(&path).unwrap();
    // `str::lines()` strips the trailing empty line produced by a
    // trailing line-ending, so we only get two lines here.
    assert_eq!(ed.lines(), &["line1", "line2"]);
}
