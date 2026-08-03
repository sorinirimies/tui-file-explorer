//! Tests for the [`super`] explorer module and its `keys`/`builder`/`entries` siblings.

use super::*;
use crossterm::event::{KeyEvent, KeyModifiers};
use std::fs;
use tempfile::{tempdir, TempDir};

// ── Fixtures ──────────────────────────────────────────────────────────────

fn temp_dir_with_files() -> TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("ubuntu.iso"), b"fake iso content").unwrap();
    fs::write(dir.path().join("debian.img"), b"fake img content").unwrap();
    fs::write(dir.path().join("readme.txt"), b"some text").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    dir
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// ── Existing tests ────────────────────────────────────────────────────────

#[test]
fn new_loads_entries() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into(), "img".into()]);
    assert!(explorer
        .entries
        .iter()
        .any(|e| e.name == "subdir" && e.is_dir));
    assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
    assert!(explorer.entries.iter().any(|e| e.name == "debian.img"));
    // .txt excluded by filter
    assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
}

#[test]
fn no_filter_shows_all_files() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(explorer.entries.iter().any(|e| e.name == "readme.txt"));
}

#[test]
fn dirs_listed_before_files() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let first_file_idx = explorer
        .entries
        .iter()
        .position(|e| !e.is_dir)
        .unwrap_or(usize::MAX);
    let last_dir_idx = explorer.entries.iter().rposition(|e| e.is_dir).unwrap_or(0);
    assert!(
        last_dir_idx < first_file_idx,
        "all dirs must appear before any file"
    );
}

#[test]
fn move_down_increments_cursor() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.move_down();
    assert_eq!(explorer.cursor, 1);
}

#[test]
fn move_up_clamps_at_zero() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.move_up();
    assert_eq!(explorer.cursor, 0);
}

#[test]
fn move_down_clamps_at_last() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let last = explorer.entries.len() - 1;
    explorer.cursor = last;
    explorer.move_down();
    assert_eq!(explorer.cursor, last);
}

#[test]
fn handle_key_down_moves_cursor() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let before = explorer.cursor;
    explorer.handle_key(key(KeyCode::Down));
    assert_eq!(explorer.cursor, before + 1);
}

#[test]
fn handle_key_esc_dismisses() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(
        explorer.handle_key(key(KeyCode::Esc)),
        ExplorerOutcome::Dismissed
    );
}

#[test]
fn handle_key_enter_on_dir_descends() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Place cursor on the directory (dirs sort first).
    let dir_idx = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("no dir in fixture");
    explorer.cursor = dir_idx;
    let expected_path = explorer.entries[dir_idx].path.clone();
    let outcome = explorer.handle_key(key(KeyCode::Enter));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.current_dir, expected_path);
}

#[test]
fn handle_key_enter_on_valid_file_selects() {
    let tmp = temp_dir_with_files();
    let mut explorer =
        FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into(), "img".into()]);
    let file_idx = explorer
        .entries
        .iter()
        .position(|e| !e.is_dir)
        .expect("no file in fixture");
    explorer.cursor = file_idx;
    let expected = explorer.entries[file_idx].path.clone();
    let outcome = explorer.handle_key(key(KeyCode::Enter));
    assert_eq!(outcome, ExplorerOutcome::Selected(expected));
}

#[test]
fn handle_key_backspace_ascends() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir, vec![]);
    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.current_dir, tmp.path());
}

#[test]
fn toggle_hidden_changes_visibility() {
    let tmp = temp_dir_with_files();
    fs::write(tmp.path().join(".hidden_file"), b"").unwrap();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.entries.iter().any(|e| e.name == ".hidden_file"));
    explorer.set_show_hidden(true);
    assert!(explorer.entries.iter().any(|e| e.name == ".hidden_file"));
}

#[test]
fn fmt_size_formats_bytes() {
    assert_eq!(fmt_size(512), "512 B");
    assert_eq!(fmt_size(1_536), "1.5 KB");
    assert_eq!(fmt_size(2_097_152), "2.0 MB");
    assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
}

#[test]
fn extension_filter_only_shows_matching_files() {
    // The real selectability contract lives in load_entries: only files
    // whose extension matches the filter appear in entries at all.
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into()]);

    // Matching file is present.
    assert!(
        explorer.entries.iter().any(|e| e.name == "ubuntu.iso"),
        "iso file should appear in entries"
    );
    // Non-matching file is absent.
    assert!(
        !explorer.entries.iter().any(|e| e.name == "debian.img"),
        "img file should be excluded by filter"
    );
    // Directories are always present regardless of the filter.
    assert!(
        explorer.entries.iter().any(|e| e.is_dir),
        "directories should always be visible"
    );
    // Every visible non-directory entry has the expected extension.
    assert!(
        explorer
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .all(|e| e.extension == "iso"),
        "all visible files must match the active filter"
    );
}

#[test]
fn navigate_to_resets_cursor_and_scroll() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 2;
    explorer.scroll_offset = 1;
    explorer.navigate_to(tmp.path().to_path_buf());
    assert_eq!(explorer.cursor, 0);
    assert_eq!(explorer.scroll_offset, 0);
}

#[test]
fn current_entry_returns_highlighted() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    let entry = explorer.current_entry().expect("should have entry");
    assert_eq!(entry, explorer.entries.first().unwrap());
}

#[test]
fn unrecognised_key_returns_unhandled() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(
        explorer.handle_key(key(KeyCode::F(5))),
        ExplorerOutcome::Unhandled
    );
}

// ── Search tests ──────────────────────────────────────────────────────────

#[test]
fn slash_activates_search_mode() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.search_active);
    explorer.handle_key(key(KeyCode::Char('/')));
    assert!(explorer.search_active);
    assert_eq!(explorer.search_query(), "");
}

#[test]
fn search_active_chars_append_to_query() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    explorer.handle_key(key(KeyCode::Char('u')));
    explorer.handle_key(key(KeyCode::Char('b')));
    explorer.handle_key(key(KeyCode::Char('u')));
    assert_eq!(explorer.search_query(), "ubu");
    assert!(explorer.search_active);
}

#[test]
fn search_filters_entries_by_name() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Activate search and type a query that matches only ubuntu.iso
    explorer.handle_key(key(KeyCode::Char('/')));
    for c in "ubu".chars() {
        explorer.handle_key(key(KeyCode::Char(c)));
    }
    // Only ubuntu.iso (and nothing else) should be visible.
    assert_eq!(explorer.entries.len(), 1);
    assert_eq!(explorer.entries[0].name, "ubuntu.iso");
}

