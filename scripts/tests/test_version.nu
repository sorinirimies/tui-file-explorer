#!/usr/bin/env nu
# Tests for scripts/version.nu
#
# Run with: nu scripts/tests/test_version.nu

use std/assert
use runner.nu *

# ── Helpers ───────────────────────────────────────────────────────────────────

# Extract the version string from the contents of a Cargo.toml string.
# This mirrors the exact pipeline used in version.nu.
def parse_version [cargo_toml: string] {
    $cargo_toml
    | lines
    | where { |l| $l =~ '^version\s*=' }
    | first
    | parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
    | get v
    | first
}

# Write a minimal Cargo.toml at the given version into a temp dir and return the dir path.
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

# ── Tests ─────────────────────────────────────────────────────────────────────

def "test version reads simple semver" [] {
    let tmp = make_cargo "1.2.3"
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "1.2.3"
}

def "test version reads zero patch" [] {
    let tmp = make_cargo "0.1.0"
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.1.0"
}

def "test version reads pre-release suffix" [] {
    let tmp = make_cargo "0.5.0-beta.1"
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "0.5.0-beta.1"
}

def "test version reads padded assignment" [] {
    # After a bump the line becomes `version      = "x.y.z"` with extra spaces.
    let tmp = (mktemp -d)
    let content = '[package]
name = "test-crate"
version      = "2.0.0"
edition = "2021"
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "2.0.0"
}

def "test version ignores dependency version lines" [] {
    # Lines like `serde = { version = "1.0" }` must not be picked up because
    # they do not start with `version`.
    let tmp = (mktemp -d)
    let content = '[package]
name = "test-crate"
version = "3.1.4"
edition = "2021"

[dependencies]
serde = { version = "1.0" }
ratatui = { version = "0.29" }
'
    $content | save --force ($tmp | path join "Cargo.toml")
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert equal $got "3.1.4"
}

def "test version output contains no whitespace" [] {
    let tmp = make_cargo "0.9.1"
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got !~ '\s') $"expected no whitespace, got: ($got)"
}

def "test version output contains no quotes" [] {
    let tmp = make_cargo "1.0.0"
    let got = parse_version (open --raw ($tmp | path join "Cargo.toml"))
    rm -rf $tmp
    assert ($got !~ '"') $"expected no quotes, got: ($got)"
}

# ── Runner ────────────────────────────────────────────────────────────────────

def main [] {
    print $"(ansi cyan)═══ test_version.nu ═══(ansi reset)"
    run-tests
}
