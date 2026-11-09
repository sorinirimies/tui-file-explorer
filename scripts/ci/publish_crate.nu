#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Publish crates to crates.io, idempotently
# ──────────────────────────────────────────────────────────────────────────────
# Publishes one crate, or every crate in PUBLISH (scripts/project.nu) in order.
# No CARGO_REGISTRY_TOKEN ⇒ skip (forks, first setup). "already exists" ⇒ success,
# so a re-run release (just release-retrigger) doesn't fail at the last step.
# --dry-run checks them all in one cargo call, which resolves path dependencies
# between unpublished workspace crates (cargo ≥ 1.90).
#
# Usage:
#   nu scripts/ci/publish_crate.nu [crate] [--dry-run]
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [PUBLISH]

export def already_published [output: string] {
    $output =~ '(already exists|already uploaded)'
}

# `-p` arguments for `crates`.
export def package_args [crates: list<string>]: nothing -> list<string> {
    $crates | each {|c| ["-p" $c] } | flatten
}

def publish [crate: string] {
    let r = do { ^cargo publish -p $crate --locked } | complete
    print $r.stdout $r.stderr
    if $r.exit_code == 0 { return }
    if (already_published $"($r.stdout)($r.stderr)") {
        print $"($crate) is already on crates.io — nothing to do"
        return
    }
    exit 1
}

def main [crate?: string, --dry-run] {
    let crates = if $crate != null { [$crate] } else { $PUBLISH }
    if $dry_run {
        ^cargo publish --dry-run --locked ...(package_args $crates)
        return
    }
    if ($env.CARGO_REGISTRY_TOKEN? | default "" | is-empty) {
        print "CARGO_REGISTRY_TOKEN not set — skipping crates.io"
        return
    }
    for c in $crates { publish $c }
}
