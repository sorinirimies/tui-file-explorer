//! # theme_switcher — live theme cycling with a catalogue sidebar
//!
//! A self-contained Ratatui application that demonstrates all 27 named
//! [`Theme`] presets side-by-side with a live file explorer.  A narrow
//! sidebar on the right lists every theme; the active entry is highlighted
//! and scrolls into view automatically.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example theme_switcher
//! ```
//!
//! ## Key bindings
//!
//! | Key | Action |
//! |-----|--------|
//! | `Tab` | Next theme |
//! | `Shift+Tab` | Previous theme |
//! | `↑` / `k` | Move cursor up |
//! | `↓` / `j` | Move cursor down |
//! | `Enter` / `→` / `l` | Descend into directory or confirm selection |
//! | `Backspace` / `←` / `h` | Ascend to parent directory |
//! | `.` | Toggle hidden files |
//! | `/` | Activate incremental search |
//! | `s` | Cycle sort mode |
//! | `Esc` / `q` | Dismiss |
//!
//! On selection the chosen path is printed to stdout and the process exits 0.
//! On dismissal (`Esc` / `q`) the process exits 1.

use std::{
    io::{self, stdout},
    path::PathBuf,
    process,
};

use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── App state ─────────────────────────────────────────────────────────────────

struct ThemeSwitcher {
    explorer: FileExplorer,
    themes: Vec<(&'static str, &'static str, Theme)>,
    theme_idx: usize,
}

impl ThemeSwitcher {
    fn new() -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let explorer = FileExplorer::builder(start).build();
        let themes = Theme::all_presets();
        Self {
            explorer,
            themes,
            theme_idx: 0,
        }
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx].2
    }

    fn next_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % self.themes.len();
    }

    fn prev_theme(&mut self) {
        self.theme_idx = self
            .theme_idx
            .checked_sub(1)
            .unwrap_or(self.themes.len() - 1);
    }
}

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

fn run() -> io::Result<Option<PathBuf>> {
    let mut app = ThemeSwitcher::new();

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
    app: &mut ThemeSwitcher,
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

        // Theme-cycling keys are handled before delegating to the explorer.
        match key.code {
            // Tab — next theme.
            KeyCode::Tab if key.modifiers.is_empty() => {
                app.next_theme();
                continue;
            }
            // Shift+Tab — previous theme.
            KeyCode::BackTab => {
                app.prev_theme();
                continue;
            }
            _ => {}
        }

        // Everything else goes to the explorer.
        match app.explorer.handle_key(key) {
            ExplorerOutcome::Selected(path) => return Ok(Some(path)),
            ExplorerOutcome::Dismissed => return Ok(None),
            ExplorerOutcome::MkdirCreated(_)
            | ExplorerOutcome::TouchCreated(_)
            | ExplorerOutcome::RenameCompleted(_)
            | ExplorerOutcome::Pending
            | ExplorerOutcome::Unhandled => {}
        }
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &mut ThemeSwitcher) {
    let theme = *app.theme();
    let area = frame.area();

    // Split horizontally: explorer (fill) | sidebar (32 cols).
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(32)])
        .split(area);

    render_themed(&mut app.explorer, frame, chunks[0], &theme);
    render_sidebar(frame, chunks[1], app, &theme);
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn render_sidebar(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &ThemeSwitcher,
    theme: &Theme,
) {
    // Three vertical sections: controls header | theme list | description footer.
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    // ── Controls header ───────────────────────────────────────────────────────
    let controls = Paragraph::new(Line::from(vec![
        Span::styled(
            "Shift+Tab ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("prev  ", Style::default().fg(theme.dim)),
        Span::styled(
            "Tab ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("next", Style::default().fg(theme.dim)),
    ]))
    .block(
        Block::default()
            .title(Span::styled(
                " \u{1F3A8} Themes ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(controls, v[0]);

    // ── Theme list ────────────────────────────────────────────────────────────
    // Keep the selected item visible by computing a scroll offset.
    let visible = v[1].height.saturating_sub(2) as usize;
    let scroll = if app.theme_idx >= visible {
        app.theme_idx - visible + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .themes
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(i, (name, _, _))| {
            let is_active = i == app.theme_idx;
            let marker = if is_active { "\u{25BA} " } else { "   " };
            let line = Line::from(vec![
                Span::styled(
                    format!("{marker}{:>2}. ", i + 1),
                    Style::default().fg(if is_active { theme.brand } else { theme.dim }),
                ),
                Span::styled(
                    name.to_string(),
                    if is_active {
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    },
                ),
            ]);
            if is_active {
                ListItem::new(line).style(Style::default().bg(theme.sel_bg))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.theme_idx.saturating_sub(scroll)));

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_stateful_widget(list, v[1], &mut list_state);

    // ── Description footer ────────────────────────────────────────────────────
    let (name, desc, _) = &app.themes[app.theme_idx];
    let desc_text = format!("{name}\n{desc}");
    let desc_para = Paragraph::new(desc_text)
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(desc_para, v[2]);
}