#[test]
fn search_backspace_pops_last_char() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    explorer.handle_key(key(KeyCode::Char('u')));
    explorer.handle_key(key(KeyCode::Char('b')));
    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.search_query(), "u");
    assert!(explorer.search_active);
}

#[test]
fn search_backspace_on_empty_deactivates() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    assert!(explorer.search_active);
    // Backspace on an empty query deactivates search.
    explorer.handle_key(key(KeyCode::Backspace));
    assert!(!explorer.search_active);
    assert_eq!(explorer.search_query(), "");
}

#[test]
fn search_esc_clears_and_deactivates_returns_pending() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    explorer.handle_key(key(KeyCode::Char('u')));
    let outcome = explorer.handle_key(key(KeyCode::Esc));
    assert_eq!(
        outcome,
        ExplorerOutcome::Pending,
        "Esc should clear search, not dismiss"
    );
    assert!(!explorer.search_active);
    assert_eq!(explorer.search_query(), "");
}

#[test]
fn esc_when_not_searching_dismisses() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.search_active);
    assert_eq!(
        explorer.handle_key(key(KeyCode::Esc)),
        ExplorerOutcome::Dismissed
    );
}

#[test]
fn search_clears_on_directory_descend() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.search_active = true;
    explorer.search_query = "sub".into();
    // Navigate into subdir
    explorer.cursor = explorer.entries.iter().position(|e| e.is_dir).unwrap();
    explorer.handle_key(key(KeyCode::Enter));
    assert!(!explorer.search_active);
    assert_eq!(explorer.search_query(), "");
}

#[test]
fn search_clears_on_ascend() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir, vec![]);

    // Manually inject search state (simulates user having typed a query
    // while already inside subdir, then pressing the ascend key).
    // When search_active is true, ALL KeyCode::Char(_) keys are consumed
    // by the search interception block — they append to the query rather
    // than triggering navigation.  Backspace pops the query.  The only
    // way to ascend while search is active is via the non-char ascend
    // keys, but those aren't exposed through handle_key without going
    // through the search block first.  Call ascend() directly: this is
    // the correct unit test for the ascend() logic itself, independent
    // of key dispatch.
    explorer.search_active = true;
    explorer.search_query = "foo".into();

    // Call ascend() directly — ascend() clears search state unconditionally.
    explorer.ascend();

    assert!(
        !explorer.search_active,
        "search must be deactivated after ascend"
    );
    assert_eq!(
        explorer.search_query(),
        "",
        "query must be cleared after ascend"
    );
    assert_eq!(
        explorer.current_dir,
        tmp.path(),
        "must have ascended to parent"
    );
}

#[test]
fn backspace_in_search_pops_char_not_ascend() {
    // Verify Backspace is consumed by search interception (pops the query)
    // and does NOT trigger ascend when search is active with a non-empty query.
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir.clone(), vec![]);
    explorer.search_active = true;
    explorer.search_query = "foo".into();

    explorer.handle_key(key(KeyCode::Backspace)); // should pop 'o', not ascend

    assert_eq!(explorer.current_dir, subdir, "must NOT have ascended");
    assert_eq!(
        explorer.search_query(),
        "fo",
        "Backspace should pop last char"
    );
    assert!(explorer.search_active, "search must still be active");
}

// ── Sort tests ────────────────────────────────────────────────────────────

#[test]
fn default_sort_mode_is_name() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(explorer.sort_mode(), SortMode::Name);
}

#[test]
fn sort_mode_cycles_on_s_key() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(explorer.sort_mode(), SortMode::Name);
    explorer.handle_key(key(KeyCode::Char('s')));
    assert_eq!(explorer.sort_mode(), SortMode::SizeDesc);
    explorer.handle_key(key(KeyCode::Char('s')));
    assert_eq!(explorer.sort_mode(), SortMode::Extension);
    explorer.handle_key(key(KeyCode::Char('s')));
    assert_eq!(explorer.sort_mode(), SortMode::Name);
}

#[test]
fn show_sizes_defaults_to_true() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(explorer.show_sizes);
}

#[test]
fn z_key_toggles_show_sizes() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(explorer.show_sizes);
    explorer.handle_key(key(KeyCode::Char('z')));
    assert!(!explorer.show_sizes);
    explorer.handle_key(key(KeyCode::Char('z')));
    assert!(explorer.show_sizes);
}

#[test]
fn builder_show_sizes_false_disables_sizes() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::builder(tmp.path().to_path_buf())
        .show_sizes(false)
        .build();
    assert!(!explorer.show_sizes);
}

#[test]
fn ensure_dir_size_before_computes_and_caches_within_budget() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    let (bytes, partial) = explorer.ensure_dir_size_before(&subdir, Some(1), Some(deadline));
    assert!(!partial);
    assert!(explorer.dir_size_cache.contains_key(&subdir));
    // A second call with an already-passed deadline should still hit the
    // (now warm) cache instead of skipping.
    let past_deadline = std::time::Instant::now();
    let (cached_bytes, _) = explorer.ensure_dir_size_before(&subdir, Some(1), Some(past_deadline));
    assert_eq!(bytes, cached_bytes);
}

#[test]
fn ensure_dir_size_before_skips_uncached_walk_past_deadline() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Deadline already in the past: an uncached directory must not be
    // walked, and no cache entry should be inserted.
    let past_deadline = std::time::Instant::now();
    let (bytes, partial) = explorer.ensure_dir_size_before(&subdir, Some(1), Some(past_deadline));
    assert_eq!(bytes, 0);
    assert!(!partial);
    assert!(!explorer.dir_size_cache.contains_key(&subdir));
}

#[test]
fn sort_size_desc_orders_largest_first() {
    let tmp = tempfile::tempdir().expect("temp dir");
    // Create files with clearly different sizes.
    fs::write(tmp.path().join("small.txt"), vec![0u8; 10]).unwrap();
    fs::write(tmp.path().join("large.txt"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("medium.txt"), vec![0u8; 1_000]).unwrap();

    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.set_sort_mode(SortMode::SizeDesc);

    let sizes: Vec<u64> = explorer.entries.iter().filter_map(|e| e.size).collect();
    let mut sorted_desc = sizes.clone();
    sorted_desc.sort_by(|a, b| b.cmp(a));
    assert_eq!(sizes, sorted_desc, "files should be sorted largest-first");
}

#[test]
fn sort_extension_groups_by_ext() {
    let tmp = tempfile::tempdir().expect("temp dir");
    fs::write(tmp.path().join("b.toml"), b"").unwrap();
    fs::write(tmp.path().join("a.rs"), b"").unwrap();
    fs::write(tmp.path().join("c.toml"), b"").unwrap();
    fs::write(tmp.path().join("z.rs"), b"").unwrap();

    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.set_sort_mode(SortMode::Extension);

    let exts: Vec<&str> = explorer
        .entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.extension.as_str())
        .collect();

    // All rs entries should appear before toml entries (r < t).
    let rs_last = exts.iter().rposition(|&e| e == "rs").unwrap_or(0);
    let toml_first = exts.iter().position(|&e| e == "toml").unwrap_or(usize::MAX);
    assert!(rs_last < toml_first, "rs group must precede toml group");
}

