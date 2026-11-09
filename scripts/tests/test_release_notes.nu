#!/usr/bin/env nu
# ── tui-file-explorer · test_release_notes.nu ────────────────────────────────

use std/assert
use runner.nu *
use ../ci/release_notes.nu [notes install_block]
use ../project.nu [PUBLISH bin_crates]

def "test notes: heading install and changes" [] {
    let n = (notes "0.2.0" "### ✨ Features\n\n- A thing")
    assert ($n | str starts-with "# tui-file-explorer 0.2.0")
    for c in (bin_crates | where {|c| $c in $PUBLISH }) {
        assert ($n | str contains $"cargo install ($c)")
    }
    assert ($n | str contains "cargo add tui-file-explorer") "library usage stays in the notes"
    assert ($n | str contains "## Changes")
    assert ($n | str contains "- A thing")
}

# The notes listed bare `cargo install` lines; the web UI was easy to miss.
def "test notes: each install line says which binary it is" [] {
    let bins = [[crate summary]; [app "App: terminal UI"] [app-web "App web: local web UI"] [app-gui ""]]
    let lines = install_block [app app-web app-gui app-dev] $bins [app app-web app-gui]
    assert equal $lines ["# App: terminal UI" "cargo install app" "" "# App web: local web UI" "cargo install app-web" "" "cargo install app-gui"]
    assert equal (install_block [app] [[crate]; [app]] [app]) ["cargo install app"]
}

def main [] { run-tests }
