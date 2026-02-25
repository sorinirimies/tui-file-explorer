//! # basic — full runnable example for `tui-file-explorer`
//!
//! Demonstrates:
//!   * Creating a [`FileExplorer`] via the builder API
//!   * Embedding the explorer in a simple Ratatui application
//!   * Handling [`ExplorerOutcome`] variants
//!   * Applying a custom [`Theme`]
//!   * Running with Ctrl-C / Esc graceful shutdown
//!
//! Run with:
//!
//! ```text
//! cargo run --example basic
//! ```
//!
//! Pass `--` followed by space-separated extensions to enable the filter:
//!
//! ```text
//! cargo run --example basic -- rs toml md
//! ```

use std::{
    io::{self, stdout},
    path::PathBuf,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    explorer: FileExplorer,
    /// Most recently selected path (shown in the output panel).
    selected: Option<PathBuf>,
    /// Ephemeral status line shown beneath the path.
    message: String,
}

impl App {
    fn new(extension_filter: Vec<String>) -> Self {
        let explorer =
            FileExplorer::builder(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
                .extension_filter(extension_filter)
                // Start with hidden files visible so the demo is more interesting.
                .show_hidden(false)
                .build();

        Self {
            explorer,
            selected: None,
            message: String::new(),
        }
    }

    fn handle_event(&mut self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            // Global: Ctrl-C always quits.
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(true); // signal exit
            }

            match self.explorer.handle_key(key) {
                ExplorerOutcome::Selected(path) => {
                    self.message = format!("✓  Selected: {}", path.display());
                    self.selected = Some(path);
                }
                ExplorerOutcome::Dismissed => {
                    return Ok(true); // exit the loop
                }
                ExplorerOutcome::Pending => {}
                ExplorerOutcome::Unhandled => {}
            }
        }
        Ok(false)
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(app: &mut App, frame: &mut Frame, theme: &Theme) {
    // Split: explorer fills most of the screen; a 5-line output panel below.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(frame.area());

    // ── File explorer (top) ──────────────────────────────────────────────────
    render_themed(&mut app.explorer, frame, chunks[0], theme);

    // ── Output panel (bottom) ────────────────────────────────────────────────
    let selected_line = match &app.selected {
        Some(p) => Span::styled(
            format!("  {}", p.display()),
            Style::default()
                .fg(Color::Rgb(80, 220, 120))
                .add_modifier(Modifier::BOLD),
        ),
        None => Span::styled(
            "  (nothing selected yet)",
            Style::default().fg(Color::Rgb(120, 120, 130)),
        ),
    };

    let hint_line = Span::styled(
        "  ↑/↓ navigate   Enter confirm   Backspace ascend   . toggle hidden   Esc/q quit",
        Style::default().fg(Color::Rgb(120, 120, 130)),
    );

    let msg_line = if app.message.is_empty() {
        Line::from(vec![])
    } else {
        Line::from(vec![Span::styled(
            format!("  {}", &app.message),
            Style::default().fg(Color::Rgb(80, 200, 255)),
        )])
    };

    let output = Paragraph::new(vec![
        Line::from(vec![selected_line]),
        msg_line,
        Line::from(vec![hint_line]),
    ])
    .block(
        Block::default()
            .title(Span::styled(
                " Output ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    )
    .alignment(Alignment::Left);

    frame.render_widget(output, chunks[1]);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    // Collect optional extension-filter args (e.g.  `-- rs toml md`).
    let extension_filter: Vec<String> = std::env::args().skip(1).collect();

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── Custom theme — a dark "grape" palette ─────────────────────────────────
    let theme = Theme::default()
        .brand(Color::Rgb(200, 120, 255)) // violet title
        .accent(Color::Rgb(130, 180, 255)) // soft blue borders
        .dir(Color::Rgb(255, 200, 80)) // warm yellow directories
        .sel_bg(Color::Rgb(50, 40, 80)) // deep purple selection row
        .success(Color::Rgb(100, 230, 140)) // mint green status
        .match_file(Color::Rgb(100, 230, 140));

    // ── App ───────────────────────────────────────────────────────────────────
    let mut app = App::new(extension_filter);

    // ── Event loop ────────────────────────────────────────────────────────────
    loop {
        terminal.draw(|frame| draw(&mut app, frame, &theme))?;

        if app.handle_event()? {
            break;
        }
    }

    // ── Restore terminal ──────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print result to stdout after restoring the terminal.
    if let Some(path) = &app.selected {
        println!("Selected: {}", path.display());
    } else {
        println!("No file selected.");
    }

    Ok(())
}