#[test]
fn builder_sort_mode_applied() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::builder(tmp.path().to_path_buf())
        .sort_mode(SortMode::SizeDesc)
        .build();
    assert_eq!(explorer.sort_mode(), SortMode::SizeDesc);
}

#[test]
fn set_sort_mode_reloads() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.set_sort_mode(SortMode::Extension);
    assert_eq!(explorer.sort_mode(), SortMode::Extension);
    // Entries should still be present after the reload triggered by set_sort_mode.
    assert!(!explorer.entries.is_empty());
}

// ── Vim key tests ─────────────────────────────────────────────────────────

#[test]
fn j_key_moves_cursor_down() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let before = explorer.cursor;
    explorer.handle_key(key(KeyCode::Char('j')));
    assert_eq!(explorer.cursor, before + 1);
}

#[test]
fn k_key_moves_cursor_up() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 2;
    explorer.handle_key(key(KeyCode::Char('k')));
    assert_eq!(explorer.cursor, 1);
}

#[test]
fn h_key_ascends_to_parent() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir, vec![]);
    explorer.handle_key(key(KeyCode::Char('h')));
    assert_eq!(explorer.current_dir, tmp.path());
}

#[test]
fn l_key_descends_into_dir() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let dir_idx = explorer.entries.iter().position(|e| e.is_dir).unwrap();
    explorer.cursor = dir_idx;
    let expected = explorer.entries[dir_idx].path.clone();
    let outcome = explorer.handle_key(key(KeyCode::Char('l')));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.current_dir, expected);
}

#[test]
fn right_arrow_descends_into_dir() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let dir_idx = explorer.entries.iter().position(|e| e.is_dir).unwrap();
    explorer.cursor = dir_idx;
    let expected = explorer.entries[dir_idx].path.clone();
    let outcome = explorer.handle_key(key(KeyCode::Right));
    assert_eq!(
        outcome,
        ExplorerOutcome::Pending,
        "Right arrow should descend into directory"
    );
    assert_eq!(
        explorer.current_dir, expected,
        "Right arrow should change into the selected directory"
    );
}

#[test]
fn right_arrow_on_file_moves_down_not_exits() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Pick the first file entry that is not the last entry so cursor can advance.
    let file_idx = explorer.entries.iter().position(|e| !e.is_dir).unwrap();
    // Ensure there is an entry after it to move to.
    assert!(
        file_idx + 1 < explorer.entries.len(),
        "fixture must have an entry after the first file"
    );
    explorer.cursor = file_idx;
    let original_dir = explorer.current_dir.clone();
    let outcome = explorer.handle_key(key(KeyCode::Right));
    assert_eq!(
        outcome,
        ExplorerOutcome::Pending,
        "Right arrow on a file must never exit (always Pending)"
    );
    assert_eq!(
        explorer.current_dir, original_dir,
        "Right arrow on a file must not change directory"
    );
    assert_eq!(
        explorer.cursor,
        file_idx + 1,
        "Right arrow on a file must advance the cursor by one"
    );
}

#[test]
fn right_arrow_on_file_at_last_entry_does_not_overflow() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let last = explorer.entries.len() - 1;
    // Force cursor onto the last entry (guaranteed to exist in the fixture).
    explorer.cursor = last;
    explorer.handle_key(key(KeyCode::Right));
    assert_eq!(
        explorer.cursor, last,
        "Right arrow at the last entry must not overflow past it"
    );
}

#[test]
fn enter_on_file_still_confirms_and_exits() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let file_idx = explorer.entries.iter().position(|e| !e.is_dir).unwrap();
    explorer.cursor = file_idx;
    let expected = explorer.entries[file_idx].path.clone();
    let outcome = explorer.handle_key(key(KeyCode::Enter));
    assert_eq!(
        outcome,
        ExplorerOutcome::Selected(expected),
        "Enter on a file should confirm (select) it and exit"
    );
}

#[test]
fn left_arrow_ascends_to_parent() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir, vec![]);
    let outcome = explorer.handle_key(key(KeyCode::Left));
    assert_eq!(
        outcome,
        ExplorerOutcome::Pending,
        "Left arrow should return Pending after ascending"
    );
    assert_eq!(
        explorer.current_dir,
        tmp.path(),
        "Left arrow should ascend to the parent directory"
    );
}

#[test]
fn right_arrow_clears_search_on_dir_descend() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Activate search so we can verify navigate() clears it.
    explorer.search_active = true;
    explorer.search_query = "sub".to_string();
    explorer.reload();
    // The search should have narrowed entries to the subdir.
    let dir_idx = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("fixture subdir must match 'sub'");
    explorer.cursor = dir_idx;
    explorer.handle_key(key(KeyCode::Right));
    assert!(
        !explorer.search_active,
        "navigate() must deactivate search on directory descend"
    );
    assert!(
        explorer.search_query.is_empty(),
        "navigate() must clear search query on directory descend"
    );
}

#[test]
fn right_arrow_clears_marks_on_dir_descend() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let dir_idx = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("fixture has a subdir");
    // Mark an entry before descending.
    explorer.toggle_mark();
    assert!(
        !explorer.marked.is_empty(),
        "should have a mark before descend"
    );
    // Reset cursor back to the directory entry.
    explorer.cursor = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("fixture has a subdir");
    explorer.handle_key(key(KeyCode::Right));
    assert!(
        explorer.marked.is_empty(),
        "navigate() must clear marks on directory descend"
    );
    let _ = dir_idx;
}

#[test]
fn backspace_still_ascends() {
    let tmp = temp_dir_with_files();
    let subdir = tmp.path().join("subdir");
    let mut explorer = FileExplorer::new(subdir, vec![]);
    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.current_dir, tmp.path());
}

#[test]
fn q_key_dismisses() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(
        explorer.handle_key(key(KeyCode::Char('q'))),
        ExplorerOutcome::Dismissed
    );
}

// ── Page / jump key tests ─────────────────────────────────────────────────

