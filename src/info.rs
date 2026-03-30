//! `--info` diagnostic helper for the `tfe` binary.
//!
//! The public entry point ([`print_info`]) gathers real environment state and
//! delegates to [`InfoReport`] methods that accept explicit parameters.  This
//! makes every section independently testable without mutating global state.

use std::io::{self, Write};
use std::path::Path;

use crate::shell_init;

// ── InfoReport ────────────────────────────────────────────────────────────────

/// Accumulates info output lines.
///
/// All `section_*` methods push lines into an internal buffer rather than
/// printing directly, so tests can inspect the output without capturing
/// stdout.
pub(crate) struct InfoReport {
    pub lines: Vec<String>,
}

impl InfoReport {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    // ── Output helpers ────────────────────────────────────────────────────

    fn heading(&mut self, title: &str) {
        self.lines.push(title.to_string());
    }

    fn field(&mut self, label: &str, value: &str) {
        self.lines.push(format!("  {label:<17}: {value}"));
    }

    fn blank(&mut self) {
        self.lines.push(String::new());
    }

    /// Write all accumulated lines to `w`.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        for line in &self.lines {
            writeln!(w, "{line}")?;
        }
        Ok(())
    }

    /// Return all accumulated lines joined with newlines (handy for tests).
    #[cfg(test)]
    pub fn output(&self) -> String {
        self.lines.join("\n")
    }

    // ── Sections ──────────────────────────────────────────────────────────

    /// Header with version string.
    pub fn section_version(&mut self, version: &str) {
        self.lines.push(format!("tfe {version}"));
        self.blank();
    }

    /// Platform: os, arch, family.
    pub fn section_platform(&mut self, os: &str, arch: &str, family: &str) {
        self.heading("Platform");
        self.field("os", os);
        self.field("arch", arch);
        self.field("family", family);
        self.blank();
    }

    /// Environment variables — each entry is `(key, value_or_not_set)`.
    pub fn section_environment(&mut self, vars: &[(&str, Option<&str>)]) {
        self.heading("Environment");
        for (key, val) in vars {
            let display = val.unwrap_or("(not set)");
            self.lines.push(format!("  ${key:<17}: {display}"));
        }
        self.blank();
    }

    /// Binary: executable path and current working directory.
    pub fn section_binary(&mut self, exe_path: Option<&Path>, cwd: Option<&Path>) {
        self.heading("Binary");
        match exe_path {
            Some(p) => self.field("executable", &p.display().to_string()),
            None => self.field("executable", "(unknown)"),
        }
        match cwd {
            Some(p) => self.field("cwd", &p.display().to_string()),
            None => self.field("cwd", "(unknown)"),
        }
        self.blank();
    }

    /// Terminal: size and stderr tty status.
    pub fn section_terminal(&mut self, size: Option<(u16, u16)>, stderr_is_tty: bool) {
        self.heading("Terminal");
        match size {
            Some((cols, rows)) => self.field("size", &format!("{cols} x {rows}")),
            None => self.field("size", "(unknown)"),
        }
        self.field(
            "stderr is tty",
            if stderr_is_tty { "true" } else { "false" },
        );
        self.blank();
    }

    /// Shell detection result.
    pub fn section_shell(&mut self, detected: Option<&str>) {
        self.heading("Shell detection");
        match detected {
            Some(s) => self.field("detected", s),
            None => self.field("detected", "(unknown)"),
        }
        self.blank();
    }

    /// Config: state file path and whether it exists.
    pub fn section_config(&mut self, state_path: Option<&Path>, state_exists: bool) {
        self.heading("Config");
        match state_path {
            Some(p) => {
                self.field("state file", &p.display().to_string());
                self.field("exists", if state_exists { "true" } else { "false" });
            }
            None => self.field("state file", "(could not determine)"),
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Print version, platform, environment, and config info then return.
///
/// This is a lightweight dump (no pass/warn/fail assessment).  Use
/// `--doctor` for the full diagnostic report.
pub fn print_info() {
    let mut r = InfoReport::new();

    // Version
    r.section_version(env!("CARGO_PKG_VERSION"));

    // Platform
    r.section_platform(
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY,
    );

    // Environment
    let env_keys = [
        "HOME",
        "SHELL",
        "TERM",
        "TERM_PROGRAM",
        "XDG_CONFIG_HOME",
        "ZDOTDIR",
        "NU_VERSION",
    ];
    let env_values: Vec<Option<String>> = env_keys.iter().map(|k| std::env::var(k).ok()).collect();
    let vars: Vec<(&str, Option<&str>)> = env_keys
        .iter()
        .zip(env_values.iter())
        .map(|(k, v)| (*k, v.as_deref()))
        .collect();
    r.section_environment(&vars);

    // Binary
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    r.section_binary(exe.as_deref(), cwd.as_deref());

    // Terminal
    let size = crossterm::terminal::size().ok();
    let stderr_tty = crossterm::tty::IsTty::is_tty(&io::stderr());
    r.section_terminal(size, stderr_tty);

    // Shell
    let shell = shell_init::detect_shell();
    let shell_name = shell.map(|s| s.name().to_string());
    r.section_shell(shell_name.as_deref());

    // Config
    let sp = tui_file_explorer::persistence::state_path();
    let sp_exists = sp.as_deref().map(|p| p.exists()).unwrap_or(false);
    r.section_config(sp.as_deref(), sp_exists);

    // Print everything
    let _ = r.write_to(&mut io::stdout().lock());
}

#[cfg(test)]
mod tests {
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
}
