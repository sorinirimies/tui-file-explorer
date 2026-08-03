//! # tfe — two-pane terminal file explorer
//!
//! A keyboard-driven dual-pane file manager.
//!
//! ## Shell integration
//!
//! Add the wrapper function to your shell rc file so that dismissing `tfe`
//! automatically `cd`s the terminal to the pane's current directory:
//!
//! ```bash
//! # bash / zsh — put this in ~/.bashrc or ~/.zshrc
//! tfe() {
//!     local dir
//!     dir=$(command tfe "$@")
//!     [ -n "$dir" ] && cd "$dir"
//! }
//! ```
//!
//! ```fish
//! # fish — put this in ~/.config/fish/functions/tfe.fish
//! function tfe
//!     set dir (command tfe $argv)
//!     if test -n "$dir"
//!         cd $dir
//!     end
//! end
//! ```
//!
//! Other examples:
//!
//! ```bash
//! tfe | xargs -r $EDITOR           # open selected file in $EDITOR
//! tfe -e rs | pbcopy               # copy a .rs path to the clipboard
//! tfe --theme catppuccin-mocha     # choose a colour theme
//! tfe --single-pane                # start in single-pane mode
//! tfe --show-themes                # open theme panel on startup
//! tfe --list-themes                # list all themes and exit
//! tfe --editor nvim                # set editor for the e key
//! tfe --editor "code --wait"       # custom editor (quoted)
//! ```
//!
//! Exit codes:
//!   0 — path printed to stdout (file selected, or dismissed with a current dir)
//!   2 — bad arguments / I/O error

mod doctor;
mod info;
mod shell_init;

use tui_file_explorer::app::Editor;

use std::{
    fs::File,
    io::{self, stdout, Write},
    path::PathBuf,
    process,
};

use clap::Parser;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_file_explorer::{
    draw, load_state, resolve_output_path, resolve_theme_idx, save_state, App, AppOptions,
    AppState, PreviewState, Theme,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "tfe",
    version,
    about = "Keyboard-driven two-pane terminal file explorer",
    after_help = "\
SHELL INTEGRATION (cd on exit):\n\
  Step 1 — enable the feature (persisted across sessions):\n\
    tfe --cd\n\
\n\
  Step 2 — install the shell wrapper (one time):\n\
    tfe --init bash        # ~/.bashrc\n\
    tfe --init zsh         # ~/.zshrc\n\
    tfe --init fish        # ~/.config/fish/functions/tfe.fish\n\
    tfe --init powershell  # $PROFILE (Windows / cross-platform PowerShell)\n\
    tfe --init nushell     # <config-dir>/nushell/config.nu\n\
\n\
  After both steps, dismissing tfe with Esc/q cd's your terminal to the\n\
  directory you were browsing.  Works on macOS, Linux, and Windows.\n\
\n\
  Open selected file in $EDITOR:     command tfe | xargs -r $EDITOR\n\
  NUL-delimited output:              command tfe -0 | xargs -0 wc -l"
)]
struct Cli {
    /// Starting directory or file to open [default: current directory]
    ///
    /// If PATH is a directory, the TUI file explorer opens there.
    /// If PATH is a file and an editor is configured (--editor or persisted),
    /// the file is opened directly in that editor without launching the TUI.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Only show/select files with these extensions (repeatable: -e rs -e toml)
    #[arg(short, long = "ext", value_name = "EXT", action = clap::ArgAction::Append)]
    extensions: Vec<String>,

    /// Show hidden (dot-file) entries on startup
    #[arg(short = 'H', long)]
    hidden: bool,

    /// Colour theme [default: persisted selection, or "default"]. Use --list-themes to see options.
    #[arg(short, long, value_name = "THEME")]
    theme: Option<String>,

    /// List all available themes and exit
    #[arg(long)]
    list_themes: bool,

    /// Open the theme panel on startup (toggle at runtime with T)
    #[arg(long)]
    show_themes: bool,

    /// Start in single-pane mode (toggle at runtime with w)
    #[arg(long)]
    single_pane: bool,

    /// Print the selected file's parent directory instead of the full path
    #[arg(long)]
    print_dir: bool,

    /// Terminate output with a NUL byte instead of a newline
    #[arg(short = '0', long)]
    null: bool,

    /// Install the shell wrapper for cd-on-exit integration and exit.
    /// Install the shell wrapper for `shell` (auto-detected when `None`).
    ///
    /// Appends the wrapper function to your rc file (creating it if needed)
    /// and prints instructions.  Supported shells: bash, zsh, fish, powershell, nushell.
    /// Examples: tfe --init zsh   tfe --init nushell   tfe --init powershell
    #[arg(long, value_name = "SHELL")]
    init: Option<String>,

    /// Enable cd-on-exit: on dismiss, print the active pane's current directory
    /// to stdout so the shell wrapper can cd to it.  The setting is persisted —
    /// run once to enable, run with --no-cd to disable.
    #[arg(long = "cd", overrides_with = "no_cd")]
    cd_on_exit: bool,

    /// Disable cd-on-exit (persisted).  Dismissing without a selection will
    /// print nothing and exit with code 1.
    #[arg(long = "no-cd", overrides_with = "cd_on_exit")]
    no_cd: bool,

    /// Print version, platform, and environment info, then exit.
    #[arg(long)]
    info: bool,

