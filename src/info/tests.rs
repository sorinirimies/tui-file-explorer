//! Unit tests for [`super`].

use super::*;
use std::path::PathBuf;

// ── InfoReport output helpers ─────────────────────────────────────────

#[test]
fn new_report_starts_empty() {
    let r = InfoReport::new();
    assert!(r.lines.is_empty());
}

#[test]
fn write_to_produces_newline_terminated_lines() {
    let mut r = InfoReport::new();
    r.lines.push("hello".into());
    r.lines.push("world".into());
    let mut buf = Vec::new();
    r.write_to(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "hello\nworld\n");
}

#[test]
fn output_joins_with_newlines() {
    let mut r = InfoReport::new();
    r.lines.push("a".into());
    r.lines.push("b".into());
    assert_eq!(r.output(), "a\nb");
}

// ── section_version ───────────────────────────────────────────────────

#[test]
fn section_version_shows_tfe_prefix() {
    let mut r = InfoReport::new();
    r.section_version("1.2.3");
    let out = r.output();
    assert!(out.contains("tfe 1.2.3"));
}

#[test]
fn section_version_followed_by_blank_line() {
    let mut r = InfoReport::new();
    r.section_version("0.9.1");
    assert_eq!(r.lines.len(), 2);
    assert_eq!(r.lines[1], "");
}

// ── section_platform ──────────────────────────────────────────────────

#[test]
fn section_platform_shows_heading() {
    let mut r = InfoReport::new();
    r.section_platform("macos", "aarch64", "unix");
    let out = r.output();
    assert!(out.contains("Platform"));
}

#[test]
fn section_platform_shows_os_arch_family() {
    let mut r = InfoReport::new();
    r.section_platform("linux", "x86_64", "unix");
    let out = r.output();
    assert!(out.contains("linux"));
    assert!(out.contains("x86_64"));
    assert!(out.contains("unix"));
}

#[test]
fn section_platform_ends_with_blank() {
    let mut r = InfoReport::new();
    r.section_platform("macos", "aarch64", "unix");
    assert_eq!(r.lines.last().unwrap(), "");
}

// ── section_environment ───────────────────────────────────────────────

#[test]
fn section_environment_shows_heading() {
    let mut r = InfoReport::new();
    r.section_environment(&[]);
    assert!(r.output().contains("Environment"));
}

#[test]
fn section_environment_shows_set_vars() {
    let mut r = InfoReport::new();
    r.section_environment(&[("HOME", Some("/home/user")), ("SHELL", Some("/bin/zsh"))]);
    let out = r.output();
    assert!(out.contains("$HOME"));
    assert!(out.contains("/home/user"));
    assert!(out.contains("$SHELL"));
    assert!(out.contains("/bin/zsh"));
}

#[test]
fn section_environment_shows_not_set_for_missing() {
    let mut r = InfoReport::new();
    r.section_environment(&[("ZDOTDIR", None)]);
    let out = r.output();
    assert!(out.contains("$ZDOTDIR"));
    assert!(out.contains("(not set)"));
}

#[test]
fn section_environment_ends_with_blank() {
    let mut r = InfoReport::new();
    r.section_environment(&[("HOME", Some("/root"))]);
    assert_eq!(r.lines.last().unwrap(), "");
}

// ── section_binary ────────────────────────────────────────────────────

#[test]
fn section_binary_shows_heading() {
    let mut r = InfoReport::new();
    r.section_binary(None, None);
    assert!(r.output().contains("Binary"));
}

#[test]
fn section_binary_shows_exe_and_cwd() {
    let mut r = InfoReport::new();
    let exe = PathBuf::from("/usr/local/bin/tfe");
    let cwd = PathBuf::from("/home/user/projects");
    r.section_binary(Some(&exe), Some(&cwd));
    let out = r.output();
    assert!(out.contains("/usr/local/bin/tfe"));
    assert!(out.contains("/home/user/projects"));
}

#[test]
fn section_binary_unknown_when_missing() {
    let mut r = InfoReport::new();
    r.section_binary(None, None);
    let out = r.output();
    assert!(out.contains("(unknown)"));
}

#[test]
fn section_binary_ends_with_blank() {
    let mut r = InfoReport::new();
    r.section_binary(None, None);
    assert_eq!(r.lines.last().unwrap(), "");
}

// ── section_terminal ──────────────────────────────────────────────────

#[test]
fn section_terminal_shows_heading() {
    let mut r = InfoReport::new();
    r.section_terminal(None, false);
    assert!(r.output().contains("Terminal"));
}

