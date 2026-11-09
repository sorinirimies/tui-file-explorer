#!/usr/bin/env nu
# ── tui-file-explorer · test_package.nu ──────────────────────────────────────
# The archive names are what installers/downloaders rely on; keep them stable.

use std/assert
use runner.nu *
use ../ci/package.nu [asset_name binary_name is_windows]
use ../ci/build_release.nu [build_args]
use ../project.nu [BINS bin_crates built_for target_os release_matrix gitea_targets]

# The first binary in BINS is the default for asset_name and binary_name.
def first_bin [] { $BINS.0.bin }

def "test package: unix targets are tar.gz" [] {
    let b = first_bin
    assert equal (asset_name "aarch64-apple-darwin") $"($b)-aarch64-apple-darwin.tar.gz"
    assert equal (asset_name "x86_64-unknown-linux-musl") $"($b)-x86_64-unknown-linux-musl.tar.gz"
    assert equal (binary_name "aarch64-unknown-linux-musl") $b
}

def "test package: windows is zip with exe" [] {
    let b = first_bin
    assert (is_windows "x86_64-pc-windows-msvc")
    assert equal (asset_name "x86_64-pc-windows-msvc") $"($b)-x86_64-pc-windows-msvc.zip"
    assert equal (binary_name "x86_64-pc-windows-msvc") $"($b).exe"
}

def "test package: a named binary gets its own archive" [] {
    assert equal (asset_name "aarch64-apple-darwin" "other") "other-aarch64-apple-darwin.tar.gz"
    assert equal (binary_name "x86_64-pc-windows-msvc" "other") "other.exe"
}

def "test build_release: release locked and one -p per binary crate" [] {
    let p = bin_crates | each {|c| [-p $c] } | flatten
    assert equal ($p | length) (2 * ($BINS.crate | uniq | length))
    let musl = bin_crates x86_64-unknown-linux-musl | each {|c| [-p $c] } | flatten
    assert equal (build_args "x86_64-unknown-linux-musl") [--release --locked --target x86_64-unknown-linux-musl ...$musl]
    assert equal (build_args) [--release --locked ...$p]
}

def "test build_release: a gui binary never builds for musl" [] {
    let bins = [[crate bin kind]; [app-gui app-gui gui] [app app cli]]
    assert equal (built_for $bins x86_64-unknown-linux-musl | get bin) [app]
    assert equal (built_for $bins x86_64-unknown-linux-gnu | get bin) [app-gui app]
    assert equal (built_for $bins | get bin) [app-gui app]
    assert equal (built_for [[crate bin]; [a a]] aarch64-unknown-linux-musl | get bin) [a] "no kind column: a plain binary"
}

def "test build_release: a binary with an os list builds only for those systems" [] {
    let bins = [[crate bin kind os]; [app-gui app-gui gui [macos]] [app app cli null]]
    assert equal (built_for $bins aarch64-apple-darwin | get bin) [app-gui app]
    assert equal (built_for $bins x86_64-unknown-linux-gnu | get bin) [app]
    assert equal (built_for $bins x86_64-pc-windows-msvc | get bin) [app]
    assert equal (built_for $bins | get bin) [app-gui app] "no target: every binary"
}

def "test build_release: target triples map to their system" [] {
    assert equal (target_os x86_64-apple-darwin) macos
    assert equal (target_os aarch64-pc-windows-msvc) windows
    assert equal (target_os x86_64-unknown-linux-musl) linux
}

def "test package: musl releases five targets, the linux ones with zig" [] {
    let m = release_matrix musl
    assert equal ($m.target | sort) [aarch64-apple-darwin aarch64-unknown-linux-musl x86_64-apple-darwin x86_64-pc-windows-msvc x86_64-unknown-linux-musl]
    assert equal ($m | where zig | get target | sort) [aarch64-unknown-linux-musl x86_64-unknown-linux-musl]
    assert equal (gitea_targets musl) [x86_64-unknown-linux-musl aarch64-unknown-linux-musl]
}

def "test package: gnu builds linux natively, arm64 on an arm runner" [] {
    let m = release_matrix gnu
    assert equal ($m | where zig | length) 0
    assert equal ($m | where target == aarch64-unknown-linux-gnu | get runner.0) ubuntu-24.04-arm
    assert ("x86_64-unknown-linux-gnu" in $m.target)
    assert equal (gitea_targets gnu) [x86_64-unknown-linux-gnu]
}

def "test package: without windows the matrix has no windows target" [] {
    let m = release_matrix musl --no-windows
    assert equal ($m | length) 4
    assert not ($m.target | any {|t| $t | str contains windows })
}

def "test package: the release workflow builds the matrix from project.nu" [] {
    let root = ($env.CURRENT_FILE | path dirname | path join ".." "..")
    let wf = (open --raw ($root | path join ".github" "workflows" "release.yml"))
    assert ($wf | str contains "nu scripts/project.nu matrix")
    assert ($wf | str contains "fromJSON(needs.validate.outputs.matrix)")
}

def main [] { run-tests }
