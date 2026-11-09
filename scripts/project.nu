#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — What a release ships
# ──────────────────────────────────────────────────────────────────────────────
# The release and CI scripts read these (build_release, package, publish_crate,
# release_notes, apt, check_static). Edit them when the workspace gains a binary
# or a published crate; test_project.nu checks BINS and PUBLISH against cargo.
#
# Usage:
#   nu scripts/project.nu                  # print what a release ships
#   nu scripts/project.nu matrix           # matrix=<json> for the release workflow
#   nu scripts/project.nu gitea-targets    # the Linux targets Gitea builds
# ──────────────────────────────────────────────────────────────────────────────

# Binaries a release packages, each with the workspace crate that builds it.
# `kind` decides the desktop packaging (scripts/ci/package_desktop.nu): cli, web,
# tui (a terminal .desktop entry, an AppImage) or gui (.desktop, AppStream, icons,
# an AppImage, the macOS .app, Start-menu shortcuts). A gui binary never builds
# for musl; set APT to its headers. An optional `os` column (a list of linux,
# macos, windows) limits a binary to those systems, e.g. a macOS-only GUI.
export const BINS = [[crate bin kind summary]; ["tui-file-explorer" "tfe" tui "tfe: keyboard-driven terminal file explorer"]]

# The desktop app. `id` names the packages' icon (assets/icons/<id>.svg and
# <id>-<size>.png), the DMG, the installer and the AUR package; `bundle_id` is the
# macOS bundle identifier; `categories` are freedesktop menu categories.
export const APP = {
    id: "tui-file-explorer"
    title: "tfe"
    summary: "Embeddable keyboard-driven multi-pane file explorer for Ratatui"
    bundle_id: "io.github.sorinirimies.tui-file-explorer"
    maintainer: "Sorin Irimies <sorinirimies@gmail.com>"
    homepage: "https://github.com/sorinirimies/tui-file-explorer"
    license: "MIT"
    categories: [System FileTools FileManager]
}

# Crates published to crates.io, dependencies first.
export const PUBLISH = ["tui-file-explorer"]

# System packages (apt) every Linux job that compiles installs first, e.g. the
# headers a GUI toolkit links against: ["libxkbcommon-dev"].
export const APT = []

# Linux release binaries: "musl" (static, any distro, cargo-zigbuild) or "gnu"
# (glibc, built natively; for GUI apps, which can not link statically).
export const LINUX = "musl"

# Ship a Windows archive. false for tools that do not support Windows.
export const WINDOWS = true

# The GitHub nightly raises the dependency requirements (cargo upgrade) and, when
# one changes and the gate is green, releases the next patch version. false: it
# only updates the lockfiles. A library's users only see its requirements.
export const AUTO_RELEASE = false

# Shell scripts that must run before Nushell exists, e.g. a `curl | sh` installer
# that installs nu and then hands over to a nu script: ["scripts/get.sh"].
# Everything else is Nushell; test_workflows.nu checks it.
export const SHELL_BOOTSTRAP = []

# The GitHub release build matrix for a LINUX flavour, one row per target.
export def release_matrix [linux: string, --no-windows]: nothing -> list<record> {
    let lin = if $linux == "gnu" {
        [
            {runner: ubuntu-latest, target: x86_64-unknown-linux-gnu, zig: false}
            {runner: ubuntu-24.04-arm, target: aarch64-unknown-linux-gnu, zig: false}
        ]
    } else {
        [
            {runner: ubuntu-latest, target: x86_64-unknown-linux-musl, zig: true}
            {runner: ubuntu-latest, target: aarch64-unknown-linux-musl, zig: true}
        ]
    }
    [
        {runner: macos-latest, target: aarch64-apple-darwin, zig: false}
        {runner: macos-latest, target: x86_64-apple-darwin, zig: false}
        ...$lin
        {runner: windows-latest, target: x86_64-pc-windows-msvc, zig: false}
    ] | where {|r| not ($no_windows and ($r.target | str contains windows)) }
}

# The Linux targets Gitea (one x86-64 runner) builds for a LINUX flavour.
export def gitea_targets [linux: string]: nothing -> list<string> {
    if $linux == "gnu" { [x86_64-unknown-linux-gnu] } else { [x86_64-unknown-linux-musl aarch64-unknown-linux-musl] }
}

# The system a target triple runs on: linux, macos or windows.
export def target_os [target: string]: nothing -> string {
    if ($target | str contains "apple-darwin") { "macos" } else if ($target | str contains "windows") { "windows" } else { "linux" }
}

# The rows of `bins` built for `target` (all without one). A `gui` binary never
# builds for musl: a GUI links glibc and the display libraries. A row with an
# `os` list builds only for those systems.
export def built_for [bins: list<any>, target?: string]: nothing -> list<any> {
    if $target == null { return $bins }
    let musl = $target | str ends-with "musl"
    let os = target_os $target
    $bins | where {|b| not ($musl and $b.kind? == "gui") and ($b.os? == null or $os in $b.os) }
}

# The binaries built for `target` (all without one).
export def bins_for [target?: string]: nothing -> list<any> { built_for $BINS $target }

# Crates that own a binary built for `target` (all without one), each once.
export def bin_crates [target?: string]: nothing -> list<string> {
    bins_for $target | get -o crate | default [] | uniq
}

# `matrix=<json>` for $GITHUB_OUTPUT.
def "main matrix" [] {
    let m = if $WINDOWS { release_matrix $LINUX } else { release_matrix $LINUX --no-windows }
    print $"matrix=({include: $m} | to json --raw)"
}

# `auto_release=true|false` for $GITHUB_OUTPUT.
def "main auto-release" [] {
    print $"auto_release=($AUTO_RELEASE)"
}

def "main gitea-targets" [] {
    print (gitea_targets $LINUX | str join " ")
}

def main [] {
    let bins = $BINS | get -o bin | default []
    print $"binaries=($bins | str join ',')"
    print $"crates_io=($PUBLISH | str join ',')"
}
