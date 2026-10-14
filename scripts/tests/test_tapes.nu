#!/usr/bin/env nu
# Asserts no VHS tape shows compilation: `cargo` may only appear between Hide and Show.
def main [] {
    mut bad = []
    for f in (glob ($env.CURRENT_FILE | path dirname | path dirname | path dirname | path join "examples/vhs/*.tape")) {
        mut hidden = false
        mut n = 0
        for line in (open $f | lines) {
            $n += 1
            let l = ($line | str trim)
            if $l == "Hide" { $hidden = true } else if $l == "Show" { $hidden = false } else if (not $hidden) and ($l | str starts-with "Type") and ($l | str contains "cargo") {
                $bad = ($bad | append $"($f | path basename):($n)")
            }
        }
    }
    if ($bad | length) > 0 { print $"visible cargo in tapes: ($bad)"; exit 1 }
    print "ok: no tape shows cargo output"
}
