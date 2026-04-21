//! # options — DualPane explorer with a live options sidebar
//!
//! Demonstrates how to build a settings panel alongside the file explorer,
//! with options grouped into bordered category cells.
//!
//! ## Groups
//!
//! ```text
//! ╭─ ⚙ Options ──────────────────────╮
//!   Shift + O close                  │
//! ╰───────────────────────────────────╯
//!   View
//! ╭───────────────────────────────────╮
//! │ h   hidden files      ○ off      │
//! │ w   single pane       ○ off      │
//! ╰───────────────────────────────────╯
//!   Sort
//! ╭───────────────────────────────────╮
//! │ s   sort mode         name       │
//! ╰───────────────────────────────────╯
//!   Theme
//! ╭───────────────────────────────────╮
//! │ t         cycle          Default │
//! │ Tab       active pane    left    │
//! ╰───────────────────────────────────╯
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example options
//! ```
//!
//! ## Key bindings
//!
//! | Key              | Action                              |
//! |------------------|-------------------------------------|
//! | `Shift + O`      | Toggle options panel                |
//! | `h`              | Toggle hidden files  (panel open)   |
//! | `s`              | Cycle sort mode      (panel open)   |
//! | `w`              | Toggle single pane   (panel open)   |
//! | `t`              | Cycle theme          (panel open)   |
//! | `Tab`            | Switch active pane                  |
//! | `↑` / `k`        | Move cursor up                      |
//! | `↓` / `j`        | Move cursor down                    |
//! | `Enter`          | Descend / select file               |
//! | `Backspace`      | Ascend                              |
//! | `Esc` / `q`      | Quit                                |
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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{
    render_dual_pane_themed, DualPane, DualPaneActive, DualPaneOutcome, SortMode, Theme,
};

// ── Editor ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Editor {
    #[default]
    None,
    Helix,
    Neovim,
    Vim,
    Nano,
    Micro,
}

impl Editor {
    fn label(&self) -> &'static str {
        match self {
            Editor::None => "none",
            Editor::Helix => "helix",
            Editor::Neovim => "nvim",
            Editor::Vim => "vim",
            Editor::Nano => "nano",
            Editor::Micro => "micro",
        }
    }

    fn binary(&self) -> Option<&'static str> {
        match self {
            Editor::None => None,
            Editor::Helix => Some("hx"),
            Editor::Neovim => Some("nvim"),
            Editor::Vim => Some("vim"),
            Editor::Nano => Some("nano"),
            Editor::Micro => Some("micro"),
        }
    }

    fn cycle(&self) -> Editor {
        match self {
            Editor::None => Editor::Helix,
            Editor::Helix => Editor::Neovim,
            Editor::Neovim => Editor::Vim,
            Editor::Vim => Editor::Nano,
            Editor::Nano => Editor::Micro,
            Editor::Micro => Editor::None,
        }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    dual: DualPane,
    themes: Vec<(&'static str, &'static str, Theme)>,
    theme_idx: usize,
    show_hidden: bool,
    sort_mode: SortMode,
    single_pane: bool,
    show_options: bool,
    editor: Editor,
    status: String,
}