#[test]
fn page_down_advances_cursor_by_ten() {
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..15 {
        fs::write(tmp.path().join(format!("file{i:02}.txt")), b"").unwrap();
    }
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    explorer.handle_key(key(KeyCode::PageDown));
    assert_eq!(explorer.cursor, 10);
}

#[test]
fn page_up_retreats_cursor_by_ten() {
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..15 {
        fs::write(tmp.path().join(format!("file{i:02}.txt")), b"").unwrap();
    }
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 12;
    explorer.handle_key(key(KeyCode::PageUp));
    assert_eq!(explorer.cursor, 2);
}

#[test]
fn home_key_jumps_to_top() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = explorer.entries.len() - 1;
    explorer.handle_key(key(KeyCode::Home));
    assert_eq!(explorer.cursor, 0);
    assert_eq!(explorer.scroll_offset, 0);
}

#[test]
fn g_key_jumps_to_top() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = explorer.entries.len() - 1;
    explorer.handle_key(key(KeyCode::Char('g')));
    assert_eq!(explorer.cursor, 0);
    assert_eq!(explorer.scroll_offset, 0);
}

#[test]
fn end_key_jumps_to_bottom() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    explorer.handle_key(key(KeyCode::End));
    assert_eq!(explorer.cursor, explorer.entries.len() - 1);
}

#[test]
fn capital_g_key_jumps_to_bottom() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    let key_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
    explorer.handle_key(key_g);
    assert_eq!(explorer.cursor, explorer.entries.len() - 1);
}

// ── Root / status tests ───────────────────────────────────────────────────

#[test]
fn ascend_at_root_sets_status() {
    // Use "/" as a reliable filesystem root on macOS/Linux.
    let root = std::path::PathBuf::from("/");
    let mut explorer = FileExplorer::new(root.clone(), vec![]);
    assert!(explorer.is_at_root());
    // Still at root after attempted ascend.
    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.current_dir, root);
    assert!(
        !explorer.status().is_empty(),
        "status should report already at root"
    );
}

#[test]
fn is_at_root_false_for_subdir() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.is_at_root());
}

// ── Accessor tests ────────────────────────────────────────────────────────

#[test]
fn is_empty_reflects_visible_entries() {
    let empty_dir = tempfile::tempdir().unwrap();
    let explorer = FileExplorer::new(empty_dir.path().to_path_buf(), vec![]);
    assert!(explorer.is_empty());

    let tmp = temp_dir_with_files();
    let explorer2 = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer2.is_empty());
}

#[test]
fn entry_count_matches_entries_len() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(explorer.entry_count(), explorer.entries.len());
    assert!(explorer.entry_count() > 0);
}

#[test]
fn search_query_empty_when_not_searching() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.is_searching());
    assert_eq!(explorer.search_query(), "");
}

// ── Case-insensitivity tests ──────────────────────────────────────────────

#[test]
fn search_is_case_insensitive() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Type "UBU" in uppercase — should still match "ubuntu.iso".
    explorer.handle_key(key(KeyCode::Char('/')));
    for c in "UBU".chars() {
        explorer.handle_key(key(KeyCode::Char(c)));
    }
    assert_eq!(explorer.entries.len(), 1);
    assert_eq!(explorer.entries[0].name, "ubuntu.iso");
}

#[test]
fn extension_filter_is_case_insensitive() {
    let tmp = tempfile::tempdir().unwrap();
    // File whose on-disk extension is upper-case.
    fs::write(tmp.path().join("disk.ISO"), b"data").unwrap();
    fs::write(tmp.path().join("other.txt"), b"text").unwrap();

    // Filter expressed in lower-case should still match the upper-case ext.
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec!["iso".into()]);
    assert!(
        explorer.entries.iter().any(|e| e.name == "disk.ISO"),
        "upper-case extension should be matched by lower-case filter"
    );
    assert!(
        !explorer.entries.iter().any(|e| e.name == "other.txt"),
        "non-matching extension should be excluded"
    );
}

// ── Builder tests ─────────────────────────────────────────────────────────

#[test]
fn builder_allow_extension_filters_entries() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::builder(tmp.path().to_path_buf())
        .allow_extension("iso")
        .build();
    assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
    assert!(!explorer.entries.iter().any(|e| e.name == "debian.img"));
    assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
}

#[test]
fn builder_show_hidden_shows_dotfiles() {
    let tmp = temp_dir_with_files();
    fs::write(tmp.path().join(".dotfile"), b"").unwrap();

    let hidden_explorer = FileExplorer::builder(tmp.path().to_path_buf())
        .show_hidden(true)
        .build();
    assert!(hidden_explorer.entries.iter().any(|e| e.name == ".dotfile"));

    let normal_explorer = FileExplorer::builder(tmp.path().to_path_buf())
        .show_hidden(false)
        .build();
    assert!(!normal_explorer.entries.iter().any(|e| e.name == ".dotfile"));
}

#[test]
fn set_extension_filter_updates_entries() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // All files visible with no filter.
    assert!(explorer.entries.iter().any(|e| e.name == "readme.txt"));

    explorer.set_extension_filter(["iso"]);
    assert!(explorer.entries.iter().any(|e| e.name == "ubuntu.iso"));
    assert!(!explorer.entries.iter().any(|e| e.name == "readme.txt"));
}

// ── entry_icon tests ──────────────────────────────────────────────────────

#[test]
fn entry_icon_directory() {
    let entry = FsEntry {
        name: "mydir".into(),
        path: std::path::PathBuf::from("/mydir"),
        is_dir: true,
        size: None,
        item_count: Some(0),
        extension: String::new(),
    };
    assert_eq!(entry_icon(&entry), "📁");
}

#[test]
fn entry_icon_recognises_known_extensions() {
    let make = |name: &str, ext: &str| FsEntry {
        name: name.into(),
        path: std::path::PathBuf::from(name),
        is_dir: false,
        size: Some(0),
        item_count: None,
        extension: ext.into(),
    };

    assert_eq!(entry_icon(&make("archive.zip", "zip")), "📦");
    assert_eq!(entry_icon(&make("doc.pdf", "pdf")), "📕");
    assert_eq!(entry_icon(&make("notes.md", "md")), "📝");
    assert_eq!(entry_icon(&make("config.toml", "toml")), "⚙ ");
    assert_eq!(entry_icon(&make("main.rs", "rs")), "🦀");
    assert_eq!(entry_icon(&make("script.py", "py")), "🐍");
    assert_eq!(entry_icon(&make("page.html", "html")), "🌐");
    assert_eq!(entry_icon(&make("image.png", "png")), "🖼 ");
    assert_eq!(entry_icon(&make("video.mp4", "mp4")), "🎬");
    assert_eq!(entry_icon(&make("song.mp3", "mp3")), "🎵");
    assert_eq!(entry_icon(&make("unknown.xyz", "xyz")), "📄");
}

