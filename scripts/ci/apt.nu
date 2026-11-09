#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Install the system packages a Linux build needs
# ──────────────────────────────────────────────────────────────────────────────
# Installs APT (scripts/project.nu) with apt-get, e.g. the headers a GUI toolkit
# links against. Does nothing off Linux or when APT is empty. Every CI job that
# compiles runs it first (test_workflows.nu checks that).
#
# Usage: nu scripts/ci/apt.nu
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [APT]

# The apt-get command lines for `packages`, with sudo unless running as root.
export def commands [packages: list<string>, root: bool]: nothing -> list<list<string>> {
    if ($packages | is-empty) { return [] }
    let pre = if $root { [] } else { [sudo] }
    [
        [...$pre apt-get -o Acquire::Retries=5 update -qq]
        [...$pre apt-get -o Acquire::Retries=5 install -y --no-install-recommends ...$packages]
    ]
}

def main [] {
    if $nu.os-info.name != "linux" or ($APT | is-empty) {
        print "apt=skipped"
        return
    }
    let root = (^id -u | str trim) == "0"
    for c in (commands $APT $root) { run-external $c.0 ...($c | skip 1) }
    print $"apt=($APT | str join ',')"
}
