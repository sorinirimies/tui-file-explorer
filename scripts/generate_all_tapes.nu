#!/usr/bin/env nu
# Build first (so GIFs show results, never compilation), then record every tape.
def main [] {
    let root = ($env.CURRENT_FILE | path dirname | path dirname)
    cd $root
    for args in [["build" "--release" "--examples" "--features" "theme"] ["build" "--bin" "tfe"] ["build" "--release"]] {
        let r = (do { ^cargo ...$args } | complete)
        if $r.exit_code != 0 { print $r.stderr; error make {msg: $"cargo ($args | str join ' ') failed"} }
    }
    for t in (glob examples/vhs/*.tape | sort) {
        print $"recording ($t)"
        ^vhs $t
    }
}
