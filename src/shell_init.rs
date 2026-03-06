//! Shell integration helpers for the `tfe` binary.
//!
//! This module handles everything related to the `--init <shell>` flag:
//!
//! * [`Shell`]            — recognised shell variants.
//! * [`detect_shell`]     — infer the current shell from `$SHELL`.
//! * [`snippet`]          — return the wrapper function text for a shell.
//! * [`rc_path`]          — return the default rc-file path for a shell.
//! * [`is_installed`]     — check whether the wrapper is already present.
//! * [`install`]          — append the snippet to the rc file, creating it
//!   (and any missing parent directories) if necessary.
//! * [`install_or_print`] — top-level entry point called by `main`: writes to
//!   the rc file when possible, falls back to printing the snippet to stdout
//!   with instructions when not.

use std::{
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

// ── Shell ─────────────────────────────────────────────────────────────────────

/// A shell that `tfe` knows how to generate an init snippet for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// Bourne-Again Shell — uses `~/.bashrc`.
    Bash,
    /// Z Shell — uses `~/.zshrc`.
    Zsh,
    /// Friendly Interactive Shell — uses `~/.config/fish/functions/tfe.fish`.
    Fish,
    /// PowerShell (Windows and cross-platform) — uses `$PROFILE`.
    /// Named `Pwsh` after the cross-platform PowerShell binary.
    Pwsh,
}

impl Shell {
    /// Parse a shell name string (case-insensitive).
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "powershell" | "pwsh" => Some(Self::Pwsh),
            _ => None,
        }
    }

    /// Canonical lowercase name used in messages and the sentinel comment.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Pwsh => "powershell",
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ── Detection ─────────────────────────────────────────────────────────────────

/// Infer the current shell from environment variables.
///
/// - Unix/macOS: reads `$SHELL` (e.g. `/bin/zsh` → `Zsh`).
/// - Windows: checks `$PSVersionTable` / `$PSModulePath` presence to detect
///   PowerShell; falls back to `None` for CMD or unknown shells.
///
/// Returns `None` when the shell cannot be determined or is not supported.
pub fn detect_shell() -> Option<Shell> {
    if cfg!(windows) {
        // On Windows $SHELL is not set. Detect PowerShell by the presence of
        // $PSModulePath which is always injected by the PowerShell host.
        if std::env::var_os("PSModulePath").is_some() {
            return Some(Shell::Pwsh);
        }
        return None;
    }
    let shell_var = std::env::var("SHELL").ok()?;
    // $SHELL is typically a full path like /bin/zsh — take just the filename.
    let name = Path::new(&shell_var).file_name()?.to_str()?;
    Shell::from_str(name)
}

// ── Snippet ───────────────────────────────────────────────────────────────────

/// The sentinel string embedded in every snippet.
///
/// Used by [`is_installed`] to detect whether the wrapper is already present
/// in the rc file without parsing shell syntax.
const SENTINEL: &str = "# tfe-shell-init";

/// Return the shell wrapper snippet for `shell`.
///
/// The snippet wraps the `tfe` binary so that dismissing the explorer with
/// `Esc` or `q` automatically `cd`s the terminal to the browsed directory.
/// A [`SENTINEL`] comment is included so [`is_installed`] can detect it.
pub fn snippet(shell: Shell) -> String {
    match shell {
        Shell::Bash | Shell::Zsh => format!(
            "\n{SENTINEL}\n\
             tfe() {{\n\
             \x20   local dir\n\
             \x20   dir=$(command tfe \"$@\")\n\
             \x20   [ -n \"$dir\" ] && cd \"$dir\"\n\
             }}\n"
        ),
        Shell::Fish => format!(
            "\n{SENTINEL}\n\
             function tfe\n\
             \x20   set dir (command tfe $argv)\n\
             \x20   if test -n \"$dir\"\n\
             \x20       cd $dir\n\
             \x20   end\n\
             end\n"
        ),
        Shell::Pwsh => format!(
            "\n{SENTINEL}\n\
             function tfe {{\n\
             \x20   $dir = & (Get-Command tfe -CommandType Application).Source @args\n\
             \x20   if ($dir) {{ Set-Location $dir }}\n\
             }}\n"
        ),
    }
}

// ── Rc-file path ──────────────────────────────────────────────────────────────