// ── fmt_size boundary tests ───────────────────────────────────────────────

#[test]
fn fmt_size_exact_boundaries() {
    // Exact powers of 1024.
    assert_eq!(fmt_size(1_024), "1.0 KB");
    assert_eq!(fmt_size(1_048_576), "1.0 MB");
    assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
    // Just below each boundary stays in the lower unit.
    assert_eq!(fmt_size(1_023), "1023 B");
    assert_eq!(fmt_size(1_047_552), "1023.0 KB"); // 1023 * 1024
}

// ── toggle_mark / clear_marks / Space key ─────────────────────────────────

#[test]
fn toggle_mark_adds_entry_to_marked_set() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(!explorer.entries.is_empty(), "need at least one entry");

    explorer.toggle_mark();

    assert_eq!(explorer.marked.len(), 1, "one entry should be marked");
}

#[test]
fn toggle_mark_removes_already_marked_entry() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

    explorer.toggle_mark(); // mark
    let cursor_after_first = explorer.cursor;
    explorer.cursor = 0; // reset to the same entry
    explorer.toggle_mark(); // unmark

    assert!(
        explorer.marked.is_empty(),
        "second toggle on same entry should unmark it"
    );
    let _ = cursor_after_first; // suppress unused warning
}

#[test]
fn toggle_mark_advances_cursor_down() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    // Ensure there are at least two entries so the cursor can advance.
    assert!(
        explorer.entries.len() >= 2,
        "fixture must have at least 2 entries"
    );

    let before = explorer.cursor;
    explorer.toggle_mark();

    assert_eq!(
        explorer.cursor,
        before + 1,
        "cursor should advance by one after toggle_mark"
    );
}

#[test]
fn toggle_mark_at_last_entry_does_not_overflow() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.cursor = explorer.entries.len() - 1;

    explorer.toggle_mark();

    assert_eq!(
        explorer.cursor,
        explorer.entries.len() - 1,
        "cursor should stay at the last entry, not overflow"
    );
}

#[test]
fn clear_marks_empties_marked_set() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

    explorer.toggle_mark();
    assert!(
        !explorer.marked.is_empty(),
        "should have a mark before clear"
    );

    explorer.clear_marks();

    assert!(
        explorer.marked.is_empty(),
        "marked set should be empty after clear_marks"
    );
}

#[test]
fn space_key_marks_current_entry() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(!explorer.entries.is_empty(), "need at least one entry");

    let outcome = explorer.handle_key(key(KeyCode::Char(' ')));

    assert_eq!(
        outcome,
        ExplorerOutcome::Pending,
        "Space should return Pending"
    );
    assert_eq!(
        explorer.marked.len(),
        1,
        "Space should mark the current entry"
    );
}

#[test]
fn space_key_toggles_mark_off() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

    explorer.handle_key(key(KeyCode::Char(' '))); // mark → cursor moves down
    explorer.cursor = 0; // reset to entry 0
    explorer.handle_key(key(KeyCode::Char(' '))); // unmark

    assert!(
        explorer.marked.is_empty(),
        "second Space on same entry should unmark it"
    );
}

#[test]
fn marks_cleared_when_ascending_to_parent() {
    let dir = temp_dir_with_files();
    // Start inside the subdir so we can ascend.
    let sub = dir.path().join("subdir");
    fs::write(sub.join("inner.txt"), b"inner").unwrap();
    let mut explorer = FileExplorer::new(sub.clone(), vec![]);

    explorer.toggle_mark();
    assert!(
        !explorer.marked.is_empty(),
        "should have a mark before ascend"
    );

    // Ascend via Backspace.
    explorer.handle_key(key(KeyCode::Backspace));

    assert!(
        explorer.marked.is_empty(),
        "marks should be cleared after ascending to parent"
    );
}

#[test]
fn marks_cleared_when_descending_into_directory() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

    // Mark the subdirectory entry.
    let sub_idx = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("fixture has a subdir");
    explorer.cursor = sub_idx;
    explorer.toggle_mark();
    assert!(
        !explorer.marked.is_empty(),
        "should have a mark before descend"
    );

    // Reset cursor back to the directory entry (toggle_mark advanced it).
    explorer.cursor = explorer
        .entries
        .iter()
        .position(|e| e.is_dir)
        .expect("fixture has a subdir");

    // Descend into the subdirectory — confirm() clears marks.
    explorer.handle_key(key(KeyCode::Enter));

    assert!(
        explorer.marked.is_empty(),
        "marks should be cleared after descending into a directory"
    );
}

#[test]
fn can_mark_multiple_entries() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    let total = explorer.entries.len();
    assert!(total >= 2, "fixture must have at least 2 entries");

    // Mark every entry.
    for _ in 0..total {
        explorer.toggle_mark();
    }

    assert_eq!(explorer.marked.len(), total, "all entries should be marked");
}

// ── Cursor / scroll boundary safety ──────────────────────────────────────

#[test]
fn move_up_at_top_does_not_underflow() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    // Should be a no-op, not a panic.
    explorer.handle_key(key(KeyCode::Up));
    assert_eq!(explorer.cursor, 0);
}

#[test]
fn move_down_at_bottom_does_not_overflow() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    let last = explorer.entries.len().saturating_sub(1);
    explorer.cursor = last;
    explorer.handle_key(key(KeyCode::Down));
    assert_eq!(explorer.cursor, last);
}

#[test]
fn move_down_on_empty_dir_does_not_panic() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(explorer.entries.is_empty());
    // Must not panic.
    explorer.handle_key(key(KeyCode::Down));
    assert_eq!(explorer.cursor, 0);
}

#[test]
fn move_up_on_empty_dir_does_not_panic() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(explorer.entries.is_empty());
    explorer.handle_key(key(KeyCode::Up));
    assert_eq!(explorer.cursor, 0);
}

#[test]
fn page_down_at_bottom_does_not_overflow() {
    let dir = tempdir().expect("tempdir");
    for i in 0..5 {
        fs::write(dir.path().join(format!("{i}.txt")), b"x").unwrap();
    }
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    let last = explorer.entries.len().saturating_sub(1);
    explorer.cursor = last;
    explorer.handle_key(key(KeyCode::PageDown));
    assert_eq!(explorer.cursor, last);
}

