//! Shell integration helpers for the `tfe` binary.
//!
//! This module handles everything related to the `--init <shell>` flag and
//! the automatic first-run wrapper installation:
//!
//! * [`Shell`]            — recognised shell variants.
//! * [`detect_shell`]     — infer the current shell from `$SHELL` / `$NU_VERSION`.
//! * [`snippet`]          — return the wrapper function text for a shell.
//! * [`rc_path_with`]     — return the default rc-file path for a shell.
//! * [`is_installed`]     — check whether the wrapper is already present in
//!   any candidate rc file for the detected shell.
//! * [`install`]          — append the snippet to the rc file, creating it
//!   (and any missing parent directories) if necessary.
//! * [`auto_install`]     — silently install on first run if not present;
//!   called automatically at startup before the TUI is shown.
//! * [`install_or_print`] — top-level entry point called by `--init`: writes
//!   to the rc file when possible, falls back to printing the snippet to
//!   stdout with instructions when not.

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
    /// Nushell — uses `<config-dir>/nushell/config.nu`.
    ///
    /// Config-dir is platform-dependent:
    /// - Linux:   `$XDG_CONFIG_HOME/nushell` or `~/.config/nushell`
    /// - macOS:   `~/Library/Application Support/nushell`
    /// - Windows: `%APPDATA%\nushell`
    Nu,
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
            "nu" | "nushell" => Some(Self::Nu),
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
            Self::Nu => "nushell",
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
/// - **Nushell** (all platforms): checked first via `$NU_VERSION`, which
///   Nushell always exports to child processes.  Also accepted when `$SHELL`
///   ends with `nu` (some distributions set it).
/// - **Unix/macOS**: reads `$SHELL` (e.g. `/bin/zsh` → `Zsh`).
/// - **Windows**: checks `$PSModulePath` presence to detect PowerShell;
///   falls back to `None` for CMD or unknown shells.
///
/// Returns `None` when the shell cannot be determined or is not supported.
pub fn detect_shell() -> Option<Shell> {
    // Nushell exports $NU_VERSION to every child process regardless of
    // platform, making it the most reliable detection signal.
    if std::env::var_os("NU_VERSION").is_some() {
        return Some(Shell::Nu);
    }

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

/// Return the platform-default Nushell configuration directory.
///
/// Priority:
/// - Linux / other Unix: `$XDG_CONFIG_HOME/nushell` → `~/.config/nushell`
/// - macOS: `~/Library/Application Support/nushell`
///   (Nushell follows macOS conventions and does *not* use `~/.config` by
///   default; `$XDG_CONFIG_HOME` still overrides when explicitly set.)
/// - Windows: `%APPDATA%\nushell`
///
/// Returns `None` when the necessary home/appdata variable is unset.
pub(crate) fn nu_config_dir_default() -> Option<PathBuf> {
    // $XDG_CONFIG_HOME overrides platform defaults on every OS.
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("nushell"));
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: ~/Library/Application Support/nushell
        if let Some(home) = std::env::var_os("HOME") {
            return Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("nushell"),
            );
        }
        None
    }

    #[cfg(windows)]
    {
        // Windows: %APPDATA%\nushell
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("nushell"));
        }
        return None;
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    {
        // Linux / other Unix: ~/.config/nushell
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join(".config").join("nushell"));
        }
        None
    }
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
/// The prefix used in the stdout protocol for source directives.
///
/// When `tfe` auto-installs the shell wrapper on first run it cannot source
/// the rc file itself (a child process cannot modify its parent's environment).
/// Instead it emits a line of the form `source:<path>` to stdout.  The shell
/// wrapper reads every output line and, when it sees this prefix, runs
/// `source <path>` in the parent shell — making the newly-written `tfe`
/// function available immediately without the user needing to open a new
/// terminal or run `source` manually.
///
/// Any other non-empty output line is treated as a directory/file path and
/// handled with `cd` / `Set-Location` as before.  The protocol is strictly
/// additive and backwards-compatible: old wrappers that do not recognise the
/// prefix will simply try to `cd` to `source:<path>`, which will fail silently
/// (the directory does not exist) and leave the shell in its current directory.
pub const SOURCE_DIRECTIVE_PREFIX: &str = "source:";