    /// Run diagnostic checks (environment, shell integration, config,
    /// terminal, editor) and exit.  Each check is assessed with
    /// pass / warn / fail indicators and actionable advice.
    #[arg(long)]
    doctor: bool,

    /// Enable verbose mode: print startup diagnostics to stderr, write all
    /// logs to a file (path printed at launch), and show a debug log panel
    /// inside the TUI.  Use `tail -f /tmp/tfe-debug.log` from another
    /// terminal for real-time monitoring.
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Editor to open when pressing `e` on a file.
    ///
    /// Accepted values: helix (hx), nvim, vim, nano, micro, or any binary
    /// name / path.  Overrides the persisted setting for this session only.
    ///
    /// Examples:
    ///   tfe --editor nvim
    ///   tfe --editor vim
    ///   tfe --editor "code --wait"
    #[arg(long, value_name = "EDITOR")]
    editor: Option<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("tfe: {e}");
        process::exit(2);
    }
}

/// Process-start instant used by [`vlog!`] to stamp every log line.
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Conditionally log a message to stderr, a buffer, and an optional file.
///
/// Uses `\r\n` line endings so the output is readable even after
/// `enable_raw_mode()` (which disables output processing, meaning a bare
/// `\n` no longer implies a carriage return).
macro_rules! vlog {
    ($verbose:expr, $log:expr, $file:expr, $($arg:tt)*) => {
        if $verbose {
            let elapsed = START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_secs_f64();
            let msg = format!("[tfe {elapsed:.3}s] {}", format!($($arg)*));
            let _ = std::io::Write::write_all(
                &mut std::io::stderr(),
                format!("{}\r\n", msg).as_bytes(),
            );
            $log.push(msg.clone());
            if let Some(ref mut f) = $file {
                let _ = writeln!(f, "{msg}");
                let _ = f.flush();
            }
        }
    };
}

/// Open (or create) the debug log file when `--verbose` is active.
fn open_debug_log_file() -> Option<File> {
    let path = std::env::temp_dir().join("tfe-debug.log");
    match File::options().create(true).append(true).open(&path) {
        Ok(mut f) => {
            // Write a session separator so consecutive runs are easy to tell apart.
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "\n--- tfe session {timestamp} ---");
            let _ = f.flush();
            eprintln!("[tfe] debug log: {}\r", path.display());
            Some(f)
        }
        Err(e) => {
            eprintln!("[tfe] warning: could not create log file: {e}\r");
            None
        }
    }
}

