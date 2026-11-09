#!/usr/bin/env nu
# ── tui-file-explorer · test_package_binaries.nu ─────────────────────────────
# Names and checksum lines of the release binaries (no cargo, no archive I/O).

use std/assert
use runner.nu *
use ../ci/package_binaries.nu [raw_name archive_name checksum_line]

def "test package_binaries: bare binary name carries the target" [] {
    assert equal (raw_name "x86_64-unknown-linux-gnu" "") "tfe-x86_64-unknown-linux-gnu"
    assert equal (raw_name "x86_64-pc-windows-msvc" ".exe") "tfe-x86_64-pc-windows-msvc.exe"
}

def "test package_binaries: unix archive is tar.gz" [] {
    assert equal (archive_name "4.0.3" "aarch64-apple-darwin" "") "tfe-4.0.3-aarch64-apple-darwin.tar.gz"
}

def "test package_binaries: windows archive is zip" [] {
    assert equal (archive_name "4.0.3" "x86_64-pc-windows-msvc" ".exe") "tfe-4.0.3-x86_64-pc-windows-msvc.zip"
}

def "test package_binaries: checksum line is sha256sum style" [] {
    assert equal (checksum_line "abc123" "tfe-x.tar.gz") "abc123  tfe-x.tar.gz\n"
}

def main [] { run-tests }
