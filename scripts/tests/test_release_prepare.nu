#!/usr/bin/env nu
# Tests for scripts/release_prepare.nu
#
# Run with: nu scripts/tests/test_release_prepare.nu

use std/assert
use runner.nu *

# ── Helpers ───────────────────────────────────────────────────────────────────

# Write a minimal Cargo.toml at the given version into a temp dir.
# Returns the dir path.
def make_cargo [version: string] {
    let tmp = (mktemp -d)
    let content = $'[package]
name = "test-crate"
version = "($version)"
edition = "2021"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    $tmp
}

# Read back the version string from a Cargo.toml file.
def read_version [cargo_path: string] {
    open --raw $cargo_path
    | lines
    | where { |l| $l =~ '^version\s*=' }
    | first
    | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
    | get v
    | first
}

# Apply the same Cargo.toml update logic that release_prepare.nu uses.
def apply_version_update [dir: string, new_version: string] {
    let cargo_path = ($dir | path join "Cargo.toml")
    let updated = open --raw $cargo_path
        | lines
        | each { |line|
            if ($line =~ '^version\s*=\s*"[^"]*"') {
                $'version      = "($new_version)"'
            } else {
                $line
            }
        }
        | str join "\n"
    $updated | save --force $cargo_path
}

# Build the release notes string the same way release_prepare.nu does.
def build_release_notes [version: string, cliff_changes: string, last_tag: string] {
    let changes_header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }

    [
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
}

# ── Tag stripping tests ────────────────────────────────────────────────────────

def "test tag v prefix is stripped" [] {
    let tag = "v1.2.3"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "1.2.3"
}

def "test tag without v prefix is unchanged" [] {
    let tag = "1.2.3"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "1.2.3"
}

def "test tag with pre-release suffix strips v only" [] {
    let tag = "v0.5.0-beta.1"
    let version = $tag | str replace --regex '^v' ''
    assert equal $version "0.5.0-beta.1"
}

def "test tag name round trips from version" [] {
    let version = "2.0.0"
    let tag = $"v($version)"
    let back = $tag | str replace --regex '^v' ''
    assert equal $back $version
}

# ── Cargo.toml update logic tests ─────────────────────────────────────────────

def "test cargo toml version is updated" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "1.1.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "1.1.0"
}

def "test cargo toml version update is verified" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "2.0.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    # Simulate the verification check
    assert equal $got "2.0.0" "verification should pass when update succeeded"
}

def "test cargo toml non-version lines survive update" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "1.0.1"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $content "name = \"test-crate\""
    assert str contains $content "edition = \"2021\""
}

def "test cargo toml dependency version lines are untouched" [] {
    let tmp = (mktemp -d)
    let content = '[package]
name = "test-crate"
version = "1.0.0"
edition = "2021"

[dependencies]
serde = { version = "1.0" }
ratatui = { version = "0.29" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    apply_version_update $tmp "1.1.0"
    let updated = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $updated 'version      = "1.1.0"'
    assert str contains $updated 'serde = { version = "1.0" }'
    assert str contains $updated 'ratatui = { version = "0.29" }'
}

# ── Last-tag detection logic tests ────────────────────────────────────────────

def "test empty last tag triggers initial release header" [] {
    let last_tag = ""
    let header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }
    assert equal $header "### Initial Release"
}

def "test non-empty last tag triggers changes-since header" [] {
    let last_tag = "v1.0.0"
    let header = if ($last_tag | is-empty) {
        "### Initial Release"
    } else {
        $"### Changes since ($last_tag):"
    }
    assert equal $header "### Changes since v1.0.0:"
}

def "test last tag is trimmed" [] {
    # git describe may include a trailing newline
    let raw = "v1.0.0\n"
    let trimmed = $raw | str trim
    assert equal $trimmed "v1.0.0"
}

# ── Release notes content tests ───────────────────────────────────────────────

def "test release notes contains version header" [] {
    let notes = build_release_notes "1.2.3" "- fix something" ""
    assert str contains $notes "# tui-file-explorer 1.2.3"
}

def "test release notes contains whats new section" [] {
    let notes = build_release_notes "1.2.3" "- fix something" ""
    assert str contains $notes "## What's New"
}

def "test release notes initial release has correct header" [] {
    let notes = build_release_notes "1.0.0" "- initial" ""
    assert str contains $notes "### Initial Release"
    assert not ($notes | str contains "### Changes since")
}

def "test release notes with previous tag has changes-since header" [] {
    let notes = build_release_notes "1.1.0" "- add feature" "v1.0.0"
    assert str contains $notes "### Changes since v1.0.0:"
    assert not ($notes | str contains "### Initial Release")
}

def "test release notes contains cliff changes" [] {
    let cliff = "- feat: add cool feature\n- fix: patch a bug"
    let notes = build_release_notes "1.2.3" $cliff ""
    assert str contains $notes "feat: add cool feature"
    assert str contains $notes "fix: patch a bug"
}

def "test release notes contains installation section" [] {
    let notes = build_release_notes "1.2.3" "- changes" ""
    assert str contains $notes "## Installation"
    assert str contains $notes "tui-file-explorer = \"1.2.3\""
    assert str contains $notes "cargo add tui-file-explorer"
}

def "test release notes contains quick start section" [] {
    let notes = build_release_notes "1.2.3" "- changes" ""
    assert str contains $notes "## Quick Start"
    assert str contains $notes "FileExplorer"
}

def "test release notes version appears in cargo toml block" [] {
    let notes = build_release_notes "3.0.0" "- changes" ""
    assert str contains $notes "tui-file-explorer = \"3.0.0\""
}

def "test release notes is a single string" [] {
    let notes = build_release_notes "1.0.0" "- changes" ""
    assert ($notes | describe | str starts-with "string")
}

# ── RELEASE_CHANGELOG.md write tests ─────────────────────────────────────────

def "test release changelog is written to file" [] {
    let tmp = (mktemp -d)
    let notes = build_release_notes "1.0.0" "- initial release" ""
    $notes | save --force ($tmp | path join "RELEASE_CHANGELOG.md")
    assert (($tmp | path join "RELEASE_CHANGELOG.md") | path exists)
    rm -rf $tmp
}

def "test release changelog file content matches notes" [] {
    let tmp = (mktemp -d)
    let notes = build_release_notes "2.0.0" "- big release" "v1.0.0"
    $notes | save --force ($tmp | path join "RELEASE_CHANGELOG.md")
    let content = open --raw ($tmp | path join "RELEASE_CHANGELOG.md")
    rm -rf $tmp
    assert str contains $content "# tui-file-explorer 2.0.0"
    assert str contains $content "### Changes since v1.0.0:"
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_release_prepare.nu ═══(ansi reset)"
    run-tests
}