#[test]
fn page_up_at_top_does_not_underflow() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.cursor = 0;
    explorer.handle_key(key(KeyCode::PageUp));
    assert_eq!(explorer.cursor, 0);
}

#[test]
fn ascend_at_root_does_not_panic() {
    let mut explorer = FileExplorer::new(std::path::PathBuf::from("/"), vec![]);
    // Pressing Backspace at root must not panic — it should stay put.
    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.current_dir, std::path::PathBuf::from("/"));
}

#[test]
fn cursor_clamped_after_reload_with_fewer_entries() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    fs::write(dir.path().join("c.txt"), b"c").unwrap();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    // Move to last entry.
    explorer.cursor = explorer.entries.len() - 1;
    // Now apply a filter that shows only one file — reload happens inside.
    explorer.set_extension_filter(["a"]);
    // Cursor must be clamped to the new (smaller) list.
    assert!(
        explorer.cursor < explorer.entries.len().max(1),
        "cursor {} out of range for {} entries",
        explorer.cursor,
        explorer.entries.len()
    );
}

#[test]
fn scroll_offset_clamped_after_reload_with_empty_entries() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("test.rs"), b"fn main(){}").unwrap();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.scroll_offset = 5; // artificially stale
    explorer.cursor = 0;
    // Apply a filter that matches nothing — entries becomes empty.
    explorer.set_extension_filter(["xyz"]);
    assert_eq!(explorer.cursor, 0);
    assert_eq!(explorer.scroll_offset, 0);
}

#[test]
fn marked_paths_returns_reference_to_marked_set() {
    let dir = temp_dir_with_files();
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);

    explorer.toggle_mark();

    assert_eq!(
        explorer.marked_paths().len(),
        explorer.marked.len(),
        "marked_paths() should reflect the same set as the field"
    );
}

// ── entry_icon — extended coverage ───────────────────────────────────────

fn make_file_entry(name: &str) -> crate::types::FsEntry {
    let ext = std::path::Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    crate::types::FsEntry {
        name: name.to_string(),
        path: std::path::PathBuf::from(name),
        is_dir: false,
        size: None,
        item_count: None,
        extension: ext,
    }
}

macro_rules! assert_entry_icon {
    ($( $test_name:ident : $filename:expr => $icon:expr ),+ $(,)?) => {
        $(
            #[test]
            fn $test_name() {
                let e = make_file_entry($filename);
                assert_eq!(entry_icon(&e), $icon);
            }
        )+
    };
}

assert_entry_icon! {
    entry_icon_iso_returns_disc:                      "release.iso"  => "💿",
    entry_icon_dmg_returns_disc:                      "app.dmg"      => "💿",
    entry_icon_zip_returns_package:                   "archive.zip"  => "📦",
    entry_icon_tar_returns_package:                   "src.tar"      => "📦",
    entry_icon_gz_returns_package:                    "data.gz"      => "📦",
    entry_icon_pdf_returns_book:                      "manual.pdf"   => "📕",
    entry_icon_md_returns_memo:                       "README.md"    => "📝",
    entry_icon_toml_returns_gear:                     "Cargo.toml"   => "⚙ ",
    entry_icon_json_returns_gear:                     "config.json"  => "⚙ ",
    entry_icon_lock_returns_lock:                     "Cargo.lock"   => "🔒",
    entry_icon_py_returns_snake:                      "script.py"    => "🐍",
    entry_icon_html_returns_globe:                    "index.html"   => "🌐",
    entry_icon_css_returns_palette:                   "style.css"    => "🎨",
    entry_icon_svg_returns_palette:                   "logo.svg"     => "🎨",
    entry_icon_png_returns_image:                     "photo.png"    => "🖼 ",
    entry_icon_jpg_returns_image:                     "photo.jpg"    => "🖼 ",
    entry_icon_mp4_returns_film:                      "video.mp4"    => "🎬",
    entry_icon_mp3_returns_music:                     "song.mp3"     => "🎵",
    entry_icon_ttf_returns_font:                      "font.ttf"     => "🔤",
    entry_icon_exe_returns_gear:                      "setup.exe"    => "⚙ ",
    entry_icon_unknown_extension_returns_document:    "mystery.xyz"  => "📄",
}

#[test]
fn entry_icon_no_extension_returns_document() {
    let e = crate::types::FsEntry {
        name: "Makefile".into(),
        path: std::path::PathBuf::from("Makefile"),
        is_dir: false,
        size: None,
        item_count: None,
        extension: String::new(),
    };
    assert_eq!(entry_icon(&e), "📄");
}

// ── fmt_size — full boundary coverage ────────────────────────────────────

#[test]
fn fmt_size_zero_bytes() {
    assert_eq!(fmt_size(0), "0 B");
}

#[test]
fn fmt_size_one_byte() {
    assert_eq!(fmt_size(1), "1 B");
}

#[test]
fn fmt_size_1023_bytes_stays_bytes() {
    assert_eq!(fmt_size(1_023), "1023 B");
}

#[test]
fn fmt_size_exactly_1_kb() {
    assert_eq!(fmt_size(1_024), "1.0 KB");
}

#[test]
fn fmt_size_1_5_kb() {
    assert_eq!(fmt_size(1_536), "1.5 KB");
}

#[test]
fn fmt_size_1_mb_boundary() {
    assert_eq!(fmt_size(1_048_576), "1.0 MB");
}

#[test]
fn fmt_size_2_mb() {
    assert_eq!(fmt_size(2_097_152), "2.0 MB");
}

#[test]
fn fmt_size_1_gb_boundary() {
    assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
}

#[test]
fn fmt_size_large_value() {
    // 10 GB
    assert_eq!(fmt_size(10 * 1_073_741_824), "10.0 GB");
}

// ── navigate_to — &str and &Path inputs ──────────────────────────────────

#[test]
fn navigate_to_accepts_str_slice() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.navigate_to(sub.to_str().unwrap());
    assert_eq!(explorer.current_dir, sub);
}

#[test]
fn navigate_to_accepts_path_ref() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("sub2");
    fs::create_dir(&sub).unwrap();

    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.navigate_to(sub.as_path());
    assert_eq!(explorer.current_dir, sub);
}

#[test]
fn navigate_to_resets_cursor_to_zero() {
    let dir = tempdir().expect("tempdir");
    let sub = dir.path().join("sub3");
    fs::create_dir(&sub).unwrap();

    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.cursor = 99;
    explorer.scroll_offset = 5;
    explorer.navigate_to(sub.as_path());
    assert_eq!(explorer.cursor, 0);
    assert_eq!(explorer.scroll_offset, 0);
}

// ── is_searching accessor ─────────────────────────────────────────────────

