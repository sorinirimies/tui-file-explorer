//! `keymap.toml`: user-configurable key bindings.
//!
//! ```toml
//! # ~/.config/tfe/keymap.toml
//! [explorer]               # per-pane navigation; replaces ALL keys of a command
//! move_up   = ["up", "k"]
//! dismiss   = ["esc"]      # drop the default `q`
//!
//! [app]                    # app-wide commands; replaces the built-in key
//! undo      = ["u", "ctrl+z"]
//! cut       = ["m"]        # `x` stops cutting
//! ```
//!
//! Values are one chord or an array of chords (see [`KeyChord::parse`]).
//! Unknown commands or keys never abort startup — they are collected as
//! [`KeymapConfig::warnings`].

use std::{fs, io, path::PathBuf};

use super::{KeyChord, Keymap};
use crate::{
    app::{AppCommand, KeyBindings},
    types::ExplorerCommand,
};

/// Parsed `keymap.toml`.
#[derive(Debug, Clone, Default)]
pub struct KeymapConfig {
    /// Explorer key bindings (defaults with the user's overrides applied).
    pub explorer: Keymap,
    /// App-level overrides, ready for [`App::set_key_bindings`](crate::App::set_key_bindings).
    pub app: KeyBindings,
    /// Human-readable problems that were skipped.
    pub warnings: Vec<String>,
}

/// Path of the keymap file (`$XDG_CONFIG_HOME/tfe/keymap.toml`).
#[must_use]
pub fn keymap_path() -> Option<PathBuf> {
    crate::persistence::config_dir().map(|d| d.join("keymap.toml"))
}

fn chords(value: &toml::Value, command: &str, warnings: &mut Vec<String>) -> Option<Vec<KeyChord>> {
    let texts: Vec<&str> = match value {
        toml::Value::String(s) => vec![s.as_str()],
        toml::Value::Array(items) => {
            let mut v = Vec::new();
            for item in items {
                if let Some(s) = item.as_str() {
                    v.push(s);
                } else {
                    warnings.push(format!("{command}: keys must be strings"));
                    return None;
                }
            }
            v
        }
        _ => {
            warnings.push(format!("{command}: expected a key or an array of keys"));
            return None;
        }
    };
    let mut out = Vec::new();
    for t in texts {
        match KeyChord::parse(t) {
            Ok(c) => out.push(c),
            Err(e) => warnings.push(format!("{command}: {e}")),
        }
    }
    // A command whose every key was invalid keeps its defaults.
    (!out.is_empty() || matches!(value, toml::Value::Array(a) if a.is_empty())).then_some(out)
}

/// Parse the text of a `keymap.toml`.
///
/// # Errors
///
/// Returns a message only for TOML *syntax* errors; semantic problems become
/// [`KeymapConfig::warnings`].
pub fn parse_keymap_config(text: &str) -> Result<KeymapConfig, String> {
    let root: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let mut cfg = KeymapConfig {
        explorer: Keymap::default(),
        ..KeymapConfig::default()
    };

    for (section, value) in &root {
        let Some(table) = value.as_table() else {
            cfg.warnings.push(format!("'{section}' should be a table"));
            continue;
        };
        match section.as_str() {
            "explorer" => {
                for (name, v) in table {
                    let Some(command) = ExplorerCommand::from_name(name) else {
                        cfg.warnings
                            .push(format!("[explorer] unknown command '{name}'"));
                        continue;
                    };
                    if let Some(keys) = chords(v, &format!("[explorer] {name}"), &mut cfg.warnings)
                    {
                        cfg.explorer =
                            std::mem::take(&mut cfg.explorer).set_command(command, &keys);
                    }
                }
            }
            "app" => {
                for (name, v) in table {
                    let Some(command) = AppCommand::from_name(name) else {
                        cfg.warnings.push(format!("[app] unknown command '{name}'"));
                        continue;
                    };
                    if let Some(keys) = chords(v, &format!("[app] {name}"), &mut cfg.warnings) {
                        cfg.app = std::mem::take(&mut cfg.app).remap(command, &keys);
                    }
                }
            }
            other => cfg.warnings.push(format!("unknown section [{other}]")),
        }
    }
    Ok(cfg)
}

/// Load `keymap.toml` from the config directory.
///
/// `Ok(None)` when the file does not exist.
///
/// # Errors
///
/// An unreadable file or invalid TOML.
pub fn load_keymap_config() -> io::Result<Option<KeymapConfig>> {
    let Some(path) = keymap_path() else {
        return Ok(None);
    };
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    parse_keymap_config(&text).map(Some).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", path.display()),
        )
    })
}

/// A complete, commented `keymap.toml` listing the built-in bindings —
/// a starting point for customisation (`tfe --dump-keymap`).
#[must_use]
pub fn default_keymap_toml() -> String {
    let mut out = String::from(
        "# tfe key bindings — save as keymap.toml in the config directory.\n\
         # Each command takes one key or an array of keys; the FIRST is canonical.\n\
         # Keys: single chars (k, G, /), esc enter tab backtab backspace delete\n\
         # up down left right home end pageup pagedown space f1..f24,\n\
         # with modifiers: ctrl+t, alt+enter, shift+tab.\n\
         # A command listed here REPLACES all its default keys.\n\n\
         [explorer]\n",
    );
    let map = Keymap::default();
    for &command in ExplorerCommand::ALL {
        let keys: Vec<String> = map
            .chords_for(command)
            .map(|c| format!("\"{c}\""))
            .collect();
        out.push_str(&format!("{} = [{}]\n", command.name(), keys.join(", ")));
    }
    out.push_str("\n[app]\n");
    for &command in AppCommand::ALL {
        let ev = command.key_event();
        let chord =
            if ev.modifiers.is_empty() || ev.modifiers == crossterm::event::KeyModifiers::SHIFT {
                KeyChord::plain(ev.code)
            } else {
                KeyChord::exact(ev.code, ev.modifiers)
            };
        out.push_str(&format!("{} = [\"{chord}\"]\n", command.name()));
    }
    out
}

#[cfg(test)]
mod tests;
