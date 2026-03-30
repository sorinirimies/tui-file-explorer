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
                self.warn(
                    &format!("{} is NOT on $PATH", cb.display()),
                    r#"add to your shell rc:  export PATH="$HOME/.cargo/bin:$PATH""#,
                );
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
            Some(h) => self.pass(&format!("$HOME = {h}")),
            None => self.fail("$HOME is not set", "many features depend on $HOME"),
        }
        match shell_var {
            Some(s) => self.pass(&format!("$SHELL = {s}")),
            None => self.warn(
                "$SHELL is not set",
                "shell auto-detection will not work; use: tfe --init <shell>",
            ),
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
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "(none)".into())
                        ));
                        self.detail(&format!(
                            "last_dir_right= {}",
                            s.last_dir_right
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "(none)".into())
                        ));
                        self.detail(&format!(
                            "sort_mode     = {}",
                            s.sort_mode
                                .map(|m| m.label().to_string())
                                .unwrap_or_else(|| "(default)".into())
                        ));
                        self.detail(&format!(
                            "show_hidden   = {}",
                            s.show_hidden
                                .map(|b| b.to_string())
                                .unwrap_or_else(|| "(default)".into())
                        ));
                        self.detail(&format!(
                            "cd_on_exit    = {}",
                            s.cd_on_exit
                                .map(|b| b.to_string())
                                .unwrap_or_else(|| "(default -> true)".into())
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
                "is $HOME or $XDG_CONFIG_HOME set?",
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
    let home_os = std::env::var_os("HOME");
    let cargo_bin = home_os
        .as_ref()
        .map(|h| PathBuf::from(h).join(".cargo").join("bin"));
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    r.check_binary(exe_path.as_deref(), cargo_bin.as_deref(), &path_dirs);

    // 3. Environment
    let home_str = std::env::var("HOME").ok();
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
        let home = std::env::var_os("HOME").map(PathBuf::from);
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
        let exists = rc.as_deref().map(|p| p.exists()).unwrap_or(false);
        let installed = rc.as_deref().map(shell_init::is_installed).unwrap_or(false);
        (rc, exists, installed)
    } else {
        (None, false, false)
    };
    r.check_shell_integration(detected, rc_path.as_deref(), rc_exists, wrapper_installed);

    // 6. Config / persisted state
    let sp = tui_file_explorer::persistence::state_path();
    let sp_exists = sp.as_deref().map(|p| p.exists()).unwrap_or(false);
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
    let bin_on_path = bin
        .as_ref()
        .map(|b| {
            let name = b.split_whitespace().next().unwrap_or(b);
            std::process::Command::new("which")
                .arg(name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    r.check_editor(editor.label(), bin.as_deref(), bin_on_path);

    // Summary
    r.summary();

    // Print everything to stderr — stdout is reserved for paths.
    let _ = r.write_to(&mut io::stderr().lock());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── DoctorReport output helpers ───────────────────────────────────────

    #[test]
    fn new_report_starts_empty() {
        let r = DoctorReport::new();
        assert_eq!(r.oks, 0);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 0);
        assert!(r.lines.is_empty());
    }

    #[test]
    fn pass_increments_oks() {
        let mut r = DoctorReport::new();
        r.pass("good");
        assert_eq!(r.oks, 1);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn warn_increments_warnings() {
        let mut r = DoctorReport::new();
        r.warn("hmm", "fix it");
        assert_eq!(r.oks, 0);
        assert_eq!(r.warnings, 1);
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn fail_increments_failures() {
        let mut r = DoctorReport::new();
        r.fail("bad", "fix it");
        assert_eq!(r.oks, 0);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 1);
    }

    #[test]
    fn pass_line_contains_ok_marker() {
        let mut r = DoctorReport::new();
        r.pass("looks good");
        assert!(r.output().contains("[ok]"));
        assert!(r.output().contains("looks good"));
    }

    #[test]
    fn warn_line_contains_warn_marker() {
        let mut r = DoctorReport::new();
        r.warn("not ideal", "try this");
        let out = r.output();
        assert!(out.contains("[warn]"));
        assert!(out.contains("not ideal"));
        assert!(out.contains("-> try this"));
    }

    #[test]
    fn fail_line_contains_fail_marker() {
        let mut r = DoctorReport::new();
        r.fail("broken", "do this");
        let out = r.output();
        assert!(out.contains("[FAIL]"));
        assert!(out.contains("broken"));
        assert!(out.contains("-> do this"));
    }

    #[test]
    fn warn_with_empty_hint_omits_arrow() {
        let mut r = DoctorReport::new();
        r.warn("minor issue", "");
        let out = r.output();
        assert!(out.contains("[warn]"));
        assert!(!out.contains("->"));
    }

    #[test]
    fn fail_with_empty_hint_omits_arrow() {
        let mut r = DoctorReport::new();
        r.fail("major issue", "");
        let out = r.output();
        assert!(out.contains("[FAIL]"));
        assert!(!out.contains("->"));
    }

    #[test]
    fn heading_appears_in_output() {
        let mut r = DoctorReport::new();
        r.heading("My Section");
        assert!(r.output().contains("My Section"));
    }

    #[test]
    fn detail_appears_indented() {
        let mut r = DoctorReport::new();
        r.detail("some detail");
        assert!(r.output().contains("         some detail"));
    }

    #[test]
    fn write_to_produces_newline_separated_output() {
        let mut r = DoctorReport::new();
        r.heading("H");
        r.pass("ok");
        let mut buf = Vec::new();
        r.write_to(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("H\n"));
        assert!(s.contains("[ok]   ok\n"));
    }

    #[test]
    fn multiple_checks_accumulate_counters() {
        let mut r = DoctorReport::new();
        r.pass("a");
        r.pass("b");
        r.warn("c", "");
        r.fail("d", "");
        assert_eq!(r.oks, 2);
        assert_eq!(r.warnings, 1);
        assert_eq!(r.failures, 1);
    }

    // ── check_platform ────────────────────────────────────────────────────

    #[test]
    fn check_platform_always_passes() {
        let mut r = DoctorReport::new();
        r.check_platform("macos", "aarch64", "unix");
        assert_eq!(r.oks, 1);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 0);
        let out = r.output();
        assert!(out.contains("Platform"));
        assert!(out.contains("macos aarch64 (unix)"));
    }

    // ── check_binary ──────────────────────────────────────────────────────

    #[test]
    fn check_binary_exe_found_cargo_on_path() {
        let mut r = DoctorReport::new();
        let exe = PathBuf::from("/usr/local/bin/tfe");
        let cargo = PathBuf::from("/home/user/.cargo/bin");
        let dirs = vec![PathBuf::from("/usr/bin"), cargo.clone()];
        r.check_binary(Some(&exe), Some(&cargo), &dirs);
        assert_eq!(r.oks, 2);
        assert_eq!(r.warnings, 0);
    }

    #[test]
    fn check_binary_exe_missing() {
        let mut r = DoctorReport::new();
        r.check_binary(None, None, &[]);
        assert_eq!(r.oks, 0);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("could not resolve"));
    }

    #[test]
    fn check_binary_cargo_not_on_path() {
        let mut r = DoctorReport::new();
        let exe = PathBuf::from("/usr/local/bin/tfe");
        let cargo = PathBuf::from("/home/user/.cargo/bin");
        let dirs = vec![PathBuf::from("/usr/bin")];
        r.check_binary(Some(&exe), Some(&cargo), &dirs);
        assert_eq!(r.oks, 1);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("NOT on $PATH"));
    }

    // ── check_environment ─────────────────────────────────────────────────

    #[test]
    fn check_environment_all_set() {
        let mut r = DoctorReport::new();
        let cwd = PathBuf::from("/home/user");
        r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
        assert_eq!(r.oks, 3);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn check_environment_home_missing_is_fail() {
        let mut r = DoctorReport::new();
        let cwd = PathBuf::from("/tmp");
        r.check_environment(None, Some("/bin/zsh"), Some(&cwd));
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("[FAIL]"));
        assert!(r.output().contains("$HOME is not set"));
    }

    #[test]
    fn check_environment_shell_missing_is_warn() {
        let mut r = DoctorReport::new();
        let cwd = PathBuf::from("/tmp");
        r.check_environment(Some("/home/user"), None, Some(&cwd));
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("$SHELL is not set"));
    }

    #[test]
    fn check_environment_cwd_missing_is_fail() {
        let mut r = DoctorReport::new();
        r.check_environment(Some("/home/user"), Some("/bin/zsh"), None);
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("cannot read working directory"));
    }

    // ── check_terminal ────────────────────────────────────────────────────

    #[test]
    fn check_terminal_good_size_and_tty() {
        let mut r = DoctorReport::new();
        r.check_terminal(Some((120, 40)), true);
        assert_eq!(r.oks, 2);
        assert_eq!(r.warnings, 0);
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn check_terminal_small_size_is_warn() {
        let mut r = DoctorReport::new();
        r.check_terminal(Some((30, 5)), true);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("very small"));
    }

    #[test]
    fn check_terminal_no_size_is_fail() {
        let mut r = DoctorReport::new();
        r.check_terminal(None, true);
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("could not query terminal size"));
    }

    #[test]
    fn check_terminal_not_tty_is_warn() {
        let mut r = DoctorReport::new();
        r.check_terminal(Some((80, 24)), false);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("stderr is NOT a tty"));
    }

    #[test]
    fn check_terminal_small_and_not_tty_both_count() {
        let mut r = DoctorReport::new();
        r.check_terminal(Some((20, 5)), false);
        assert_eq!(r.warnings, 2);
        assert_eq!(r.oks, 0);
    }

    // ── check_shell_integration ───────────────────────────────────────────

    #[test]
    fn check_shell_no_detection_is_warn() {
        let mut r = DoctorReport::new();
        r.check_shell_integration(None, None, false, false);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("could not detect shell"));
    }

    #[test]
    fn check_shell_detected_wrapper_installed() {
        let mut r = DoctorReport::new();
        let rc = PathBuf::from("/home/user/.zshrc");
        r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, true);
        assert_eq!(r.oks, 3); // detected + rc file + wrapper installed
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn check_shell_detected_wrapper_missing_is_fail() {
        let mut r = DoctorReport::new();
        let rc = PathBuf::from("/home/user/.zshrc");
        r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, false);
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("wrapper NOT found"));
    }

    #[test]
    fn check_shell_detected_rc_missing_is_warn() {
        let mut r = DoctorReport::new();
        let rc = PathBuf::from("/home/user/.zshrc");
        r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), false, false);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("rc file does not exist yet"));
    }

    #[test]
    fn check_shell_detected_no_rc_path_is_fail() {
        let mut r = DoctorReport::new();
        r.check_shell_integration(Some(shell_init::Shell::Bash), None, false, false);
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("could not determine rc file path"));
    }

    // ── check_config ──────────────────────────────────────────────────────

    #[test]
    fn check_config_no_state_path_is_fail() {
        let mut r = DoctorReport::new();
        r.check_config(None, false, None);
        assert_eq!(r.failures, 1);
        assert!(r.output().contains("could not determine state file path"));
    }

    #[test]
    fn check_config_fresh_install_is_warn() {
        let mut r = DoctorReport::new();
        let sp = PathBuf::from("/home/user/.config/tfe/state");
        r.check_config(Some(&sp), false, None);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("state file does not exist yet"));
    }

    #[test]
    fn check_config_existing_state_passes() {
        let mut r = DoctorReport::new();
        let sp = PathBuf::from("/home/user/.config/tfe/state");
        let state = tui_file_explorer::AppState {
            theme: Some("nord".into()),
            cd_on_exit: Some(true),
            ..Default::default()
        };
        r.check_config(Some(&sp), true, Some(&state));
        assert_eq!(r.oks, 2); // state file + exists
        assert_eq!(r.failures, 0);
        let out = r.output();
        assert!(out.contains("theme         = nord"));
        assert!(out.contains("cd_on_exit    = true"));
    }

    #[test]
    fn check_config_default_state_shows_defaults() {
        let mut r = DoctorReport::new();
        let sp = PathBuf::from("/tmp/state");
        let state = tui_file_explorer::AppState::default();
        r.check_config(Some(&sp), true, Some(&state));
        let out = r.output();
        assert!(out.contains("theme         = (default)"));
        assert!(out.contains("cd_on_exit    = (default -> true)"));
        assert!(out.contains("editor        = (default -> none)"));
    }

    // ── check_editor ──────────────────────────────────────────────────────

    #[test]
    fn check_editor_configured_and_on_path() {
        let mut r = DoctorReport::new();
        r.check_editor("helix", Some("hx"), true);
        assert_eq!(r.oks, 2);
        assert_eq!(r.warnings, 0);
        let out = r.output();
        assert!(out.contains("helix"));
        assert!(out.contains("hx"));
    }

    #[test]
    fn check_editor_configured_but_not_on_path() {
        let mut r = DoctorReport::new();
        r.check_editor("helix", Some("hx"), false);
        assert_eq!(r.oks, 1);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("not found on $PATH"));
    }

    #[test]
    fn check_editor_not_configured() {
        let mut r = DoctorReport::new();
        r.check_editor("none", None, false);
        assert_eq!(r.oks, 0);
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("no editor configured"));
    }

    // ── summary ───────────────────────────────────────────────────────────

    #[test]
    fn summary_all_pass() {
        let mut r = DoctorReport::new();
        r.pass("a");
        r.pass("b");
        r.summary();
        let out = r.output();
        assert!(out.contains("2 passed, 0 warning(s), 0 failure(s)"));
        assert!(out.contains("Everything looks good!"));
    }

    #[test]
    fn summary_with_warnings() {
        let mut r = DoctorReport::new();
        r.pass("a");
        r.warn("b", "hint");
        r.summary();
        let out = r.output();
        assert!(out.contains("1 passed, 1 warning(s), 0 failure(s)"));
        assert!(out.contains("No failures!"));
    }

    #[test]
    fn summary_with_failures() {
        let mut r = DoctorReport::new();
        r.pass("a");
        r.fail("b", "hint");
        r.summary();
        let out = r.output();
        assert!(out.contains("1 passed, 0 warning(s), 1 failure(s)"));
        assert!(out.contains("Fix the items marked [FAIL]"));
    }

    // ── Integration: full report ──────────────────────────────────────────

    #[test]
    fn full_report_healthy_system() {
        let mut r = DoctorReport::new();
        let exe = PathBuf::from("/usr/local/bin/tfe");
        let cargo = PathBuf::from("/home/user/.cargo/bin");
        let dirs = vec![cargo.clone()];
        let cwd = PathBuf::from("/home/user");
        let rc = PathBuf::from("/home/user/.zshrc");
        let sp = PathBuf::from("/home/user/.config/tfe/state");
        let state = tui_file_explorer::AppState {
            theme: Some("dracula".into()),
            cd_on_exit: Some(true),
            ..Default::default()
        };

        r.check_platform("macos", "aarch64", "unix");
        r.check_binary(Some(&exe), Some(&cargo), &dirs);
        r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
        r.check_terminal(Some((120, 40)), true);
        r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, true);
        r.check_config(Some(&sp), true, Some(&state));
        r.check_editor("helix", Some("hx"), true);
        r.summary();

        assert_eq!(r.failures, 0);
        assert_eq!(r.warnings, 0);
        assert!(r.oks >= 10);
        assert!(r.output().contains("Everything looks good!"));
    }

    #[test]
    fn full_report_fresh_install() {
        let mut r = DoctorReport::new();
        let exe = PathBuf::from("/home/user/.cargo/bin/tfe");
        let cargo = PathBuf::from("/home/user/.cargo/bin");
        let dirs = vec![cargo.clone()];
        let cwd = PathBuf::from("/home/user");
        let rc = PathBuf::from("/home/user/.zshrc");
        let sp = PathBuf::from("/home/user/.config/tfe/state");

        r.check_platform("macos", "aarch64", "unix");
        r.check_binary(Some(&exe), Some(&cargo), &dirs);
        r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
        r.check_terminal(Some((80, 24)), true);
        r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), false, false);
        r.check_config(Some(&sp), false, None);
        r.check_editor("none", None, false);
        r.summary();

        assert_eq!(r.failures, 0);
        assert!(r.warnings >= 3); // rc missing, state missing, no editor
        assert!(r.output().contains("No failures!"));
    }

    #[test]
    fn full_report_broken_install() {
        let mut r = DoctorReport::new();

        r.check_platform("linux", "x86_64", "unix");
        r.check_binary(None, None, &[]);
        r.check_environment(None, None, None);
        r.check_terminal(None, false);
        r.check_shell_integration(None, None, false, false);
        r.check_config(None, false, None);
        r.check_editor("none", None, false);
        r.summary();

        assert!(r.failures >= 4); // HOME, cwd, terminal size, state path
        assert!(r.output().contains("Fix the items marked [FAIL]"));
    }
}
