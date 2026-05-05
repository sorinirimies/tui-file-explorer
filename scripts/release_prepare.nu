#!/usr/bin/env nu
# Release preparation script for CI.
#
# Called by the GitHub Actions / Gitea release workflow when a `v*` tag is
# pushed.  It:
#   1. Strips the `v` prefix to get the semver version.
#   2. Updates the `version` field in Cargo.toml.
#   3. Generates a per-release changelog via git-cliff.
#   4. Writes RELEASE_CHANGELOG.md (consumed by the GH Release body).
#
# Usage:  nu scripts/release_prepare.nu <tag>
# Example: nu scripts/release_prepare.nu v1.0.8

def main [tag: string] {
    let green  = (ansi green)
    let cyan   = (ansi cyan)
    let yellow = (ansi yellow)
    let red    = (ansi red)
    let reset  = (ansi reset)

    print $"($cyan)═══ Release Prepare ═══($reset)"
    print $"Tag: ($yellow)($tag)($reset)"

    # ── 1. Strip `v` prefix ───────────────────────────────────────────────────
    let version = $tag | str replace --regex '^v' ''
    print $"Version: ($green)($version)($reset)"

    if not ($version =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$') {
        error make { msg: $"($red)Error: Invalid version '($version)' — expected X.Y.Z or X.Y.Z-suffix($reset)" }
    }

    # ── 2. Update Cargo.toml ──────────────────────────────────────────────────
    print $"($cyan)Updating Cargo.toml...($reset)"

    let cargo_path = "Cargo.toml"
    let cargo_lines = (open --raw $cargo_path | lines)

    let updated = (
        $cargo_lines
        | each { |line|
            if ($line =~ '^version\s*=\s*"[^"]*"') {
                $'version      = "($version)"'
            } else {
                $line
            }
        }
        | str join "\n"
    )
    $updated | save --force $cargo_path

    # Verify
    let verify = (
        open --raw $cargo_path
        | lines
        | where { |l| $l =~ '^version\s*=' }
        | first
        | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
        | get v
        | first
    )
    if $verify != $version {
        error make { msg: $"($red)Cargo.toml update failed — got '($verify)', expected '($version)'($reset)" }
    }
    print $"($green)✓ Cargo.toml → ($version)($reset)"

    # ── 3. Generate changelog with git-cliff ──────────────────────────────────
    print $"($cyan)Generating changelog...($reset)"

    # Find the previous tag (the one before the current release tag).
    let last_tag = (
        do { git describe --tags --abbrev=0 HEAD~1 } | complete
        | if $in.exit_code == 0 { $in.stdout | str trim } else { "" }
    )
    print $"Previous tag: ($yellow)(if ($last_tag | is-empty) { '<none>' } else { $last_tag })($reset)"

    let cliff_changes = if (which git-cliff | length) > 0 {
        let cliff_result = if ($last_tag | is-empty) {
            do { git-cliff --unreleased --strip header } | complete
        } else {
            do { git-cliff --unreleased --strip header --tag $tag } | complete
        }
        if $cliff_result.exit_code == 0 {
            $cliff_result.stdout | str trim
        } else {
            "- See commit history for changes."
        }
    } else {
        print $"($yellow)⚠ git-cliff not found — using fallback changelog($reset)"
        "- See commit history for changes."
    }

    # Also regenerate the full CHANGELOG.md if git-cliff is available.
    if (which git-cliff | length) > 0 {
        do { git-cliff --tag $tag -o CHANGELOG.md } | complete
        print $"($green)✓ CHANGELOG.md regenerated($reset)"
    }

    # ── 4. Build release notes ────────────────────────────────────────────────
    let changes_header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }

    let release_notes = [
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

    $release_notes | save --force RELEASE_CHANGELOG.md
    print $"($green)✓ RELEASE_CHANGELOG.md written($reset)"

    # ── Done ──────────────────────────────────────────────────────────────────
    print ""
    print $"($green)✓ Release preparation complete for ($tag)($reset)"
}
