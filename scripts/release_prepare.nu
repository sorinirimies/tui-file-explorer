#!/usr/bin/env nu
# Prepare a release: update Cargo.toml version, regenerate CHANGELOG.md,
# and write RELEASE_CHANGELOG.md with full release notes.
#
# Usage: nu scripts/release_prepare.nu <tag>
# Example: nu scripts/release_prepare.nu v0.3.0

def main [
    tag: string,  # The release tag, e.g. "v0.3.0"
] {
    let green  = (ansi green)
    let cyan   = (ansi cyan)
    let red    = (ansi red)
    let reset  = (ansi reset)

    # Strip leading 'v' to get the bare version number
    let version = $tag | str replace --regex '^v' ''

    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($cyan)  Release Prepare: ($tag)($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""

    # ── Step 1: Update Cargo.toml ─────────────────────────────────────────────
    print $"($cyan)Step 1/4: Updating Cargo.toml to ($version)...($reset)"

    let cargo_lines = open Cargo.toml --raw | lines
    let updated_cargo = $cargo_lines
        | each { |line|
            if ($line =~ '^version\s*=\s*"[^"]*"') {
                $'version      = "($version)"'
            } else {
                $line
            }
        }
        | str join "\n"
    $updated_cargo | save --force Cargo.toml

    # Verify
    let got = open Cargo.toml --raw
        | lines
        | where { |l| $l =~ '^version\s*=' }
        | first
        | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
        | get v
        | first
    if $got != $version {
        error make { msg: $"($red)Failed to update Cargo.toml \(got ($got), expected ($version)\)($reset)" }
    }
    print $"($green)✓ Cargo.toml updated to ($version)($reset)"

    # ── Step 2: Regenerate CHANGELOG.md ──────────────────────────────────────
    print ""
    print $"($cyan)Step 2/4: Regenerating CHANGELOG.md...($reset)"
    run-external "git-cliff" "--config" "cliff.toml" "--latest" "--output" "CHANGELOG.md"
    print $"($green)✓ CHANGELOG.md written($reset)"

    # ── Step 3: Generate per-release diff for release notes ───────────────────
    print ""
    print $"($cyan)Step 3/4: Generating release diff...($reset)"

    let last_tag_result = do { run-external "git" "describe" "--tags" "--abbrev=0" "HEAD^" } | complete
    let last_tag = if $last_tag_result.exit_code == 0 {
        $last_tag_result.stdout | str trim
    } else {
        ""
    }

    if ($last_tag | is-empty) {
        run-external "git-cliff" "--config" "cliff.toml" "--tag" $tag "--strip" "header" "--output" "CLIFF_CHANGES.md"
    } else {
        run-external "git-cliff" "--config" "cliff.toml" $"($last_tag)..($tag)" "--strip" "header" "--output" "CLIFF_CHANGES.md"
    }
    print $"($green)✓ Release diff written($reset)"

    # ── Step 4: Build RELEASE_CHANGELOG.md ───────────────────────────────────
    print ""
    print $"($cyan)Step 4/4: Writing RELEASE_CHANGELOG.md...($reset)"

    let cliff_changes = open CLIFF_CHANGES.md --raw

    let changes_header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }

    let notes = [
        $"# tui-file-explorer ($version)"
        ""
        "## What's New"
        ""
        $changes_header
        ""
        $cliff_changes
        ""
        "## Installation"
        ""
        "Add to your `Cargo.toml`:"
        ""
        "```toml"
        "[dependencies]"
        $"tui-file-explorer = \"($version)\""
        "```"
        ""
        "Or via cargo-add:"
        ""
        "```bash"
        "cargo add tui-file-explorer"
        "```"
        ""
        "## Quick Start"
        ""
        "```rust"
        "use tui_file_explorer::{FileExplorer, ExplorerOutcome, render};"
        ""
        "let mut explorer = FileExplorer::new("
        "    std::env::current_dir().unwrap(),"
        "    vec![\"iso\".into(), \"img\".into()],"
        ");"
        "```"
    ] | str join "\n"

    $notes | save --force RELEASE_CHANGELOG.md
    print $"($green)✓ RELEASE_CHANGELOG.md written($reset)"

    # ── Summary ───────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($green)✓ Release ($tag) prepared successfully! 🚀($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""
    print "Files written:"
    print $"  ($green)Cargo.toml($reset)            version → ($version)"
    print $"  ($green)CHANGELOG.md($reset)          full history"
    print $"  ($green)RELEASE_CHANGELOG.md($reset)  release notes for ($tag)"
}
