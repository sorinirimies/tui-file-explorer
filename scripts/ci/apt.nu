#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — apt-get command lines for CI packagers
# ──────────────────────────────────────────────────────────────────────────────
# Shared by package_linux.nu. Run directly to install packages (Linux only):
#
# Usage: nu scripts/ci/apt.nu [packages...]
# ──────────────────────────────────────────────────────────────────────────────

# The apt-get command lines for `packages`, with sudo unless running as root.
export def commands [packages: list<string>, root: bool]: nothing -> list<list<string>> {
    if ($packages | is-empty) { return [] }
    let pre = if $root { [] } else { [sudo] }
    [
        [...$pre apt-get -o Acquire::Retries=5 update -qq]
        [...$pre apt-get -o Acquire::Retries=5 install -y --no-install-recommends ...$packages]
    ]
}

def main [...packages: string] {
    if $nu.os-info.name != "linux" or ($packages | is-empty) {
        print "apt=skipped"
        return
    }
    let root = (^id -u | str trim) == "0"
    for c in (commands $packages $root) { run-external $c.0 ...($c | skip 1) }
    print $"apt=($packages | str join ',')"
}