#[test]
fn is_searching_false_by_default() {
    let dir = tempdir().expect("tempdir");
    let explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(!explorer.is_searching());
}

#[test]
fn is_searching_true_after_slash_key() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    assert!(explorer.is_searching());
}

#[test]
fn is_searching_false_after_esc_cancels_search() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('/')));
    explorer.handle_key(key(KeyCode::Esc));
    assert!(!explorer.is_searching());
}

// ── status cleared on reload ──────────────────────────────────────────────

#[test]
fn status_is_empty_on_fresh_explorer() {
    let dir = tempdir().expect("tempdir");
    let explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(explorer.status().is_empty());
}

#[test]
fn status_cleared_after_reload() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    // Manually set a stale status message.
    explorer.status = "stale message".into();
    explorer.reload();
    assert!(
        explorer.status().is_empty(),
        "reload should clear the status message"
    );
}

// ── load_entries with an empty directory ──────────────────────────────────

#[test]
fn load_entries_empty_dir_returns_empty_vec() {
    let dir = tempdir().expect("tempdir");
    let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
    assert!(
        entries.is_empty(),
        "empty directory should produce no entries"
    );
}

#[test]
fn load_entries_hidden_excluded_by_default() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join(".hidden"), b"h").unwrap();
    fs::write(dir.path().join("visible.txt"), b"v").unwrap();

    let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "visible.txt");
}

#[test]
fn load_entries_hidden_included_when_show_hidden_true() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join(".hidden"), b"h").unwrap();
    fs::write(dir.path().join("visible.txt"), b"v").unwrap();

    let entries = load_entries(dir.path(), true, &[], crate::types::SortMode::Name, "");
    assert_eq!(entries.len(), 2);
}

#[test]
fn load_entries_nonexistent_dir_returns_empty_vec() {
    let entries = load_entries(
        std::path::Path::new("/nonexistent/path/that/does/not/exist"),
        false,
        &[],
        crate::types::SortMode::Name,
        "",
    );
    assert!(entries.is_empty());
}

#[test]
fn load_entries_search_query_is_case_insensitive() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("README.md"), b"r").unwrap();
    fs::write(dir.path().join("main.rs"), b"m").unwrap();

    let entries = load_entries(
        dir.path(),
        false,
        &[],
        crate::types::SortMode::Name,
        "readme",
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "README.md");
}

#[test]
fn load_entries_dirs_always_precede_files() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("z_file.txt"), b"z").unwrap();
    fs::create_dir(dir.path().join("a_dir")).unwrap();

    let entries = load_entries(dir.path(), false, &[], crate::types::SortMode::Name, "");
    assert!(entries[0].is_dir, "directory must come before file");
    assert!(!entries[1].is_dir);
}

#[test]
fn load_entries_ext_filter_excludes_non_matching_files() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("main.rs"), b"r").unwrap();
    fs::write(dir.path().join("Cargo.toml"), b"t").unwrap();

    let filter = vec!["rs".to_string()];
    let entries = load_entries(dir.path(), false, &filter, crate::types::SortMode::Name, "");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].extension, "rs");
}

#[test]
fn load_entries_ext_filter_always_includes_dirs() {
    let dir = tempdir().expect("tempdir");
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("file.txt"), b"t").unwrap();

    // Filter for .rs — the dir should still appear, the .txt file should not.
    let filter = vec!["rs".to_string()];
    let entries = load_entries(dir.path(), false, &filter, crate::types::SortMode::Name, "");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].is_dir);
}

// ── Rename mode ───────────────────────────────────────────────────────────

#[test]
fn r_key_activates_rename_mode_with_prefilled_name() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    // Move cursor to a known file.
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .expect("readme.txt present");
    explorer.cursor = idx;

    let outcome = explorer.handle_key(key(KeyCode::Char('r')));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(explorer.is_rename_active());
    assert_eq!(explorer.rename_input(), "readme.txt");
}

#[test]
fn r_key_on_empty_dir_does_not_activate_rename() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    assert!(explorer.entries.is_empty());

    let outcome = explorer.handle_key(key(KeyCode::Char('r')));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.is_rename_active());
}

#[test]
fn rename_mode_chars_append_to_input() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('r')));
    assert!(explorer.is_rename_active());

    // Clear the prefilled name and type a fresh one.
    let original_len = explorer.rename_input().len();
    for _ in 0..original_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    explorer.handle_key(key(KeyCode::Char('n')));
    explorer.handle_key(key(KeyCode::Char('e')));
    explorer.handle_key(key(KeyCode::Char('w')));

    assert_eq!(explorer.rename_input(), "new");
    assert!(explorer.is_rename_active());
}

#[test]
fn rename_mode_backspace_pops_last_char() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('r')));

    // Pop all chars then type "ab".
    let original_len = explorer.rename_input().len();
    for _ in 0..original_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    explorer.handle_key(key(KeyCode::Char('a')));
    explorer.handle_key(key(KeyCode::Char('b')));
    assert_eq!(explorer.rename_input(), "ab");

    explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(explorer.rename_input(), "a");
}

#[test]
fn rename_mode_esc_cancels_without_renaming() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .expect("readme.txt present");
    explorer.cursor = idx;

    explorer.handle_key(key(KeyCode::Char('r')));
    assert!(explorer.is_rename_active());

    let outcome = explorer.handle_key(key(KeyCode::Esc));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.is_rename_active());
    assert_eq!(explorer.rename_input(), "");
    // File must still exist under the old name.
    assert!(tmp.path().join("readme.txt").exists());
}

#[test]
fn rename_mode_enter_renames_file_and_returns_rename_completed() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .expect("readme.txt present");
    explorer.cursor = idx;

    // Activate rename, clear prefill, type new name.
    explorer.handle_key(key(KeyCode::Char('r')));
    let prefill_len = explorer.rename_input().len();
    for _ in 0..prefill_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    for c in "notes.txt".chars() {
        explorer.handle_key(key(KeyCode::Char(c)));
    }

    let outcome = explorer.handle_key(key(KeyCode::Enter));

    assert!(!explorer.is_rename_active());
    assert_eq!(explorer.rename_input(), "");
    assert!(tmp.path().join("notes.txt").exists(), "new name must exist");
    assert!(
        !tmp.path().join("readme.txt").exists(),
        "old name must be gone"
    );
    assert!(
        matches!(outcome, ExplorerOutcome::RenameCompleted(p) if p.file_name().unwrap() == "notes.txt")
    );
}

