#!/usr/bin/env nu
# ── tui-file-explorer · test_package_desktop.nu ─────────────────────────────
# The desktop packaging generators, on a fixture app with every kind of binary,
# and the project's own APP and BINS.

use std/assert
use runner.nu *
use ../project.nu [APP BINS]
use ../ci/package_desktop.nu *

const APP_KEYS = [id title summary bundle_id maintainer homepage license categories]

def app [] {
    {
        id: "demo", title: "Demo", summary: "Does demo things", bundle_id: "io.github.me.demo"
        maintainer: "Me <me@example.com>", homepage: "https://github.com/me/demo"
        license: "MIT", categories: [Utility Development]
    }
}

def bins [] {
    [
        {crate: demo-gui, bin: demo-gui, kind: gui, summary: "Demo desktop app"}
        {crate: demo, bin: demo, kind: cli, summary: "Demo CLI"}
        {crate: demo-tui, bin: demo-tui, kind: tui, summary: "Demo terminal app"}
        {crate: demo-web, bin: demo-web, kind: web, summary: "Demo web dashboard"}
    ]
}

def bin [name: string] { bins | where bin == $name | first }

def "test package_desktop: the project declares a valid app" [] {
    for k in $APP_KEYS { assert ($k in ($APP | columns)) $"APP.($k)" }
    assert ($APP.bundle_id =~ '^[A-Za-z0-9.-]+$') "a macOS bundle id has no underscores"
    assert ($APP.id =~ '^[a-z0-9][a-z0-9-]*$') "APP.id is a package name"
    for b in $BINS {
        assert ($b.kind in [cli tui gui web]) $"($b.bin) kind"
        assert ($b.summary | is-not-empty) $"($b.bin) summary"
    }
}

def "test package_desktop: architectures follow the target" [] {
    assert equal (deb-arch aarch64-unknown-linux-gnu) arm64
    assert equal (deb-arch x86_64-unknown-linux-gnu) amd64
    assert equal (rpm-arch aarch64-unknown-linux-gnu) aarch64
    assert equal (rpm-arch x86_64-unknown-linux-gnu) x86_64
}

def "test package_desktop: only a gui depends on display libraries" [] {
    let gui = deb-control (app) (bin demo-gui) v1.2.3 amd64
    assert ($gui | str contains "Package: demo-gui\nVersion: 1.2.3\nArchitecture: amd64\n")
    assert ($gui | str contains "Depends: libxkbcommon0\n")
    assert ($gui | str contains "Recommends: libwayland-client0")
    assert ($gui | str contains "Maintainer: Me <me@example.com>")
    let cli = deb-control (app) (bin demo) 1.2.3 arm64
    assert not ($cli | str contains "Depends:")
    assert not ($cli | str contains "Recommends:")
}

def "test package_desktop: a tui entry opens a terminal, a gui entry its window" [] {
    let tui = desktop-entry (app) (bin demo-tui)
    assert ($tui | str contains "Terminal=true")
    assert ($tui | str contains "Icon=utilities-terminal")
    assert ($tui | str contains "Categories=Utility;Development;ConsoleOnly;")
    let gui = desktop-entry (app) (bin demo-gui)
    assert ($gui | str contains "Terminal=false")
    assert ($gui | str contains "Icon=demo\n")
    assert ($gui | str contains "StartupWMClass=demo-gui")
}

def "test package_desktop: metainfo is valid xml naming the launcher" [] {
    let x = metainfo (app) (bin demo-gui) | from xml
    let tag = {|n| $x.content | where tag == $n | get 0.content.0.content }
    assert equal (do $tag id) demo-gui
    assert equal (do $tag launchable) demo-gui.desktop
    assert equal (do $tag project_license) MIT
}