#[test]
fn section_terminal_shows_size() {
    let mut r = InfoReport::new();
    r.section_terminal(Some((120, 40)), true);
    let out = r.output();
    assert!(out.contains("120 x 40"));
}

#[test]
fn section_terminal_unknown_size() {
    let mut r = InfoReport::new();
    r.section_terminal(None, true);
    assert!(r.output().contains("(unknown)"));
}

#[test]
fn section_terminal_tty_true() {
    let mut r = InfoReport::new();
    r.section_terminal(Some((80, 24)), true);
    assert!(r.output().contains("true"));
}

#[test]
fn section_terminal_tty_false() {
    let mut r = InfoReport::new();
    r.section_terminal(Some((80, 24)), false);
    assert!(r.output().contains("false"));
}

#[test]
fn section_terminal_ends_with_blank() {
    let mut r = InfoReport::new();
    r.section_terminal(Some((80, 24)), true);
    assert_eq!(r.lines.last().unwrap(), "");
}

// ── section_shell ─────────────────────────────────────────────────────

#[test]
fn section_shell_shows_heading() {
    let mut r = InfoReport::new();
    r.section_shell(None);
    assert!(r.output().contains("Shell detection"));
}

#[test]
fn section_shell_detected() {
    let mut r = InfoReport::new();
    r.section_shell(Some("zsh"));
    assert!(r.output().contains("zsh"));
}

#[test]
fn section_shell_not_detected() {
    let mut r = InfoReport::new();
    r.section_shell(None);
    assert!(r.output().contains("(unknown)"));
}

// ── section_config ────────────────────────────────────────────────────

#[test]
fn section_config_shows_heading() {
    let mut r = InfoReport::new();
    r.section_config(None, false);
    assert!(r.output().contains("Config"));
}

#[test]
fn section_config_shows_path_and_exists() {
    let mut r = InfoReport::new();
    let sp = PathBuf::from("/home/user/.config/tfe/state");
    r.section_config(Some(&sp), true);
    let out = r.output();
    assert!(out.contains("/home/user/.config/tfe/state"));
    assert!(out.contains("true"));
}

#[test]
fn section_config_shows_path_not_exists() {
    let mut r = InfoReport::new();
    let sp = PathBuf::from("/home/user/.config/tfe/state");
    r.section_config(Some(&sp), false);
    let out = r.output();
    assert!(out.contains("/home/user/.config/tfe/state"));
    assert!(out.contains("false"));
}

#[test]
fn section_config_no_path() {
    let mut r = InfoReport::new();
    r.section_config(None, false);
    assert!(r.output().contains("(could not determine)"));
}

// ── Integration ───────────────────────────────────────────────────────

#[test]
fn full_info_report_contains_all_sections() {
    let mut r = InfoReport::new();
    let exe = PathBuf::from("/usr/local/bin/tfe");
    let cwd = PathBuf::from("/home/user");
    let sp = PathBuf::from("/home/user/.config/tfe/state");

    r.section_version("0.9.1");
    r.section_platform("macos", "aarch64", "unix");
    r.section_environment(&[
        ("HOME", Some("/home/user")),
        ("SHELL", Some("/bin/zsh")),
        ("TERM", Some("xterm-256color")),
        ("TERM_PROGRAM", None),
        ("XDG_CONFIG_HOME", None),
        ("ZDOTDIR", None),
        ("NU_VERSION", None),
    ]);
    r.section_binary(Some(&exe), Some(&cwd));
    r.section_terminal(Some((120, 40)), true);
    r.section_shell(Some("zsh"));
    r.section_config(Some(&sp), true);

    let out = r.output();
    assert!(out.contains("tfe 0.9.1"));
    assert!(out.contains("Platform"));
    assert!(out.contains("Environment"));
    assert!(out.contains("Binary"));
    assert!(out.contains("Terminal"));
    assert!(out.contains("Shell detection"));
    assert!(out.contains("Config"));
}

#[test]
fn full_info_report_fresh_install() {
    let mut r = InfoReport::new();
    let sp = PathBuf::from("/home/user/.config/tfe/state");

    r.section_version("0.9.1");
    r.section_platform("linux", "x86_64", "unix");
    r.section_environment(&[("HOME", Some("/home/user")), ("SHELL", None)]);
    r.section_binary(None, None);
    r.section_terminal(None, false);
    r.section_shell(None);
    r.section_config(Some(&sp), false);

    let out = r.output();
    assert!(out.contains("tfe 0.9.1"));
    assert!(out.contains("(not set)"));
    assert!(out.contains("(unknown)"));
    assert!(out.contains("false"));
}