#[test]
fn rename_mode_cursor_moves_to_renamed_entry() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .expect("readme.txt present");
    explorer.cursor = idx;

    explorer.handle_key(key(KeyCode::Char('r')));
    let prefill_len = explorer.rename_input().len();
    for _ in 0..prefill_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    for c in "zzz_last.txt".chars() {
        explorer.handle_key(key(KeyCode::Char(c)));
    }
    explorer.handle_key(key(KeyCode::Enter));

    let new_idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "zzz_last.txt")
        .expect("renamed entry in list");
    assert_eq!(explorer.cursor, new_idx);
}

#[test]
fn rename_mode_enter_with_empty_input_is_noop() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .expect("readme.txt present");
    explorer.cursor = idx;

    explorer.handle_key(key(KeyCode::Char('r')));
    // Erase the prefilled name entirely, then confirm.
    let prefill_len = explorer.rename_input().len();
    for _ in 0..prefill_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    assert_eq!(explorer.rename_input(), "");

    let outcome = explorer.handle_key(key(KeyCode::Enter));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.is_rename_active());
    // Original file must still exist.
    assert!(tmp.path().join("readme.txt").exists());
}

#[test]
fn rename_mode_can_rename_directory() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    let idx = explorer
        .entries
        .iter()
        .position(|e| e.name == "subdir" && e.is_dir)
        .expect("subdir present");
    explorer.cursor = idx;

    explorer.handle_key(key(KeyCode::Char('r')));
    let prefill_len = explorer.rename_input().len();
    for _ in 0..prefill_len {
        explorer.handle_key(key(KeyCode::Backspace));
    }
    for c in "renamed_dir".chars() {
        explorer.handle_key(key(KeyCode::Char(c)));
    }
    let outcome = explorer.handle_key(key(KeyCode::Enter));

    assert!(tmp.path().join("renamed_dir").exists());
    assert!(!tmp.path().join("subdir").exists());
    assert!(matches!(outcome, ExplorerOutcome::RenameCompleted(_)));
}

#[test]
fn rename_mode_unrecognised_key_returns_pending_without_cancelling() {
    let tmp = temp_dir_with_files();
    let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    explorer.handle_key(key(KeyCode::Char('r')));
    assert!(explorer.is_rename_active());

    // F1 is not handled inside rename mode.
    let outcome = explorer.handle_key(key(KeyCode::F(1)));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(explorer.is_rename_active(), "rename mode must stay active");
}

#[test]
fn is_rename_active_false_by_default() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert!(!explorer.is_rename_active());
}

#[test]
fn rename_input_empty_by_default() {
    let tmp = temp_dir_with_files();
    let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
    assert_eq!(explorer.rename_input(), "");
}

// ── handle_input_mode! macro tests ───────────────────────────────────────
// These tests exercise the Char-push, Backspace-pop, Esc-cancel, and
// unknown-key fallthrough paths that the macro generates for every mode.

// ── mkdir_mode via macro ──────────────────────────────────────────────────

#[test]
fn mkdir_mode_char_pushes_to_input_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.mkdir_active = true;
    explorer.mkdir_input.clear();

    let outcome = explorer.handle_key(key(KeyCode::Char('a')));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.mkdir_input, "a");
    assert!(explorer.mkdir_active, "mode must remain active after Char");
}

#[test]
fn mkdir_mode_backspace_pops_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.mkdir_active = true;
    explorer.mkdir_input = "ab".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.mkdir_input, "a");
    assert!(explorer.mkdir_active);
}

#[test]
fn mkdir_mode_esc_cancels_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.mkdir_active = true;
    explorer.mkdir_input = "half".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Esc));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.mkdir_active, "mode must be deactivated by Esc");
    assert!(
        explorer.mkdir_input.is_empty(),
        "input must be cleared by Esc"
    );
}

#[test]
fn mkdir_mode_unknown_key_returns_pending_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.mkdir_active = true;
    explorer.mkdir_input = "foo".to_string();

    let outcome = explorer.handle_key(key(KeyCode::F(2)));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    // Mode and input must be unchanged.
    assert!(explorer.mkdir_active);
    assert_eq!(explorer.mkdir_input, "foo");
}

// ── touch_mode via macro ──────────────────────────────────────────────────

#[test]
fn touch_mode_char_pushes_to_input_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.touch_active = true;
    explorer.touch_input.clear();

    let outcome = explorer.handle_key(key(KeyCode::Char('z')));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.touch_input, "z");
    assert!(explorer.touch_active);
}

#[test]
fn touch_mode_backspace_pops_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.touch_active = true;
    explorer.touch_input = "xy".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.touch_input, "x");
    assert!(explorer.touch_active);
}

#[test]
fn touch_mode_esc_cancels_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.touch_active = true;
    explorer.touch_input = "half".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Esc));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.touch_active);
    assert!(explorer.touch_input.is_empty());
}

#[test]
fn touch_mode_unknown_key_returns_pending_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.touch_active = true;
    explorer.touch_input = "bar".to_string();

    let outcome = explorer.handle_key(key(KeyCode::F(3)));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(explorer.touch_active);
    assert_eq!(explorer.touch_input, "bar");
}

// ── rename_mode via macro ─────────────────────────────────────────────────

#[test]
fn rename_mode_char_pushes_to_input_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.rename_active = true;
    explorer.rename_input.clear();

    let outcome = explorer.handle_key(key(KeyCode::Char('r')));
    // NOTE: 'r' is normally the "activate rename" key, but because
    // rename_active is already true the mode interception runs first and
    // pushes 'r' to the input — it never reaches the normal key dispatch.
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.rename_input, "r");
    assert!(explorer.rename_active);
}

#[test]
fn rename_mode_backspace_pops_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.rename_active = true;
    explorer.rename_input = "cd".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Backspace));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert_eq!(explorer.rename_input, "c");
    assert!(explorer.rename_active);
}

#[test]
fn rename_mode_esc_cancels_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.rename_active = true;
    explorer.rename_input = "draft".to_string();

    let outcome = explorer.handle_key(key(KeyCode::Esc));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(!explorer.rename_active);
    assert!(explorer.rename_input.is_empty());
}

#[test]
fn rename_mode_unknown_key_returns_pending_via_macro() {
    let dir = tempdir().expect("tempdir");
    let mut explorer = FileExplorer::new(dir.path().to_path_buf(), vec![]);
    explorer.rename_active = true;
    explorer.rename_input = "baz".to_string();

    let outcome = explorer.handle_key(key(KeyCode::F(4)));
    assert_eq!(outcome, ExplorerOutcome::Pending);
    assert!(explorer.rename_active);
    assert_eq!(explorer.rename_input, "baz");
}
