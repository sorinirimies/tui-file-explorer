#![no_main]
#[path = "common.rs"]
mod common;
use arbitrary::Arbitrary;
use common::*;
use libfuzzer_sys::fuzz_target;
use tui_file_explorer::{render_themed, FileExplorer, SelectionMode};

#[derive(Arbitrary, Debug)]
struct Input {
    area: FuzzArea,
    hidden: bool,
    sizes: bool,
    dir_sizes: bool,
    multi: bool,
    page: usize,
    sort: FuzzSort,
    filter: Vec<String>,
    keys: Vec<FuzzKey>,
}

fuzz_target!(|i: Input| {
    let mut ex = FileExplorer::builder(fixture_dir().to_path_buf())
        .filesystem(read_only_fs())
        .show_hidden(i.hidden)
        .show_sizes(i.sizes)
        .show_dir_sizes(i.dir_sizes)
        .page_size(i.page)
        .sort_mode(sort_mode(i.sort))
        .selection_mode(if i.multi { SelectionMode::Any } else { SelectionMode::Files })
        .extension_filter(i.filter.iter().take(16).cloned().collect())
        .build();
    let mut term = i.area.terminal();
    let rect = i.area.rect();
    let theme = theme();
    let _ = term.draw(|f| render_themed(&mut ex, f, rect, &theme));
    for k in i.keys.iter().take(64) {
        let _ = ex.handle_key(k.event());
        let _ = term.draw(|f| render_themed(&mut ex, f, rect, &theme));
    }
});
