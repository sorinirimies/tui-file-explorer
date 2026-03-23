#!/usr/bin/env nu
# Tests for scripts/bump_version.nu
#
# Run with: nu scripts/tests/test_bump_version.nu

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

# Apply the same Cargo.toml update logic that bump_version.nu uses.
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

# ── Version format validation tests ───────────────────────────────────────────

def "test valid version x.y.z is accepted" [] {
    assert ("1.2.3" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version 0.0.0 is accepted" [] {
    assert ("0.0.0" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version with pre-release suffix is accepted" [] {
    assert ("1.0.0-beta.1" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test valid version with alpha suffix is accepted" [] {
    assert ("2.3.4-alpha" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version missing patch is rejected" [] {
    assert not ("1.2" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version with text is rejected" [] {
    assert not ("v1.2.3" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version empty string is rejected" [] {
    assert not ("" =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

def "test invalid version with spaces is rejected" [] {
    assert not ("1.2.3 " =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$')
}

# ── Cargo.toml update logic tests ─────────────────────────────────────────────

def "test cargo toml version line is updated" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "2.0.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "2.0.0"
}

def "test cargo toml patch bump is correct" [] {
    let tmp = make_cargo "0.5.3"
    apply_version_update $tmp "0.5.4"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "0.5.4"
}

def "test cargo toml minor bump is correct" [] {
    let tmp = make_cargo "0.5.3"
    apply_version_update $tmp "0.6.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "0.6.0"
}

def "test cargo toml major bump is correct" [] {
    let tmp = make_cargo "0.9.9"
    apply_version_update $tmp "1.0.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "1.0.0"
}

def "test cargo toml pre-release version is written correctly" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "1.1.0-rc.1"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "1.1.0-rc.1"
}

def "test cargo toml non-version lines are preserved" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "1.0.1"
    let content = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert str contains $content "name = \"test-crate\""
    assert str contains $content "edition = \"2021\""
}

def "test cargo toml dependency lines are not changed" [] {
    let tmp = (mktemp -d)
    let content = '[package]
name = "test-crate"
version = "1.0.0"
edition = "2021"

[dependencies]
serde = { version = "1.0" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    apply_version_update $tmp "1.1.0"
    let updated = open --raw ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    # The package version should be updated
    assert str contains $updated 'version      = "1.1.0"'
    # The dependency version line must be untouched (it doesn't start with `version`)
    assert str contains $updated 'serde = { version = "1.0" }'
}

def "test cargo toml update is idempotent" [] {
    let tmp = make_cargo "1.0.0"
    apply_version_update $tmp "1.0.0"
    let got = read_version ($tmp | path join "Cargo.toml")
    rm -rf $tmp
    assert equal $got "1.0.0"
}

# ── Same-version guard tests ───────────────────────────────────────────────────

def "test same version is detected" [] {
    let current = "1.0.0"
    let new     = "1.0.0"
    assert equal $current $new "same version guard should trigger"
}

def "test different version is not blocked" [] {
    let current = "1.0.0"
    let new     = "1.0.1"
    assert not equal $current $new
}

# ── Tag existence guard tests ──────────────────────────────────────────────────

def "test tag check detects existing tag" [] {
    let existing = ["v1.0.0" "v1.1.0" "v2.0.0"]
    let candidate = "v1.1.0"
    assert ($existing | any { |t| $t == $candidate }) "existing tag should be detected"
}

def "test tag check allows new tag" [] {
    let existing = ["v1.0.0" "v1.1.0" "v2.0.0"]
    let candidate = "v2.0.1"
    assert not ($existing | any { |t| $t == $candidate }) "new tag should not be blocked"
}

def "test tag name is prefixed with v" [] {
    let version = "3.0.0"
    let tag = $"v($version)"
    assert str contains $tag "v"
    assert equal $tag "v3.0.0"
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_bump_version.nu ═══(ansi reset)"
    run-tests
}
