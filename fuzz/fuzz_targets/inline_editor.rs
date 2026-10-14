#![no_main]
#[path = "common.rs"]
mod common;
use arbitrary::Arbitrary;
use common::*;
use libfuzzer_sys::fuzz_target;
use tui_file_explorer::{render_inline_editor, InlineEditor};

#[derive(Arbitrary, Debug)]
struct Input {
    area: FuzzArea,
    content: String,
    keys: Vec<FuzzKey>,
}

fuzz_target!(|i: Input| {
    // Never save: the file is only read; Ctrl-S writes the scratch file.
    let path = fixture_dir().join("editor-scratch.txt");
    let _ = std::fs::write(&path, &i.content);
    let Ok(mut ed) = InlineEditor::open(&path) else { return };
    let mut term = i.area.terminal();
    let rect = i.area.rect();
    let theme = theme();
    let _ = term.draw(|f| render_inline_editor(f, rect, &ed, &theme));
    for k in i.keys.iter().take(128) {
        let _ = ed.handle_key(k.event());
        let _ = term.draw(|f| render_inline_editor(f, rect, &ed, &theme));
    }
});
