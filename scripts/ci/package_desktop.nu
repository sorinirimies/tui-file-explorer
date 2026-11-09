#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# tui-file-explorer — Desktop packages
# ──────────────────────────────────────────────────────────────────────────────
# Everything comes from APP and BINS in scripts/project.nu; each binary's `kind`
# decides what it gets:
#   cli, web   /usr/bin only; "Command Line Tools" in the DMG; on PATH on Windows
#   tui        + a terminal .desktop entry and an AppImage
#   gui        + .desktop, AppStream metainfo, icons, an AppImage, the macOS .app
#              (the first gui binary) and Start-menu shortcuts
# Files in packaging/linux/<bin>.desktop and <bin>.metainfo.xml (<bin> or its
# APP.desktop_ids name) win over the
# generated ones; icons are assets/icons/<id>.svg and <id>-<size>.png, the macOS
# icon packaging/macos/<id>.icns and the Windows icon packaging/windows/<id>.ico
# (each optional). A committed packaging/aur/PKGBUILD wins over the generated one.
#
# Usage (build first: nu scripts/ci/build_release.nu <target>):
#   nu scripts/ci/package_desktop.nu tools <linux|macos|windows>   # install the packagers
#   nu scripts/ci/package_desktop.nu linux <version> <target> [--appimage]
#   nu scripts/ci/package_desktop.nu macos <version>               # universal DMG
#   nu scripts/ci/package_desktop.nu windows <version>             # NSIS installer
#   nu scripts/ci/package_desktop.nu aur <version>                 # dist/aur/PKGBUILD
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [APP BINS built_for]
use apt.nu [commands]

export const ICON_SIZES = [16 22 24 32 48 64 96 128 256 512]
# A 1×1 transparent PNG: the AppImage icon of an app without assets/icons/<id>-256.png.
export const PLACEHOLDER_ICON = 0x[89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415478DA63F8FFFF3F0005FE02FEA7D6A1E50000000049454E44AE426082]
export const MAC_TARGETS = ["aarch64-apple-darwin" "x86_64-apple-darwin"]
const WINDOWS_TARGET = "x86_64-pc-windows-msvc"

# Uninstall: drop the install folder from the system PATH again.
const UNPATH = r#'    ExecWait 'powershell -NoProfile -Command "$$p = [Environment]::GetEnvironmentVariable(\"Path\",\"Machine\"); [Environment]::SetEnvironmentVariable(\"Path\", (($$p -split \";\") | Where-Object { $$_ -ne \"$INSTDIR\" }) -join \";\", \"Machine\")"''#

export def plain-version [version: string]: nothing -> string { $version | str replace --regex '^v' '' }

# Debian / RPM architecture names of a Rust target.
export def deb-arch [target: string]: nothing -> string { if ($target | str starts-with "aarch64") { "arm64" } else { "amd64" } }
export def rpm-arch [target: string]: nothing -> string { if ($target | str starts-with "aarch64") { "aarch64" } else { "x86_64" } }

# Arguments for alien. The .deb path is made absolute here, before alien runs in its own
# scratch directory, where a relative `dist/…` path no longer resolves.
export def alien-args [deb: string]: nothing -> list<string> {
    ["--to-rpm" "--scripts" ($deb | path expand)]
}

# Binaries that open a window or a terminal app, i.e. get a .desktop entry.
export def launchable [bins: list<record>]: nothing -> list<record> { $bins | where kind in [gui tui] }

# Debian runtime dependencies of a kind. A GUI dlopen()s X11, Wayland and GL, so
# one binary serves every session type; only xkbcommon is linked.
export def deb-deps [kind: string]: nothing -> record<depends: string, recommends: string> {
    if $kind == "gui" {
        {depends: "libxkbcommon0", recommends: "libwayland-client0, libx11-6, libvulkan1 | libgl1, fontconfig"}
    } else {
        {depends: "", recommends: ""}
    }
}

# A binary's package name. Debian allows only lowercase letters, digits and `+-.`, so a
# binary like `app_cli` ships as the package `app-cli` (the command keeps its name).
export def deb-name [bin: string]: nothing -> string {
    $bin | str downcase | str replace --all '_' '-'
}

