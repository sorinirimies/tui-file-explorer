//! Unit tests for [`super`].

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn parse_simple_and_special_keys() {
    assert_eq!(
        KeyChord::parse("k"),
        Ok(KeyChord::plain(KeyCode::Char('k')))
    );
    assert_eq!(
        KeyChord::parse("G"),
        Ok(KeyChord::plain(KeyCode::Char('G')))
    );
    assert_eq!(KeyChord::parse("esc"), Ok(KeyChord::plain(KeyCode::Esc)));
    assert_eq!(
        KeyChord::parse("Space"),
        Ok(KeyChord::plain(KeyCode::Char(' ')))
    );
    assert_eq!(KeyChord::parse("f5"), Ok(KeyChord::plain(KeyCode::F(5))));
    assert_eq!(
        KeyChord::parse("+"),
        Ok(KeyChord::plain(KeyCode::Char('+')))
    );
}

#[test]
fn parse_modifiers() {
    assert_eq!(
        KeyChord::parse("ctrl+t"),
        Ok(KeyChord::exact(KeyCode::Char('t'), KeyModifiers::CONTROL))
    );
    assert_eq!(
        KeyChord::parse("Ctrl+Alt+Left"),
        Ok(KeyChord::exact(
            KeyCode::Left,
            KeyModifiers::CONTROL | KeyModifiers::ALT
        ))
    );
}

#[test]
fn shift_letter_equals_uppercase() {
    let a = KeyChord::parse("shift+g").unwrap();
    assert_eq!(a.code, KeyCode::Char('G'));
    assert!(a.matches(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)));
    assert!(a.matches(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE)));
}

#[test]
fn parse_errors_are_descriptive() {
    assert!(KeyChord::parse("").is_err());
    assert!(KeyChord::parse("hyper+x")
        .unwrap_err()
        .reason
        .contains("hyper"));
    assert!(KeyChord::parse("nonsense")
        .unwrap_err()
        .reason
        .contains("nonsense"));
    assert!(KeyChord::parse("f99").is_err());
}

#[test]
fn display_round_trips_through_parse() {
    for text in [
        "k",
        "G",
        "ctrl+t",
        "alt+enter",
        "space",
        "pageup",
        "f12",
        "shift+tab",
        "esc",
    ] {
        let chord = KeyChord::parse(text).unwrap();
        assert_eq!(KeyChord::parse(&chord.to_string()), Ok(chord), "{text}");
    }
}

#[test]
fn plain_chord_rejects_ctrl_but_any_accepts() {
    let plain = KeyChord::plain(KeyCode::Char('s'));
    let any = KeyChord::any(KeyCode::Char('s'));
    let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert!(!plain.matches(ctrl_s));
    assert!(any.matches(ctrl_s));
}

#[test]
fn default_map_resolves_aliases() {
    let map = Keymap::default();
    for code in [KeyCode::Left, KeyCode::Backspace, KeyCode::Char('h')] {
        assert_eq!(map.resolve(key(code)), Some(ExplorerCommand::Ascend));
    }
}

#[test]
fn every_default_command_round_trips_via_canonical_key() {
    let map = Keymap::default();
    for &command in ExplorerCommand::ALL {
        assert_eq!(
            map.resolve(map.canonical_key(command)),
            Some(command),
            "{command:?}"
        );
    }
}

#[test]
fn set_command_replaces_keys_and_steals_conflicts() {
    // Give MoveUp the key 'z' (currently ToggleSizes) and drop its old keys.
    let map = Keymap::default().set_command(
        ExplorerCommand::MoveUp,
        &[KeyChord::plain(KeyCode::Char('z'))],
    );
    assert_eq!(
        map.resolve(key(KeyCode::Char('z'))),
        Some(ExplorerCommand::MoveUp)
    );
    assert_eq!(map.resolve(key(KeyCode::Up)), None);
    assert_eq!(map.chords_for(ExplorerCommand::ToggleSizes).count(), 0);
}

#[test]
fn bind_adds_alias_without_touching_others() {
    let map = Keymap::default().bind(
        KeyChord::exact(KeyCode::Char('p'), KeyModifiers::CONTROL),
        ExplorerCommand::MoveUp,
    );
    assert_eq!(
        map.resolve(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
        Some(ExplorerCommand::MoveUp)
    );
    assert_eq!(map.resolve(key(KeyCode::Up)), Some(ExplorerCommand::MoveUp));
}

#[test]
fn command_names_round_trip() {
    for &c in ExplorerCommand::ALL {
        assert_eq!(ExplorerCommand::from_name(c.name()), Some(c));
    }
    assert_eq!(
        ExplorerCommand::from_name("Toggle-Hidden"),
        Some(ExplorerCommand::ToggleHidden)
    );
    assert_eq!(ExplorerCommand::from_name("nope"), None);
}
