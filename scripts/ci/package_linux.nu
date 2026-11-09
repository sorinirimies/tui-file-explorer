#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Linux packages: .deb, .rpm and AppImage
# ──────────────────────────────────────────────────────────────────────────────
# Builds, from target/<target>/release/tfe, into dist/:
#   tfe_<version>_<amd64|arm64>.deb
#   tfe-<version>-<x86_64|aarch64>.rpm       (the .deb converted with alien)
#   tfe-<version>-<x86_64|aarch64>.AppImage  (with --appimage)
# The packages install /usr/bin/tfe and a terminal .desktop entry. Run it on the
# oldest hosted runner: a binary needs at least the glibc it was linked against.
#
# Usage (build first):
#   nu scripts/ci/package_linux.nu tools
#   nu scripts/ci/package_linux.nu build <version> <target> [--appimage]
# ──────────────────────────────────────────────────────────────────────────────

use apt.nu [commands]

const BIN = "tfe"
const SUMMARY = "Embeddable keyboard-driven multi-pane file explorer for Ratatui"
const MAINTAINER = "Sorin Irimies <sorinirimies@gmail.com>"
const HOMEPAGE = "https://github.com/sorinirimies/tui-file-explorer"
# A 1×1 transparent PNG: the AppImage needs an icon file.
const PLACEHOLDER_ICON = 0x[89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415478DA63F8FFFF3F0005FE02FEA7D6A1E50000000049454E44AE426082]

export def plain_version [version: string]: nothing -> string { $version | str replace --regex '^v' '' }

# Debian / RPM architecture names of a Rust target.
export def deb_arch [target: string]: nothing -> string { if ($target | str starts-with "aarch64") { "arm64" } else { "amd64" } }
export def rpm_arch [target: string]: nothing -> string { if ($target | str starts-with "aarch64") { "aarch64" } else { "x86_64" } }

export def deb_name [version: string, target: string]: nothing -> string { $"($BIN)_(plain_version $version)_(deb_arch $target).deb" }
export def rpm_name [version: string, target: string]: nothing -> string { $"($BIN)-(plain_version $version)-(rpm_arch $target).rpm" }
export def appimage_name [version: string, target: string]: nothing -> string { $"($BIN)-(plain_version $version)-(rpm_arch $target).AppImage" }

# Arguments for alien. The .deb path is made absolute, since alien runs in its own
# scratch directory where a relative `dist/…` path no longer resolves.
export def alien_args [deb: string]: nothing -> list<string> {
    ["--to-rpm" "--scripts" ($deb | path expand)]
}

# DEBIAN/control.
export def deb_control [version: string, arch: string]: nothing -> string {
    $"Package: ($BIN)
Version: (plain_version $version)
Architecture: ($arch)
Maintainer: ($MAINTAINER)
Section: utils
Priority: optional
Description: ($SUMMARY)
 tfe is a keyboard-driven, multi-pane terminal file manager built on Ratatui.
Homepage: ($HOMEPAGE)
"
}

# The freedesktop entry: a terminal app in the file-manager category.
export def desktop_entry []: nothing -> string {
    $"[Desktop Entry]
Type=Application
Version=1.0
Name=tfe \(terminal\)
Comment=($SUMMARY)
Exec=($BIN)
Icon=utilities-terminal
Terminal=true
Categories=System;FileTools;FileManager;ConsoleOnly;
"
}

def fail [msg: string] { print -e $"(ansi red)❌ ($msg)(ansi reset)"; exit 1 }

# Install the packagers (CI).
def "main tools" [] {
    let root = (^id -u | str trim) == "0"
    for c in (commands [dpkg-dev rpm alien fakeroot file] $root) { run-external $c.0 ...($c | skip 1) }
    print "tools=linux"
}

# The path of an installed program, or null (`which` also lists custom commands).
export def on_path [name: string]: nothing -> any {
    which $name | where type == external | get -o 0.path
}

def appimagetool []: nothing -> string {
    let installed = on_path appimagetool
    if $installed != null { return $installed }
    let arch = uname | get machine
    let tool = $"target/tools/appimagetool-($arch).AppImage"
    if not ($tool | path exists) {
        mkdir target/tools
        http get --raw $"https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-($arch).AppImage" | save --force $tool
        ^chmod +x $tool
    }
    $tool
}

def "main build" [version: string, target: string, --appimage] {
    let exe = $"target/($target)/release/($BIN)"
    if not ($exe | path exists) { fail $"($exe) missing: build it first" }
    mkdir dist

    let root = "dist/deb-root"
    rm -rf $root
    mkdir $"($root)/DEBIAN" $"($root)/usr/bin" $"($root)/usr/share/applications" $"($root)/usr/share/doc/($BIN)"
    cp $exe $"($root)/usr/bin/($BIN)"
    ^chmod 755 $"($root)/usr/bin/($BIN)"
    desktop_entry | save --force $"($root)/usr/share/applications/($BIN).desktop"
    cp LICENSE $"($root)/usr/share/doc/($BIN)/copyright"
    deb_control $version (deb_arch $target) | save --force $"($root)/DEBIAN/control"
    let deb = $"dist/(deb_name $version $target)"
    ^dpkg-deb --root-owner-group --build $root $deb

    # alien writes into the working directory: give it its own.
    let scratch = "dist/rpm-scratch"
    rm -rf $scratch
    mkdir $scratch
    let args = alien_args $deb
    do { cd $scratch; ^fakeroot alien ...$args }
    mv (glob $"($scratch)/*.rpm" | first) $"dist/(rpm_name $version $target)"
    rm -rf $root $scratch
    print -e $"✅ ($deb) + (rpm_name $version $target)"

    if $appimage {
        let dir = "dist/AppDir"
        rm -rf $dir
        mkdir $"($dir)/usr/bin"
        cp $exe $"($dir)/usr/bin/($BIN)"
        desktop_entry | str replace --regex '(?m)^Icon=.*$' $"Icon=($BIN)" | save --force $"($dir)/($BIN).desktop"
        $PLACEHOLDER_ICON | save --force $"($dir)/($BIN).png"
        $"#!/bin/sh\nexec \"$\(dirname \"$\(readlink -f \"$0\")\")/usr/bin/($BIN)\" \"$@\"\n" | save --force $"($dir)/AppRun"
        ^chmod +x $"($dir)/AppRun"
        let out = $"dist/(appimage_name $version $target)"
        # Hosted runners have no FUSE; the tool extracts itself instead.
        with-env {ARCH: (rpm_arch $target), APPIMAGE_EXTRACT_AND_RUN: "1"} { run-external (appimagetool) $dir $out }
        rm -rf $dir
        print -e $"✅ ($out)"
    }
}

def main [] { print $"binary=($BIN)" }
