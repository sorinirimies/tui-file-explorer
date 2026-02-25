//! # tfe — terminal file explorer
//!
//! A keyboard-driven file-browser that can be used as a standalone CLI tool
//! or embedded as a library widget in any Ratatui application.
//!
//! ## Shell integration
//!
//! ```bash
//! # Open the selected file in $EDITOR
//! tfe | xargs -r $EDITOR
//!
//! # cd into the directory of the selected file
//! cd "$(tfe --print-dir)"
//!
//! # Select a Rust source file and copy its path to the clipboard
//! tfe -e rs | pbcopy
//!
//! # Use a specific theme
//! tfe --theme catppuccin-mocha
//! ```
//!
//! Exit codes:
//!   0 — a file was selected (path printed to stdout)
//!   1 — explorer was dismissed without a selection
//!   2 — bad arguments / I/O error

use std::{
    io::{self, stdout, Write},
    path::PathBuf,
    process,
};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── CLI arguments ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "tfe",
    version,
    about = "Keyboard-driven terminal file explorer",
    long_about = "\
tfe opens an interactive file browser in your terminal.\n\
Navigate with arrow keys or vim keys, press Enter to select a file,\n\
and the chosen path is printed to stdout — ready for shell pipelines.\n\
\n\
Exit codes:\n\
  0   file selected — path printed to stdout\n\
  1   dismissed (Esc / q) without selecting anything\n\
  2   bad arguments or I/O error",
    after_help = "\
SHELL INTEGRATION:\n\
  Open selected file in $EDITOR:\n\
    tfe | xargs -r $EDITOR\n\
\n\
  cd into directory of selected file:\n\
    cd \"$(tfe --print-dir)\"\n\
\n\
  Use with fzf-style null-delimited output:\n\
    tfe -0 | xargs -0 wc -l"
)]
struct Cli {
    /// Starting directory [default: current directory]
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Only show/select files with these extensions (repeatable)
    ///
    /// Pass the extension without a leading dot, e.g. `-e rs -e toml`.
    /// Directories are always navigable regardless of the filter.
    #[arg(short, long = "ext", value_name = "EXT", action = clap::ArgAction::Append)]
    extensions: Vec<String>,

    /// Show hidden (dot-file) entries on startup
    ///
    /// Hidden files can always be toggled at runtime with the `.` key.
    #[arg(short = 'H', long)]
    hidden: bool,

    /// Colour theme to use
    ///
    /// Theme names are case-insensitive and hyphens/spaces are interchangeable
    /// (e.g. "catppuccin-mocha", "Catppuccin Mocha", and "catppuccin mocha"
    /// all resolve to the same theme).  Use --list-themes to see all options.
    #[arg(short, long, value_name = "THEME", default_value = "default")]
    theme: String,

    /// List all available themes and exit
    #[arg(long)]
    list_themes: bool,

    /// Print the selected file's parent directory instead of the full path
    ///
    /// Useful for `cd "$(tfe --print-dir)"` shell aliases.
    #[arg(long)]
    print_dir: bool,

    /// Terminate output with a NUL byte instead of a newline
    ///
    /// Useful for pipelines that handle filenames with spaces or newlines,
    /// e.g. `tfe -0 | xargs -0 wc -l`.
    #[arg(short = '0', long)]
    null: bool,
}

// ── Theme resolution ──────────────────────────────────────────────────────────

/// Normalise a theme name for comparison: lower-case, hyphens → spaces.
fn normalise(s: &str) -> String {
    s.to_lowercase().replace('-', " ")
}

/// Find a preset by name (case-insensitive, hyphens == spaces).
/// Returns `Theme::default()` and prints a warning if the name is unknown.
fn resolve_theme(name: &str) -> Theme {
    let key = normalise(name);
    for (preset_name, _, theme) in Theme::all_presets() {
        if normalise(preset_name) == key {
            return theme;
        }
    }
    eprintln!(
        "tfe: unknown theme {:?} — falling back to default. \
         Run `tfe --list-themes` to see available options.",
        name
    );
    Theme::default()
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    explorer: FileExplorer,
    /// The file the user confirmed, if any.
    selected: Option<PathBuf>,
}

impl App {
    fn new(start_dir: PathBuf, extensions: Vec<String>, show_hidden: bool) -> Self {
        let explorer = FileExplorer::builder(start_dir)
            .extension_filter(extensions)
            .show_hidden(show_hidden)
            .build();

        Self {
            explorer,
            selected: None,
        }
    }

    /// Process one terminal event.
    /// Returns `true` when the app should exit.
    fn handle_event(&mut self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(true);
            }
            match self.explorer.handle_key(key) {
                ExplorerOutcome::Selected(path) => {
                    self.selected = Some(path);
                    return Ok(true);
                }
                ExplorerOutcome::Dismissed => return Ok(true),
                _ => {}
            }
        }
        Ok(false)
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(app: &mut App, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();

    // Reserve a single footer line for the hint bar.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    render_themed(&mut app.explorer, frame, chunks[0], theme);
    render_hint_bar(frame, chunks[1], theme);
}

fn render_hint_bar(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let hint = " ↑/↓ navigate  Enter select  ← ascend  . toggle hidden  Esc/q quit";

    let bar = Paragraph::new(Span::styled(hint, Style::default().fg(theme.dim))).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );

    frame.render_widget(bar, area);
}

// ── Output ────────────────────────────────────────────────────────────────────

/// Write `path` to stdout, terminated by a newline or a NUL byte.
fn emit_path(path: &std::path::Path, null: bool) -> io::Result<()> {
    let mut out = stdout().lock();
    write!(out, "{}", path.display())?;
    if null {
        out.write_all(b"\0")?;
    } else {
        out.write_all(b"\n")?;
    }
    out.flush()
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

    // ── --list-themes ─────────────────────────────────────────────────────────
    if cli.list_themes {
        let presets = Theme::all_presets();
        let max_name = presets.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);
        println!("{:<width$}  DESCRIPTION", "THEME", width = max_name);
        println!("{}", "─".repeat(max_name + 2 + 50));
        for (name, desc, _) in presets {
            println!("{name:<width$}  {desc}", width = max_name);
        }
        return Ok(());
    }

    // ── Resolve starting directory ────────────────────────────────────────────
    let start_dir = match cli.path {
        Some(ref p) => {
            let canonical = p.canonicalize().unwrap_or_else(|_| p.clone());
            if canonical.is_dir() {
                canonical
            } else {
                eprintln!("tfe: {:?} is not a directory", p);
                process::exit(2);
            }
        }
        None => std::env::current_dir()?,
    };

    // ── Resolve theme ─────────────────────────────────────────────────────────
    let theme = resolve_theme(&cli.theme);

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── App ───────────────────────────────────────────────────────────────────
    let mut app = App::new(start_dir, cli.extensions.clone(), cli.hidden);

    // ── Event loop ────────────────────────────────────────────────────────────
    let result = run_loop(&mut terminal, &mut app, &theme);

    // ── Restore terminal (always, even on error) ──────────────────────────────
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    // Surface any event-loop error now that the terminal is restored.
    result?;

    // ── Output ────────────────────────────────────────────────────────────────
    match app.selected {
        Some(path) => {
            let output = if cli.print_dir {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(path.clone())
            } else {
                path
            };
            emit_path(&output, cli.null)?;
            // exit 0 — implicit
        }
        None => {
            // Dismissed without selection → exit 1
            process::exit(1);
        }
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    theme: &Theme,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(app, frame, theme))?;
        if app.handle_event()? {
            break;
        }
    }
    Ok(())
}
