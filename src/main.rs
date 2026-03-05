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
//! ```
//!
//! Exit codes:
//!   0 — path printed to stdout (file selected, or dismissed with a current dir)
//!   2 — bad arguments / I/O error

mod app;
mod fs;
mod persistence;
mod ui;

use std::{
    io::{self, stdout},
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
SHELL INTEGRATION:\n\
  Wrap tfe so Esc/q automatically cd's to the browsed directory:\n\
\n\
    # bash/zsh (~/.bashrc or ~/.zshrc)\n\
    tfe() { local d; d=$(command tfe \"$@\"); [ -n \"$d\" ] && cd \"$d\"; }\n\
\n\
    # fish (~/.config/fish/functions/tfe.fish)\n\
    function tfe; set d (command tfe $argv); test -n \"$d\" && cd $d; end\n\
\n\
  Open selected file in $EDITOR:     tfe | xargs -r $EDITOR\n\
  NUL-delimited output:              tfe -0 | xargs -0 wc -l"
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

    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
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
    });

    // Emit a path to stdout so a shell wrapper can act on it (e.g. `cd`).
    //
    // • File selected  → emit the selected path (or its parent with --print-dir).
    // • Dismissed      → emit the active pane's current directory so the shell
    //                    wrapper can `cd` there even when no file was chosen.
    //
    // In both cases we exit 0 — the shell wrapper receives a non-empty path
    // and calls `cd`.  Exit code 2 is reserved for argument / I/O errors.
    let output = match app.selected {
        Some(path) => resolve_output_path(path, cli.print_dir),
        None => app.active_pane().current_dir.clone(),
    };
    fs::emit_path(&output, cli.null)?;

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;
        if app.handle_event()? {
            break;
        }
    }
    Ok(())
}