def "test package_desktop: package files by kind" [] {
    let have = ["assets/icons/demo.svg" "assets/icons/demo-256.png"]
    let dsts = {|b| package-files (app) (bin $b) x86_64-unknown-linux-gnu $have | get dst }
    assert equal (do $dsts demo) [usr/bin/demo]
    assert equal (do $dsts demo-web) [usr/bin/demo-web]
    assert equal (do $dsts demo-tui) [usr/bin/demo-tui usr/share/applications/demo-tui.desktop]
    assert equal (do $dsts demo-gui) [
        usr/bin/demo-gui usr/share/applications/demo-gui.desktop usr/share/metainfo/demo-gui.metainfo.xml
        usr/share/icons/hicolor/scalable/apps/demo.svg usr/share/icons/hicolor/256x256/apps/demo.png
    ]
    let exe = package-files (app) (bin demo) aarch64-unknown-linux-gnu [] | first
    assert equal $exe.src target/aarch64-unknown-linux-gnu/release/demo
}

def "test package_desktop: a desktop id names the entry and metainfo files" [] {
    let a = app | insert desktop_ids {demo-gui: "io.github.me.Demo"}
    assert equal (entry-id $a (bin demo-gui)) "io.github.me.Demo"
    assert equal (entry-id $a (bin demo-tui)) "demo-tui" "binaries without one keep their name"
    let dsts = package-files $a (bin demo-gui) x86_64-unknown-linux-gnu [] | get dst
    assert ("usr/share/applications/io.github.me.Demo.desktop" in $dsts)
    assert ("usr/share/metainfo/io.github.me.Demo.metainfo.xml" in $dsts)
    let x = metainfo $a (bin demo-gui) | from xml
    assert equal ($x.content | where tag == id | get 0.content.0.content) "io.github.me.Demo"
    assert equal ($x.content | where tag == launchable | get 0.content.0.content) "io.github.me.Demo.desktop"
}

def "test package_desktop: only icons that exist are packaged" [] {
    assert equal (icon-files (app) []) []
    assert equal (icon-files (app) ["assets/icons/demo-16.png"] | get dst) [usr/share/icons/hicolor/16x16/apps/demo.png]
    let named = app | insert icon "io.github.me.Demo"
    assert equal (icon-files $named ["assets/icons/demo.svg"]) [{src: "assets/icons/demo.svg", dst: "usr/share/icons/hicolor/scalable/apps/io.github.me.Demo.svg"}] "APP.icon names the installed icon"
    assert (desktop-entry $named (bin demo-gui) | str contains "Icon=io.github.me.Demo\n")
}

def "test package_desktop: the placeholder icon is a png" [] {
    assert equal ($PLACEHOLDER_ICON | bytes at 0..7) 0x[89504E470D0A1A0A]
    assert equal ($PLACEHOLDER_ICON | bytes at (-8)..) 0x[49454E44AE426082] "ends with IEND"
}

def "test package_desktop: launchable binaries are the gui and tui ones" [] {
    assert equal (launchable (bins) | get bin) [demo-gui demo-tui]
}

def "test package_desktop: the plist names the bundle and an optional icon" [] {
    let p = plist (app) demo-gui v2.0.0 true
    assert ($p | str contains "<key>CFBundleExecutable</key><string>demo-gui</string>")
    assert ($p | str contains "<string>io.github.me.demo</string>")
    assert ($p | str contains "<key>CFBundleShortVersionString</key><string>2.0.0</string>")
    assert ($p | str contains "CFBundleIconFile")
    assert not (plist (app) demo-gui 2.0.0 false | str contains "CFBundleIconFile")
}

def "test package_desktop: universal binaries use what was built" [] {
    assert equal (merge-plan true true) lipo
    assert equal (merge-plan true false) arm64
    assert equal (merge-plan false true) x86_64
    assert equal (merge-plan false false) missing
}

def "test package_desktop: release file names drop the v" [] {
    assert equal (dmg-name (app) v1.0.0) demo-1.0.0-macos.dmg
    assert equal (setup-name (app) 1.0.0) demo-1.0.0-windows-x86_64-setup.exe
    assert equal (appimage-name demo-gui v1.0.0 x86_64) demo-gui-1.0.0-x86_64.AppImage
}

