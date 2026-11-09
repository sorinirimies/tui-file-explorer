#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Package the release binaries
# ──────────────────────────────────────────────────────────────────────────────
# Packs each binary in BINS (scripts/project.nu),
# target/<target>/release/<bin>[.exe], into
#   dist/<bin>-<target>.tar.gz   (zip on Windows)
# with the binary at the archive root — what download scripts and installers expect
# — plus a .sha256 next to it. Build them first: scripts/ci/build_release.nu.
#
# Usage:
#   nu scripts/ci/package.nu <target> [--out dist]
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [BINS bins_for]

export def is_windows [target: string]: nothing -> bool {
    $target | str contains "windows"
}

def pick [bin?: string]: nothing -> string {
    if $bin != null { $bin } else { $BINS.0.bin }
}

# Archive file name for a target; `bin` defaults to the first binary in BINS.
export def asset_name [target: string, bin?: string]: nothing -> string {
    let ext = if (is_windows $target) { "zip" } else { "tar.gz" }
    $"(pick $bin)-($target).($ext)"
}

# Binary file name for a target; `bin` defaults to the first binary in BINS.
export def binary_name [target: string, bin?: string]: nothing -> string {
    let b = pick $bin
    if (is_windows $target) { $"($b).exe" } else { $b }
}

def pack [target: string, bin: string, out: string] {
    let file = (binary_name $target $bin)
    let src = ($"target/($target)/release/($file)")
    if not ($src | path exists) {
        print --stderr $"(ansi red)❌ ($src) not found — build it first:(ansi reset) nu scripts/ci/build_release.nu ($target)"
        exit 1
    }
    let stage = (mktemp -d)
    cp $src $stage
    let asset = (asset_name $target $bin)
    let dest = ($out | path join $asset | path expand)
    if (is_windows $target) {
        if (which 7z | is-not-empty) {
            do { cd $stage; ^7z a -tzip $dest $file } | ignore
        } else if (which zip | is-not-empty) {
            do { cd $stage; ^zip -q $dest $file }
        } else {
            # Windows runners: PowerShell is always there.
            ^pwsh -NoProfile -Command $"Compress-Archive -Path '($stage | path join $file)' -DestinationPath '($dest)' -Force"
        }
    } else {
        ^tar -C $stage -czf $dest $file
    }
    rm -rf $stage
    let sum = (open --raw $dest | hash sha256)
    $"($sum)  ($asset)\n" | save --force $"($dest).sha256"
    print $"(ansi green)✅ ($dest)(ansi reset)"
}

def main [target: string, --out: string = "dist"] {
    mkdir $out
    for b in (bins_for $target | get bin) { pack $target $b $out }
}
