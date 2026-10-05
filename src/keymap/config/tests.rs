//! Unit tests for [`super`].

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn empty_file_is_the_default_keymap() {
    let cfg = parse_keymap_config("").unwrap();
    assert_eq!(cfg.explorer, Keymap::default());
    assert!(cfg.app.is_empty());
    assert!(cfg.warnings.is_empty());
}

#[test]
fn explorer_override_replaces_all_keys_of_a_command() {
    let cfg = parse_keymap_config("[explorer]\nmove_up = [\"w\"]\n").unwrap();
    assert_eq!(
        cfg.explorer.resolve(key(KeyCode::Char('w'))),
        Some(ExplorerCommand::MoveUp)
    );
    assert_eq!(cfg.explorer.resolve(key(KeyCode::Up)), None);
    assert_eq!(cfg.explorer.resolve(key(KeyCode::Char('k'))), None);
    // Untouched commands keep their defaults.
    assert_eq!(
        cfg.explorer.resolve(key(KeyCode::Down)),
        Some(ExplorerCommand::MoveDown)
    );
}

#[test]
fn single_string_value_is_accepted() {
    let cfg = parse_keymap_config("[explorer]\nsearch = \"ctrl+f\"\n").unwrap();
    assert_eq!(
        cfg.explorer
            .resolve(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
        Some(ExplorerCommand::Search)
    );
    assert_eq!(cfg.explorer.resolve(key(KeyCode::Char('/'))), None);
}

#[test]
fn conflicting_key_is_taken_from_its_old_command() {
    let cfg = parse_keymap_config("[explorer]\ncycle_sort = [\"z\"]\n").unwrap();
    assert_eq!(
        cfg.explorer.resolve(key(KeyCode::Char('z'))),
        Some(ExplorerCommand::CycleSort)
    );
    assert_eq!(
        cfg.explorer
            .chords_for(ExplorerCommand::ToggleSizes)
            .count(),
        0
    );
}

#[test]
fn app_override_remaps_and_swallows_the_builtin_key() {
    let cfg = parse_keymap_config("[app]\ncut = [\"m\"]\n").unwrap();
    // New key translates to cut's built-in key.
    assert_eq!(
        cfg.app.translate(key(KeyCode::Char('m'))),
        Some(AppCommand::Cut.key_event())
    );
    // The old built-in key no longer cuts.
    assert_eq!(cfg.app.translate(key(KeyCode::Char('x'))), None);
    // Unrelated keys pass through.
    assert_eq!(
        cfg.app.translate(key(KeyCode::Char('p'))),
        Some(key(KeyCode::Char('p')))
    );
}

#[test]
fn app_override_may_keep_the_builtin_key() {
    let cfg = parse_keymap_config("[app]\nundo = [\"u\", \"ctrl+z\"]\n").unwrap();
    assert_eq!(
        cfg.app
            .translate(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL)),
        Some(AppCommand::Undo.key_event())
    );
    assert!(cfg.app.translate(key(KeyCode::Char('u'))).is_some());
}

#[test]
fn problems_become_warnings_not_errors() {
    let cfg = parse_keymap_config(
        "[explorer]\nbogus = [\"x\"]\nmove_up = [\"hyper+k\"]\n[nope]\na = 1\n[app]\nundo = 5\n",
    )
    .unwrap();
    assert_eq!(cfg.warnings.len(), 4, "{:?}", cfg.warnings);
    // An all-invalid command keeps its defaults.
    assert_eq!(
        cfg.explorer.resolve(key(KeyCode::Up)),
        Some(ExplorerCommand::MoveUp)
    );
}

#[test]
fn empty_array_unbinds_a_command() {
    let cfg = parse_keymap_config("[explorer]\nsearch = []\n").unwrap();
    assert_eq!(cfg.explorer.chords_for(ExplorerCommand::Search).count(), 0);
}

#[test]
fn syntax_errors_are_reported() {
    assert!(parse_keymap_config("[explorer\n").is_err());
}

#[test]
fn dumped_template_parses_back_to_the_defaults() {
    let text = default_keymap_toml();
    let cfg = parse_keymap_config(&text).unwrap();
    assert!(cfg.warnings.is_empty(), "{:?}", cfg.warnings);
    for &c in ExplorerCommand::ALL {
        let shown = |m: &Keymap| m.chords_for(c).map(|k| k.to_string()).collect::<Vec<_>>();
        assert_eq!(shown(&cfg.explorer), shown(&Keymap::default()), "{c:?}");
    }
}