impl App {
    fn new() -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let themes = Theme::all_presets();
        Self {
            dual: DualPane::builder(start).build(),
            themes,
            theme_idx: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
            single_pane: false,
            show_options: true,
            editor: Editor::default(),
            status: String::new(),
        }
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx].2
    }

    fn theme_name(&self) -> &'static str {
        self.themes[self.theme_idx].0
    }

    fn cycle_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % self.themes.len();
    }

    fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.dual.left.set_sort_mode(self.sort_mode);
        self.dual.right.set_sort_mode(self.sort_mode);
    }

    fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.dual.left.set_show_hidden(self.show_hidden);
        self.dual.right.set_show_hidden(self.show_hidden);
    }

    fn toggle_single_pane(&mut self) {
        self.single_pane = !self.single_pane;
        self.dual.single_pane = self.single_pane;
    }

    fn cycle_editor(&mut self) {
        self.editor = self.editor.cycle();
    }

    fn active_label(&self) -> &'static str {
        match self.dual.active_side {
            DualPaneActive::Left => "left",
            DualPaneActive::Right => "right",
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

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(None);
        }

        // Shift + O — toggle options panel (always active).
        if key.code == KeyCode::Char('O') && key.modifiers.is_empty() {
            app.show_options = !app.show_options;
            continue;
        }

        // Keys that only fire while the options panel is visible.
        if app.show_options {
            match key.code {
                KeyCode::Char('h') if key.modifiers.is_empty() => {
                    app.toggle_hidden();
                    continue;
                }
                KeyCode::Char('s') if key.modifiers.is_empty() => {
                    app.cycle_sort();
                    continue;
                }
                KeyCode::Char('w') if key.modifiers.is_empty() => {
                    app.toggle_single_pane();
                    continue;
                }
                KeyCode::Char('t') if key.modifiers.is_empty() => {
                    app.cycle_theme();
                    continue;
                }
                KeyCode::Char('e') if key.modifiers.is_empty() => {
                    app.cycle_editor();
                    continue;
                }
                _ => {}
            }
        }

        // e when panel is closed — open current file in configured editor.
        if key.code == KeyCode::Char('e') && key.modifiers.is_empty() && !app.show_options {
            if let Some(binary) = app.editor.binary() {
                let active = match app.dual.active_side {
                    DualPaneActive::Left => &app.dual.left,
                    DualPaneActive::Right => &app.dual.right,
                };
                if let Some(entry) = active.current_entry() {
                    if !entry.path.is_dir() {
                        let path = entry.path.clone();
                        open_in_editor(terminal, binary, &path)?;
                        app.dual.left.reload();
                        app.dual.right.reload();
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        app.status = format!("returned from {} — {fname}", app.editor.label());
                        continue;
                    }
                }
            }
            // No editor configured — tell the user how to set one.
            app.status = "No editor set — open Options (O) and press e to pick one".into();
            continue;
        }

        match app.dual.handle_key(key) {
            DualPaneOutcome::Selected(path) => {
                // If a file (not a dir) is selected and an editor is set,
                // open it instead of exiting.
                if !path.is_dir() {
                    if let Some(binary) = app.editor.binary() {
                        open_in_editor(terminal, binary, &path)?;
                        app.dual.left.reload();
                        app.dual.right.reload();
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        app.status = format!("returned from {} — {fname}", app.editor.label());
                        continue;
                    }
                    // No editor configured — stay in TUI and tell the user.
                    app.status = "No editor set — open Options (O) and press e to pick one".into();
                    continue;
                }
                return Ok(Some(path));
            }
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
            DualPaneOutcome::Pending | DualPaneOutcome::Unhandled => {}
        }
    }
}

/// Tear down the TUI, run `binary path`, then restore the TUI.
fn open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    binary: &str,
    path: &std::path::Path,
) -> io::Result<()> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };

    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    let _status = {
        #[cfg(unix)]
        {
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty");
            let mut cmd = std::process::Command::new(binary);
            cmd.arg(path);
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
            std::process::Command::new(binary).arg(path).status()
        }
    };

    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    let _ = terminal.clear();
    Ok(())
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &mut App) {
    let theme = *app.theme();
    let area = frame.area();

    // Vertical split: main area (fill) | status bar (3 rows) when a status exists.
    let (main_area, status_area) = if app.status.is_empty() {
        (area, None)
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        (rows[0], Some(rows[1]))
    };

    let chunks = if app.show_options {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(42)])
            .split(main_area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)])
            .split(main_area)
    };

    render_dual_pane_themed(&mut app.dual, frame, chunks[0], &theme);

    if app.show_options {
        render_options(frame, chunks[1], app, &theme);
    }

    if let Some(slot) = status_area {
        let status_para = Paragraph::new(Span::styled(
            format!(" {}", app.status),
            Style::default().fg(theme.success),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.dim)),
        );
        frame.render_widget(status_para, slot);
    }
}

// ── Options panel ─────────────────────────────────────────────────────────────

