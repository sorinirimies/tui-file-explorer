#!/usr/bin/env nu
# ── tui-file-explorer · test_package_binaries.nu ─────────────────────────────
# Names and file contents of the release archives and Linux packages (no cargo,
# no packagers).

use std/assert
use runner.nu *
use ../ci/package_binaries.nu [asset_name binary_name checksum_line]
use ../ci/package_linux.nu [deb_name rpm_name appimage_name deb_arch rpm_arch deb_control desktop_entry alien_args]
use ../ci/apt.nu [commands]

def "test package_binaries: unix archive is tar.gz named by target" [] {
    assert equal (asset_name "aarch64-apple-darwin") "tfe-aarch64-apple-darwin.tar.gz"
    assert equal (binary_name "aarch64-apple-darwin") "tfe"
}

def "test package_binaries: windows archive is zip with an exe" [] {
    assert equal (asset_name "x86_64-pc-windows-msvc") "tfe-x86_64-pc-windows-msvc.zip"
    assert equal (binary_name "x86_64-pc-windows-msvc") "tfe.exe"
}

def "test package_binaries: checksum line is sha256sum style" [] {
    assert equal (checksum_line "abc123" "tfe-x.tar.gz") "abc123  tfe-x.tar.gz\n"
}

def "test package_linux: architectures follow the target" [] {
    assert equal (deb_arch "x86_64-unknown-linux-gnu") "amd64"
    assert equal (deb_arch "aarch64-unknown-linux-gnu") "arm64"
    assert equal (rpm_arch "x86_64-unknown-linux-gnu") "x86_64"
    assert equal (rpm_arch "aarch64-unknown-linux-gnu") "aarch64"
}

def "test package_linux: file names carry the plain version" [] {
    assert equal (deb_name "v4.0.3" "x86_64-unknown-linux-gnu") "tfe_4.0.3_amd64.deb"
    assert equal (rpm_name "4.0.3" "aarch64-unknown-linux-gnu") "tfe-4.0.3-aarch64.rpm"
    assert equal (appimage_name "4.0.3" "x86_64-unknown-linux-gnu") "tfe-4.0.3-x86_64.AppImage"
}

def "test package_linux: deb control names package, version and arch" [] {
    let c = deb_control "v4.0.3" "arm64"
    assert ($c | str contains "Package: tfe\n")
    assert ($c | str contains "Version: 4.0.3\n")
    assert ($c | str contains "Architecture: arm64\n")
}

def "test package_linux: desktop entry is a terminal app" [] {
    let d = desktop_entry
    assert ($d | str contains "Exec=tfe\n")
    assert ($d | str contains "Terminal=true\n")
}

def "test package_linux: alien gets an absolute deb path" [] {
    let a = alien_args "dist/x.deb"
    assert ($a | last | str starts-with "/")
}

def "test apt: sudo unless root" [] {
    assert equal (commands [] false) []
    assert equal (commands [rpm] true | last) [apt-get -o Acquire::Retries=5 install -y --no-install-recommends rpm]
    assert equal (commands [rpm] false | first | first) "sudo"
}

def main [] { run-tests }