pub fn snippet(shell: Shell) -> String {
    // Templates live next to this file so they can be read, diffed and
    // shellchecked as real scripts. `{{SENTINEL}}` is replaced with the
    // marker comment; every template starts with a blank line so the snippet
    // is cleanly separated from whatever precedes it in the rc file.
    let template = match shell {
        // bash / zsh: loop over every output line. Lines beginning with
        // "source:" are sourced in the current shell; any other non-empty
        // line is treated as a cd target.
        Shell::Bash | Shell::Zsh => include_str!("shell_init/templates/tfe.bash"),
        Shell::Fish => include_str!("shell_init/templates/tfe.fish"),
        Shell::Pwsh => include_str!("shell_init/templates/tfe.ps1"),
        // Nushell: `def --env` is required so `cd` propagates to the caller;
        // `--wrapped` forwards all flags unchanged. `source` is a parse-time
        // keyword and cannot take a runtime string, so the `source:` protocol
        // is intentionally not handled here.
        Shell::Nu => include_str!("shell_init/templates/tfe.nu"),
    };
    format!("\n{}", template.replace("{{SENTINEL}}", SENTINEL))
}

// ── Rc-file path ──────────────────────────────────────────────────────────────

/// Return the default rc-file path for `shell`.
///
/// Alternative locations are checked in priority order before falling back to
/// the conventional path:
///
/// - **zsh**: `$ZDOTDIR/.zshrc` → `~/.zshrc` → `~/.zshenv` (if it exists)
///   → `~/.zshrc` (created fresh)
/// - **bash**: `$HOME/.bash_profile` (if it exists) → `$HOME/.bashrc`
/// - **fish**: `$XDG_CONFIG_HOME/fish/functions/tfe.fish` → `$HOME/.config/…`
/// - **nushell**: `nu_config_dir/config.nu` (platform-dependent; see
///   [`nu_config_dir_default`] for the resolution order)
///
/// Pass explicit overrides (`home`, `xdg_config_home`, `zdotdir`,
/// `bash_profile`, `zshenv`, `nu_config_dir`) to keep tests hermetic without
/// mutating global env vars.
pub fn rc_path_with(
    shell: Shell,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
    zdotdir: Option<&Path>,
    bash_profile: Option<&Path>,
    zshenv: Option<&Path>,
    nu_config_dir: Option<&Path>,
) -> Option<PathBuf> {
    match shell {
        // bash: prefer ~/.bash_profile when it already exists (common on macOS),
        // because bash reads it for interactive login shells and skips .bashrc.
        Shell::Bash => {
            if let Some(bp) = bash_profile {
                if bp.exists() {
                    return Some(bp.to_path_buf());
                }
            }
            home.map(|h| h.join(".bashrc"))
        }
        // zsh startup order: .zshenv (all sessions) → .zprofile (login) →
        // .zshrc (interactive) → .zlogin (login).
        //
        // Priority for writing the wrapper:
        //   1. $ZDOTDIR/.zshrc  — when ZDOTDIR is set this is the interactive rc
        //   2. ~/.zshrc         — standard location when it already exists
        //   3. ~/.zshenv        — colleague uses this instead of .zshrc; write
        //                         there if it exists and .zshrc does not
        //   4. ~/.zshrc         — default: create it fresh
        Shell::Zsh => {
            if let Some(z) = zdotdir {
                return Some(z.join(".zshrc"));
            }
            if let Some(h) = home {
                let zshrc = h.join(".zshrc");
                if zshrc.exists() {
                    return Some(zshrc);
                }
                if let Some(env) = zshenv {
                    if env.exists() {
                        return Some(env.to_path_buf());
                    }
                }
                // Neither exists — create .zshrc fresh.
                return Some(zshrc);
            }
            None
        }
        Shell::Fish => {
            let config = xdg_config_home
                .map(std::path::Path::to_path_buf)
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
        // Nushell: <config-dir>/nushell/config.nu
        // The config dir is platform-dependent (see nu_config_dir_default).
        // An explicit `nu_config_dir` override takes priority so tests stay
        // hermetic.
        Shell::Nu => nu_config_dir.map(|d| d.join("config.nu")),
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
}

fn zdotdir() -> Option<PathBuf> {
    std::env::var_os("ZDOTDIR").map(PathBuf::from)
}

fn bash_profile(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|h| h.join(".bash_profile"))
}

fn zshenv(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|h| h.join(".zshenv"))
}

fn nu_config_dir() -> Option<PathBuf> {
    nu_config_dir_default()
}

// ── Already-installed check ───────────────────────────────────────────────────

