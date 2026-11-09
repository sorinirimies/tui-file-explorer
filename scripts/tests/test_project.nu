#!/usr/bin/env nu
# ── tui-file-explorer · test_project.nu ──────────────────────────────────────
# scripts/project.nu must match the workspace: a stale list breaks a release.

use std/assert
use runner.nu *
use ../project.nu [BINS PUBLISH]

def packages [] {
    let root = ($env.CURRENT_FILE | path dirname | path join ".." "..")
    do { cd $root; ^cargo metadata --no-deps --offline --format-version 1 } | from json | get packages
}

def "test project: every binary is a bin target of its crate" [] {
    let pkgs = packages
    for b in $BINS {
        let p = $pkgs | where name == $b.crate
        assert equal ($p | length) 1 $"($b.crate) is a workspace package"
        let bins = $p.0.targets | where {|t| "bin" in $t.kind } | get name
        assert ($b.bin in $bins) $"($b.crate) builds the binary ($b.bin)"
    }
}

def "test project: every published crate is publishable and listed after its path dependencies" [] {
    let pkgs = packages
    for i in 0..<($PUBLISH | length) {
        let c = $PUBLISH | get $i
        let p = $pkgs | where name == $c
        assert equal ($p | length) 1 $"($c) is a workspace package"
        assert ($p.0.publish != []) $"($c) is not publish = false"
        let local = $p.0.dependencies | where {|d| $d.path? != null and $d.kind? != "dev" } | get name
        for d in $local {
            let at = $PUBLISH | enumerate | where item == $d | get -o index.0
            assert ($at != null and $at < $i) $"($d) is published before ($c)"
        }
    }
}

def "test project: the readme of every published crate exists" [] {
    let pkgs = packages
    for c in $PUBLISH {
        let p = $pkgs | where name == $c | first
        if $p.readme? != null {
            let path = $p.manifest_path | path dirname | path join $p.readme
            assert ($path | path exists) $"($c): readme ($p.readme) exists, or cargo publish fails"
        }
    }
}

def main [] { run-tests }
