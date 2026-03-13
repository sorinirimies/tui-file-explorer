//! # create_entries — new folder and new file creation demo
//!
//! A self-contained Ratatui application that demonstrates the `n` (new folder)
//! and `N` (new file) creation bindings added to [`FileExplorer`].
//!
//! ## Key bindings (on top of the standard explorer bindings)
//!
//! | Key     | Action                                          |
//! |---------|-------------------------------------------------|
//! | `n`     | Create a new folder — type name, Enter confirm  |
//! | `N`     | Create a new file   — type name, Enter confirm  |
//! | `Esc`   | Cancel current input mode, or dismiss explorer  |
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example create_entries
//! ```
//!
//! The status bar at the bottom shows the outcome of the last create operation.
//! On selection the chosen path is printed to stdout and the process exits 0.
//! On dismissal (`Esc` / `q`) the process exits 1.

use std::{
    io::{self, stdout},
    path::PathBuf,
    process,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    explorer: FileExplorer,
    theme: Theme,
    /// Last status message (created folder / file, or errors).
    status: String,
}

impl App {
    fn new() -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            explorer: FileExplorer::builder(start).build(),
            theme: Theme::catppuccin_mocha(),
            status: String::from(
                "n  new folder    N  new file    Enter/l  confirm    Esc  cancel / dismiss",
            ),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    match run() {
        Ok(Some(path)) => {
            println!("{}", path.display());
            process::exit(0);
        }
        Ok(None) => process::exit(1),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    }
}

fn run() -> io::Result<Option<PathBuf>> {
    let mut app = App::new();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    );
    let _ = terminal.show_cursor();

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> io::Result<Option<PathBuf>> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        // Ctrl-C — hard exit.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(None);
        }

        match app.explorer.handle_key(key) {
            ExplorerOutcome::Selected(path) => return Ok(Some(path)),
            ExplorerOutcome::Dismissed => return Ok(None),

            ExplorerOutcome::MkdirCreated(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("📂 Created folder '{name}'");
            }

            ExplorerOutcome::TouchCreated(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("📄 Created file '{name}'");
            }

            ExplorerOutcome::RenameCompleted(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("✏️  Renamed to '{name}'");
            }

            ExplorerOutcome::Pending | ExplorerOutcome::Unhandled => {
                // Clear the status when the user is actively typing a new name
                // so the bar doesn't distract from the footer input prompt.
                if app.explorer.is_mkdir_active()
                    || app.explorer.is_touch_active()
                    || app.explorer.is_rename_active()
                    || app.explorer.is_searching()
                {
                    // keep existing status — the footer already shows the input
                } else if key.code == KeyCode::Esc {
                    // Restore the default hint line after cancelling a mode.
                    app.status =
                        "n  new folder    N  new file    Enter/l  confirm    Esc  cancel / dismiss"
                            .into();
                }
            }
        }
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &mut App) {
    let theme = &app.theme;
    let area = frame.area();

    // Outer layout: explorer (fill) | status bar (3 rows).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    render_themed(&mut app.explorer, frame, rows[0], theme);
    render_status(frame, rows[1], app);
}

fn render_status(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;

    // Colour the status green for success, accent for hints, brand for errors.
    let is_success = app.status.starts_with('📂') || app.status.starts_with('📄');
    let is_error =
        app.status.to_lowercase().contains("failed") || app.status.to_lowercase().contains("error");

    let color = if is_error {
        theme.brand
    } else if is_success {
        theme.success
    } else {
        theme.dim
    };

    // Split into left (status) and right (mode indicator) halves.
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(24)])
        .split(area);

    let left = Paragraph::new(Span::styled(
        format!(" {}", app.status),
        Style::default().fg(color),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(left, h[0]);

    // Right side: show which mode is currently active.
    let (mode_label, mode_color) = if app.explorer.is_mkdir_active() {
        (
            format!(" 📂 mkdir: {}█ ", app.explorer.mkdir_input()),
            theme.success,
        )
    } else if app.explorer.is_touch_active() {
        (
            format!(" 📄 touch: {}█ ", app.explorer.touch_input()),
            theme.accent,
        )
    } else if app.explorer.is_searching() {
        (
            format!(" 🔍 /{}█ ", app.explorer.search_query()),
            theme.brand,
        )
    } else {
        (" idle ".into(), theme.dim)
    };

    let right = Paragraph::new(Line::from(Span::styled(
        mode_label,
        Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right, h[1]);
}
