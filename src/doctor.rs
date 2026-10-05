//! `--doctor` diagnostic helper for the `tfe` binary.
//!
//! The public entry point ([`run_doctor`]) gathers real environment state
//! and delegates to [`DoctorReport`] methods that accept explicit
//! parameters.  This makes every check independently testable without
//! mutating global state.
//!
//! See also: `info.rs` for the lightweight `--info` dump.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::shell_init;

// ── DoctorReport ──────────────────────────────────────────────────────────────

/// Accumulates diagnostic output lines and pass / warn / fail counters.
///
/// All `check_*` methods push lines into an internal buffer rather than
/// printing directly, so tests can inspect the output without capturing
/// stdout.
pub(crate) struct DoctorReport {
    pub lines: Vec<String>,
    pub oks: u32,
    pub warnings: u32,
    pub failures: u32,
}

impl DoctorReport {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            oks: 0,
            warnings: 0,
            failures: 0,
        }
    }

    // ── Output helpers ────────────────────────────────────────────────────

    pub fn heading(&mut self, title: &str) {
        self.lines.push(title.to_string());
    }

    pub fn blank(&mut self) {
        self.lines.push(String::new());
    }

    pub fn pass(&mut self, msg: &str) {
        self.lines.push(format!("  [ok]   {msg}"));
        self.oks += 1;
    }

    pub fn warn(&mut self, msg: &str, hint: &str) {
        self.lines.push(format!("  [warn] {msg}"));
        if !hint.is_empty() {
            self.lines.push(format!("         -> {hint}"));
        }
        self.warnings += 1;
    }

    pub fn fail(&mut self, msg: &str, hint: &str) {
        self.lines.push(format!("  [FAIL] {msg}"));
        if !hint.is_empty() {
            self.lines.push(format!("         -> {hint}"));
        }
        self.failures += 1;
    }

    pub fn detail(&mut self, msg: &str) {
        self.lines.push(format!("         {msg}"));
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

    // ── Individual checks ─────────────────────────────────────────────────

    /// Check 1: Platform — always passes, just records os / arch / family.
    pub fn check_platform(&mut self, os: &str, arch: &str, family: &str) {
        self.heading("Platform");
        self.pass(&format!("{os} {arch} ({family})"));
        self.blank();
    }

    /// Check 2: Binary location and whether `~/.cargo/bin` is on `$PATH`.
    pub fn check_binary(
        &mut self,
        exe_path: Option<&Path>,
        cargo_bin: Option<&Path>,
        path_dirs: &[PathBuf],
    ) {
        self.heading("Binary");
        match exe_path {
            Some(p) => self.pass(&format!("executable at {}", p.display())),
            None => self.warn("could not resolve executable path", ""),
        }
        if let Some(cb) = cargo_bin {
            if path_dirs.iter().any(|d| d == cb) {
                self.pass(&format!("{} is on $PATH", cb.display()));
            } else {
                let hint = if cfg!(windows) {
                    "ensure the Rust/Cargo bin directory is in your PATH"
                } else {
                    r#"add to your shell rc:  export PATH="$HOME/.cargo/bin:$PATH""#
                };
                self.warn(&format!("{} is NOT on $PATH", cb.display()), hint);
            }
        }
        self.blank();
    }

    /// Check 3: Essential environment variables and working directory.
    pub fn check_environment(
        &mut self,
        home: Option<&str>,
        shell_var: Option<&str>,
        cwd: Option<&Path>,
    ) {
        self.heading("Environment");
        match home {
            Some(h) => self.pass(&format!("home directory = {h}")),
            None => self.fail(
                "home directory is not set ($HOME / %USERPROFILE%)",
                "many features depend on a known home directory",
            ),
        }
        match shell_var {
            Some(s) => self.pass(&format!("shell = {s}")),
            None => {
                if cfg!(windows) {
                    // On Windows $SHELL is never set; detection uses $PSModulePath.
                    self.pass(
                        "$SHELL not set (normal on Windows; PowerShell detected via $PSModulePath)",
                    );
                } else {
                    self.warn(
                        "$SHELL is not set",
                        "shell auto-detection will not work; use: tfe --init <shell>",
                    );
                }
            }
        }
        match cwd {
            Some(d) => self.pass(&format!("working directory: {}", d.display())),
            None => self.fail(
                "cannot read working directory",
                "tfe defaults to cwd on startup; it will fall back to /",
            ),
        }
        self.blank();
    }

    /// Check 4: Terminal size and stderr tty status.
    pub fn check_terminal(&mut self, size: Option<(u16, u16)>, stderr_is_tty: bool) {
        self.heading("Terminal");
        match size {
            Some((cols, rows)) if cols >= 40 && rows >= 10 => {
                self.pass(&format!("size {cols} x {rows}"));
            }
            Some((cols, rows)) => {
                self.warn(
                    &format!("size {cols} x {rows} is very small"),
                    "tfe needs at least ~40 x 10 to render properly",
                );
            }
            None => {
                self.fail(
                    "could not query terminal size",
                    "raw-mode / alternate-screen may not work in this terminal",
                );
            }
        }
        if stderr_is_tty {
            self.pass("stderr is a tty (TUI will render correctly)");
        } else {
            self.warn(
                "stderr is NOT a tty",
                "the TUI renders on stderr; if it is redirected the UI will be invisible",
            );
        }
        self.blank();
    }

    /// Check 5: Shell detection, rc file existence, wrapper installation.
    pub fn check_shell_integration(
        &mut self,
        detected_shell: Option<shell_init::Shell>,
        rc_path: Option<&Path>,
        rc_exists: bool,
        wrapper_installed: bool,
    ) {
        self.heading("Shell integration (cd-on-exit)");
        match detected_shell {
            Some(shell) => {
                self.pass(&format!("detected shell: {shell}"));
                match rc_path {
                    Some(path) => {
                        self.pass(&format!("rc file: {}", path.display()));
                        if rc_exists {
                            if wrapper_installed {
                                self.pass("shell wrapper is installed");
                            } else {
                                self.fail(
                                    "shell wrapper NOT found in rc file",
                                    "run:  tfe --init  (or just launch tfe once to auto-install)",
                                );
                            }
                        } else {
                            self.warn(
                                &format!("rc file does not exist yet: {}", path.display()),
                                "it will be created automatically on first tfe launch",
                            );
                        }
                    }
                    None => self.fail(
                        "could not determine rc file path",
                        "is $HOME set? try: tfe --init <shell>",
                    ),
                }
            }
            None => self.warn(
                "could not detect shell from $SHELL / $NU_VERSION",
                "run: tfe --init bash/zsh/fish/powershell/nushell",
            ),
        }
        self.blank();
    }

    /// Check 6: Config directory and persisted application state.
    pub fn check_config(
        &mut self,
        state_path: Option<&Path>,
        state_exists: bool,
        state: Option<&tui_file_explorer::AppState>,
    ) {
        self.heading("Config & persisted state");
        match state_path {
            Some(path) => {
                self.pass(&format!("state file: {}", path.display()));
                if state_exists {
                    self.pass("state file exists (not a fresh install)");
                    if let Some(s) = state {
                        self.detail(&format!(
                            "theme         = {}",
                            s.theme.as_deref().unwrap_or("(default)")
                        ));
                        self.detail(&format!(
                            "last_dir      = {}",
                            s.last_dir
                                .as_ref()
                                .map_or_else(|| "(none)".into(), |p| p.display().to_string())
                        ));
                        self.detail(&format!(
                            "last_dir_right= {}",
                            s.last_dir_right
                                .as_ref()
                                .map_or_else(|| "(none)".into(), |p| p.display().to_string())
                        ));
                        self.detail(&format!(
                            "sort_mode     = {}",
                            s.sort_mode
                                .map_or_else(|| "(default)".into(), |m| m.label().to_string())
                        ));
                        self.detail(&format!(
                            "show_hidden   = {}",
                            s.show_hidden
                                .map_or_else(|| "(default)".into(), |b| b.to_string())
                        ));
                        self.detail(&format!(
                            "cd_on_exit    = {}",
                            s.cd_on_exit
                                .map_or_else(|| "(default -> true)".into(), |b| b.to_string())
                        ));
                        self.detail(&format!(
                            "editor        = {}",
                            s.editor.as_deref().unwrap_or("(default -> none)")
                        ));
                    }
                } else {
                    self.warn(
                        "state file does not exist yet",
                        "this is normal on a fresh install; it will be created on first exit",
                    );
                }
            }
            None => self.fail(
                "could not determine state file path",
                if cfg!(windows) {
                    "is %USERPROFILE% or %APPDATA% set?"
                } else {
                    "is $HOME or $XDG_CONFIG_HOME set?"
                },
            ),
        }
        self.blank();
    }

    /// Check 7: Configured editor and whether its binary is available.
    pub fn check_editor(
        &mut self,
        editor_label: &str,
        editor_binary: Option<&str>,
        binary_on_path: bool,
    ) {
        self.heading("Editor");
        match editor_binary {
            Some(bin) => {
                self.pass(&format!(
                    "configured editor: {editor_label} (binary: {bin})"
                ));
                if binary_on_path {
                    self.pass("editor binary found on $PATH");
                } else {
                    self.warn(
                        &format!("editor binary '{bin}' not found on $PATH"),
                        "install it or change with: tfe --editor <name>",
                    );
                }
            }
            None => self.warn(
                "no editor configured (pressing 'e' will show an error)",
                "set one with: tfe --editor nvim  (or press Shift+E in the TUI)",
            ),
        }
        self.blank();
    }

    /// Append the final summary block.
    pub fn summary(&mut self) {
        self.heading("---");
        self.detail(&format!(
            "Summary: {} passed, {} warning(s), {} failure(s)",
            self.oks, self.warnings, self.failures
        ));
        if self.failures > 0 {
            self.blank();
            self.detail("Some checks failed. Fix the items marked [FAIL] above.");
        } else if self.warnings > 0 {
            self.blank();
            self.detail("No failures! Warnings above are informational.");
        } else {
            self.blank();
            self.detail("Everything looks good!");
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run all diagnostic checks, print the report to stderr, and return.
///
/// Diagnostic output goes to stderr so the shell wrapper (which captures
/// stdout for cd-on-exit) does not try to `cd` into each report line.
pub fn run_doctor() {
    let version = env!("CARGO_PKG_VERSION");
    eprintln!("tfe doctor  v{version}");
    eprintln!();

    let mut r = DoctorReport::new();

    // 1. Platform
    r.check_platform(
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY,
    );

    // 2. Binary / PATH
    let exe_path = std::env::current_exe().ok();
    let home_os = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let cargo_bin = std::env::var_os("CARGO_HOME")
        .map(|c| PathBuf::from(c).join("bin"))
        .or_else(|| {
            home_os
                .as_ref()
                .map(|h| PathBuf::from(h).join(".cargo").join("bin"))
        });
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    r.check_binary(exe_path.as_deref(), cargo_bin.as_deref(), &path_dirs);

    // 3. Environment
    let home_str = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    let shell_str = std::env::var("SHELL").ok();
    let cwd = std::env::current_dir().ok();
    r.check_environment(home_str.as_deref(), shell_str.as_deref(), cwd.as_deref());

    // 4. Terminal
    let size = crossterm::terminal::size().ok();
    let stderr_tty = crossterm::tty::IsTty::is_tty(&io::stderr());
    r.check_terminal(size, stderr_tty);

    // 5. Shell integration
    let detected = shell_init::detect_shell();
    let (rc_path, rc_exists, wrapper_installed) = if let Some(shell) = detected {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
        let zdotdir = std::env::var_os("ZDOTDIR").map(PathBuf::from);
        let bash_profile = home.as_deref().map(|h| h.join(".bash_profile"));
        let zshenv_path = home.as_deref().map(|h| h.join(".zshenv"));
        let nu_config = shell_init::nu_config_dir_default();
        let rc = shell_init::rc_path_with(
            shell,
            home.as_deref(),
            xdg.as_deref(),
            zdotdir.as_deref(),
            bash_profile.as_deref(),
            zshenv_path.as_deref(),
            nu_config.as_deref(),
        );
        let exists = rc.as_deref().is_some_and(std::path::Path::exists);
        let installed = rc.as_deref().is_some_and(shell_init::is_installed);
        (rc, exists, installed)
    } else {
        (None, false, false)
    };
    r.check_shell_integration(detected, rc_path.as_deref(), rc_exists, wrapper_installed);

    // 6. Config / persisted state
    let sp = tui_file_explorer::persistence::state_path();
    let sp_exists = sp.as_deref().is_some_and(std::path::Path::exists);
    let saved = if sp_exists {
        Some(tui_file_explorer::load_state())
    } else {
        None
    };
    r.check_config(sp.as_deref(), sp_exists, saved.as_ref());

    // 7. Editor
    use tui_file_explorer::app::Editor;
    let loaded = tui_file_explorer::load_state();
    let editor = match loaded.editor.as_deref() {
        Some(raw) => Editor::from_key(raw).unwrap_or_default(),
        None => Editor::default(),
    };
    let bin = editor.binary();
    let bin_on_path = bin.as_ref().is_some_and(|b| {
        let name = b.split_whitespace().next().unwrap_or(b);
        let lookup = if cfg!(windows) { "where" } else { "which" };
        std::process::Command::new(lookup)
            .arg(name)
            .output()
            .is_ok_and(|o| o.status.success())
    });
    r.check_editor(editor.label(), bin.as_deref(), bin_on_path);

    // Summary
    r.summary();

    // Print everything to stderr — stdout is reserved for paths.
    let _ = r.write_to(&mut io::stderr().lock());
}

#[cfg(test)]
mod tests;