# DEBIAN/control of one binary's package.
export def deb-control [app: record, bin: record, version: string, arch: string]: nothing -> string {
    let deps = deb-deps $bin.kind
    let dep = if ($deps.depends | is-empty) { "" } else { $"Depends: ($deps.depends)\n" }
    let rec = if ($deps.recommends | is-empty) { "" } else { $"Recommends: ($deps.recommends)\n" }
    $"Package: (deb-name $bin.bin)
Version: (plain-version $version)
Architecture: ($arch)
Maintainer: ($app.maintainer)
Section: utils
Priority: optional
Description: ($bin.summary)
 ($app.summary)
Homepage: ($app.homepage)
($dep)($rec)"
}

# The file name of a binary's .desktop entry and metainfo, without extension:
# APP.desktop_ids.<bin> (a reverse-DNS id, as Flathub requires) or the binary name.
export def entry-id [app: record, bin: record]: nothing -> string {
    $app | get -o desktop_ids | default {} | get -o $bin.bin | default $bin.bin
}

# The freedesktop entry of a gui or tui binary.
export def desktop-entry [app: record, bin: record]: nothing -> string {
    let tui = $bin.kind == "tui"
    let name = if $tui { $"($app.title) \(terminal\)" } else { $app.title }
    let categories = ($app.categories ++ (if $tui { [ConsoleOnly] } else { [] }) | str join ";") + ";"
    let wm = if $tui { "" } else { $"StartupWMClass=($bin.bin)\nStartupNotify=true\n" }
    $"[Desktop Entry]
Type=Application
Version=1.0
Name=($name)
Comment=($bin.summary)
Exec=($bin.bin)
Icon=(if $tui { 'utilities-terminal' } else { icon-name $app })
Terminal=($tui)
Categories=($categories)
($wm)"
}

# AppStream metainfo of a gui binary.
export def metainfo [app: record, bin: record]: nothing -> string {
    let cats = $app.categories | each {|c| $"<category>($c)</category>" } | str join ""
    $"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<component type=\"desktop-application\">
  <id>(entry-id $app $bin)</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>($app.license)</project_license>
  <name>($app.title)</name>
  <summary>($bin.summary)</summary>
  <description><p>($app.summary)</p></description>
  <launchable type=\"desktop-id\">(entry-id $app $bin).desktop</launchable>
  <url type=\"homepage\">($app.homepage)</url>
  <provides><binary>($bin.bin)</binary></provides>
  <categories>($cats)</categories>
  <content_rating type=\"oars-1.1\"/>
</component>
"
}

# A committed packaging/linux/<file> when it exists, else the generated text.
def linux-file [file: string, generated: string]: nothing -> string {
    let committed = $"packaging/linux/($file)"
    if ($committed | path exists) { open --raw $committed } else { $generated }
}

# The installed icon's theme name: APP.icon (e.g. a reverse-DNS id) or APP.id.
export def icon-name [app: record]: nothing -> string { $app.icon? | default $app.id }

# The app's icon files that exist (assets/icons/<id>…), as {src, dst} under
# /usr/share/icons, installed under the icon name.
export def icon-files [app: record, existing: list<string>]: nothing -> list<record<src: string, dst: string>> {
    let id = $app.id
    let name = icon-name $app
    let svg = [{src: $"assets/icons/($id).svg", dst: $"usr/share/icons/hicolor/scalable/apps/($name).svg"}]
    let pngs = $ICON_SIZES | each {|n| {src: $"assets/icons/($id)-($n).png", dst: $"usr/share/icons/hicolor/($n)x($n)/apps/($name).png"} }
    $svg ++ $pngs | where src in $existing
}

# The files of one binary's package: {src, dst} copies and {text, dst} writes.
export def package-files [app: record, bin: record, target: string, existing: list<string>]: nothing -> list<record> {
    let exe = [{src: $"target/($target)/release/($bin.bin)", dst: $"usr/bin/($bin.bin)"}]
    let entry = if $bin.kind in [gui tui] {
        [{text: (linux-file $"(entry-id $app $bin).desktop" (desktop-entry $app $bin)), dst: $"usr/share/applications/(entry-id $app $bin).desktop"}]
    } else { [] }
    let gui = if $bin.kind == "gui" {
        [{text: (linux-file $"(entry-id $app $bin).metainfo.xml" (metainfo $app $bin)), dst: $"usr/share/metainfo/(entry-id $app $bin).metainfo.xml"}] ++ (icon-files $app $existing)
    } else { [] }
    $exe ++ $entry ++ $gui
}

