#!/usr/bin/env nu
# ── tui-file-explorer · test_fixture.nu ───────────────────────────────

use std/assert
use runner.nu *
use ../../examples/vhs/fixture.nu [blob build]

def "test fixture: blob writes the requested size" [] {
    let dir = mktemp --directory
    let file = $dir | path join b.bin
    blob $file 1234
    assert equal (ls $file | get size.0 | into int) 1234
    rm --recursive $dir
}

def "test fixture: build makes the demo tree" [] {
    if (which zip | is-empty) { print -e "skip: zip is not installed"; return }
    let dir = mktemp --directory
    let root = $dir | path join demo
    build $root
    assert ($"($root)-home" | path exists)
    assert equal (ls ($root | path join assets/video.bin) | get size.0 | into int) 3400000
    assert equal ($root | path join latest | path type) "symlink"
    assert (^tar tzf ($root | path join archives/docs.tar.gz) | lines | any {|l| $l ends-with "guide.md" })
    assert (($root | path join archives/bundle.zip) | path exists)
    let status = ^git -C ($root | path join repo) status --porcelain | lines | sort
    assert equal $status [" M src/main.rs", "?? scratch.txt", "A  src/added.rs", "D  TODO.md"]
    rm --recursive $dir
}

def main [] { run-tests }