fn run() -> io::Result<()> {
    let cli = Cli::parse();
    let verbose = cli.verbose;
    let mut log_buf: Vec<String> = Vec::new();
    let mut log_file: Option<File> = if verbose { open_debug_log_file() } else { None };

    vlog!(
        verbose,
        log_buf,
        log_file,
        "version {}",
        env!("CARGO_PKG_VERSION")
    );
    vlog!(
        verbose,
        log_buf,
        log_file,
        "os={} arch={}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    vlog!(
        verbose,
        log_buf,
        log_file,
        "args: {:?}",
        std::env::args().collect::<Vec<_>>()
    );
    vlog!(
        verbose,
        log_buf,
        log_file,
        "$HOME={:?} $SHELL={:?}",
        std::env::var("HOME").ok(),
        std::env::var("SHELL").ok()
    );
    vlog!(
        verbose,
        log_buf,
        log_file,
        "terminal size={:?} stderr_is_tty={}",
        crossterm::terminal::size().ok(),
        crossterm::tty::IsTty::is_tty(&io::stderr())
    );

    // --info: print version / platform / environment and exit.
    if cli.info {
        info::print_info();
        return Ok(());
    }

    // --doctor: run diagnostic checks and exit.
    if cli.doctor {
        doctor::run_doctor();
        return Ok(());
    }

    // --init: install the shell wrapper (or print it as fallback) then exit.
    vlog!(verbose, log_buf, log_file, "checking --init flag");
    if let Some(ref shell_name) = cli.init {
        let shell = match shell_init::Shell::from_str(shell_name) {
            Some(s) => Some(s),
            None => {
                eprintln!(
                    "tfe: unrecognised shell '{shell_name}'. \
                     Supported: bash, zsh, fish, powershell, nushell"
                );
                process::exit(2);
            }
        };
        use shell_init::InitOutcome;
        match shell_init::install_or_print(shell) {
            InitOutcome::Installed(path) => {
                eprintln!("tfe: shell integration installed to {}", path.display());
                eprintln!("  Activate now : source {}", path.display());
                eprintln!("  Or just open a new terminal window.");
            }
            InitOutcome::AlreadyInstalled(path) => {
                eprintln!(
                    "tfe: shell integration already present in {}",
                    path.display()
                );
            }
            InitOutcome::PrintedToStdout | InitOutcome::UnknownShell => {
                // Diagnostic already printed by install_or_print.
            }
        }
        return Ok(());
    }

    // Auto-install the shell wrapper on first run if not already present.
    // Runs before terminal setup so stderr is still connected to the terminal.
    // We store the outcome and surface a notice *after* the TUI exits so that
    // the message is not swallowed by the alternate screen.
    //
    // Note: shell_init::auto_install() internally calls nu_config_dir_default()
    // to resolve the Nushell config path on all platforms, so no extra argument
    // is needed at this call site.
    vlog!(
        verbose,
        log_buf,
        log_file,
        "running auto_install (shell wrapper)"
    );
    let auto_install_outcome = shell_init::auto_install();
    vlog!(
        verbose,
        log_buf,
        log_file,
        "auto_install outcome: {:?}",
        auto_install_outcome
    );

    let themes = Theme::all_presets();
    vlog!(verbose, log_buf, log_file, "loaded {} themes", themes.len());

    // --list-themes: print catalogue and exit.
    if cli.list_themes {
        let max = themes.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);
        println!("{:<width$}  DESCRIPTION", "THEME", width = max);
        println!("{}", "\u{2500}".repeat(max + 52));
        for (name, desc, _) in &themes {
            println!("{name:<width$}  {desc}", width = max);
        }
        return Ok(());
    }

    // Load all persisted state once at startup.
    vlog!(verbose, log_buf, log_file, "loading persisted state");
    let saved = load_state();
    vlog!(
        verbose,
        log_buf,
        log_file,
        "state: theme={:?} last_dir={:?} cd_on_exit={:?} editor={:?}",
        saved.theme,
        saved.last_dir,
        saved.cd_on_exit,
        saved.editor
    );

    // Priority: explicit --theme flag > persisted selection > built-in default.
    let theme_name = cli
        .theme
        .or_else(|| saved.theme.clone())
        .unwrap_or_else(|| "default".to_string());
    let theme_idx = resolve_theme_idx(&theme_name, &themes);
    vlog!(
        verbose,
        log_buf,
        log_file,
        "resolved theme: '{}' (index {})",
        theme_name,
        theme_idx
    );

    // ── Fix misparse: `tfe --editor file.txt` ─────────────────────────────────
    //
    // When the user types `tfe --editor file.txt`, clap consumes `file.txt` as
    // the value for `--editor` and leaves `path` as None.  Detect this case:
    //   • `--editor` value is NOT a known editor name
    //   • `--editor` value exists on disk as a file
    //   • no PATH argument was provided
    // When all three are true, treat the editor value as the PATH and fall back
    // to the persisted (or default) editor.
    // Known editor names that `--editor` legitimately accepts.
    // Anything NOT in this list that also exists as a file on disk
    // was almost certainly meant as the PATH argument.
    const KNOWN_EDITORS: &[&str] = &[
        "none",
        "helix",
        "hx",
        "nvim",
        "vim",
        "nano",
        "micro",
        "emacs",
        "vscode",
        "code",
        "zed",
        "xcode",
        "android-studio",
        "rustrover",
        "intellij",
        "webstorm",
        "pycharm",
        "goland",
        "clion",
        "fleet",
        "sublime",
        "rubymine",
        "phpstorm",
        "rider",
        "eclipse",
    ];

    let (cli_editor, cli_path) = match (&cli.editor, &cli.path) {
        (Some(editor_val), None) => {
            let is_known_editor = KNOWN_EDITORS
                .iter()
                .any(|&name| name.eq_ignore_ascii_case(editor_val));
            let exists_as_file = std::path::Path::new(editor_val).is_file();

            if !is_known_editor && exists_as_file {
                // `--editor` swallowed the file path — correct it.
                (None, Some(PathBuf::from(editor_val)))
            } else {
                (Some(editor_val.clone()), None)
            }
        }
        _ => (cli.editor.clone(), cli.path.clone()),
    };

    // Resolve editor early — needed before path handling so `tfe <file>` can
    // open the file directly in the editor without launching the TUI.
    //
    // Priority: CLI flag > persisted value > compiled-in default.
    let editor = if let Some(ref raw) = cli_editor {
        Editor::from_key(raw).unwrap_or_else(|| Editor::Custom(raw.clone()))
    } else if let Some(ref raw) = saved.editor {
        Editor::from_key(raw).unwrap_or_default()
    } else {
        Editor::default()
    };
    vlog!(
        verbose,
        log_buf,
        log_file,
        "editor: {} (binary: {:?})",
        editor.label(),
        editor.binary()
    );

    // Priority: explicit --path arg > persisted last directory > cwd.
    //
    // When PATH points to a file (not a directory), open it directly in the
    // configured editor without launching the TUI.  This lets `tfe myfile.rs`
    // behave like a quick "open in editor" shortcut.
    vlog!(
        verbose,
        log_buf,
        log_file,
        "resolving start directory (cli_path={:?})",
        cli_path
    );
    let start_dir = match cli_path {
        Some(ref p) => {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            if c.is_dir() {
                vlog!(
                    verbose,
                    log_buf,
                    log_file,
                    "cli path is directory: {}",
                    c.display()
                );
                c
            } else if c.is_file() {
                vlog!(
                    verbose,
                    log_buf,
                    log_file,
                    "cli path is file, opening directly: {}",
                    c.display()
                );
                // File path — try to open it directly in the editor.
                open_file_directly(&c, &editor)?;
                // open_file_directly either exits or returns Ok(()) on success.
                return Ok(());
            } else {
                eprintln!("tfe: {:?} — no such file or directory", p);
                process::exit(2);
            }
        }
        None => saved
            .last_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
    };
    vlog!(
        verbose,
        log_buf,
        log_file,
        "start_dir: {}",
        start_dir.display()
    );

    // Right pane (legacy 2-pane fallback): use the persisted right-pane
    // directory when available, otherwise mirror the left pane's starting
    // directory.
    let right_start_dir = saved
        .last_dir_right
        .clone()
        .unwrap_or_else(|| start_dir.clone());
    vlog!(
        verbose,
        log_buf,
        log_file,
        "right_start_dir: {}",
        right_start_dir.display()
    );

    // Resolve the full list of pane starting directories. When the user
    // passed an explicit path on the CLI, always start with exactly the
    // classic 2-pane layout (matches pre-N-pane behaviour). Otherwise prefer
    // the persisted multi-pane list, falling back to the legacy 2-entry
    // left/right reconstruction for older state files.
    let pane_dirs: Vec<PathBuf> = if cli_path.is_some() {
        vec![start_dir.clone(), right_start_dir.clone()]
    } else if let Some(ref dirs) = saved.pane_dirs {
        dirs.clone()
    } else {
        vec![start_dir.clone(), right_start_dir.clone()]
    };
    vlog!(verbose, log_buf, log_file, "pane_dirs: {:?}", pane_dirs);

    // CLI flags win when explicitly set (true); otherwise fall back to
    // persisted values.  Simple bool flags can't distinguish "not passed" from
    // "passed as false", so the convention is: CLI `true` always wins.
    let show_hidden = if cli.hidden {
        true
    } else {
        saved.show_hidden.unwrap_or(false)
    };
    let show_sizes = saved.show_sizes.unwrap_or(true);
    let single_pane = if cli.single_pane {
        true
    } else {
        saved.single_pane.unwrap_or(false)
    };
    let sort_mode = saved.sort_mode.unwrap_or_default();

    // Resolve cd-on-exit: CLI flags take priority, then persisted value.
    let cd_on_exit = if cli.cd_on_exit {
        true
    } else if cli.no_cd {
        false
    } else {
        saved.cd_on_exit.unwrap_or(true)
    };
    vlog!(
        verbose,
        log_buf,
        log_file,
        "show_hidden={} single_pane={} cd_on_exit={}",
        show_hidden,
        single_pane,
        cd_on_exit
    );

    // Terminal setup.
    //
    // Render the TUI on stderr rather than stdout.  This works on all
    // platforms (Unix, macOS, Windows) without any platform-specific code:
    //
    // * The shell wrapper captures stdout with `dir=$(command tfe)`.
    //   stderr is never captured by $() so the TUI renders correctly.
    // * On Windows, stderr is connected to the console just like stdout,
    //   so crossterm's raw-mode and alternate-screen work without needing
    //   CONOUT$ or any other Windows-specific console API.
    // * The final directory/file path is written to real stdout after the
    //   TUI exits, where the shell wrapper's $() captures it correctly.
    vlog!(
        verbose,
        log_buf,
        log_file,
        "setting up terminal (backend=stderr)"
    );

    // Choose an image-preview protocol **without** doing a stdin round-trip.
    //
    // The obvious API — `ratatui_image::picker::Picker::from_query_stdio()` —
    // writes capability queries to stdout and then blocks a *background
    // thread* on `io::stdin().read()` with a 2-second timeout.  If the
    // terminal never answers (or answers in a shape the parser doesn't
    // expect), the timeout fires and `from_query_stdio()` falls back to
    // halfblocks — but the background thread is still alive, still blocking
    // on stdin.  The TUI event loop then reads from the same stdin and the
    // two race: the zombie thread swallows the user's first keypress(es),
    // so the app looks frozen or as though it never opened.  This was
    // observed on macOS with Ghostty, where the probe reliably times out
    // (~2 s) and eats the opening keystrokes.
    //
    // To sidestep the race entirely we never call `from_query_stdio()`.
    // Instead we:
    //   1. pick the graphics protocol from the terminal's identity
    //      (TERM / TERM_PROGRAM / marker env vars) — a pure lookup, and
    //   2. read the cell pixel size from `crossterm::terminal::window_size()`,
    //      which uses the `TIOCGWINSZ` ioctl (no escape sequences, no stdin,
    //      no thread, cannot hang).
    // At worst — an unknown terminal, or a kernel that doesn't report pixel
    // dimensions — we fall back to halfblocks with a nominal cell size, which
    // always renders *something* and never blocks.
    //
    // The probe is additionally pointless when stdout is not a real terminal
    // (e.g. inside the shell wrapper `dir=$(command tfe)`): there is no
    // terminal to describe, so we go straight to halfblocks.
    let stdout_is_tty = crossterm::tty::IsTty::is_tty(&io::stdout());
    let term = std::env::var("TERM").ok();
    let term_program = std::env::var("TERM_PROGRAM").ok();
    let detected_protocol = if stdout_is_tty {
        detect_image_protocol(term.as_deref(), term_program.as_deref(), |name| {
            std::env::var_os(name).is_some()
        })
    } else {
        ratatui_image::picker::ProtocolType::Halfblocks
    };
    // Cell size in pixels via ioctl — safe, synchronous, cannot block on stdin.
    let font_size = crossterm::terminal::window_size()
        .ok()
        .filter(|ws| ws.columns > 0 && ws.rows > 0 && ws.width > 0 && ws.height > 0)
        .map(|ws| (ws.width / ws.columns, ws.height / ws.rows));
    vlog!(
        verbose,
        log_buf,
        log_file,
        "selecting image protocol (stdout_is_tty={stdout_is_tty}, \
         TERM={term:?}, TERM_PROGRAM={term_program:?}, \
         protocol={detected_protocol:?}, font_size={font_size:?})"
    );
    let image_picker = match font_size {
        Some(fs) if detected_protocol != ratatui_image::picker::ProtocolType::Halfblocks => {
            // `from_fontsize` is deprecated in favour of `from_query_stdio`,
            // but it is the only constructor that lets us supply a known cell
            // size without triggering the racy stdin probe.  We immediately
            // override the protocol with the one we detected from the
            // terminal's identity.
            #[allow(deprecated)]
            let mut picker = ratatui_image::picker::Picker::from_fontsize(fs);
            picker.set_protocol_type(detected_protocol);
            picker
        }
        _ => ratatui_image::picker::Picker::halfblocks(),
    };
    vlog!(
        verbose,
        log_buf,
        log_file,
        "image picker ready: protocol={:?}, font_size={:?}",
        image_picker.protocol_type(),
        image_picker.font_size()
    );

    let backend = CrosstermBackend::new(io::stderr());

    vlog!(verbose, log_buf, log_file, "enable_raw_mode");
    enable_raw_mode()?;
    vlog!(verbose, log_buf, log_file, "enable_raw_mode ok");
    vlog!(verbose, log_buf, log_file, "entering alternate screen");
    execute!(io::stderr(), EnterAlternateScreen)?;
    // Defensively disable mouse capture at startup.  Previous versions of
    // tfe (or other TUI programs) may have enabled mouse tracking and
    // crashed / exited without cleaning up.  Some terminals (notably on
    // macOS) persist mouse-tracking state across process boundaries,
    // flooding stdin with SGR escape sequences that bury real key-presses.
    // Disabling it here costs nothing and ensures a clean slate.
    execute!(io::stderr(), DisableMouseCapture)?;
    vlog!(verbose, log_buf, log_file, "alternate screen ok");

    vlog!(verbose, log_buf, log_file, "creating terminal");
    let mut terminal = Terminal::new(backend)?;
    vlog!(verbose, log_buf, log_file, "terminal created ok");

    vlog!(verbose, log_buf, log_file, "creating App");
    let mut app = App::new(AppOptions {
        pane_dirs,
        extensions: cli.extensions,
        show_hidden,
        show_sizes,
        theme_idx,
        show_theme_panel: cli.show_themes,
        single_pane,
        sort_mode,
        cd_on_exit,
        editor,
        verbose,
        startup_log: log_buf,
    });
    // Inject the pre-queried image picker so preview uses the best protocol.
    app.preview_state = PreviewState::with_picker(image_picker);
    // Restore persisted active pane: prefer the explicit index, falling
    // back to the legacy "left"/"right" string for older state files.
    if let Some(idx) = saved.active_pane_idx {
        if idx < app.panes.len() {
            app.active_idx = idx;
        }
    } else if let Some(ref pane_str) = saved.active_pane {
        app.active_idx = match pane_str.as_str() {
            "right" => 1.min(app.panes.len().saturating_sub(1)),
            _ => 0,
        };
    }

    // From here on, use app.log() — the startup buffer has been moved into App.
    // Re-bind log_buf to app's debug_log for the vlog! macro.
    app.log("app created, entering run loop".to_string());
    app.log(format!(
        "{} pane(s), {} entries in active pane",
        app.panes.len(),
        app.active_pane().entry_count()
    ));

    let result = run_loop(&mut terminal, &mut app, &mut log_file);
    app.log(format!("run loop exited (result ok={})", result.is_ok()));
    if let Err(ref e) = result {
        app.log(format!("run loop error: {e}"));
    }

    // Always restore terminal, even on error.
    app.log("restoring terminal (leave alternate screen, disable raw mode)".to_string());
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();
    // Drop the terminal before writing to stdout so the alternate screen is
    // fully restored before the path appears.
    drop(terminal);
    // stderr is normal again after this point — can use eprintln safely.
    if verbose {
        eprintln!("[tfe] terminal restored");
    }

    result?;

    // If the shell wrapper was freshly installed during this run:
    //
    // 1. Emit a `source:<path>` directive to stdout BEFORE any cd path so
    //    the shell wrapper (which reads our stdout line-by-line) sources the
    //    rc file in the parent shell process.  This makes the newly-written
    //    `tfe` function available immediately — the user does not need to
    //    open a new terminal or run `source` manually.
    //
    // 2. Print a brief notice to stderr so the user knows what happened.
    //    The notice no longer instructs them to run `source` themselves
    //    because the wrapper already handled it.
    //
    // Note: emit_source_directive writes to stdout with a plain `\n`
    // terminator regardless of --null, because it is a control message
    // consumed by the wrapper, not a data path consumed by the caller.
    if let shell_init::InitOutcome::Installed(ref rc_path) = auto_install_outcome {
        if verbose {
            eprintln!("[tfe] emitting source directive for {}", rc_path.display());
        }
        shell_init::emit_source_directive(rc_path);
        eprintln!("tfe: shell integration installed to {}", rc_path.display());
        eprintln!("  The wrapper function has been sourced into this session automatically.");
        eprintln!("  cd-on-exit is now active — no restart needed.");
    }

    // Persist full application state on clean exit.
    //
    // `pane_dirs` captures every open pane's current directory (the source
    // of truth going forward). `last_dir` / `last_dir_right` are still
    // written alongside it so that older `tfe` binaries reading this file
    // continue to see a sensible 2-pane layout.
    //
    // When single-pane mode is active, hidden panes were never independently
    // navigated by the user this session, so blindly saving their
    // current_dir would clobber genuinely-persisted state from a previous
    // session. In that case we re-use whatever was previously persisted.
    let pane_dirs: Vec<PathBuf> = if app.single_pane {
        saved
            .pane_dirs
            .clone()
            .unwrap_or_else(|| app.panes.iter().map(|p| p.current_dir.clone()).collect())
    } else {
        app.panes.iter().map(|p| p.current_dir.clone()).collect()
    };
    let last_dir_right = if app.single_pane {
        // Preserve the value we loaded at startup (may be None if this is a
        // fresh install or the path was deleted).
        saved.last_dir_right.clone()
    } else {
        pane_dirs.get(1).cloned()
    };

    if verbose {
        eprintln!("[tfe] saving state");
    }
    save_state(&AppState {
        theme: Some(app.theme_name().to_string()),
        last_dir: Some(app.panes[0].current_dir.clone()),
        last_dir_right,
        sort_mode: Some(app.panes[0].sort_mode),
        show_hidden: Some(app.panes[0].show_hidden),
        show_sizes: Some(app.panes[0].show_sizes),
        single_pane: Some(app.single_pane),
        // Always persist the final value from app — this captures both CLI
        // flags (--cd / --no-cd) and any in-TUI toggle via the O panel / C key.
        cd_on_exit: Some(app.cd_on_exit),
        // Persist whichever editor the user ended up on (including any cycling
        // done through the options panel).
        editor: Some(app.editor.to_key()),
        active_pane: Some(if app.active_idx == 0 { "left" } else { "right" }.to_string()),
        pane_dirs: Some(pane_dirs),
        active_pane_idx: Some(app.active_idx),
    });

    // Emit a path to stdout so a shell wrapper can act on it (e.g. `cd`).
    //
    // * File selected            -> always emit the selected path (or its parent
    //                               with --print-dir), exit 0.
    // * Dismissed + cd_on_exit   -> emit the active pane's current directory so
    //                               the shell wrapper can `cd` there, exit 0.
    // * Dismissed + !cd_on_exit  -> print nothing, exit 1 (classic behaviour).
    match app.selected {
        Some(path) => {
            if verbose {
                eprintln!("[tfe] file selected: {}", path.display());
            }
            let output = resolve_output_path(path, cli.print_dir);
            let mut out = stdout();
            write!(out, "{}", output.display())?;
            out.write_all(if cli.null { b"\0" } else { b"\n" })?;
            out.flush()?;
        }
        // Use app.cd_on_exit — reflects both CLI flags and any in-TUI toggle.
        None if app.cd_on_exit => {
            if verbose {
                eprintln!("[tfe] dismissed with cd_on_exit=true");
            }
            let output = app.active_pane().current_dir.clone();
            let mut out = stdout();
            write!(out, "{}", output.display())?;
            out.write_all(if cli.null { b"\0" } else { b"\n" })?;
            out.flush()?;
        }
        None => {
            if verbose {
                eprintln!("[tfe] dismissed without selection, cd_on_exit=false, exit 1");
            }
            process::exit(1);
        }
    }

    Ok(())
}

