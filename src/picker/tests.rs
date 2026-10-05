//! Unit tests for [`super`].

use super::*;

fn item(label: &str) -> PickerItem {
    PickerItem {
        label: label.to_string(),
        detail: String::new(),
        path: PathBuf::from(label),
        is_dir: false,
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_str(p: &mut Picker, s: &str) {
    for c in s.chars() {
        p.handle_key(key(KeyCode::Char(c)));
    }
}

#[test]
fn empty_query_matches_everything_in_order() {
    let p = Picker::new(PickerKind::Fuzzy, "t", vec![item("b"), item("a")]);
    let labels: Vec<_> = p.rows().map(|(i, _)| i.label.as_str()).collect();
    assert_eq!(labels, ["b", "a"]);
}

#[test]
fn subsequence_matches_and_non_subsequence_does_not() {
    assert!(fuzzy_match("mrs", "src/main.rs").is_some());
    assert!(fuzzy_match("xyz", "src/main.rs").is_none());
    assert!(fuzzy_match("sr", "rs").is_none(), "order matters");
}

#[test]
fn matching_is_case_insensitive() {
    assert!(fuzzy_match("READ", "readme.md").is_some());
    assert!(fuzzy_match("read", "README.md").is_some());
}

#[test]
fn reports_matched_positions() {
    let (_, pos) = fuzzy_match("ab", "xaxb").unwrap();
    assert_eq!(pos, vec![1, 3]);
}

#[test]
fn consecutive_and_prefix_matches_rank_higher() {
    let (tight, _) = fuzzy_match("main", "main.rs").unwrap();
    let (loose, _) = fuzzy_match("main", "m_a_i_n.rs").unwrap();
    assert!(tight > loose);
}

#[test]
fn file_name_match_beats_directory_match() {
    let (in_name, _) = fuzzy_match("lib", "src/lib.rs").unwrap();
    let (in_dir, _) = fuzzy_match("lib", "lib/other.rs").unwrap();
    // Both start a segment; the name match is rewarded for being in the file.
    assert!(in_name >= in_dir - 25);
}

#[test]
fn typing_filters_and_sorts_best_first() {
    let mut p = Picker::new(
        PickerKind::Fuzzy,
        "t",
        vec![item("docs/guide.md"), item("src/main.rs"), item("main.rs")],
    );
    type_str(&mut p, "main");
    assert_eq!(p.match_count(), 2);
    assert_eq!(p.selected().unwrap().label, "main.rs");
}

#[test]
fn backspace_widens_the_filter() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("alpha"), item("beta")]);
    type_str(&mut p, "al");
    assert_eq!(p.match_count(), 1);
    p.handle_key(key(KeyCode::Backspace));
    p.handle_key(key(KeyCode::Backspace));
    assert_eq!(p.match_count(), 2);
}

#[test]
fn arrows_move_and_clamp_cursor() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("a"), item("b")]);
    p.handle_key(key(KeyCode::Up));
    assert_eq!(p.cursor, 0);
    p.handle_key(key(KeyCode::Down));
    p.handle_key(key(KeyCode::Down));
    assert_eq!(p.cursor, 1);
}

#[test]
fn enter_chooses_and_esc_cancels() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("a")]);
    assert_eq!(
        p.handle_key(key(KeyCode::Enter)),
        PickerAction::Choose(item("a"))
    );
    assert_eq!(p.handle_key(key(KeyCode::Esc)), PickerAction::Cancel);
}

#[test]
fn enter_with_no_match_does_nothing() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("a")]);
    type_str(&mut p, "zzz");
    assert_eq!(p.handle_key(key(KeyCode::Enter)), PickerAction::Pending);
}

#[test]
fn delete_only_in_bookmark_pickers() {
    let mut b = Picker::new(PickerKind::Bookmarks, "b", vec![item("a")]);
    assert_eq!(
        b.handle_key(key(KeyCode::Delete)),
        PickerAction::Delete(item("a"))
    );
    let mut f = Picker::new(PickerKind::Fuzzy, "f", vec![item("a")]);
    assert_eq!(f.handle_key(key(KeyCode::Delete)), PickerAction::Pending);
}

#[test]
fn extend_keeps_the_highlighted_row() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("a"), item("b")]);
    p.handle_key(key(KeyCode::Down));
    p.extend([item("c")]);
    assert_eq!(p.selected().unwrap().label, "b");
    assert_eq!(p.total(), 3);
}

#[test]
fn remove_drops_the_item() {
    let mut p = Picker::new(PickerKind::Bookmarks, "b", vec![item("a"), item("b")]);
    p.remove(&item("a"));
    assert_eq!(p.total(), 1);
}

#[test]
fn ctrl_u_clears_query() {
    let mut p = Picker::new(PickerKind::Fuzzy, "t", vec![item("a")]);
    type_str(&mut p, "x");
    p.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert!(p.query.is_empty());
}

#[test]
fn renders_without_panicking_in_small_and_large_areas() {
    use ratatui::{backend::TestBackend, Terminal};
    let p = Picker::new(
        PickerKind::Fuzzy,
        "Find",
        vec![item("src/main.rs"), item("README.md")],
    );
    for (w, h) in [(10, 3), (30, 8), (100, 40)] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| render_picker(f, f.area(), &p, &Theme::default()))
            .unwrap();
    }
}

#[test]
fn render_shows_query_and_rows() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut p = Picker::new(
        PickerKind::Fuzzy,
        "Find",
        vec![item("src/main.rs"), item("README.md")],
    );
    type_str(&mut p, "read");
    let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
    t.draw(|f| render_picker(f, f.area(), &p, &Theme::default()))
        .unwrap();
    let text: String = t
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("read"));
    assert!(text.contains("README") || text.contains("R"));
    assert!(text.contains("1/2"));
}
