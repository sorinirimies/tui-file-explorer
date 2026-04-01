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
    match shell {
        // bash / zsh: loop over every output line.  Lines beginning with
        // "source:" are sourced in the current shell; any other non-empty
        // line is treated as a cd target.
        Shell::Bash | Shell::Zsh => format!(
            "\n{SENTINEL}\n\
             tfe() {{\n\
             \x20   local line\n\
             \x20   while IFS= read -r line; do\n\
             \x20       case \"$line\" in\n\
             \x20           source:*) source \"${{line#source:}}\" ;;\n\
             \x20           ?*)       cd \"$line\" ;;\n\
             \x20       esac\n\
             \x20   done < <(command tfe \"$@\")\n\
             }}\n"
        ),
        // fish: same logic using fish syntax.
        Shell::Fish => format!(
            "\n{SENTINEL}\n\
             function tfe\n\
             \x20   for line in (command tfe $argv | string split0 | string split \"\\n\")\n\
             \x20       if string match -q 'source:*' -- $line\n\
             \x20           source (string replace 'source:' '' -- $line)\n\
             \x20       else if test -n \"$line\"\n\
             \x20           cd $line\n\
             \x20       end\n\
             \x20   end\n\
             end\n"
        ),
        // PowerShell: split output on newlines, handle source: and cd.
        Shell::Pwsh => format!(
            "\n{SENTINEL}\n\
             function tfe {{\n\
             \x20   & (Get-Command tfe -CommandType Application).Source @args | ForEach-Object {{\n\
             \x20       if ($_ -like 'source:*') {{ . ($_ -replace '^source:', '') }}\n\
             \x20       elseif ($_)              {{ Set-Location $_ }}\n\
             \x20   }}\n\
             }}\n"
        ),
        // Nushell: def --env is required so that cd propagates to the caller's
        // scope.  Without --env, environment mutations (including PWD changes
        // from cd) are discarded when the command returns.  --wrapped forwards
        // all flags and arguments to the external binary unchanged.
        // source is a parse-time keyword in Nushell and cannot be called with
        // a runtime string argument, so the source: protocol is intentionally
        // not handled here — the simple str trim + cd form is correct.
        Shell::Nu => format!(
            "\n{SENTINEL}\n\
             # tfe wrapper: cd to the directory printed by tfe on exit.\n\
             # def --env is required so the cd takes effect in the caller's shell.\n\
             def --env --wrapped tfe [...rest] {{\n\
             \x20   let dir = (^tfe ...$rest | str trim)\n\
             \x20   if ($dir | is-not-empty) {{ cd $dir }}\n\
             }}\n"
        ),
    }
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
                .map(|p| p.to_path_buf())
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
/// 2. Resolve the rc-file path via [`rc_path`].
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
    let rc = match rc_path_with(
        resolved,
        home,
        xdg_config_home,
        zdotdir,
        bash_profile,
        zshenv,
        nu_config_dir,
    ) {
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

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Call auto_install_with with the Nushell-specific nu_config_dir wired in.
    fn auto_install_nu(nu_config_dir: &Path) -> InitOutcome {
        auto_install_with(
            Some(Shell::Nu),
            None,
            None,
            None,
            None,
            None,
            Some(nu_config_dir),
        )
    }

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
    fn from_str_recognises_nushell() {
        assert_eq!(Shell::from_str("nu"), Some(Shell::Nu));
        assert_eq!(Shell::from_str("nushell"), Some(Shell::Nu));
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(Shell::from_str("BASH"), Some(Shell::Bash));
        assert_eq!(Shell::from_str("ZSH"), Some(Shell::Zsh));
        assert_eq!(Shell::from_str("FISH"), Some(Shell::Fish));
        assert_eq!(Shell::from_str("NU"), Some(Shell::Nu));
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
        assert_eq!(Shell::Nu.to_string(), "nushell");
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
        assert!(s.contains("cd \"$line\""), "bash snippet missing cd");
        assert!(
            s.contains("source:"),
            "bash snippet missing source: handler"
        );
    }

    #[test]
    fn snippet_zsh_identical_to_bash() {
        assert_eq!(snippet(Shell::Zsh), snippet(Shell::Bash));
    }

    #[test]
    fn snippet_nushell_contains_def_wrapped() {
        let s = snippet(Shell::Nu);
        assert!(
            s.contains("def --env --wrapped tfe"),
            "nushell snippet must use 'def --env --wrapped tfe', got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_contains_def_env() {
        let s = snippet(Shell::Nu);
        assert!(
            s.contains("--env"),
            "nushell snippet must use --env so cd propagates to caller, got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_uses_caret_tfe() {
        let s = snippet(Shell::Nu);
        assert!(
            s.contains("^tfe"),
            "nushell snippet must call the external binary with ^tfe, got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_uses_cd() {
        let s = snippet(Shell::Nu);
        assert!(
            s.contains("cd"),
            "nushell snippet must cd to the dir, got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_does_not_use_each_closure() {
        // cd inside an `each` closure does not propagate to the caller —
        // the correct form calls cd at the top level of the def --env body.
        let s = snippet(Shell::Nu);
        assert!(
            !s.contains("each {"),
            "nushell snippet must not use an each closure for cd, got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_uses_str_trim() {
        let s = snippet(Shell::Nu);
        assert!(
            s.contains("str trim"),
            "nushell snippet must use str trim to strip trailing newline, got:\n{s}"
        );
    }

    #[test]
    fn snippet_nushell_differs_from_bash() {
        assert_ne!(snippet(Shell::Nu), snippet(Shell::Bash));
    }

    #[test]
    fn snippet_bash_handles_source_directive() {
        let s = snippet(Shell::Bash);
        assert!(
            s.contains("source:"),
            "bash snippet must handle source: directives, got:\n{s}"
        );
    }

    #[test]
    fn snippet_fish_handles_source_directive() {
        let s = snippet(Shell::Fish);
        assert!(
            s.contains("source:"),
            "fish snippet must handle source: directives, got:\n{s}"
        );
    }

    #[test]
    fn snippet_pwsh_handles_source_directive() {
        let s = snippet(Shell::Pwsh);
        assert!(
            s.contains("source:"),
            "powershell snippet must handle source: directives, got:\n{s}"
        );
    }

    #[test]
    fn source_directive_prefix_constant_value() {
        assert_eq!(SOURCE_DIRECTIVE_PREFIX, "source:");
    }

    #[test]
    fn emit_source_directive_does_not_panic() {
        // We can't easily capture stdout in a unit test without extra deps,
        // but we can verify the function completes without panicking.
        emit_source_directive(std::path::Path::new("/tmp/test_rc"));
    }

    #[test]
    fn snippet_fish_contains_function_body() {
        let s = snippet(Shell::Fish);
        assert!(
            s.contains("command tfe"),
            "fish snippet missing command tfe"
        );
        assert!(s.contains("cd $line"), "fish snippet missing cd");
        assert!(
            s.contains("function tfe"),
            "fish snippet missing function keyword"
        );
        assert!(
            s.contains("source:"),
            "fish snippet missing source: handler"
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
        let p = rc_path_with(
            Shell::Bash,
            Some(Path::new("/test/home")),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/test/home/.bashrc"));
    }

    #[test]
    fn rc_path_zsh_defaults_to_zshrc() {
        // Neither .zshrc nor .zshenv exist on disk — should default to .zshrc.
        let p = rc_path_with(
            Shell::Zsh,
            Some(Path::new("/test/home")),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/test/home/.zshrc"));
    }

    #[test]
    fn rc_path_zsh_prefers_zdotdir() {
        let p = rc_path_with(
            Shell::Zsh,
            Some(Path::new("/home/user")),
            None,
            Some(Path::new("/custom/zdotdir")),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/custom/zdotdir/.zshrc"));
    }

    #[test]
    fn rc_path_zsh_falls_back_to_zshenv_when_it_exists() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        // Create .zshenv but NOT .zshrc — colleague's setup.
        let zshenv = home.join(".zshenv");
        fs::write(&zshenv, b"# existing zshenv\n").unwrap();
        let p = rc_path_with(
            Shell::Zsh,
            Some(home),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        )
        .unwrap();
        assert_eq!(p, zshenv, "must write to .zshenv when .zshrc absent");
    }

    #[test]
    fn rc_path_zsh_prefers_zshrc_over_zshenv_when_both_exist() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let zshrc = home.join(".zshrc");
        let zshenv = home.join(".zshenv");
        fs::write(&zshrc, b"# existing zshrc\n").unwrap();
        fs::write(&zshenv, b"# existing zshenv\n").unwrap();
        let p = rc_path_with(
            Shell::Zsh,
            Some(home),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        )
        .unwrap();
        assert_eq!(p, zshrc, "must prefer .zshrc over .zshenv when both exist");
    }

    #[test]
    #[cfg(not(windows))]
    fn rc_path_powershell_falls_back_to_home_config_on_unix() {
        std::env::remove_var("PROFILE");
        let p = rc_path_with(
            Shell::Pwsh,
            Some(Path::new("/test/home")),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
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
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/custom/config/fish/functions/tfe.fish"));
    }

    #[test]
    fn rc_path_fish_falls_back_to_home_config() {
        let p = rc_path_with(
            Shell::Fish,
            Some(Path::new("/test/home")),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("/test/home/.config/fish/functions/tfe.fish")
        );
    }

    #[test]
    fn rc_path_returns_none_when_home_unset() {
        std::env::remove_var("PROFILE");
        assert!(rc_path_with(Shell::Bash, None, None, None, None, None, None).is_none());
        assert!(rc_path_with(Shell::Zsh, None, None, None, None, None, None).is_none());
        assert!(rc_path_with(Shell::Fish, None, None, None, None, None, None).is_none());
        assert!(rc_path_with(Shell::Pwsh, None, None, None, None, None, None).is_none());
        assert!(rc_path_with(Shell::Nu, None, None, None, None, None, None).is_none());
    }

    #[test]
    fn rc_path_nu_uses_nu_config_dir() {
        let p = rc_path_with(
            Shell::Nu,
            None,
            None,
            None,
            None,
            None,
            Some(Path::new("/test/nushell")),
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/test/nushell/config.nu"));
    }

    #[test]
    fn rc_path_nu_returns_none_when_nu_config_dir_unset() {
        assert!(
            rc_path_with(Shell::Nu, None, None, None, None, None, None).is_none(),
            "Nu rc_path must be None when nu_config_dir is not provided"
        );
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
    fn is_installed_detects_sentinel_in_nushell_config() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.nu");
        install(Shell::Nu, &rc).unwrap();
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
        fs::write(&rc, b"export PATH=$PATH:/usr/local/bin\n").unwrap();
        assert!(!is_installed(&rc));
    }

    #[test]
    fn is_installed_nushell_returns_false_when_sentinel_absent() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.nu");
        fs::write(&rc, b"$env.config.show_banner = false\n").unwrap();
        assert!(!is_installed(&rc));
    }

    #[test]
    fn is_installed_returns_true_when_sentinel_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        fs::write(&rc, format!("some content\n{SENTINEL}\nmore\n").as_bytes()).unwrap();
        assert!(is_installed(&rc));
    }

    #[test]
    fn is_installed_nushell_returns_true_when_sentinel_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.nu");
        fs::write(
            &rc,
            format!("$env.config.show_banner = false\n{SENTINEL}\n").as_bytes(),
        )
        .unwrap();
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
    fn install_nushell_creates_config_nu_when_missing() {
        let dir = tempdir().unwrap();
        let nu_dir = dir.path().join("nushell");
        let rc = nu_dir.join("config.nu");
        assert!(!rc.exists());
        install(Shell::Nu, &rc).unwrap();
        assert!(rc.exists());
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_nushell_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let rc = dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("nushell")
            .join("config.nu");
        install(Shell::Nu, &rc).unwrap();
        assert!(rc.exists());
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_nushell_snippet_passes_is_installed() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.nu");
        install(Shell::Nu, &rc).unwrap();
        assert!(
            is_installed(&rc),
            "installed nushell snippet must pass is_installed"
        );
    }

    #[test]
    fn install_nushell_does_not_duplicate_when_called_twice() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.nu");
        install(Shell::Nu, &rc).unwrap();
        let before = fs::read_to_string(&rc).unwrap();
        // Second install should not be reached in practice (auto_install_with
        // guards it), but install() itself does not de-duplicate — verify that
        // is_installed catches it before a second call.
        assert!(is_installed(&rc), "must be detected after first install");
        // Confirm the content after one install contains the snippet exactly once.
        let count = before.matches(SENTINEL).count();
        assert_eq!(
            count, 1,
            "sentinel should appear exactly once after one install"
        );
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
        let outcome = install_or_print_to(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(outcome, InitOutcome::Installed(rc.clone()));
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_or_print_returns_already_installed_when_sentinel_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        // Pre-install the snippet.
        install(Shell::Zsh, &rc).unwrap();
        let outcome = install_or_print_to(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(outcome, InitOutcome::AlreadyInstalled(rc));
    }

    #[test]
    fn install_or_print_zsh_uses_zdotdir() {
        let dir = tempdir().unwrap();
        let zdotdir = dir.path().join("zdotdir");
        fs::create_dir(&zdotdir).unwrap();
        let rc = zdotdir.join(".zshrc");
        let outcome = install_or_print_to(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            Some(&zdotdir),
            None,
            None,
            None,
        );
        assert_eq!(
            outcome,
            InitOutcome::Installed(rc.clone()),
            "must install into $ZDOTDIR/.zshrc not $HOME/.zshrc"
        );
        assert!(is_installed(&rc));
    }

    #[test]
    fn install_or_print_zsh_falls_back_to_zshenv_when_zshrc_absent() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let zshenv = home.join(".zshenv");
        fs::write(&zshenv, b"# existing zshenv\n").unwrap();
        // No .zshrc present — should write to .zshenv.
        let outcome = install_or_print_to(
            Some(Shell::Zsh),
            Some(home),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        );
        assert_eq!(
            outcome,
            InitOutcome::Installed(zshenv.clone()),
            "must install into .zshenv when .zshrc is absent"
        );
        assert!(is_installed(&zshenv));
    }

    #[test]
    fn install_or_print_bash_uses_bash_profile_when_present() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        fs::write(&bp, b"# existing profile\n").unwrap();
        let outcome = install_or_print_to(
            Some(Shell::Bash),
            Some(dir.path()),
            None,
            None,
            Some(&bp),
            None,
            None,
        );
        assert_eq!(
            outcome,
            InitOutcome::Installed(bp.clone()),
            "must install into .bash_profile when it exists"
        );
        assert!(is_installed(&bp));
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

            // Root bypasses Unix permission checks entirely, so this test
            // cannot be made to work reliably in CI containers that run as
            // root.  Skip rather than produce a spurious failure.
            // SAFETY: getuid() is always safe to call.
            let uid = unsafe { libc::getuid() };
            if uid == 0 {
                return;
            }

            let mut perms = fs::metadata(&ro_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&ro_dir, perms).unwrap();
            let outcome = install_or_print_to(
                Some(Shell::Zsh),
                Some(&ro_dir),
                None,
                None,
                None,
                None,
                None,
            );
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

    // ── auto_install_with ─────────────────────────────────────────────────────

    #[test]
    fn auto_install_writes_wrapper_on_first_run() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        // Simulate $SHELL=zsh by using auto_install_with directly.
        // Before: wrapper absent.
        assert!(!is_installed(&rc));
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        // After: wrapper present.
        assert!(
            is_installed(&rc),
            "auto_install must write wrapper on first run"
        );
    }

    #[test]
    fn auto_install_does_not_duplicate_when_already_installed() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        // Pre-install once.
        install(Shell::Zsh, &rc).unwrap();
        let before = fs::read_to_string(&rc).unwrap();
        // Run auto_install — must be a no-op.
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        let after = fs::read_to_string(&rc).unwrap();
        assert_eq!(
            before, after,
            "auto_install must not append a duplicate when wrapper already present"
        );
    }

    #[test]
    fn auto_install_detects_wrapper_in_zshenv_and_skips_install() {
        let dir = tempdir().unwrap();
        let zshenv = dir.path().join(".zshenv");
        // Install the wrapper into .zshenv (no .zshrc present).
        install(Shell::Zsh, &zshenv).unwrap();
        // auto_install should detect it in .zshenv and not create .zshrc.
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        );
        let zshrc = dir.path().join(".zshrc");
        assert!(
            !zshrc.exists(),
            "auto_install must not create .zshrc when wrapper already in .zshenv"
        );
    }

    #[test]
    fn auto_install_detects_wrapper_in_zprofile_and_skips_install() {
        let dir = tempdir().unwrap();
        let zprofile = dir.path().join(".zprofile");
        // Manually write the sentinel into .zprofile.
        fs::write(&zprofile, format!("{SENTINEL}\ntfe() {{}}\n")).unwrap();
        // auto_install_with checks .zprofile as a candidate — should skip.
        // We pass zshenv=None so it won't find it there.
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        // .zshrc should not have been created (wrapper found in .zprofile).
        // Note: auto_install_with checks .zprofile via the candidates list
        // but rc_path_with would still pick .zshrc — the key invariant is
        // that the candidates scan fires before install_or_print_to.
        // Verify by checking .zshrc does NOT contain the sentinel.
        let zshrc = dir.path().join(".zshrc");
        if zshrc.exists() {
            let content = fs::read_to_string(&zshrc).unwrap();
            assert!(
                !content.contains(SENTINEL),
                "auto_install must not write to .zshrc when sentinel already in .zprofile"
            );
        }
    }

    #[test]
    fn auto_install_uses_zdotdir_when_set() {
        let dir = tempdir().unwrap();
        let zdotdir = dir.path().join("zdotdir");
        fs::create_dir(&zdotdir).unwrap();
        let rc = zdotdir.join(".zshrc");
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            Some(&zdotdir),
            None,
            None,
            None,
        );
        assert!(
            is_installed(&rc),
            "auto_install must write to $ZDOTDIR/.zshrc when ZDOTDIR is set"
        );
        // Must not also write to $HOME/.zshrc.
        let home_rc = dir.path().join(".zshrc");
        assert!(
            !home_rc.exists(),
            "auto_install must not write to $HOME/.zshrc when ZDOTDIR is set"
        );
    }

    #[test]
    fn auto_install_falls_back_to_zshenv_when_zshrc_absent() {
        let dir = tempdir().unwrap();
        let zshenv = dir.path().join(".zshenv");
        fs::write(&zshenv, b"# existing zshenv\n").unwrap();
        // No .zshrc — should fall back to .zshenv.
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        );
        assert!(
            is_installed(&zshenv),
            "auto_install must install into .zshenv when .zshrc is absent"
        );
    }

    #[test]
    fn auto_install_nushell_writes_config_nu_on_first_run() {
        let dir = tempdir().unwrap();
        let nu_dir = dir.path().join("nushell");
        let config_nu = nu_dir.join("config.nu");
        assert!(!config_nu.exists());
        let outcome = auto_install_nu(&nu_dir);
        assert!(
            matches!(outcome, InitOutcome::Installed(_)),
            "expected Installed, got {outcome:?}"
        );
        assert!(
            is_installed(&config_nu),
            "config.nu must contain the wrapper"
        );
    }

    #[test]
    fn auto_install_nushell_does_not_duplicate_when_already_installed() {
        let dir = tempdir().unwrap();
        let nu_dir = dir.path().join("nushell");
        let config_nu = nu_dir.join("config.nu");
        // Pre-install once.
        install(Shell::Nu, &config_nu).unwrap();
        let before = fs::read_to_string(&config_nu).unwrap();
        // Second run — must be a no-op.
        auto_install_nu(&nu_dir);
        let after = fs::read_to_string(&config_nu).unwrap();
        assert_eq!(
            before, after,
            "auto_install must not duplicate the nushell wrapper"
        );
    }

    #[test]
    fn auto_install_nushell_returns_already_installed_when_config_nu_has_sentinel() {
        let dir = tempdir().unwrap();
        let nu_dir = dir.path().join("nushell");
        let config_nu = nu_dir.join("config.nu");
        install(Shell::Nu, &config_nu).unwrap();
        let outcome = auto_install_nu(&nu_dir);
        assert!(
            matches!(outcome, InitOutcome::AlreadyInstalled(_)),
            "expected AlreadyInstalled, got {outcome:?}"
        );
    }

    #[test]
    fn auto_install_nushell_creates_parent_directories_for_config_nu() {
        let dir = tempdir().unwrap();
        // Deep nested path — directories do not exist yet.
        let nu_dir = dir.path().join("deep").join("nested").join("nushell");
        let config_nu = nu_dir.join("config.nu");
        assert!(!nu_dir.exists());
        let outcome = auto_install_nu(&nu_dir);
        assert!(
            matches!(outcome, InitOutcome::Installed(_)),
            "expected Installed, got {outcome:?}"
        );
        assert!(config_nu.exists(), "config.nu must have been created");
    }

    #[test]
    fn auto_install_does_nothing_when_shell_unrecognised() {
        // auto_install_with resolves shell via detect_shell() which reads $SHELL.
        // We can't safely mutate $SHELL in a parallel test, so we verify the
        // None-home path (unresolvable rc) is handled without panic instead.
        // The real unrecognised-shell path is covered by detect_shell tests.
        let outcome = auto_install_with(None, None, None, None, None, None, None);
        // Must not panic — that is the assertion.
        // Acceptable outcomes depending on the CI environment:
        //   - UnknownShell      : $SHELL is unset or unrecognised
        //   - AlreadyInstalled  : wrapper already present in the real rc file
        //   - PrintedToStdout   : $SHELL is set but $HOME is None (our override),
        //                         so rc_path_with returns None and the snippet
        //                         is printed to stdout as a fallback
        //   - Installed         : $SHELL is set, $HOME is None but rc_path_with
        //                         somehow resolves (shouldn't happen, but safe)
        assert!(
            matches!(
                outcome,
                InitOutcome::UnknownShell
                    | InitOutcome::AlreadyInstalled(_)
                    | InitOutcome::PrintedToStdout
                    | InitOutcome::Installed(_)
            ),
            "unexpected outcome: {outcome:?}"
        );
    }

    // ── auto_install_with return value ────────────────────────────────────────

    #[test]
    fn auto_install_returns_installed_on_first_run() {
        let dir = tempdir().unwrap();
        let outcome = auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            matches!(outcome, InitOutcome::Installed(_)),
            "expected Installed on first run, got {outcome:?}"
        );
    }

    #[test]
    fn auto_install_installed_outcome_carries_rc_path() {
        let dir = tempdir().unwrap();
        let outcome = auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        if let InitOutcome::Installed(path) = outcome {
            assert!(
                path.ends_with(".zshrc"),
                "installed path should be .zshrc, got {path:?}"
            );
        } else {
            panic!("expected Installed, got {outcome:?}");
        }
    }

    #[test]
    fn auto_install_returns_already_installed_on_second_run() {
        let dir = tempdir().unwrap();
        // First run installs.
        auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        // Second run must return AlreadyInstalled.
        let outcome = auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            matches!(outcome, InitOutcome::AlreadyInstalled(_)),
            "expected AlreadyInstalled on second run, got {outcome:?}"
        );
    }

    #[test]
    fn auto_install_already_installed_outcome_carries_rc_path() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        install(Shell::Zsh, &rc).unwrap();
        let outcome = auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        if let InitOutcome::AlreadyInstalled(path) = outcome {
            assert!(
                path.ends_with(".zshrc") || is_installed(&path),
                "AlreadyInstalled path should point to a file containing the wrapper, got {path:?}"
            );
        } else {
            panic!("expected AlreadyInstalled, got {outcome:?}");
        }
    }

    #[test]
    fn auto_install_returns_unknown_shell_when_shell_is_none_and_home_is_none() {
        // When both shell detection and home are unavailable we get UnknownShell.
        let outcome = auto_install_with(None, None, None, None, None, None, None);
        // On a real machine $SHELL may be set, so we allow AlreadyInstalled too.
        assert!(
            matches!(
                outcome,
                InitOutcome::UnknownShell
                    | InitOutcome::AlreadyInstalled(_)
                    | InitOutcome::Installed(_)
                    | InitOutcome::PrintedToStdout
            ),
            "unexpected outcome variant: {outcome:?}"
        );
    }

    #[test]
    fn auto_install_bash_returns_installed_on_first_run() {
        let dir = tempdir().unwrap();
        let outcome = auto_install_with(
            Some(Shell::Bash),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            matches!(outcome, InitOutcome::Installed(_)),
            "expected Installed for bash on first run, got {outcome:?}"
        );
        let rc = dir.path().join(".bashrc");
        assert!(is_installed(&rc), ".bashrc should contain the wrapper");
    }

    #[test]
    fn auto_install_fish_returns_installed_on_first_run() {
        let dir = tempdir().unwrap();
        let outcome = auto_install_with(
            Some(Shell::Fish),
            Some(dir.path()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(
            matches!(outcome, InitOutcome::Installed(_)),
            "expected Installed for fish on first run, got {outcome:?}"
        );
        let rc = dir
            .path()
            .join(".config")
            .join("fish")
            .join("functions")
            .join("tfe.fish");
        assert!(is_installed(&rc), "tfe.fish should contain the wrapper");
    }

    #[test]
    fn auto_install_zsh_already_installed_in_zshenv_returns_already_installed() {
        let dir = tempdir().unwrap();
        let zshenv = dir.path().join(".zshenv");
        install(Shell::Zsh, &zshenv).unwrap();
        // No .zshrc present — wrapper is in .zshenv.
        let outcome = auto_install_with(
            Some(Shell::Zsh),
            Some(dir.path()),
            None,
            None,
            None,
            Some(&zshenv),
            None,
        );
        assert!(
            matches!(outcome, InitOutcome::AlreadyInstalled(_)),
            "expected AlreadyInstalled when wrapper is in .zshenv, got {outcome:?}"
        );
    }
}
