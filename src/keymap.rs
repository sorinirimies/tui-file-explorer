//! Key chords and the explorer keymap.
//!
//! A [`Keymap`] maps [`KeyChord`]s to [`ExplorerCommand`]s. The default map
//! reproduces the built-in bindings; hosts (and the `tfe` binary, via
//! `keymap.toml`) can replace the keys of any command.
//!
//! Chord syntax (used in config files): `k`, `G`, `ctrl+t`, `shift+tab`,
//! `alt+enter`, `esc`, `space`, `pageup`, `f5`, … — see [`KeyChord::parse`].

use std::{fmt, sync::OnceLock};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::types::ExplorerCommand;

// ── KeyChord ──────────────────────────────────────────────────────────────────

/// How strictly a chord compares the modifier keys of an incoming event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordMods {
    /// Modifiers are ignored entirely (`Up`, vim `k`, …).
    Any,
    /// No Ctrl/Alt; Shift is tolerated only on uppercase letters (terminals
    /// speaking the kitty keyboard protocol report it there).
    Plain,
    /// Exactly these modifiers.
    Exact(KeyModifiers),
}

/// One physical key plus a modifier rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub mods: ChordMods,
}

impl KeyChord {
    /// Plain key, modifiers ignored.
    #[must_use]
    pub const fn any(code: KeyCode) -> Self {
        Self {
            code,
            mods: ChordMods::Any,
        }
    }

    /// Plain key without Ctrl/Alt.
    #[must_use]
    pub const fn plain(code: KeyCode) -> Self {
        Self {
            code,
            mods: ChordMods::Plain,
        }
    }

    /// Key with exactly `mods`.
    #[must_use]
    pub const fn exact(code: KeyCode, mods: KeyModifiers) -> Self {
        Self {
            code,
            mods: ChordMods::Exact(mods),
        }
    }

    /// Does `key` trigger this chord?
    #[must_use]
    pub fn matches(&self, key: KeyEvent) -> bool {
        if self.code != key.code {
            return false;
        }
        match self.mods {
            ChordMods::Any => true,
            ChordMods::Plain => {
                key.modifiers.is_empty()
                    || (key.modifiers == KeyModifiers::SHIFT
                        && matches!(key.code, KeyCode::Char(c) if c.is_uppercase()))
            }
            ChordMods::Exact(m) => {
                // Terminals differ on whether uppercase letters carry SHIFT.
                let strip = |m: KeyModifiers| {
                    if matches!(self.code, KeyCode::Char(c) if c.is_uppercase()) {
                        m - KeyModifiers::SHIFT
                    } else {
                        m
                    }
                };
                strip(m) == strip(key.modifiers)
            }
        }
    }

    /// The event this chord would be triggered by (for synthesising input).
    #[must_use]
    pub fn to_event(&self) -> KeyEvent {
        let mods = match self.mods {
            ChordMods::Exact(m) => m,
            _ => match self.code {
                KeyCode::Char(c) if c.is_uppercase() => KeyModifiers::SHIFT,
                _ => KeyModifiers::NONE,
            },
        };
        KeyEvent::new(self.code, mods)
    }

    /// Parse a chord such as `"ctrl+t"` or `"G"`.
    ///
    /// # Errors
    ///
    /// Returns a [`KeymapError`] for an empty string, an unknown modifier or
    /// an unknown key name.
    pub fn parse(text: &str) -> Result<Self, KeymapError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(KeymapError::new(text, "empty key"));
        }
        // A lone "+" is the plus key itself.
        let parts: Vec<&str> = if trimmed == "+" {
            vec!["+"]
        } else {
            trimmed.split('+').collect()
        };
        let (key_part, mod_parts) = parts.split_last().expect("split yields one part");

        let mut mods = KeyModifiers::NONE;
        for m in mod_parts {
            mods |= match m.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" | "meta" | "opt" | "option" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                other => {
                    return Err(KeymapError::new(
                        text,
                        format!("unknown modifier '{other}'"),
                    ))
                }
            };
        }

        let key_part = key_part.trim();
        let mut code = parse_key_name(key_part)
            .ok_or_else(|| KeymapError::new(text, format!("unknown key '{key_part}'")))?;
        // shift+g and G are the same chord.
        if mods.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = code {
                if c.is_alphabetic() {
                    code = KeyCode::Char(c.to_ascii_uppercase());
                }
            }
        }
        if mods.is_empty() {
            Ok(Self::plain(code))
        } else {
            Ok(Self::exact(code, mods))
        }
    }
}

