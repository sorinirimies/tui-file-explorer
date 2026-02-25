//! # theme_switcher — live theme cycling for `tui-file-explorer`
//!
//! Demonstrates switching between all named [`Theme`] presets at runtime.
//! The sidebar shows the full catalogue and highlights the active theme.
//!
//! All themes come directly from [`Theme::all_presets()`] — no hand-crafted
//! colours live in this file. The full list includes the built-in decorative
//! palettes (Grape, Ocean, Sunset, Forest, Rose, Mono, Neon) as well as the
//! well-known editor / terminal schemes (Dracula, Nord, Solarized, Gruvbox,
//! Catppuccin × 4, Tokyo Night × 3, Kanagawa × 3, Moonfly, Nightfly, Oxocarbon).
//!
//! ## Controls
//!
//! | Key          | Action                               |
//! |--------------|--------------------------------------|
//! | `Tab`        | Cycle to the next theme              |
//! | `Shift+Tab`  | Cycle to the previous theme          |
//! | `↑/↓/j/k`   | Navigate the file list               |
//! | `Enter`      | Descend into directory / select file |
//! | `Backspace`  | Ascend to parent directory           |
//! | `.`          | Toggle hidden files                  |
//! | `Esc` / `q`  | Quit                                 |
//! | `Ctrl-C`     | Force quit                           |
//!
//! ## Run
//!
//! ```text
//! cargo run --example theme_switcher
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
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── Named themes ──────────────────────────────────────────────────────────────

struct NamedTheme {
    name: &'static str,
    description: &'static str,
    theme: Theme,
}

/// Build the full catalogue directly from [`Theme::all_presets()`].
fn all_themes() -> Vec<NamedTheme> {
    Theme::all_presets()
        .into_iter()
        .map(|(name, description, theme)| NamedTheme {
            name,
            description,
            theme,
        })
        .collect()
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    explorer: FileExplorer,
    themes: Vec<NamedTheme>,
    /// Index into `themes` for the currently active theme.
    theme_idx: usize,
    /// Most recently selected path.
    selected: Option<PathBuf>,
}

impl App {
    fn new() -> Self {
        let explorer =
            FileExplorer::builder(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
                .show_hidden(false)
                .build();

        Self {
            explorer,
            themes: all_themes(),
            theme_idx: 0,
            selected: None,
        }
    }

    fn current_theme(&self) -> &Theme {
        &self.themes[self.theme_idx].theme
    }

    fn current_named(&self) -> &NamedTheme {
        &self.themes[self.theme_idx]
    }

    fn next_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % self.themes.len();
    }

    fn prev_theme(&mut self) {
        self.theme_idx = (self.theme_idx + self.themes.len() - 1) % self.themes.len();
    }

    /// Handle one terminal event. Returns `true` when the app should exit.
    fn handle_event(&mut self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            // ── Global shortcuts ─────────────────────────────────────────────
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(true);
            }

            // Tab / Shift-Tab cycle themes before passing to the explorer so
            // they are never accidentally forwarded.
            if key.code == KeyCode::Tab {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_theme();
                } else {
                    self.next_theme();
                }
                return Ok(false);
            }

            // ── Explorer key handling ────────────────────────────────────────
            match self.explorer.handle_key(key) {
                ExplorerOutcome::Selected(path) => {
                    self.selected = Some(path);
                }
                ExplorerOutcome::Dismissed => return Ok(true),
                _ => {}
            }
        }
        Ok(false)
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(app: &mut App, frame: &mut Frame) {
    // Clone the theme up-front so we don't hold an immutable borrow into `app`
    // while we also need a mutable borrow of `app.explorer` for `render_themed`.
    let theme = app.current_theme().clone();

    // ── Outer layout: explorer | right sidebar ───────────────────────────────
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(36)])
        .split(frame.area());

    // ── Left column: file explorer ───────────────────────────────────────────
    render_themed(&mut app.explorer, frame, h_chunks[0], &theme);

    // ── Right column: theme panel ─────────────────────────────────────────────
    // Split the sidebar vertically: swatch list on top, selected-path below.
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(h_chunks[1]);

    draw_theme_panel(app, frame, v_chunks[0], &theme);
    draw_selection_panel(app, frame, v_chunks[1], &theme);
}

/// Render the theme list with the active entry highlighted.
fn draw_theme_panel(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let named = app.current_named();

    let mut lines: Vec<Line> = Vec::new();

    // Header hint
    lines.push(Line::from(vec![Span::styled(
        "  Tab / Shift+Tab to switch",
        Style::default().fg(theme.dim),
    )]));
    lines.push(Line::from(vec![]));

    // Flat list of all themes
    for i in 0..app.themes.len() {
        push_theme_row(&mut lines, app, i, theme);
    }

    // Blank separator + active theme description
    lines.push(Line::from(vec![]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", named.description),
        Style::default().fg(theme.accent),
    )]));

    let panel = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                " 🎨 Themes ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );

    frame.render_widget(panel, area);
}

/// Append a single theme row to `lines`, highlighted if it is the active one.
fn push_theme_row(lines: &mut Vec<Line>, app: &App, idx: usize, theme: &Theme) {
    let is_active = idx == app.theme_idx;
    let indicator = if is_active { " ▶ " } else { "   " };

    let label_style = if is_active {
        Style::default()
            .fg(theme.brand)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };

    lines.push(Line::from(vec![
        Span::styled(indicator, label_style),
        Span::styled(
            format!("{:>2}. ", idx + 1),
            Style::default().fg(theme.accent),
        ),
        Span::styled(app.themes[idx].name, label_style),
    ]));
}

/// Render the most-recently-selected path (or a placeholder).
fn draw_selection_panel(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let path_line = match &app.selected {
        Some(p) => Line::from(vec![Span::styled(
            format!("  {}", p.display()),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        )]),
        None => Line::from(vec![Span::styled(
            "  (no file selected yet)",
            Style::default().fg(theme.dim),
        )]),
    };

    let hint_line = Line::from(vec![Span::styled(
        "  Enter to select  Esc/q to quit",
        Style::default().fg(theme.dim),
    )]);

    let panel = Paragraph::new(vec![path_line, Line::from(vec![]), hint_line]).block(
        Block::default()
            .title(Span::styled(
                " Selected ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );

    frame.render_widget(panel, area);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── App ───────────────────────────────────────────────────────────────────
    let mut app = App::new();

    // ── Event loop ────────────────────────────────────────────────────────────
    loop {
        terminal.draw(|frame| draw(&mut app, frame))?;

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

    // Print the result after restoring the terminal.
    println!("Theme: {}", app.themes[app.theme_idx].name);
    if let Some(path) = &app.selected {
        println!("Selected: {}", path.display());
    } else {
        println!("No file selected.");
    }

    Ok(())
}
