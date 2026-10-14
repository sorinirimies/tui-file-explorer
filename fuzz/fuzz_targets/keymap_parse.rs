#![no_main]
use libfuzzer_sys::fuzz_target;
use tui_file_explorer::keymap::{KeyChord, Keymap};
use tui_file_explorer::ExplorerCommand;

fuzz_target!(|data: &str| {
    if let Ok(chord) = KeyChord::parse(data) {
        let ev = chord.to_event();
        let _ = chord.matches(ev);
        let km = Keymap::builtin().clone().bind(chord, ExplorerCommand::MoveDown);
        let _ = km.resolve(ev);
        let _ = km.bindings().len();
    }
});
