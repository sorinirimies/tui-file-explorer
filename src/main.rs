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

mod app;
mod fs;
mod persistence;
mod shell_init;
mod ui;

use app::Editor;

use std::{
    io::{self, stdout, Write},
    path::PathBuf,
    process,
};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_file_explorer::Theme;

use app::App;
use fs::resolve_output_path;
use ui::draw;

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
\n\
  After both steps, dismissing tfe with Esc/q cd's your terminal to the\n\
  directory you were browsing.  Works on macOS, Linux, and Windows.\n\
\n\
  Open selected file in $EDITOR:     command tfe | xargs -r $EDITOR\n\
  NUL-delimited output:              command tfe -0 | xargs -0 wc -l"
)]
struct Cli {
    /// Starting directory [default: current directory]
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
    ///
    /// Appends the wrapper function to your rc file (creating it if needed)
    /// and prints instructions.  Supported shells: bash, zsh, fish, powershell.
    /// Examples: tfe --init zsh   tfe --init powershell
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

fn run() -> io::Result<()> {
    let cli = Cli::parse();

    // --init: install the shell wrapper (or print it as fallback) then exit.
    if let Some(ref shell_name) = cli.init {
        let shell = match shell_init::Shell::from_str(shell_name) {
            Some(s) => Some(s),
            None => {
                eprintln!("tfe: unrecognised shell '{shell_name}'. Supported: bash, zsh, fish");
                process::exit(2);
            }
        };
        use shell_init::InitOutcome;
        match shell_init::install_or_print(shell) {
            InitOutcome::Installed(path) => {
                eprintln!("tfe: shell integration installed to {}", path.display());
                eprintln!("Restart your shell or run: source {}", path.display());
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

    let themes = Theme::all_presets();

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
    let saved = persistence::load_state();

    // Priority: explicit --theme flag > persisted selection > built-in default.
    let theme_name = cli
        .theme
        .or_else(|| saved.theme.clone())
        .unwrap_or_else(|| "default".to_string());
    let theme_idx = persistence::resolve_theme_idx(&theme_name, &themes);

    // Priority: explicit --path arg > persisted last directory > cwd.
    let start_dir = match cli.path {
        Some(ref p) => {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            if c.is_dir() {
                c
            } else {
                eprintln!("tfe: {:?} is not a directory", p);
                process::exit(2);
            }
        }
        None => saved
            .last_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
    };

    // Right pane: use the persisted right-pane directory when available,
    // otherwise mirror the left pane's starting directory.
    let right_start_dir = saved
        .last_dir_right
        .clone()
        .unwrap_or_else(|| start_dir.clone());

    // CLI flags win when explicitly set (true); otherwise fall back to
    // persisted values.  Simple bool flags can't distinguish "not passed" from
    // "passed as false", so the convention is: CLI `true` always wins.
    let show_hidden = if cli.hidden {
        true
    } else {
        saved.show_hidden.unwrap_or(false)
    };
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
        saved.cd_on_exit.unwrap_or(false)
    };

    // Resolve editor: CLI flag > persisted value > compiled-in default (Helix).
    let editor = if let Some(ref raw) = cli.editor {
        Editor::from_key(raw).unwrap_or_else(|| Editor::Custom(raw.clone()))
    } else if let Some(ref raw) = saved.editor {
        Editor::from_key(raw).unwrap_or_default()
    } else {
        Editor::default()
    };

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
    let backend = CrosstermBackend::new(io::stderr());

    enable_raw_mode()?;
    execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(app::AppOptions {
        left_dir: start_dir,
        right_dir: right_start_dir,
        extensions: cli.extensions,
        show_hidden,
        theme_idx,
        show_theme_panel: cli.show_themes,
        single_pane,
        sort_mode,
        cd_on_exit,
        editor,
    });

    let result = run_loop(&mut terminal, &mut app);

    // Always restore terminal, even on error.
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

    result?;

    // Persist full application state on clean exit.
    //
    // When single-pane mode is active the right pane is hidden and its
    // current_dir was never independently navigated — it still mirrors the
    // left pane's starting directory.  Blindly saving it would clobber the
    // real last_dir_right that was loaded at startup, so in that case we
    // re-use whatever was previously persisted instead.
    let last_dir_right = if app.single_pane {
        // Preserve the value we loaded at startup (may be None if this is a
        // fresh install or the path was deleted).
        saved.last_dir_right.clone()
    } else {
        Some(app.right.current_dir.clone())
    };

    persistence::save_state(&persistence::AppState {
        theme: Some(app.theme_name().to_string()),
        last_dir: Some(app.left.current_dir.clone()),
        last_dir_right,
        sort_mode: Some(app.left.sort_mode),
        show_hidden: Some(app.left.show_hidden),
        single_pane: Some(app.single_pane),
        // Always persist the final value from app — this captures both CLI
        // flags (--cd / --no-cd) and any in-TUI toggle via the O panel / C key.
        cd_on_exit: Some(app.cd_on_exit),
        // Persist whichever editor the user ended up on (including any cycling
        // done through the options panel).
        editor: Some(app.editor.to_key()),
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
            let output = resolve_output_path(path, cli.print_dir);
            let mut out = stdout();
            write!(out, "{}", output.display())?;
            out.write_all(if cli.null { b"\0" } else { b"\n" })?;
            out.flush()?;
        }
        // Use app.cd_on_exit — reflects both CLI flags and any in-TUI toggle.
        None if app.cd_on_exit => {
            let output = app.active_pane().current_dir.clone();
            let mut out = stdout();
            write!(out, "{}", output.display())?;
            out.write_all(if cli.null { b"\0" } else { b"\n" })?;
            out.flush()?;
        }
        None => process::exit(1),
    }

    Ok(())
}

fn run_loop<W: io::Write>(
    terminal: &mut Terminal<CrosstermBackend<W>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;

        if app.handle_event()? {
            break;
        }

        // ── Editor launch ─────────────────────────────────────────────────────
        // `handle_event` sets `open_with_editor` when the user presses `e` on
        // a file.  We handle it here (in run_loop) because we need access to
        // the Terminal to tear it down and restore it.
        if let Some(path) = app.open_with_editor.take() {
            // Defensive guard: Editor::None should never set open_with_editor,
            // but if it somehow does, silently discard and move on.
            if let Some(binary) = app.editor.binary() {
                let binary = binary.to_string();
                let editor_label = app.editor.label().to_string();

                // 1. Tear down the TUI so the editor gets a clean terminal.
                let _ = disable_raw_mode();
                let _ = execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture);

                // 2. Spawn the editor synchronously and wait for it to exit.
                let status = std::process::Command::new(&binary).arg(&path).status();

                // 3. Restore the TUI regardless of whether the editor succeeded.
                let _ = enable_raw_mode();
                let _ = execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture);
                let _ = terminal.clear();

                // 4. Reload both panes so any on-disk changes are visible.
                app.left.reload();
                app.right.reload();

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
