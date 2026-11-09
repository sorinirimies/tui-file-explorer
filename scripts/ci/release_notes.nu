#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Release notes
# ──────────────────────────────────────────────────────────────────────────────
# Writes RELEASE_NOTES.md for a tag: install instructions plus this release's
# section of the changelog (git-cliff --latest; the workflow runs at the tag, so
# --unreleased would be empty).
#
# Usage:
#   nu scripts/ci/release_notes.nu v0.2.0
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [BINS PUBLISH bin_crates]

# `cargo install` lines for the published `crates`, each under a comment with its
# binary's summary from `bins` (so a reader sees which one is the GUI, the TUI or
# the web UI), separated by blank lines.
export def install_block [crates: list<string>, bins: table, publish: list<string>]: nothing -> list<string> {
    $crates | where {|c| $c in $publish } | each {|c|
        let summary = $bins | where crate == $c | get -o 0.summary
        let label = if ($summary | is-empty) { [] } else { [$"# ($summary)"] }
        [...$label $"cargo install ($c)"]
    } | each {|lines| [...$lines ""] } | flatten | drop
}

# `cargo install` lines for the published crates that ship a binary.
export def install_lines []: nothing -> list<string> {
    install_block (bin_crates) $BINS $PUBLISH
}

# The notes text for `version` with the changelog section `changes`.
export def notes [version: string, changes: string]: nothing -> string {
    [
        $"# tui-file-explorer ($version)"
        ""
        "Download the archive for your platform below \(binary at the archive root\),"
        "or install from crates.io:"
        ""
        "```sh"
        ...(install_lines)
        "```"
        ""
        $"Use it as a library \(docs: https://docs.rs/tui-file-explorer/($version)\):"
        ""
        "```sh"
        "cargo add tui-file-explorer"
        "```"
        ""
        "## Changes"
        ""
        $changes
        ""
    ] | str join "\n"
}

def main [raw_tag: string] {
    let version = ($raw_tag | str replace --regex '^v' '')
    let changes = if (which git-cliff | is-not-empty) {
        let r = (do { ^git-cliff --latest --strip header } | complete)
        if $r.exit_code == 0 and ($r.stdout | str trim | is-not-empty) { $r.stdout | str trim } else { "- See the commit history." }
    } else {
        "- See the commit history."
    }
    notes $version $changes | save --force RELEASE_NOTES.md
    print $"(ansi green)✅ RELEASE_NOTES.md for ($version)(ansi reset)"
}
