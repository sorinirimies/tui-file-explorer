//! # open_file — file explorer that opens files in your editor
//!
//! Demonstrates how to intercept [`ExplorerOutcome::Selected`] to open the
//! chosen file in an external editor, then resume the TUI so you can keep
//! browsing and open more files.
//!
//! The editor is resolved in this priority order:
//!   1. First CLI argument: `cargo run --example open_file -- nvim`
//!   2. `$VISUAL` environment variable
//!   3. `$EDITOR` environment variable
//!   4. Falls back to `vi`
//!
//! ## Usage
//!
//! ```bash
//! # Uses $VISUAL / $EDITOR
//! cargo run --example open_file
//!
//! # Explicit editor
//! cargo run --example open_file -- hx
//! cargo run --example open_file -- "code --wait"
//! ```
//!
//! ## Key bindings
//!
//! | Key              | Action                                      |
//! |------------------|---------------------------------------------|
//! | `↑` / `k`        | Move cursor up                              |
//! | `↓` / `j`        | Move cursor down                            |
//! | `Enter` / `l`    | Descend into directory, or open file        |
//! | `Backspace` / `h`| Ascend to parent directory                  |
//! | `/`              | Activate incremental search                 |
//! | `s`              | Cycle sort mode                             |
//! | `.`              | Toggle hidden files                         |
//! | `Esc` / `q`      | Quit                                        |
//!
//! Pressing `Enter` / `l` on a file tears down the TUI, opens it in the
//! configured editor, then restores the TUI so you can continue browsing.
//! `Esc` / `q` exits and prints nothing.

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
    editor: String,
    status: String,
    last_opened: Option<PathBuf>,
}

impl App {
    fn new(editor: String) -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            explorer: FileExplorer::builder(start).build(),
            status: format!("editor: {editor}   Enter/l — open file   Esc/q — quit"),
            editor,
            last_opened: None,
        }
    }
}

// ── Editor resolution ─────────────────────────────────────────────────────────

/// Resolve the editor binary to use.
///
/// Priority: CLI arg → $VISUAL → $EDITOR → "vi".
fn resolve_editor() -> String {
    // 1. Explicit CLI argument.
    if let Some(arg) = std::env::args().nth(1) {
        if !arg.is_empty() {
            return arg;
        }
    }
    // 2. $VISUAL
    if let Ok(v) = std::env::var("VISUAL") {
        if !v.is_empty() {
            return v;
        }
    }
    // 3. $EDITOR
    if let Ok(e) = std::env::var("EDITOR") {
        if !e.is_empty() {
            return e;
        }
    }
    // 4. Fallback
    "vi".to_string()
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let editor = resolve_editor();

    match run(editor) {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    }
}

fn run(editor: String) -> io::Result<()> {
    let mut app = App::new(editor);
    let theme = Theme::default();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app, &theme);

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
    theme: &Theme,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app, theme))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        match app.explorer.handle_key(key) {
            ExplorerOutcome::Selected(path) => {
                if path.is_dir() {
                    // Directories are descended into by the explorer — nothing
                    // extra to do here; the explorer already navigated.
                    continue;
                }

                // ── Open file in editor ───────────────────────────────────────
                // 1. Tear down the TUI so the editor gets a clean terminal.
                let _ = disable_raw_mode();
                let _ = execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                );

                // 2. Shell-split the editor string so "code --wait" works.
                let mut parts = app.editor.split_whitespace();
                let binary = parts.next().unwrap_or("vi").to_string();
                let extra_args: Vec<&str> = parts.collect();

                let status = {
                    #[cfg(unix)]
                    {
                        // Open /dev/tty so the editor always gets the real
                        // terminal even when stdout is piped.
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
                            use std::os::unix::io::{FromRawFd, IntoRawFd};
                            let tty_fd = tty_file.into_raw_fd();
                            unsafe {
                                let stdin_tty = std::fs::File::from_raw_fd(libc::dup(tty_fd));
                                let stdout_tty = std::fs::File::from_raw_fd(libc::dup(tty_fd));
                                let stderr_tty = std::fs::File::from_raw_fd(tty_fd);
                                cmd.stdin(stdin_tty).stdout(stdout_tty).stderr(stderr_tty);
                            }
                        }
                        cmd.status()
                    }
                    #[cfg(not(unix))]
                    {
                        let mut cmd = std::process::Command::new(&binary);
                        for a in &extra_args {
                            cmd.arg(a);
                        }
                        cmd.arg(&path).status()
                    }
                };

                // 3. Restore the TUI.
                let _ = enable_raw_mode();
                let _ = execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture,
                );
                let _ = terminal.clear();

                // 4. Reload the pane so edits are reflected.
                app.explorer.reload();

                // 5. Update status.
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();

                app.status = match status {
                    Ok(s) if s.success() => {
                        format!("returned from {} — {fname}", app.editor)
                    }
                    Ok(s) => format!("editor exited with status {}", s.code().unwrap_or(-1)),
                    Err(e) => format!("error launching '{}': {e}", app.editor),
                };

                app.last_opened = Some(path);
            }
            ExplorerOutcome::Dismissed => return Ok(()),
            ExplorerOutcome::Pending | ExplorerOutcome::Unhandled => {}
        }
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();

    // Vertical split: explorer (fill) | status bar (3 rows).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    render_themed(&mut app.explorer, frame, rows[0], theme);
    render_status(frame, rows[1], app, theme);
}

fn render_status(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, theme: &Theme) {
    // Split into left (status message) and right (editor badge).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(24)])
        .split(area);

    // Left: status message.
    let status_colour = if app.status.starts_with("error") {
        theme.brand
    } else {
        theme.success
    };
    let left = Paragraph::new(Span::styled(
        format!(" {}", app.status),
        Style::default().fg(status_colour),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(left, cols[0]);

    // Right: editor badge.
    let right = Paragraph::new(Line::from(vec![
        Span::styled(" editor: ", Style::default().fg(theme.dim)),
        Span::styled(
            app.editor.clone(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right, cols[1]);
}
