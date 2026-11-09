#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Package the `tfe` binary for one target
# ──────────────────────────────────────────────────────────────────────────────
# Packs target/<target>/release/tfe[.exe] into
#   dist/tfe-<target>.tar.gz   (tfe-<target>.zip on Windows)
# with the binary at the archive root — what download scripts and installers
# expect, same layout as GitKraft — plus a .sha256 next to it. Build first:
# cargo build --release --bin tfe --target <target>.
#
# Usage:
#   nu scripts/ci/package_binaries.nu <target> [--out dist]
# ──────────────────────────────────────────────────────────────────────────────

const BIN = "tfe"

export def is_windows [target: string]: nothing -> bool {
    $target | str contains "windows"
}

# Archive file name for a target: tfe-<target>.tar.gz, or .zip on Windows.
export def asset_name [target: string]: nothing -> string {
    let ext = if (is_windows $target) { "zip" } else { "tar.gz" }
    $"($BIN)-($target).($ext)"
}

# Binary file name for a target: tfe, or tfe.exe on Windows.
export def binary_name [target: string]: nothing -> string {
    if (is_windows $target) { $"($BIN).exe" } else { $BIN }
}

# A sha256sum-style line, newline-terminated: "<hex>  <file>\n".
export def checksum_line [hash: string, file: string]: nothing -> string {
    $"($hash)  ($file)\n"
}

def main [target: string, --out: string = "dist"] {
    let file = (binary_name $target)
    let src = $"target/($target)/release/($file)"
    if not ($src | path exists) {
        print --stderr $"(ansi red)❌ ($src) not found — build it first(ansi reset)"
        exit 1
    }
    mkdir $out
    let stage = (mktemp -d)
    cp $src $stage
    let asset = (asset_name $target)
    let dest = ($out | path join $asset | path expand)
    if (is_windows $target) {
        if (which 7z | is-not-empty) {
            do { cd $stage; ^7z a -tzip $dest $file } | ignore
        } else if (which zip | is-not-empty) {
            do { cd $stage; ^zip -q $dest $file }
        } else {
            ^pwsh -NoProfile -Command $"Compress-Archive -Path '($stage | path join $file)' -DestinationPath '($dest)' -Force"
        }
    } else {
        ^tar -C $stage -czf $dest $file
    }
    rm -rf $stage
    checksum_line (open --raw $dest | hash sha256) $asset | save --force $"($dest).sha256"
    print $"(ansi green)✅ ($dest)(ansi reset)"
}