/// Return the default rc-file path for `shell`.
///
/// Uses `$HOME` for bash/zsh and `$XDG_CONFIG_HOME` (falling back to
/// `$HOME/.config`) for fish.  Returns `None` when neither variable is set.
///
/// Pass explicit `home` and `xdg_config_home` overrides to keep tests hermetic
/// without mutating global environment variables.
pub fn rc_path_with(
    shell: Shell,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
) -> Option<PathBuf> {
    match shell {
        Shell::Bash => home.map(|h| h.join(".bashrc")),
        Shell::Zsh => home.map(|h| h.join(".zshrc")),
        Shell::Fish => {
            let config = xdg_config_home
                .map(|p| p.to_path_buf())
                .or_else(|| home.map(|h| h.join(".config")))?;
            Some(config.join("fish/functions/tfe.fish"))
        }
        Shell::Pwsh => {
            // Use $PROFILE when available (set by PowerShell itself).
            // Fall back to the conventional Documents\PowerShell path under $HOME.
            if let Some(profile) = std::env::var_os("PROFILE") {
                return Some(PathBuf::from(profile));
            }
            home.map(|h| {
                if cfg!(windows) {
                    h.join("Documents")
                        .join("PowerShell")
                        .join("Microsoft.PowerShell_profile.ps1")
                } else {
                    // PowerShell on Linux/macOS uses ~/.config/powershell
                    h.join(".config")
                        .join("powershell")
                        .join("Microsoft.PowerShell_profile.ps1")
                }
            })
        }
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
}

// ── Already-installed check ───────────────────────────────────────────────────

/// Return `true` when the `tfe` wrapper snippet is already present in `path`.
///
/// Two signatures are recognised so that manually-added wrappers (without the
/// sentinel) are never duplicated alongside a sentinel-based install:
///
/// 1. The [`SENTINEL`] comment `# tfe-shell-init` — present in all wrappers
///    written by `--init`.
/// 2. The bare function signatures `tfe()` (bash/zsh) and `function tfe`
///    (fish) preceded by `command tfe` on any nearby line — catches wrappers
///    that were copy-pasted by hand before `--init` existed.
///
/// Returns `false` when `path` does not exist or cannot be read.
pub fn is_installed(path: &Path) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Fast path: sentinel present.
    if content.contains(SENTINEL) {
        return true;
    }
    // Slow path: detect a hand-written wrapper by looking for the tfe function
    // body pattern — the function declaration followed by `command tfe` within
    // a short window of lines.
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "tfe() {" || trimmed == "function tfe" {
            // Check the next 6 lines for `command tfe`.
            let window_end = (i + 7).min(lines.len());
            if lines[i..window_end]
                .iter()
                .any(|l| l.contains("command tfe"))
            {
                return true;
            }
        }
    }
    false
}

// ── Install ───────────────────────────────────────────────────────────────────

/// Append the snippet for `shell` to `rc_path`, creating the file and any
/// missing parent directories if necessary.
///
/// # Errors
///
/// Returns an [`io::Error`] if the parent directory cannot be created or the
/// file cannot be opened for appending.
pub fn install(shell: Shell, rc: &Path) -> io::Result<()> {
    if let Some(parent) = rc.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::OpenOptions::new().create(true).append(true).open(rc)?;
    file.write_all(snippet(shell).as_bytes())?;
    file.flush()
}

// ── Top-level entry point ─────────────────────────────────────────────────────

/// Result of [`install_or_print`].
#[derive(Debug, PartialEq, Eq)]
pub enum InitOutcome {
    /// Snippet was appended to the rc file at the given path.
    Installed(PathBuf),
    /// Snippet was already present — nothing written.
    AlreadyInstalled(PathBuf),
    /// Could not write to the rc file; snippet printed to stdout instead.
    PrintedToStdout,
    /// `$SHELL` was not set or not recognised; snippet printed to stdout.
    UnknownShell,
}

/// Install the shell wrapper for `shell` (auto-detected when `None`).
///
/// Behaviour:
/// 1. Resolve the shell — use `shell` when `Some`, otherwise call
///    [`detect_shell`].  If neither yields a known shell, print the bash/zsh
///    snippet to stdout with a hint and return [`InitOutcome::UnknownShell`].
/// 2. Resolve the rc-file path via [`rc_path`].
/// 3. If the snippet is already present ([`is_installed`]), return
///    [`InitOutcome::AlreadyInstalled`] without writing anything.
/// 4. Try to [`install`].  On success return [`InitOutcome::Installed`].
/// 5. On failure, print the snippet to stdout so the user can install it
///    manually, and return [`InitOutcome::PrintedToStdout`].
pub fn install_or_print(shell: Option<Shell>) -> InitOutcome {
    install_or_print_to(shell, home().as_deref(), xdg_config_home().as_deref())
}

