#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Package the `tfe` binary for one target
# ──────────────────────────────────────────────────────────────────────────────
# Reads target/<target>/release/tfe[.exe] and writes into dist/:
#
#   tfe-<target>[.exe]                      the bare binary (for curl / direct download)
#   tfe-<version>-<target>.tar.gz           portable archive (Linux, macOS)
#   tfe-<version>-<target>.zip              portable archive (Windows, suffix ".exe")
#   <each of the above>.sha256              sha256sum-compatible checksum
#
# Archives hold the binary plus README.md and LICENSE in one top-level folder.
#
# Usage: nu scripts/ci/package_binaries.nu <version> <target> [suffix]
# ──────────────────────────────────────────────────────────────────────────────

const DIST = "dist"
const BIN = "tfe"

# Name of the bare binary for a target, e.g. tfe-x86_64-unknown-linux-gnu.
export def raw_name [target: string, suffix: string]: nothing -> string {
    $"($BIN)-($target)($suffix)"
}

# Name of the portable archive: .zip for Windows (suffix set), .tar.gz otherwise.
export def archive_name [version: string, target: string, suffix: string]: nothing -> string {
    if $suffix == "" {
        $"($BIN)-($version)-($target).tar.gz"
    } else {
        $"($BIN)-($version)-($target).zip"
    }
}

# A sha256sum-style line, newline-terminated: "<hex>  <file>\n".
export def checksum_line [hash: string, file: string]: nothing -> string {
    $"($hash)  ($file)\n"
}

def sha256 [path: string]: nothing -> string {
    open --raw $path | hash sha256
}

def main [version: string, target: string, suffix: string = ""] {
    let src = $"target/($target)/release/($BIN)($suffix)"
    if not ($src | path exists) {
        print --stderr $"(ansi red)❌ missing ($src)(ansi reset)"
        exit 1
    }

    mkdir $DIST

    let raw = (raw_name $target $suffix)
    cp $src $"($DIST)/($raw)"
    (checksum_line (sha256 $"($DIST)/($raw)") $raw) | save --force $"($DIST)/($raw).sha256"

    # Stage a folder named like the archive so unpacking never litters the cwd.
    let stem = $"($BIN)-($version)-($target)"
    let stage = $"($DIST)/($stem)"
    rm -rf $stage
    mkdir $stage
    cp $src $"($stage)/($BIN)($suffix)"
    cp README.md LICENSE $stage

    let archive = (archive_name $version $target $suffix)
    if $suffix == "" {
        ^tar -C $DIST -czf $"($DIST)/($archive)" $stem
    } else {
        # bsdtar (Windows 10+, macOS) writes zip with -a; no zip/7z dependency.
        ^tar -C $DIST -a -cf $"($DIST)/($archive)" $stem
    }
    rm -rf $stage

    (checksum_line (sha256 $"($DIST)/($archive)") $archive) | save --force $"($DIST)/($archive).sha256"

    print $"(ansi green)✅ ($DIST)/($archive) + ($raw)(ansi reset)"
}