# Info.plist of the macOS .app around `exe`.
export def plist [app: record, exe: string, version: string, icon: bool]: nothing -> string {
    let v = plain-version $version
    let icon_key = if $icon { $"    <key>CFBundleIconFile</key><string>($app.id)</string>\n" } else { "" }
    $"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
    <key>CFBundleExecutable</key><string>($exe)</string>
    <key>CFBundleIdentifier</key><string>($app.bundle_id)</string>
    <key>CFBundleName</key><string>($app.title)</string>
    <key>CFBundleDisplayName</key><string>($app.title)</string>
($icon_key)    <key>CFBundleVersion</key><string>($v)</string>
    <key>CFBundleShortVersionString</key><string>($v)</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"
}

export def dmg-name [app: record, version: string]: nothing -> string { $"($app.id)-(plain-version $version)-macos.dmg" }
export def setup-name [app: record, version: string]: nothing -> string { $"($app.id)-(plain-version $version)-windows-x86_64-setup.exe" }
export def appimage-name [bin: string, version: string, arch: string]: nothing -> string { $"($bin)-(plain-version $version)-($arch).AppImage" }

# How to make the universal macOS binary from the per-arch builds that exist.
export def merge-plan [arm: bool, x86: bool]: nothing -> string {
    if $arm and $x86 { "lipo" } else if $arm { "arm64" } else if $x86 { "x86_64" } else { "missing" }
}

# The NSIS installer script. NSIS resolves relative paths against the compiled
# script's folder (dist/), so `!cd` anchors them at the repository root.
export def nsis [app: record, bins: list<record>, version: string, root: string, icon: bool]: nothing -> string {
    let v = plain-version $version
    let exe = {|b| $"target\\($WINDOWS_TARGET)\\release\\($b.bin).exe" }
    let ico = $"packaging\\windows\\($app.id).ico"
    let icons = if $icon { $"!define MUI_ICON \"($ico)\"\n!define MUI_UNICON \"($ico)\"\n" } else { "" }
    let sections = $bins | enumerate | each {|it|
        let b = $it.item
        let ro = if $it.index == 0 { "    SectionIn RO\n" } else { "" }
        let links = if $b.kind == "gui" {
            $"    CreateDirectory \"$SMPROGRAMS\\($app.title)\"\n    CreateShortcut \"$SMPROGRAMS\\($app.title)\\($app.title).lnk\" \"$INSTDIR\\($b.bin).exe\"\n    CreateShortcut \"$DESKTOP\\($app.title).lnk\" \"$INSTDIR\\($b.bin).exe\"\n"
        } else { "" }
        $"Section \"($b.bin)\" Sec($it.index)\n($ro)    SetOutPath \"$INSTDIR\"\n    File \"(do $exe $b)\"\n($links)    WriteRegStr HKLM \"Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\($b.bin).exe\" \"\" \"$INSTDIR\\($b.bin).exe\"\nSectionEnd\n"
    } | str join "\n"
    let removes = $bins | each {|b| $"    Delete \"$INSTDIR\\($b.bin).exe\"\n    DeleteRegKey HKLM \"Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\($b.bin).exe\"\n" } | str join ""
    $"; Generated by scripts/ci/package_desktop.nu — edit APP and BINS in scripts/project.nu.
!cd \"($root)\"
!define PRODUCT_NAME \"($app.title)\"
!define PRODUCT_VERSION \"($v)\"
!define UNINST_KEY \"Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${PRODUCT_NAME}\"
!define PATH_KEY \"SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment\"

Name \"${PRODUCT_NAME} ${PRODUCT_VERSION}\"
OutFile \"dist\\(setup-name $app $version)\"
InstallDir \"$PROGRAMFILES64\\($app.title)\"
InstallDirRegKey HKLM \"${UNINST_KEY}\" \"InstallLocation\"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include \"MUI2.nsh\"
!define MUI_ABORTWARNING
($icons)!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE \"LICENSE\"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE \"English\"

($sections)
Section \"Add to PATH\" SecPath
    ReadRegStr $0 HKLM \"${PATH_KEY}\" \"Path\"
    WriteRegExpandStr HKLM \"${PATH_KEY}\" \"Path\" \"$0;$INSTDIR\"
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 \"STR:Environment\" /TIMEOUT=5000
SectionEnd

Section -PostInstall
    WriteRegStr HKLM \"${UNINST_KEY}\" \"DisplayName\" \"${PRODUCT_NAME} ${PRODUCT_VERSION}\"
    WriteRegStr HKLM \"${UNINST_KEY}\" \"UninstallString\" \"$INSTDIR\\uninstall.exe\"
    WriteRegStr HKLM \"${UNINST_KEY}\" \"InstallLocation\" \"$INSTDIR\"
    WriteRegStr HKLM \"${UNINST_KEY}\" \"Publisher\" \"($app.maintainer | str replace --regex ' <.*>$' '')\"
    WriteRegStr HKLM \"${UNINST_KEY}\" \"URLInfoAbout\" \"($app.homepage)\"
    WriteRegStr HKLM \"${UNINST_KEY}\" \"DisplayVersion\" \"${PRODUCT_VERSION}\"
    WriteRegDWORD HKLM \"${UNINST_KEY}\" \"NoModify\" 1
    WriteRegDWORD HKLM \"${UNINST_KEY}\" \"NoRepair\" 1
    WriteUninstaller \"$INSTDIR\\uninstall.exe\"
SectionEnd

Section Uninstall
($removes)    Delete \"$INSTDIR\\uninstall.exe\"
    Delete \"$SMPROGRAMS\\($app.title)\\($app.title).lnk\"
    Delete \"$DESKTOP\\($app.title).lnk\"
    RMDir \"$SMPROGRAMS\\($app.title)\"
    RMDir \"$INSTDIR\"
($UNPATH)
    DeleteRegKey HKLM \"${UNINST_KEY}\"
SectionEnd
"
}

