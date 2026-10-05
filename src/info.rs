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

/// Print version, platform, environment, and config info to stderr, then return.
///
/// Diagnostic output goes to stderr so the shell wrapper (which captures
/// stdout for cd-on-exit) does not try to `cd` into each info line.
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

    // Environment — include platform-relevant variables.
    let env_keys: Vec<&str> = if cfg!(windows) {
        vec![
            "USERPROFILE",
            "APPDATA",
            "PSModulePath",
            "CARGO_HOME",
            "TERM",
            "TERM_PROGRAM",
            "NU_VERSION",
        ]
    } else {
        vec![
            "HOME",
            "SHELL",
            "TERM",
            "TERM_PROGRAM",
            "XDG_CONFIG_HOME",
            "ZDOTDIR",
            "NU_VERSION",
            "CARGO_HOME",
        ]
    };
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
    let sp_exists = sp.as_deref().is_some_and(std::path::Path::exists);
    r.section_config(sp.as_deref(), sp_exists);
    match tui_file_explorer::keymap_path() {
        Some(p) => {
            r.field("keymap file", &p.display().to_string());
            r.field("keymap exists", if p.exists() { "true" } else { "false" });
        }
        None => r.field("keymap file", "(could not determine)"),
    }

    // Print everything to stderr — stdout is reserved for paths.
    let _ = r.write_to(&mut io::stderr().lock());
}

#[cfg(test)]
mod tests;