/// Return `true` when the `tfe` wrapper snippet is already present in `path`.
///
/// Three signatures are recognised so that manually-added wrappers (without
/// the sentinel) are never duplicated alongside a sentinel-based install:
///
/// 1. The [`SENTINEL`] comment `# tfe-shell-init` — present in all wrappers
///    written by `--init`.
/// 2. The bare POSIX/fish function signatures `tfe()` / `function tfe`
///    preceded by `command tfe` on a nearby line — catches bash/zsh/fish
///    wrappers copy-pasted by hand before `--init` existed.
/// 3. The Nushell signature `def --wrapped tfe` followed by `^tfe` on a
///    nearby line — catches hand-written Nushell wrappers.
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
    // Slow path: detect a hand-written wrapper by looking for known function
    // declaration patterns followed by the tfe invocation within a short window.
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let window_end = (i + 7).min(lines.len());
        // bash / zsh hand-written wrapper
        if (trimmed == "tfe() {" || trimmed == "function tfe")
            && lines[i..window_end]
                .iter()
                .any(|l| l.contains("command tfe"))
        {
            return true;
        }
        // Nushell hand-written wrapper
        if trimmed.starts_with("def --wrapped tfe")
            && lines[i..window_end].iter().any(|l| l.contains("^tfe"))
        {
            return true;
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

// ── Auto-install (first-run) ──────────────────────────────────────────────────

/// Write a `source:<rc_path>` directive to stdout so the shell wrapper
/// can source the newly-installed rc file in the parent shell process.
///
/// This is emitted once — immediately after auto-install — so that the
/// current shell session picks up the `tfe` wrapper function without the
/// user having to open a new terminal or run `source` manually.
///
/// The directive is always terminated with `\n` regardless of any `--null`
/// flag; it is a control message, not a data path.
pub fn emit_source_directive(rc_path: &Path) {
    // Write directly to stdout.  Errors are silently swallowed — failing to
    // emit the directive is not fatal; the user just has to source manually.
    let _ = std::io::stdout()
        .write_all(format!("{}{}\n", SOURCE_DIRECTIVE_PREFIX, rc_path.display()).as_bytes());
    let _ = std::io::stdout().flush();
}

/// Silently install the shell wrapper on first run if it is not already
/// present in any candidate rc file for the detected shell.
///
/// Called automatically at startup before the TUI is shown.  The function
/// is intentionally silent on success except for a single informational line
/// to stderr telling the user what was written and how to activate it.
///
/// Behaviour:
/// - Detects the shell from `$SHELL`.  Does nothing when unrecognised.
/// - Collects **all** candidate rc files for the shell (e.g. both `.zshrc`
///   and `.zshenv` for zsh) and checks each with [`is_installed`].
/// - If the wrapper is found in any candidate, returns immediately.
/// - Otherwise calls [`install_or_print`] to write to the best rc file and
///   prints one line to stderr.
///
/// Returns the [`InitOutcome`] so the caller can surface a post-TUI notice
/// when the snippet was freshly written (the TUI would have swallowed any
/// `eprintln!` emitted during startup via the alternate screen).
pub fn auto_install() -> InitOutcome {
    let h = home();
    auto_install_with(
        None,
        h.as_deref(),
        xdg_config_home().as_deref(),
        zdotdir().as_deref(),
        bash_profile(h.as_deref()).as_deref(),
        zshenv(h.as_deref()).as_deref(),
        nu_config_dir().as_deref(),
    )
}

/// Like [`auto_install`] but with explicit overrides for hermetic testing.
///
/// `shell` — pass `Some(Shell::Zsh)` etc. to skip `$SHELL` detection;
/// `None` falls back to [`detect_shell`] (production behaviour).
pub(crate) fn auto_install_with(
    shell: Option<Shell>,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
    zdotdir: Option<&Path>,
    bash_profile: Option<&Path>,
    zshenv: Option<&Path>,
    nu_config_dir: Option<&Path>,
) -> InitOutcome {
    let shell = match shell.or_else(detect_shell) {
        Some(s) => s,
        None => return InitOutcome::UnknownShell, // unrecognised shell — do nothing silently
    };

    // Build the full list of candidate rc files to check for an existing
    // installation.  We check every file that the shell might source so that
    // a manually-added wrapper in any of them prevents a duplicate install.
    let candidates: Vec<PathBuf> = match shell {
        Shell::Zsh => {
            let mut v = Vec::new();
            if let Some(z) = zdotdir {
                v.push(z.join(".zshrc"));
            }
            if let Some(h) = home {
                v.push(h.join(".zshrc"));
                v.push(h.join(".zshenv"));
                v.push(h.join(".zprofile"));
            }
            v
        }
        Shell::Bash => {
            let mut v = Vec::new();
            if let Some(h) = home {
                v.push(h.join(".bashrc"));
                v.push(h.join(".bash_profile"));
                v.push(h.join(".profile"));
            }
            v
        }
        Shell::Fish => {
            let config = xdg_config_home
                .map(std::path::Path::to_path_buf)
                .or_else(|| home.map(|h| h.join(".config")));
            if let Some(c) = config {
                vec![c.join("fish/functions/tfe.fish")]
            } else {
                vec![]
            }
        }
        Shell::Pwsh => {
            if let Some(profile) = std::env::var_os("PROFILE") {
                vec![PathBuf::from(profile)]
            } else if let Some(h) = home {
                vec![h
                    .join(".config")
                    .join("powershell")
                    .join("Microsoft.PowerShell_profile.ps1")]
            } else {
                vec![]
            }
        }
        Shell::Nu => {
            if let Some(d) = nu_config_dir {
                vec![d.join("config.nu")]
            } else {
                vec![]
            }
        }
    };

    // Already installed in any candidate — nothing to do.
    if candidates.iter().any(|p| is_installed(p)) {
        return InitOutcome::AlreadyInstalled(
            // Return the first candidate that has it installed so callers can
            // display the path if needed.
            candidates
                .into_iter()
                .find(|p| is_installed(p))
                .unwrap_or_default(),
        );
    }

    // Not found anywhere — install silently into the best rc file.
    // Do NOT eprintln! here: at startup the TUI alternate screen is either
    // already active or about to be activated, which would swallow the message.
    // The caller (main.rs) is responsible for surfacing the notice *after* the
    // TUI exits, when stderr is once again connected to a visible terminal.
    install_or_print_to(
        Some(shell),
        home,
        xdg_config_home,
        zdotdir,
        bash_profile,
        zshenv,
        nu_config_dir,
    )
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
/// 2. Resolve the shell rc-file path.
/// 3. If the snippet is already present ([`is_installed`]), return
///    [`InitOutcome::AlreadyInstalled`] without writing anything.
/// 4. Try to [`install`].  On success return [`InitOutcome::Installed`].
/// 5. On failure, print the snippet to stdout so the user can install it
///    manually, and return [`InitOutcome::PrintedToStdout`].
pub fn install_or_print(shell: Option<Shell>) -> InitOutcome {
    let h = home();
    install_or_print_to(
        shell,
        h.as_deref(),
        xdg_config_home().as_deref(),
        zdotdir().as_deref(),
        bash_profile(h.as_deref()).as_deref(),
        zshenv(h.as_deref()).as_deref(),
        nu_config_dir().as_deref(),
    )
}

/// Like [`install_or_print`] but with explicit path overrides for every
/// environment variable that influences rc-file resolution.
///
/// Used in tests to avoid mutating global environment variables.
pub(crate) fn install_or_print_to(
    shell: Option<Shell>,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
    zdotdir: Option<&Path>,
    bash_profile: Option<&Path>,
    zshenv: Option<&Path>,
    nu_config_dir: Option<&Path>,
) -> InitOutcome {
    // On Windows, only PowerShell and Nushell are supported for --init.
    // CMD has no equivalent of shell functions.  WSL users should run the
    // Linux tfe binary inside WSL and use tfe --init bash/zsh/fish there.
    if cfg!(windows) {
        if let Some(s) = shell {
            if s != Shell::Pwsh && s != Shell::Nu {
                eprintln!(
                    "tfe: on Windows only PowerShell and Nushell are supported.\n\
                     Use: tfe --init powershell   or   tfe --init nushell\n\
                     For WSL (bash/zsh/fish) run tfe --init <shell> inside WSL."
                );
                return InitOutcome::UnknownShell;
            }
        }
    }

    // Step 1 — resolve shell.
    let resolved = if let Some(s) = shell.or_else(detect_shell) {
        s
    } else {
        eprintln!(
            "tfe: could not detect shell from $SHELL. \
             Re-run with an explicit shell: tfe --init zsh"
        );
        // Fall back to printing the bash/zsh snippet — most likely to be useful.
        print!("{}", snippet(Shell::Bash));
        return InitOutcome::UnknownShell;
    };

    // Step 2 — resolve rc path.
    let rc = if let Some(p) = rc_path_with(
        resolved,
        home,
        xdg_config_home,
        zdotdir,
        bash_profile,
        zshenv,
        nu_config_dir,
    ) {
        p
    } else {
        eprintln!(
            "tfe: could not determine rc file path ($HOME is not set). \
             Add the following to your shell config manually:"
        );
        print!("{}", snippet(resolved));
        return InitOutcome::PrintedToStdout;
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
mod tests;
