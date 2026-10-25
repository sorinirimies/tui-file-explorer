#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;

// Feed arbitrary bytes to the archive reader as each supported format.
fuzz_target!(|data: &[u8]| {
    let dir = std::env::temp_dir().join("tfe-fuzz-archive");
    let _ = std::fs::create_dir_all(&dir);
    for name in ["a.zip", "a.tar", "a.tar.gz"] {
        let p = dir.join(name);
        if std::fs::File::create(&p).and_then(|mut f| f.write_all(data)).is_err() {
            return;
        }
        let _ = tui_file_explorer::archive::list(&p);
        let out = dir.join("out");
        let _ = tui_file_explorer::archive::extract(&p, &out);
        let _ = std::fs::remove_dir_all(&out);
    }
});
