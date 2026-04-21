//! # dual_pane — two-pane file explorer using the library API
//!
//! A fully self-contained Ratatui application that demonstrates the new
//! [`DualPane`] widget from the `tui-file-explorer` library:
//!
//! - [`DualPane::builder`] with independent left / right starting directories
//! - [`render_dual_pane_themed`] for rendering both panes
//! - [`DualPaneOutcome`] variants — `Selected`, `Dismissed`, `Pending`
//! - `Tab` to switch focus between panes
//! - `w` to toggle single-pane / dual-pane mode
//! - All standard single-pane bindings work on whichever pane is active
//!
//! ## Usage
//!
//! ```bash
//! # Both panes start in the current directory
//! cargo run --example dual_pane
//!
//! # Left pane starts in $HOME, right pane starts in /tmp
//! cargo run --example dual_pane -- /tmp
//! ```
//!
//! ## Key bindings
//!
//! | Key              | Action                                      |
//! |------------------|---------------------------------------------|
//! | `Tab`            | Switch focus between left and right pane    |
//! | `w`              | Toggle single-pane / dual-pane mode         |
//! | `↑` / `k`        | Move cursor up                              |
//! | `↓` / `j`        | Move cursor down                            |
//! | `Enter` / `l`    | Descend into directory or confirm file      |
//! | `Backspace` / `h`| Ascend to parent directory                  |
//! | `/`              | Activate incremental search                 |
//! | `s`              | Cycle sort mode                             |
//! | `.`              | Toggle hidden files                         |
//! | `Esc` / `q`      | Dismiss                                     |
//!
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
use tui_file_explorer::{
    render_dual_pane_themed, DualPane, DualPaneActive, DualPaneOutcome, Theme,
};

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    match run() {
        Ok(Some(path)) => {
            println!("{}", path.display());
            process::exit(0);
        }
        Ok(None) => {
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    dual: DualPane,
    theme: Theme,
    status: String,
}

impl App {
    fn new(right_dir: Option<PathBuf>) -> Self {
        let left_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        let mut builder = DualPane::builder(left_dir);
        if let Some(dir) = right_dir {
            builder = builder.right_dir(dir);
        }

        Self {
            dual: builder.build(),
            theme: Theme::default(),
            status: String::from("Tab — switch pane   w — toggle single/dual   q — quit"),
        }
    }

    /// Human-readable label for the active pane side.
    fn active_label(&self) -> &'static str {
        match self.dual.active_side {
            DualPaneActive::Left => "LEFT",
            DualPaneActive::Right => "RIGHT",
        }
    }

    /// Human-readable layout mode label.
    fn mode_label(&self) -> &'static str {
        if self.dual.single_pane {
            "single-pane"
        } else {
            "dual-pane"
        }
    }
}

// ── Run / event loop ──────────────────────────────────────────────────────────

fn run() -> io::Result<Option<PathBuf>> {
    // Optional second argument: starting directory for the right pane.
    let right_dir = std::env::args().nth(1).map(PathBuf::from).and_then(|p| {
        if p.is_dir() {
            Some(p)
        } else {
            None
        }
    });

    let mut app = App::new(right_dir);

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    // ── Always restore the terminal ───────────────────────────────────────────
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    );
    let _ = terminal.show_cursor();

    result
}

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

        match app.dual.handle_key(key) {
            DualPaneOutcome::Selected(path) => return Ok(Some(path)),
            DualPaneOutcome::Dismissed => return Ok(None),
            DualPaneOutcome::MkdirCreated(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("📂 Created folder '{name}'");
            }
            DualPaneOutcome::TouchCreated(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("📄 Created file '{name}'");
            }
            DualPaneOutcome::RenameCompleted(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.status = format!("✏️  Renamed to '{name}'");
            }
            DualPaneOutcome::Pending => {
                // Update status to reflect current mode after any key.
                app.status = format!(
                    "Active: {}  Mode: {}   Tab — switch   w — toggle   q — quit",
                    app.active_label(),
                    app.mode_label(),
                );
            }
            DualPaneOutcome::Unhandled => {}
        }
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.theme;
    let area = frame.area();

    // Outer layout: explorer area (fill) + status bar (3 rows).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Render the dual pane (handles single/dual mode internally).
    render_dual_pane_themed(&mut app.dual, frame, rows[0], &theme);

    // Status bar.
    render_status(frame, rows[1], app, &theme);
}

fn render_status(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, theme: &Theme) {
    let active_colour = theme.accent;
    let mode_colour = if app.dual.single_pane {
        theme.brand
    } else {
        theme.success
    };

    let line = Line::from(vec![
        Span::styled(" Active: ", Style::default().fg(theme.dim)),
        Span::styled(
            app.active_label(),
            Style::default()
                .fg(active_colour)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   Mode: ", Style::default().fg(theme.dim)),
        Span::styled(
            app.mode_label(),
            Style::default()
                .fg(mode_colour)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "   Tab switch   w toggle   / search   s sort   . hidden   q quit",
            Style::default().fg(theme.dim),
        ),
    ]);

    let para = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );

    frame.render_widget(para, area);
}
