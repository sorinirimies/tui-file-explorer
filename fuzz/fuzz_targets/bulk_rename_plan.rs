#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use std::path::PathBuf;
use tui_file_explorer::bulk_rename::{names_file_content, plan};

#[derive(Arbitrary, Debug)]
struct Input {
    sources: Vec<String>,
    edited: String,
    occupied: Vec<String>,
}

fuzz_target!(|i: Input| {
    let sources: Vec<PathBuf> = i.sources.iter().take(64).map(PathBuf::from).collect();
    let _ = names_file_content(&sources);
    let occ: Vec<PathBuf> = i.occupied.iter().map(PathBuf::from).collect();
    let _ = plan(&sources, &i.edited, |p| occ.iter().any(|o| o == p));
});