/// Pick the terminal graphics protocol for image previews from the terminal's
/// *identity* alone — no escape-sequence probe, no stdin round-trip.
///
/// The usual API, [`ratatui_image::picker::Picker::from_query_stdio`], writes
/// capability queries to stdout and blocks a background thread on stdin waiting
/// for the reply.  Terminals that don't answer (or answer in an unexpected
/// shape — observed on macOS/Ghostty) make the probe time out *and* leave that
/// thread alive; it then races the TUI event loop for stdin and swallows the
/// user's first keypresses, so the app looks like it never opened.
///
/// We avoid the probe completely by mapping well-known terminals to the
/// protocol they implement, defaulting to the always-safe half-blocks renderer
/// for anything we don't recognise.  `env_present` reports whether a given
/// environment variable is set, keeping this function pure and testable.
fn detect_image_protocol(
    term: Option<&str>,
    term_program: Option<&str>,
    env_present: impl Fn(&str) -> bool,
) -> ratatui_image::picker::ProtocolType {
    use ratatui_image::picker::ProtocolType;

    // Terminal-specific marker variables are the most reliable signal because
    // they are set by the terminal itself; when present they are unambiguous.
    // kitty & Ghostty speak the kitty graphics protocol; Konsole implements it
    // too.  iTerm2 and WezTerm speak the iTerm2 inline-image protocol.
    const KITTY_ENV_MARKERS: &[&str] = &[
        "KITTY_WINDOW_ID",       // kitty
        "GHOSTTY_RESOURCES_DIR", // ghostty
        "KONSOLE_VERSION",       // Konsole (kitty graphics)
    ];
    const ITERM2_ENV_MARKERS: &[&str] = &[
        "WEZTERM_PANE",       // WezTerm
        "WEZTERM_EXECUTABLE", // WezTerm
        "ITERM_SESSION_ID",   // iTerm2
    ];
    if KITTY_ENV_MARKERS.iter().any(|m| env_present(m)) {
        return ProtocolType::Kitty;
    }
    if ITERM2_ENV_MARKERS.iter().any(|m| env_present(m)) {
        return ProtocolType::Iterm2;
    }

    // TERM_PROGRAM identifies a handful of graphics-capable terminals.
    if let Some(tp) = term_program {
        match tp {
            "ghostty" => return ProtocolType::Kitty,
            "WezTerm" | "iTerm.app" => return ProtocolType::Iterm2,
            _ => {}
        }
    }

    // TERM substrings catch the common cases (e.g. kitty sets
    // `TERM=xterm-kitty`, ghostty `xterm-ghostty`, foot `foot`).
    if let Some(t) = term {
        let t = t.to_ascii_lowercase();
        if t.contains("kitty") || t.contains("ghostty") {
            return ProtocolType::Kitty;
        }
        if t.contains("wezterm") {
            return ProtocolType::Iterm2;
        }
        // `foot` implements sixel.
        if t.contains("foot") {
            return ProtocolType::Sixel;
        }
    }

    ProtocolType::Halfblocks
}