fn render_options(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let on_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(theme.dim);
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg);
    let dim_style = Style::default().fg(theme.dim);
    let title_style = Style::default()
        .fg(theme.brand)
        .add_modifier(Modifier::BOLD);

    // Slots (top → bottom):
    //  [0]  hints header          2 rows  (top border: title, bottom border: o close)
    //  [1]  gap                   1 row
    //  [2]  "View" title          1 row
    //  [3]  View group cell       5 rows  (h + w + Tab, three inner rows)
    //  [4]  gap                   1 row
    //  [5]  "Sort" title          1 row
    //  [6]  Sort group cell       3 rows  (s, one inner row)
    //  [7]  gap                   1 row
    //  [8]  "Theme" title         1 row
    //  [9]  Theme group cell      3 rows  (t, one inner row)
    //  [10] gap                   1 row
    //  [11] "Editor" title        1 row
    //  [12] Editor group cell     3 rows  (e, one inner row)
    //  [13] slack
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // [0] hints header (border-only, no body)
            Constraint::Length(1), // [1] gap
            Constraint::Length(1), // [2] "View" title
            Constraint::Length(5), // [3] View group  (border + 3 rows + border)
            Constraint::Length(1), // [4] gap
            Constraint::Length(1), // [5] "Sort" title
            Constraint::Length(3), // [6] Sort group  (border + 1 row + border)
            Constraint::Length(1), // [7] gap
            Constraint::Length(1), // [8] "Theme" title
            Constraint::Length(3), // [9] Theme group (border + 1 row + border)
            Constraint::Length(1), // [10] gap
            Constraint::Length(1), // [11] "Editor" title
            Constraint::Length(3), // [12] Editor group (border + 1 row + border)
            Constraint::Min(0),    // [13] slack
        ])
        .split(area);

    // ── Hints header ─────────────────────────────────────────────────────────
    // Title sits on the top border line; hints sit on the bottom border line.
    // No body row is needed — the block is just 2 rows (top + bottom borders).
    let header = Block::default()
        .title(Span::styled(" ⚙ Options ", title_style))
        .title_bottom(Line::from(vec![
            Span::styled(" Shift + O ", key_style),
            Span::styled("close", dim_style),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(header, slots[0]);

    // ── Helper: floating section title ────────────────────────────────────────
    // Renders " Label ────────" in dim, no border.
    let section_title = |frame: &mut Frame, slot: Rect, label: &str| {
        let dashes = "─".repeat((slot.width as usize).saturating_sub(label.len() + 2));
        let para = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), dim_style),
            Span::styled(dashes, dim_style),
        ]));
        frame.render_widget(para, slot);
    };

    // ── Helper: one row inside a group cell ───────────────────────────────────
    // Returns a Line with key (left-padded), label, and value span.
    let option_row = |key: &str, label: &str, value: Span<'static>| -> Line {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{key:<12}"), key_style),
            Span::styled(format!("{label:<16}"), label_style),
            value,
        ])
    };

    // ── Helper: bool value span ───────────────────────────────────────────────
    let bool_span = |on: bool| -> Span {
        if on {
            Span::styled("● on", on_style)
        } else {
            Span::styled("○ off", off_style)
        }
    };

    // ── View group ────────────────────────────────────────────────────────────
    section_title(frame, slots[2], "View");

    let view_rows = vec![
        option_row("h", "hidden files", bool_span(app.show_hidden)),
        option_row("w", "single pane", bool_span(app.single_pane)),
        option_row(
            "Tab",
            "switch pane",
            Span::styled(app.active_label(), on_style),
        ),
    ];
    let view_cell = Paragraph::new(view_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(view_cell, slots[3]);

    // ── Sort group ────────────────────────────────────────────────────────────
    section_title(frame, slots[5], "Sort");

    let sort_rows = vec![option_row(
        "s",
        "sort mode",
        Span::styled(app.sort_mode.label(), on_style),
    )];
    let sort_cell = Paragraph::new(sort_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(sort_cell, slots[6]);

    // ── Theme group ───────────────────────────────────────────────────────────
    section_title(frame, slots[8], "Theme");

    let theme_rows = vec![option_row(
        "t",
        "cycle theme",
        Span::styled(app.theme_name(), on_style),
    )];
    let theme_cell = Paragraph::new(theme_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(theme_cell, slots[9]);

    // ── Editor group ──────────────────────────────────────────────────────────
    section_title(frame, slots[11], "Editor");

    let editor_val_style = if app.editor == Editor::None {
        off_style
    } else {
        on_style
    };
    let editor_rows = vec![option_row(
        "e",
        "open with",
        Span::styled(app.editor.label(), editor_val_style),
    )];
    let editor_cell = Paragraph::new(editor_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(editor_cell, slots[12]);
}
