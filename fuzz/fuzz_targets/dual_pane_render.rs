#![no_main]
#[path = "common.rs"]
mod common;
use arbitrary::Arbitrary;
use common::*;
use libfuzzer_sys::fuzz_target;
use tui_file_explorer::{render_dual_pane_themed, DualPane};

#[derive(Arbitrary, Debug)]
struct Input {
    area: FuzzArea,
    single: bool,
    hidden: bool,
    page: usize,
    keys: Vec<FuzzKey>,
}

fuzz_target!(|i: Input| {
    let mut dp = DualPane::builder(fixture_dir().to_path_buf())
        .filesystem(read_only_fs())
        .single_pane(i.single)
        .show_hidden(i.hidden)
        .page_size(i.page)
        .build();
    let mut term = i.area.terminal();
    let rect = i.area.rect();
    let theme = theme();
    let _ = term.draw(|f| render_dual_pane_themed(&mut dp, f, rect, &theme));
    for k in i.keys.iter().take(64) {
        let _ = dp.handle_key(k.event());
        let _ = term.draw(|f| render_dual_pane_themed(&mut dp, f, rect, &theme));
    }
});