fn run_loop<W: io::Write>(
    terminal: &mut Terminal<CrosstermBackend<W>>,
    app: &mut App,
    log_file: &mut Option<File>,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;

        if app.handle_event()? {
            app.log("handle_event signalled exit".to_string());
            if let Some(ref mut f) = log_file {
                let _ = writeln!(f, "[tfe] handle_event signalled exit");
            }
            break;
        }

        // ── Editor launch ─────────────────────────────────────────────────────
        // `handle_event` sets `open_with_editor` when the user presses `e` or
        // Enter on a file.  We handle it here (in run_loop) because we need
        // access to the Terminal to tear it down and restore it.
        if let Some(path) = app.open_with_editor.take() {
            // Defensive guard: Editor::None should never set open_with_editor,
            // but if it somehow does, silently discard and move on.
            if let Some(binary_str) = app.editor.binary() {
                let editor_label = app.editor.label().to_string();
                let launch_msg = format!(
                    "launching editor '{}' ({}) for {}",
                    editor_label,
                    binary_str,
                    path.display()
                );
                app.log(launch_msg.clone());
                if let Some(ref mut f) = log_file {
                    let _ = writeln!(f, "[tfe] {launch_msg}");
                }

                // Shell-split the binary string so that Custom("code --wait")
                // is correctly parsed into binary="code" + extra_arg="--wait".
                // We do a minimal whitespace split — no shell quoting needed
                // since the user-supplied value is already a single token or
                // was quoted by the shell before reaching us.
                let mut parts = binary_str.split_whitespace();
                let binary = parts.next().unwrap_or(&binary_str).to_string();
                let extra_args: Vec<&str> = parts.collect();

                // 1. Tear down the TUI so the editor gets a clean terminal.
                //    Write to the terminal backend (same handle as the TUI) so
                //    we don't accidentally open a second stderr file descriptor.
                let _ = disable_raw_mode();
                let _ = execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                );

                // 2. Spawn the editor synchronously and wait for it to exit.
                //
                //    Cross-platform notes:
                //    - macOS / Linux: Command::new(binary) works for any binary
                //      on $PATH. Shell functions and aliases are NOT available
                //      (Command bypasses the shell), but that is fine — editors
                //      are always real executables.
                //    - Windows: Many editors (code, nvim via scoop/winget,
                //      micro) are installed as `.cmd` or `.bat` shell scripts.
                //      Command::new("code") will NOT find them because Windows
                //      only runs `.cmd` files through cmd.exe. We therefore
                //      route all launches through `cmd /C` on Windows so that
                //      PATH resolution and .cmd shims work correctly.
                let status = {
                    #[cfg(windows)]
                    {
                        // cmd /C <binary> [extra_args…] <file>
                        let mut cmd = std::process::Command::new("cmd");
                        cmd.arg("/C").arg(&binary);
                        for a in &extra_args {
                            cmd.arg(a);
                        }
                        cmd.arg(&path).status()
                    }
                    #[cfg(not(windows))]
                    {
                        // Open /dev/tty explicitly so that the editor always
                        // gets the real terminal even when tfe's stdout is a
                        // pipe (e.g. inside the `dir=$(command tfe)` wrapper).
                        let tty = std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open("/dev/tty");
                        let mut cmd = std::process::Command::new(&binary);
                        for a in &extra_args {
                            cmd.arg(a);
                        }
                        cmd.arg(&path);
                        if let Ok(tty_file) = tty {
                            use std::os::unix::io::IntoRawFd;
                            let tty_fd = tty_file.into_raw_fd();
                            // SAFETY: tty_fd is a valid, open file descriptor
                            // for the terminal device.  We dup it for each of
                            // stdin/stdout/stderr so each gets its own fd.
                            unsafe {
                                use std::os::unix::io::FromRawFd;
                                let stdin_tty = std::fs::File::from_raw_fd(libc::dup(tty_fd));
                                let stdout_tty = std::fs::File::from_raw_fd(libc::dup(tty_fd));
                                let stderr_tty = std::fs::File::from_raw_fd(tty_fd);
                                cmd.stdin(stdin_tty).stdout(stdout_tty).stderr(stderr_tty);
                            }
                        }
                        cmd.status()
                    }
                };

                // 3. Restore the TUI regardless of whether the editor succeeded.
                let _ = enable_raw_mode();
                let _ = execute!(terminal.backend_mut(), EnterAlternateScreen);
                // The editor may have enabled mouse tracking — disable it
                // so leftover events don't flood our event loop.
                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                let _ = terminal.clear();

                // 4. Reload all panes so any on-disk changes are visible.
                app.log("reloading panes after editor exit".to_string());
                for p in app.panes.iter_mut() {
                    p.reload();
                }

                // 5. Set a status message.
                match status {
                    Ok(s) if s.success() => {
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        app.status_msg = format!("Returned from {editor_label} \u{2014} {fname}");
                    }
                    Ok(s) => {
                        app.status_msg =
                            format!("Editor exited with status {}", s.code().unwrap_or(-1));
                    }
                    Err(e) => {
                        app.status_msg = format!("Error launching '{binary}': {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

// ── Direct file opening ──────────────────────────────────────────────────────

/// Open a file directly in the configured editor (no TUI).
///
/// Called when the user runs `tfe <file>` instead of `tfe <directory>`.
/// Prints informative messages to stderr and exits with the editor's exit code.
fn open_file_directly(path: &std::path::Path, editor: &Editor) -> io::Result<()> {
    let binary_str = match editor.binary() {
        Some(b) => b,
        None => {
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            eprintln!("tfe: cannot open '{fname}' — no editor configured.");
            eprintln!();
            eprintln!("  Set an editor with one of:");
            eprintln!("    tfe --editor nvim          # use Neovim");
            eprintln!("    tfe --editor vim           # use Vim");
            eprintln!("    tfe --editor helix         # use Helix");
            eprintln!("    tfe --editor nano          # use Nano");
            eprintln!("    tfe --editor \"code --wait\"  # use VS Code");
            eprintln!();
            eprintln!("  Or pick one interactively: run tfe, then press Shift+E.");
            eprintln!("  The selection is persisted across sessions.");
            process::exit(2);
        }
    };

    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let editor_label = editor.label();

    eprintln!("tfe: opening '{fname}' in {editor_label}…");

    // Shell-split so Custom("code --wait") works correctly.
    let mut parts = binary_str.split_whitespace();
    let binary = parts.next().unwrap_or(&binary_str).to_string();
    let extra_args: Vec<&str> = parts.collect();

    let status = {
        #[cfg(windows)]
        {
            let mut cmd = std::process::Command::new("cmd");
            cmd.arg("/C").arg(&binary);
            for a in &extra_args {
                cmd.arg(a);
            }
            cmd.arg(path).status()
        }
        #[cfg(not(windows))]
        {
            let mut cmd = std::process::Command::new(&binary);
            for a in &extra_args {
                cmd.arg(a);
            }
            cmd.arg(path).status()
        }
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            eprintln!("tfe: {editor_label} exited with status {code}");
            process::exit(code);
        }
        Err(e) => {
            eprintln!("tfe: failed to launch '{binary}': {e}");
            eprintln!();
            eprintln!("  Make sure '{binary}' is installed and on your PATH.");
            eprintln!("  Change the editor with: tfe --editor <name>");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::detect_image_protocol;
    use ratatui_image::picker::ProtocolType;

    /// Helper: no environment markers present.
    fn no_env(_: &str) -> bool {
        false
    }

    #[test]
    fn apple_terminal_falls_back_to_halfblocks() {
        // Regression: Apple_Terminal has no graphics protocol — must never be
        // probed and must render via halfblocks.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("Apple_Terminal"), no_env),
            ProtocolType::Halfblocks,
        );
    }

    #[test]
    fn warp_and_unknown_terminals_use_halfblocks() {
        // Warp and other silent terminals previously hung the app on startup.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("WarpTerminal"), no_env),
            ProtocolType::Halfblocks,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), None, no_env),
            ProtocolType::Halfblocks,
        );
        // No TERM and no TERM_PROGRAM at all (e.g. a bare pty) must be safe too.
        assert_eq!(
            detect_image_protocol(None, None, no_env),
            ProtocolType::Halfblocks,
        );
    }

    #[test]
    fn kitty_is_detected_via_term_and_env() {
        assert_eq!(
            detect_image_protocol(Some("xterm-kitty"), None, no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), None, |name| name
                == "KITTY_WINDOW_ID"),
            ProtocolType::Kitty,
        );
    }

    #[test]
    fn ghostty_uses_kitty_protocol() {
        // The macOS/Ghostty regression: Ghostty must be recognised (kitty
        // graphics) without a stdin probe.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("ghostty"), no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-ghostty"), None, no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(None, None, |name| name == "GHOSTTY_RESOURCES_DIR"),
            ProtocolType::Kitty,
        );
    }

    #[test]
    fn wezterm_and_iterm_use_iterm2_protocol() {
        for tp in ["WezTerm", "iTerm.app"] {
            assert_eq!(
                detect_image_protocol(Some("xterm-256color"), Some(tp), no_env),
                ProtocolType::Iterm2,
                "expected {tp} to use the iTerm2 protocol"
            );
        }
        assert_eq!(
            detect_image_protocol(Some("wezterm"), None, no_env),
            ProtocolType::Iterm2,
        );
    }

    #[test]
    fn env_markers_map_to_expected_protocols() {
        for marker in [
            "KITTY_WINDOW_ID",
            "GHOSTTY_RESOURCES_DIR",
            "KONSOLE_VERSION",
        ] {
            assert_eq!(
                detect_image_protocol(None, None, |name| name == marker),
                ProtocolType::Kitty,
                "expected env marker {marker} to map to kitty"
            );
        }
        for marker in ["WEZTERM_PANE", "WEZTERM_EXECUTABLE", "ITERM_SESSION_ID"] {
            assert_eq!(
                detect_image_protocol(None, None, |name| name == marker),
                ProtocolType::Iterm2,
                "expected env marker {marker} to map to iTerm2"
            );
        }
    }

    #[test]
    fn foot_uses_sixel() {
        assert_eq!(
            detect_image_protocol(Some("foot"), None, no_env),
            ProtocolType::Sixel,
        );
    }
}
