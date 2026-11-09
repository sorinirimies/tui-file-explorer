#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Assert the Linux release binaries are static and run
# ──────────────────────────────────────────────────────────────────────────────
# The musl release binaries must run on any distro: no dynamic loader, no libc.
#
# Checks target/<target>/release/<bin> for each binary in BINS (scripts/project.nu).
#
# Skipped when LINUX is "gnu". --build builds them first (build_release.nu).
#
# Usage: nu scripts/ci/check_static.nu <target> [--build]
# ──────────────────────────────────────────────────────────────────────────────

# ldd's verdict on a static binary differs between glibc versions.
export def is_static [ldd_output: string] {
    $ldd_output =~ '(not a dynamic executable|statically linked)'
}

use ../project.nu [bins_for LINUX]

def check [binary: path] {
    let ldd = do { ^ldd $binary } | complete
    if not (is_static $"($ldd.stdout)($ldd.stderr)") {
        print $"❌ ($binary) is dynamically linked:\n($ldd.stdout)"
        exit 1
    }
    ^$binary --version
}

def main [target: string, --build] {
    if $LINUX != "musl" {
        print "static=skipped (LINUX is gnu)"
        return
    }
    if $build { ^nu ($env.CURRENT_FILE | path dirname | path join build_release.nu) $target }
    for b in (bins_for $target | get bin) { check $"target/($target)/release/($b)" }
}