def "test package_desktop: the installer anchors paths and ships every binary" [] {
    let n = nsis (app) (bins) v1.0.0 'D:\a\demo\demo' false
    assert ($n | str starts-with "; Generated")
    assert ($n | str contains "!cd \"D:\\a\\demo\\demo\"")
    assert ($n | str contains "OutFile \"dist\\demo-1.0.0-windows-x86_64-setup.exe\"")
    for b in (bins) { assert ($n | str contains $"File \"target\\x86_64-pc-windows-msvc\\release\\($b.bin).exe\"") $b.bin }
    assert equal ($n | split row "SectionIn RO" | length) 2 "only the first binary is required"
    assert equal ($n | split row "CreateShortcut \"$DESKTOP" | length) 2 "only the gui gets a desktop shortcut"
    assert not ($n | str contains "MUI_ICON") "no icon file, no icon define"
    assert (nsis (app) (bins) 1.0.0 'C:\r' true | str contains "!define MUI_ICON \"packaging\\windows\\demo.ico\"")
    assert ($n | str contains "\"Publisher\" \"Me\"")
    assert ($n | str contains "\"$$p = [Environment]") "a literal $ is $$ in NSIS"
    assert not ($n | str contains " $p ") "an unescaped $p would be an NSIS variable"
}

def "test package_desktop: a committed pkgbuild gets the new version and checksum" [] {
    let old = "pkgname=demo\npkgver=0.1.0\npkgrel=3\nsha256sums=('SKIP')\n"
    assert equal (bump-pkgbuild $old v0.2.0 abc) "pkgname=demo\npkgver=0.2.0\npkgrel=1\nsha256sums=('abc')\n"
}

def "test package_desktop: the generated pkgbuild builds and installs every binary" [] {
    let p = pkgbuild (app) (bins) v1.0.0 abc ["assets/icons/demo.svg"]
    assert ($p | str contains "pkgname=demo\npkgver=1.0.0\npkgrel=1\n")
    assert ($p | str contains "sha256sums=('abc')")
    assert ($p | str contains "source=(\"$pkgname-$pkgver.tar.gz::https://github.com/me/demo/archive/refs/tags/v$pkgver.tar.gz\")")
    assert ($p | str contains "cargo build --frozen --release --package demo-gui --package demo --package demo-tui --package demo-web")
    for b in (bins) { assert ($p | str contains $"\"$pkgdir/usr/bin/($b.bin)\"") $b.bin }
    assert ($p | str contains "'vulkan-icd-loader'") "a gui pulls the display libraries"
    assert ($p | str contains "usr/share/applications/demo-tui.desktop\" <<'EOF'\n[Desktop Entry]")
    assert ($p | str contains "\"assets/icons/demo.svg\"")
    let cli_only = pkgbuild (app) [(bin demo)] 1.0.0 abc []
    assert ($cli_only | str contains "depends=('gcc-libs')")
    assert not ($cli_only | str contains "<<'EOF'")
}

# alien runs inside dist/rpm-<bin>; a relative .deb path broke the .rpm build.
def "test desktop: alien gets an absolute deb path" [] {
    let args = alien-args "dist/app_1.0.0_arm64.deb"
    assert equal ($args | last) ($env.PWD | path join dist app_1.0.0_arm64.deb)
}

# `which` lists this script's own `appimagetool` command; returning its name made
# run-external fail with "Command `appimagetool` not found" on every Linux release.
def "test desktop: a custom command is not an installed program" [] {
    assert equal (on-path on-path) null
    assert ((on-path nu) | path exists)
}

# dpkg-deb rejected netrunner's `netrunner_cli`: package names can't hold `_`.
def "test desktop: package names are valid debian names" [] {
    assert equal (deb-name netrunner_cli) netrunner-cli
    assert equal (deb-name App_GUI) app-gui
    let ctl = deb-control (app) {crate: x, bin: demo_cli, kind: cli, summary: s} 1.0.0 amd64
    assert ($ctl | str starts-with "Package: demo-cli\n")
    for b in $BINS { assert ((deb-name $b.bin) =~ '^[a-z0-9][a-z0-9+.-]+$') $"($b.bin)" }
}

def main [] { run-tests }
