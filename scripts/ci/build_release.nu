#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Release build of every shipped binary
# ──────────────────────────────────────────────────────────────────────────────
# Builds the crates in BINS (scripts/project.nu) with --release --locked.
# --zig cross-compiles with cargo-zigbuild (static musl Linux binaries).
#
# Usage:
#   nu scripts/ci/build_release.nu                       # host target
#   nu scripts/ci/build_release.nu <target> [--zig]
#   nu scripts/ci/build_release.nu gitea [--out dist]     # build + package Gitea's targets
#   nu scripts/ci/build_release.nu install                # cargo install them locally
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [bin_crates gitea_targets LINUX]

# The cargo arguments for a release build, for `target` or the host.
export def build_args [target?: string]: nothing -> list<string> {
    let t = if $target != null { ["--target" $target] } else { [] }
    ["--release" "--locked" ...$t ...(bin_crates $target | each {|c| ["-p" $c] } | flatten)]
}

# Gitea's release: its Linux targets, cross-built with zig when they are musl.
def "main gitea" [--out: string = "dist"] {
    let here = $env.CURRENT_FILE | path dirname
    for t in (gitea_targets $LINUX) {
        let cmd = if $LINUX == "musl" { "zigbuild" } else { "build" }
        ^cargo $cmd ...(build_args $t)
        ^nu ($here | path join package.nu) $t --out $out
    }
}

# Install the shipped binaries from this checkout into ~/.cargo/bin.
def "main install" [] {
    let pkgs = ^cargo metadata --no-deps --format-version 1 | from json | get packages
    for c in (bin_crates) {
        let dir = $pkgs | where name == $c | get manifest_path.0 | path dirname
        ^cargo install --path $dir --locked --force
    }
}

def main [target?: string, --zig] {
    let cmd = if $zig { "zigbuild" } else { "build" }
    ^cargo $cmd ...(build_args $target)
}