/// Like [`install_or_print`] but with explicit `home` and `xdg` path overrides.
///
/// Used in tests to avoid mutating global environment variables.
pub(crate) fn install_or_print_to(
    shell: Option<Shell>,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
) -> InitOutcome {
    // On Windows, only PowerShell is supported for --init.
    // CMD has no equivalent of shell functions. WSL users should run the
    // Linux tfe binary inside WSL and use tfe --init zsh/bash there.
    if cfg!(windows) {
        if let Some(s) = shell {
            if s != Shell::Pwsh {
                eprintln!(
                    "tfe: on Windows only PowerShell is supported: tfe --init powershell\n\
                     For WSL (bash/zsh/fish) run tfe --init <shell> inside WSL."
                );
                return InitOutcome::UnknownShell;
            }
        }
    }

    // Step 1 — resolve shell.
    let resolved = match shell.or_else(detect_shell) {
        Some(s) => s,
        None => {
            eprintln!(
                "tfe: could not detect shell from $SHELL. \
                 Re-run with an explicit shell: tfe --init zsh"
            );
            // Fall back to printing the bash/zsh snippet — most likely to be useful.
            print!("{}", snippet(Shell::Bash));
            return InitOutcome::UnknownShell;
        }
    };

    // Step 2 — resolve rc path.
    let rc = match rc_path_with(resolved, home, xdg_config_home) {
        Some(p) => p,
        None => {
            eprintln!(
                "tfe: could not determine rc file path ($HOME is not set). \
                 Add the following to your shell config manually:"
            );
            print!("{}", snippet(resolved));
            return InitOutcome::PrintedToStdout;
        }
    };

    // Step 3 — already installed?
    if is_installed(&rc) {
        return InitOutcome::AlreadyInstalled(rc);
    }

    // Step 4 — try to install.
    match install(resolved, &rc) {
        Ok(()) => InitOutcome::Installed(rc),
        Err(e) => {
            // Step 5 — fallback: print snippet to stdout with instructions.
            eprintln!(
                "tfe: could not write to {}: {e}\n\
                 Add the following to your shell config manually:",
                rc.display()
            );
            print!("{}", snippet(resolved));
            InitOutcome::PrintedToStdout
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── Shell::from_str ───────────────────────────────────────────────────────

    #[test]
    fn from_str_recognises_bash() {
        assert_eq!(Shell::from_str("bash"), Some(Shell::Bash));
    }

    #[test]
    fn from_str_recognises_zsh() {
        assert_eq!(Shell::from_str("zsh"), Some(Shell::Zsh));
    }

    #[test]
    fn from_str_recognises_fish() {
        assert_eq!(Shell::from_str("fish"), Some(Shell::Fish));
    }

    #[test]
    fn from_str_recognises_powershell() {
        assert_eq!(Shell::from_str("powershell"), Some(Shell::Pwsh));
        assert_eq!(Shell::from_str("pwsh"), Some(Shell::Pwsh));
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(Shell::from_str("ZSH"), Some(Shell::Zsh));
        assert_eq!(Shell::from_str("Bash"), Some(Shell::Bash));
        assert_eq!(Shell::from_str("FISH"), Some(Shell::Fish));
        assert_eq!(Shell::from_str("PowerShell"), Some(Shell::Pwsh));
        assert_eq!(Shell::from_str("PWSH"), Some(Shell::Pwsh));
    }

    #[test]
    fn from_str_returns_none_for_unknown() {
        assert_eq!(Shell::from_str("cmd"), None);
        assert_eq!(Shell::from_str(""), None);
        assert_eq!(Shell::from_str("sh"), None);
    }

    #[test]
    fn display_returns_lowercase_name() {
        assert_eq!(Shell::Bash.to_string(), "bash");
        assert_eq!(Shell::Zsh.to_string(), "zsh");
        assert_eq!(Shell::Fish.to_string(), "fish");
        assert_eq!(Shell::Pwsh.to_string(), "powershell");
    }

    // ── snippet ───────────────────────────────────────────────────────────────

    #[test]
    fn snippet_contains_sentinel() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh] {
            assert!(
                snippet(shell).contains(SENTINEL),
                "{shell} snippet missing sentinel"
            );
        }
    }

    #[test]
    fn snippet_bash_contains_function_body() {
        let s = snippet(Shell::Bash);
        assert!(
            s.contains("command tfe"),
            "bash snippet missing command tfe"
        );
        assert!(s.contains("cd \"$dir\""), "bash snippet missing cd");
    }

    #[test]
    fn snippet_zsh_identical_to_bash() {
        assert_eq!(snippet(Shell::Zsh), snippet(Shell::Bash));
    }

    #[test]
    fn snippet_fish_contains_function_body() {
        let s = snippet(Shell::Fish);
        assert!(
            s.contains("command tfe"),
            "fish snippet missing command tfe"
        );
        assert!(s.contains("cd $dir"), "fish snippet missing cd");
        assert!(
            s.contains("function tfe"),
            "fish snippet missing function keyword"
        );
    }

    #[test]
    fn snippet_fish_differs_from_bash() {
        assert_ne!(snippet(Shell::Fish), snippet(Shell::Bash));
    }

    #[test]
    fn snippet_powershell_contains_function_body() {
        let s = snippet(Shell::Pwsh);
        assert!(s.contains("function tfe"), "missing function tfe");
        assert!(s.contains("Set-Location"), "missing Set-Location");
        assert!(s.contains("Get-Command tfe"), "missing Get-Command tfe");
    }

    #[test]
    fn snippet_powershell_differs_from_bash() {
        assert_ne!(snippet(Shell::Pwsh), snippet(Shell::Bash));
    }

    // ── rc_path_with ──────────────────────────────────────────────────────────

    #[test]
    fn rc_path_bash_ends_with_bashrc() {
        let p = rc_path_with(Shell::Bash, Some(Path::new("/test/home")), None).unwrap();
        assert_eq!(p, PathBuf::from("/test/home/.bashrc"));
    }

    #[test]
    fn rc_path_zsh_ends_with_zshrc() {
        let p = rc_path_with(Shell::Zsh, Some(Path::new("/test/home")), None).unwrap();
        assert_eq!(p, PathBuf::from("/test/home/.zshrc"));
    }

    #[test]
    #[cfg(not(windows))]
    fn rc_path_powershell_falls_back_to_home_config_on_unix() {
        std::env::remove_var("PROFILE");
        let p = rc_path_with(Shell::Pwsh, Some(Path::new("/test/home")), None).unwrap();
        assert_eq!(
            p,
            PathBuf::from("/test/home/.config/powershell/Microsoft.PowerShell_profile.ps1")
        );
    }

    #[test]
    fn rc_path_fish_uses_xdg_config_home_when_set() {
        let p = rc_path_with(
            Shell::Fish,
            Some(Path::new("/test/home")),
            Some(Path::new("/custom/config")),
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/custom/config/fish/functions/tfe.fish"));
    }

    #[test]
    fn rc_path_fish_falls_back_to_home_config() {
        let p = rc_path_with(Shell::Fish, Some(Path::new("/test/home")), None).unwrap();
        assert_eq!(
            p,
            PathBuf::from("/test/home/.config/fish/functions/tfe.fish")
        );
    }

    #[test]
    fn rc_path_returns_none_when_home_unset() {
        std::env::remove_var("PROFILE");
        assert!(rc_path_with(Shell::Bash, None, None).is_none());
        assert!(rc_path_with(Shell::Zsh, None, None).is_none());
        assert!(rc_path_with(Shell::Fish, None, None).is_none());
        assert!(rc_path_with(Shell::Pwsh, None, None).is_none());
    }

    // ── is_installed ──────────────────────────────────────────────────────────

    #[test]
    fn is_installed_detects_sentinel_based_wrapper() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        install(Shell::Zsh, &rc).unwrap();
        assert!(is_installed(&rc));
    }

    #[test]
    fn is_installed_detects_hand_written_bash_wrapper() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(
            &rc,
            "# tfe — cd to the directory browsed when dismissing the file explorer\n\
             tfe() {\n\
             \x20   local dir\n\
             \x20   dir=$(command tfe \"$@\")\n\
             \x20   [ -n \"$dir\" ] && cd \"$dir\"\n\
             }\n",
        )
        .unwrap();
        assert!(
            is_installed(&rc),
            "hand-written bash wrapper must be detected"
        );
    }

    #[test]
    fn is_installed_detects_hand_written_fish_wrapper() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("tfe.fish");
        fs::write(
            &rc,
            "function tfe\n\
             \x20   set dir (command tfe $argv)\n\
             \x20   if test -n \"$dir\"\n\
             \x20       cd $dir\n\
             \x20   end\n\
             end\n",
        )
        .unwrap();
        assert!(
            is_installed(&rc),
            "hand-written fish wrapper must be detected"
        );
    }

    #[test]
    fn is_installed_returns_false_for_missing_file() {
        let dir = tempdir().unwrap();
        assert!(!is_installed(&dir.path().join("nonexistent")));
    }

    #[test]
    fn is_installed_returns_false_for_empty_file() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(&rc, b"").unwrap();
        assert!(!is_installed(&rc));
    }

    #[test]
    fn is_installed_returns_false_when_sentinel_absent() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(&rc, b"export PATH=$PATH:~/.cargo/bin\n").unwrap();
        assert!(!is_installed(&rc));
    }

    #[test]
    fn is_installed_returns_true_when_sentinel_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(&rc, format!("some stuff\n{SENTINEL}\ntfe() {{}}\n")).unwrap();
        assert!(is_installed(&rc));
    }

    // ── install ───────────────────────────────────────────────────────────────

    #[test]
    fn install_creates_rc_file_when_missing() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        assert!(!rc.exists());
        install(Shell::Zsh, &rc).unwrap();
        assert!(rc.exists());
    }

    #[test]
    fn install_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("fish/functions/tfe.fish");
        install(Shell::Fish, &rc).unwrap();
        assert!(rc.exists());
    }

    #[test]
    fn install_appends_snippet_to_existing_file() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(&rc, b"export FOO=bar\n").unwrap();
        install(Shell::Zsh, &rc).unwrap();
        let content = fs::read_to_string(&rc).unwrap();
        assert!(
            content.starts_with("export FOO=bar\n"),
            "existing content must be preserved"
        );
        assert!(content.contains(SENTINEL), "snippet must be appended");
    }

    #[test]
    fn install_written_snippet_passes_is_installed() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        install(Shell::Bash, &rc).unwrap();
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_does_not_duplicate_when_called_twice() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        install(Shell::Zsh, &rc).unwrap();
        // Guard: caller should check is_installed first — but even if install
        // is called again the sentinel appears twice (expected append behaviour).
        // This test documents the expected behaviour rather than asserting idempotency.
        install(Shell::Zsh, &rc).unwrap();
        let content = fs::read_to_string(&rc).unwrap();
        let count = content.matches(SENTINEL).count();
        assert_eq!(
            count, 2,
            "raw install appends each time — guard is is_installed"
        );
    }

    // ── install_or_print ─────────────────────────────────────────────────────

    #[test]
    fn install_or_print_installs_when_rc_writable() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        let outcome = install_or_print_to(Some(Shell::Zsh), Some(dir.path()), None);
        assert_eq!(outcome, InitOutcome::Installed(rc.clone()));
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_or_print_returns_already_installed_when_sentinel_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        // Pre-install the snippet.
        install(Shell::Zsh, &rc).unwrap();
        let outcome = install_or_print_to(Some(Shell::Zsh), Some(dir.path()), None);
        assert_eq!(outcome, InitOutcome::AlreadyInstalled(rc));
    }

    #[test]
    fn install_or_print_returns_printed_when_rc_not_writable() {
        let dir = tempdir().unwrap();
        // Make the dir read-only so writing fails.
        let ro_dir = dir.path().join("readonly");
        fs::create_dir(&ro_dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&ro_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&ro_dir, perms).unwrap();
            let outcome = install_or_print_to(Some(Shell::Zsh), Some(&ro_dir), None);
            assert_eq!(
                outcome,
                InitOutcome::PrintedToStdout,
                "read-only dir must fall back to stdout"
            );
        }
        #[cfg(not(unix))]
        {
            // On non-unix we can't reliably make dirs unwritable — skip.
            let _ = ro_dir;
        }
    }
}