# The release's source tarball on GitHub.
export def tarball-url [app: record, version: string]: nothing -> string {
    $"($app.homepage)/archive/refs/tags/v(plain-version $version).tar.gz"
}

# A committed PKGBUILD moved to `version` and `sha`.
export def bump-pkgbuild [text: string, version: string, sha: string]: nothing -> string {
    $text
    | str replace --regex '(?m)^pkgver=.*$' $"pkgver=(plain-version $version)"
    | str replace --regex '(?m)^pkgrel=.*$' 'pkgrel=1'
    | str replace --regex "sha256sums=\\('[^']*'\\)" $"sha256sums=\('($sha)')"
}

# An AUR PKGBUILD that builds every binary from the release's source tarball.
# Generated .desktop/metainfo files are written from heredocs.
export def pkgbuild [app: record, bins: list<record>, version: string, sha: string, existing: list<string>]: nothing -> string {
    let gui = $bins | any {|b| $b.kind == "gui" }
    let depends = if $gui { "'gcc-libs' 'libxkbcommon' 'wayland' 'libx11' 'vulkan-icd-loader' 'fontconfig'" } else { "'gcc-libs'" }
    let crates = $bins | get crate | uniq | each {|c| $"--package ($c)" } | str join " "
    let installs = $bins | each {|b|
        let exe = [$"    install -Dm755 \"target/release/($b.bin)\" \"$pkgdir/usr/bin/($b.bin)\""]
        let entry = if $b.kind in [gui tui] {
            [$"    install -Dm644 /dev/stdin \"$pkgdir/usr/share/applications/(entry-id $app $b).desktop\" <<'EOF'\n(linux-file $"(entry-id $app $b).desktop" (desktop-entry $app $b) | str trim)\nEOF"]
        } else { [] }
        let meta = if $b.kind == "gui" {
            [$"    install -Dm644 /dev/stdin \"$pkgdir/usr/share/metainfo/(entry-id $app $b).metainfo.xml\" <<'EOF'\n(linux-file $"(entry-id $app $b).metainfo.xml" (metainfo $app $b) | str trim)\nEOF"] ++ (icon-files $app $existing | each {|i| $"    install -Dm644 \"($i.src)\" \"$pkgdir/($i.dst)\"" })
        } else { [] }
        $exe ++ $entry ++ $meta
    } | flatten | str join "\n"
    $"# Maintainer: ($app.maintainer)
# Generated by scripts/ci/package_desktop.nu.
pkgname=($app.id)
pkgver=(plain-version $version)
pkgrel=1
pkgdesc=\"($app.summary)\"
arch=\('x86_64' 'aarch64')
url=\"($app.homepage)\"
license=\('($app.license)')
depends=\(($depends))
makedepends=\('cargo')
source=\(\"$pkgname-$pkgver.tar.gz::(tarball-url $app $version | str replace (plain-version $version) '$pkgver')\")
sha256sums=\('($sha)')

prepare\() {
    cd \"$pkgname-$pkgver\"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked
}

build\() {
    cd \"$pkgname-$pkgver\"
    export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=target
    cargo build --frozen --release ($crates)
}

package\() {
    cd \"$pkgname-$pkgver\"
($installs)
    install -Dm644 LICENSE \"$pkgdir/usr/share/licenses/$pkgname/LICENSE\"
}
"
}

# Files under the repository that the generators look for.
def existing-files []: nothing -> list<string> {
    glob assets/icons/* | each {|p| $p | path relative-to $env.PWD }
}

def built [bin: string, target: string]: nothing -> string {
    let exe = if ($target | str contains windows) { $"($bin).exe" } else { $bin }
    $"target/($target)/release/($exe)"
}

def fail [msg: string] { print -e $"(ansi red)❌ ($msg)(ansi reset)"; exit 1 }

# The packager installs for a platform (CI); appimagetool is fetched on demand.
def "main tools" [platform: string] {
    match $platform {
        "linux" => {
            let root = (^id -u | str trim) == "0"
            for c in (commands [dpkg-dev rpm alien fakeroot file] $root) { run-external $c.0 ...($c | skip 1) }
        }
        "macos" => { ^brew install create-dmg }
        "windows" => { ^choco install nsis -y --no-progress }
        _ => { fail $"unknown platform: ($platform) \(linux, macos, windows\)" }
    }
    print $"tools=($platform)"
}

def stage [root: string, files: list<record>] {
    for f in $files {
        let dst = $root | path join $f.dst
        mkdir ($dst | path dirname)
        if ($f.text? != null) { $f.text | save --force $dst } else if ($f.src | path type) == "dir" { cp -r $f.src $dst } else { cp $f.src $dst }
    }
}

# .deb and .rpm (via alien) for every binary, plus AppImages with --appimage.
def "main linux" [version: string, target: string, --appimage] {
    let v = plain-version $version
    let existing = existing-files
    mkdir dist
    for b in (built_for $BINS $target) {
        if not ((built $b.bin $target) | path exists) { fail $"(built $b.bin $target) missing: build first" }
        let root = $"dist/deb-($b.bin)"
        rm -rf $root
        mkdir $"($root)/DEBIAN"
        stage $root (package-files $APP $b $target $existing)
        ^chmod 755 $"($root)/usr/bin/($b.bin)"
        let pkg = deb-name $b.bin
        mkdir $"($root)/usr/share/doc/($pkg)"
        cp LICENSE $"($root)/usr/share/doc/($pkg)/copyright"
        deb-control $APP $b $version (deb-arch $target) | save --force $"($root)/DEBIAN/control"
        let deb = $"dist/($pkg)_($v)_(deb-arch $target).deb"
        ^dpkg-deb --root-owner-group --build $root $deb
        # alien writes into the working directory: give each conversion its own.
        let scratch = $"dist/rpm-($b.bin)"
        rm -rf $scratch
        mkdir $scratch
        let args = alien-args $deb
        do { cd $scratch; ^fakeroot alien ...$args }
        let rpm = glob $"($scratch)/*.rpm" | first
        mv $rpm $"dist/($pkg)-($v)-(rpm-arch $target).rpm"
        rm -rf $root $scratch
        print -e $"✅ ($deb) + .rpm"
    }
    if $appimage { appimages $version $target $existing }
}

# The path of an installed program, or null. `which` also lists custom commands (this
# script defines `appimagetool` itself), so only external programs count.
export def on-path [name: string]: nothing -> any {
    which $name | where type == external | get -o 0.path
}

def appimagetool []: nothing -> string {
    let installed = on-path appimagetool
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

def appimages [version: string, target: string, existing: list<string>] {
    let apps = launchable (built_for $BINS $target)
    let icon = $"assets/icons/($APP.id)-256.png"
    if ($apps | is-empty) { print -e "ℹ️  no gui or tui binary: no AppImage"; return }
    if $icon not-in $existing { print -e $"⚠️  ($icon) missing: the AppImages get a blank icon" }
    let arch = rpm-arch $target
    for b in $apps {
        let dir = $"dist/AppDir-($b.bin)"
        rm -rf $dir
        stage $dir [{src: (built $b.bin $target), dst: $"usr/bin/($b.bin)"}]
        let entry = linux-file $"(entry-id $APP $b).desktop" (desktop-entry $APP $b) | str replace --regex '(?m)^Icon=.*$' $"Icon=(icon-name $APP)"
        $entry | save --force $"($dir)/(entry-id $APP $b).desktop"
        if $icon in $existing { cp $icon $"($dir)/(icon-name $APP).png" } else { $PLACEHOLDER_ICON | save --force $"($dir)/(icon-name $APP).png" }
        $"#!/bin/sh\nexec \"$\(dirname \"$\(readlink -f \"$0\")\")/usr/bin/($b.bin)\" \"$@\"\n" | save --force $"($dir)/AppRun"
        ^chmod +x $"($dir)/AppRun"
        let out = $"dist/(appimage-name $b.bin $version $arch)"
        # Hosted runners have no FUSE; the tool extracts itself instead.
        with-env {ARCH: $arch, APPIMAGE_EXTRACT_AND_RUN: "1"} { run-external (appimagetool) $dir $out }
        rm -rf $dir
        print -e $"✅ ($out)"
    }
}

# The universal DMG: the first gui binary as the .app, the rest as command line tools.
def "main macos" [version: string] {
    let uni = "target/universal-apple-darwin/release"
    let bins = built_for $BINS aarch64-apple-darwin
    mkdir $uni
    for b in $bins {
        let arm = built $b.bin "aarch64-apple-darwin"
        let x86 = built $b.bin "x86_64-apple-darwin"
        match (merge-plan ($arm | path exists) ($x86 | path exists)) {
            "lipo" => { ^lipo -create $arm $x86 -output $"($uni)/($b.bin)" }
            "arm64" => { cp $arm $uni }
            "x86_64" => { cp $x86 $uni }
            _ => { fail $"($b.bin) was not built for macOS" }
        }
    }
    let root = "dist/dmg-root"
    rm -rf $root
    let app_bin = $bins | where kind == gui | get -o 0
    let icns = $"packaging/macos/($APP.id).icns"
    if $app_bin != null {
        let bundle = $"($root)/($APP.title).app/Contents"
        mkdir $"($bundle)/MacOS" $"($bundle)/Resources"
        cp $"($uni)/($app_bin.bin)" $"($bundle)/MacOS/"
        if ($icns | path exists) { cp $icns $"($bundle)/Resources/" }
        plist $APP $app_bin.bin $version ($icns | path exists) | save --force $"($bundle)/Info.plist"
        # Ad-hoc signature: Apple silicon refuses unsigned arm64 code.
        ^codesign --force --deep --sign - $"($root)/($APP.title).app"
    }
    let tools = $bins | where {|b| $b.bin != $app_bin.bin? }
    if ($tools | is-not-empty) {
        let cli = $"($root)/Command Line Tools"
        mkdir $cli
        for b in $tools { cp $"($uni)/($b.bin)" $cli }
        let names = $tools | get bin | str join " "
        $"Copy these to a folder on your PATH:\n\n    sudo cp ($names) /usr/local/bin/\n\nIf macOS blocks them:\n\n    xattr -dr com.apple.quarantine ($names)\n" | save --force $"($cli)/README.txt"
    }
    let out = $"dist/(dmg-name $APP $version)"
    rm -f $out
    ^ln -sf /Applications $"($root)/Applications"
    ^hdiutil create -volname $"($APP.title) (plain-version $version)" -srcfolder $root -ov -format UDZO $out
    rm -rf $root
    print -e $"✅ ($out)"
}

def "main windows" [version: string] {
    let icon = $"packaging/windows/($APP.id).ico" | path exists
    mkdir dist
    nsis $APP (built_for $BINS x86_64-pc-windows-msvc) $version ($env.PWD | path expand) $icon | save --force dist/installer.nsi
    let makensis = if (which makensis | is-not-empty) { "makensis" } else { 'C:\Program Files (x86)\NSIS\makensis.exe' }
    run-external $makensis dist/installer.nsi
    print -e $"✅ dist/(setup-name $APP $version)"
}

def "main aur" [version: string] {
    let sha = http get --raw (tarball-url $APP $version) | hash sha256
    let committed = "packaging/aur/PKGBUILD"
    mkdir dist/aur
    let text = if ($committed | path exists) {
        bump-pkgbuild (open --raw $committed) $version $sha
    } else {
        pkgbuild $APP (built_for $BINS x86_64-unknown-linux-gnu) $version $sha (existing-files)
    }
    $text | save --force dist/aur/PKGBUILD
    print "pkgbuild=dist/aur/PKGBUILD"
}

def main [] {
    print $"app=($APP.id)"
    print $"binaries=($BINS | each {|b| $'($b.bin):($b.kind)' } | str join ',')"
}