fn parse_key_name(name: &str) -> Option<KeyCode> {
    let lower = name.to_ascii_lowercase();
    let code = match lower.as_str() {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" | "pgdown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        _ => {
            if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                return (1..=24).contains(&n).then_some(KeyCode::F(n));
            }
            let mut chars = name.chars();
            return match (chars.next(), chars.next()) {
                (Some(c), None) => Some(KeyCode::Char(c)),
                _ => None,
            };
        }
    };
    Some(code)
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mods = match self.mods {
            ChordMods::Exact(m) => m,
            _ => KeyModifiers::NONE,
        };
        if mods.contains(KeyModifiers::CONTROL) {
            f.write_str("ctrl+")?;
        }
        if mods.contains(KeyModifiers::ALT) {
            f.write_str("alt+")?;
        }
        // Uppercase letters already imply shift; don't print it twice.
        let upper = matches!(self.code, KeyCode::Char(c) if c.is_uppercase());
        if mods.contains(KeyModifiers::SHIFT) && !upper {
            f.write_str("shift+")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::F(n) => write!(f, "f{n}"),
            KeyCode::Up => f.write_str("up"),
            KeyCode::Down => f.write_str("down"),
            KeyCode::Left => f.write_str("left"),
            KeyCode::Right => f.write_str("right"),
            KeyCode::Enter => f.write_str("enter"),
            KeyCode::Esc => f.write_str("esc"),
            KeyCode::Tab => f.write_str("tab"),
            KeyCode::BackTab => f.write_str("backtab"),
            KeyCode::Backspace => f.write_str("backspace"),
            KeyCode::Delete => f.write_str("delete"),
            KeyCode::Insert => f.write_str("insert"),
            KeyCode::Home => f.write_str("home"),
            KeyCode::End => f.write_str("end"),
            KeyCode::PageUp => f.write_str("pageup"),
            KeyCode::PageDown => f.write_str("pagedown"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// A key string that could not be understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapError {
    pub input: String,
    pub reason: String,
}

impl KeymapError {
    fn new(input: &str, reason: impl Into<String>) -> Self {
        Self {
            input: input.to_string(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid key '{}': {}", self.input, self.reason)
    }
}

impl std::error::Error for KeymapError {}

// ── Keymap ────────────────────────────────────────────────────────────────────

/// Key → [`ExplorerCommand`] bindings.
///
/// The **first** chord bound to a command is its canonical key — the one
/// [`FileExplorer::handle_command`](crate::FileExplorer::handle_command)
/// synthesises. Later chords are aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<(KeyChord, ExplorerCommand)>,
}

impl Default for Keymap {
    fn default() -> Self {
        use ExplorerCommand as C;
        let any = KeyChord::any;
        let plain = KeyChord::plain;
        let ch = KeyCode::Char;
        Self {
            bindings: vec![
                (any(KeyCode::Esc), C::Dismiss),
                (plain(ch('q')), C::Dismiss),
                (any(KeyCode::Up), C::MoveUp),
                (any(ch('k')), C::MoveUp),
                (any(KeyCode::Down), C::MoveDown),
                (any(ch('j')), C::MoveDown),
                (any(KeyCode::PageUp), C::PageUp),
                (any(KeyCode::PageDown), C::PageDown),
                (any(KeyCode::Home), C::Top),
                (any(ch('g')), C::Top),
                (any(KeyCode::End), C::Bottom),
                (any(ch('G')), C::Bottom),
                // Left / Backspace / h all ascend to the parent directory.
                (any(KeyCode::Left), C::Ascend),
                (any(KeyCode::Backspace), C::Ascend),
                (any(ch('h')), C::Ascend),
                // Right is pure navigation (never exits); Enter / l confirm.
                (any(KeyCode::Right), C::Navigate),
                (any(KeyCode::Enter), C::Confirm),
                (any(ch('l')), C::Confirm),
                (any(ch('.')), C::ToggleHidden),
                (plain(ch('z')), C::ToggleSizes),
                (plain(ch('Z')), C::ToggleDirSizes),
                (plain(ch('/')), C::Search),
                (plain(ch('s')), C::CycleSort),
                (any(ch(' ')), C::ToggleMark),
                (plain(ch('n')), C::NewDirectory),
                (plain(ch('N')), C::NewFile),
                (plain(ch('r')), C::Rename),
            ],
        }
    }
}

impl Keymap {
    /// The built-in bindings (shared, allocation-free after first use).
    #[must_use]
    pub fn builtin() -> &'static Keymap {
        static DEFAULT: OnceLock<Keymap> = OnceLock::new();
        DEFAULT.get_or_init(Keymap::default)
    }

    /// A map with no bindings at all.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// The command triggered by `key`, if any. First match wins.
    #[must_use]
    pub fn resolve(&self, key: KeyEvent) -> Option<ExplorerCommand> {
        self.bindings
            .iter()
            .find(|(chord, _)| chord.matches(key))
            .map(|&(_, command)| command)
    }

    /// Every chord bound to `command`, canonical key first.
    pub fn chords_for(&self, command: ExplorerCommand) -> impl Iterator<Item = KeyChord> + '_ {
        self.bindings
            .iter()
            .filter(move |&&(_, c)| c == command)
            .map(|&(chord, _)| chord)
    }

    /// The canonical key event for `command` (`KeyCode::Null` if unbound).
    #[must_use]
    pub fn canonical_key(&self, command: ExplorerCommand) -> KeyEvent {
        self.chords_for(command).next().map_or_else(
            || KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            |c| c.to_event(),
        )
    }

    /// Bind `chord` to `command`, stealing it from whatever used it before.
    #[must_use]
    pub fn bind(mut self, chord: KeyChord, command: ExplorerCommand) -> Self {
        self.bindings.retain(|(c, _)| *c != chord);
        self.bindings.push((chord, command));
        self
    }

    /// Replace **all** keys of `command` with `chords` (first = canonical).
    ///
    /// Each chord is also removed from any other command it was bound to, so
    /// the last assignment wins.
    #[must_use]
    pub fn set_command(mut self, command: ExplorerCommand, chords: &[KeyChord]) -> Self {
        // Keep the position of the old canonical binding so the table stays
        // readable; simply drop old bindings and append the new ones.
        self.bindings
            .retain(|&(c, cmd)| cmd != command && !chords.contains(&c));
        self.bindings.extend(chords.iter().map(|&c| (c, command)));
        self
    }

    /// Remove every binding of `command`.
    #[must_use]
    pub fn unbind_command(mut self, command: ExplorerCommand) -> Self {
        self.bindings.retain(|&(_, c)| c != command);
        self
    }

    /// All bindings in resolution order.
    #[must_use]
    pub fn bindings(&self) -> &[(KeyChord, ExplorerCommand)] {
        &self.bindings
    }
}

#[cfg(feature = "full-app")]
mod config;
#[cfg(feature = "full-app")]
pub use config::{
    default_keymap_toml, keymap_path, load_keymap_config, parse_keymap_config, KeymapConfig,
};

#[cfg(test)]
mod tests;
