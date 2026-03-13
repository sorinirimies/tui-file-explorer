//! # full — complete tui-file-explorer showcase
//!
//! A self-contained example that demonstrates every major feature of the
//! `tui-file-explorer` library assembled into one application:
//!
//! - Dual-pane file browsing with active/inactive theme differentiation
//! - Options panel (toggles, sort, editor) toggled with `Shift + O`
//! - Theme panel with live preview toggled with `Shift + T`
//! - Editor file opening — tears down TUI, opens file, restores TUI
//! - Navigation hint bar and action/status bar at the bottom
//! - `cd`-on-exit: prints the active pane's directory on dismiss
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example full
//! ```
//!
//! ## Key bindings
//!
//! | Key              | Action                                        |
//! |------------------|-----------------------------------------------|
//! | `Tab`            | Switch active pane                            |
//! | `w`              | Toggle single / dual pane                     |
//! | `Shift + O`      | Toggle options panel                          |
//! | `Shift + T`      | Toggle theme panel                            |
//! | `↑` / `k`        | Move cursor up                                |
//! | `↓` / `j`        | Move cursor down                              |
//! | `→` / `l`        | Descend into directory                        |
//! | `←` / `h`        | Ascend to parent                              |
//! | `Enter`          | Descend into dir, or open file in editor      |
//! | `/`              | Search                                        |
//! | `s`              | Cycle sort mode                               |
//! | `.`              | Toggle hidden files                           |
//! | `Esc` / `q`      | Dismiss / quit                                |
//!
//! ### Options panel keys (while panel is open)
//!
//! | Key              | Action                                        |
//! |------------------|-----------------------------------------------|
//! | `Shift + C`      | Toggle cd-on-exit                             |
//! | `w`              | Toggle single pane                            |
//! | `e`              | Cycle editor                                  |
//! | `[` / `t`        | Cycle theme (prev / next)                     |

use std::{
    io::{self, stderr},
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
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, DualPane, DualPaneActive, DualPaneOutcome, Theme};

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
    // Theme
    themes: Vec<(&'static str, &'static str, Theme)>,
    theme_idx: usize,
    // Panels
    show_options: bool,
    show_themes: bool,
    // Options state
    single_pane: bool,
    cd_on_exit: bool,
    editor: Editor,
    // Status
    status: String,
}

impl App {
    fn new() -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let themes = Theme::all_presets();
        let dual = DualPane::builder(start).build();
        Self {
            dual,
            themes,
            theme_idx: 0,
            show_options: false,
            show_themes: false,
            single_pane: false,
            cd_on_exit: false,
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

    fn theme_desc(&self) -> &'static str {
        self.themes[self.theme_idx].1
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

    fn toggle_single_pane(&mut self) {
        self.single_pane = !self.single_pane;
        self.dual.single_pane = self.single_pane;
    }

    fn toggle_cd_on_exit(&mut self) {
        self.cd_on_exit = !self.cd_on_exit;
        let state = if self.cd_on_exit { "on" } else { "off" };
        self.status = format!("cd-on-exit: {state}");
    }

    fn cycle_editor(&mut self) {
        self.editor = self.editor.cycle();
        self.status = format!("editor: {}", self.editor.label());
    }

    fn active_dir(&self) -> PathBuf {
        match self.dual.active_side {
            DualPaneActive::Left => self.dual.left.current_dir.clone(),
            DualPaneActive::Right => self.dual.right.current_dir.clone(),
        }
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
        Ok(maybe_path) => {
            if let Some(p) = maybe_path {
                println!("{}", p.display());
            }
            process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    }
}

fn run() -> io::Result<Option<PathBuf>> {
    let mut app = App::new();

    // Render on stderr so stdout stays clean for the shell wrapper.
    enable_raw_mode()?;
    execute!(stderr(), EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr());
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    );
    let _ = terminal.show_cursor();
    drop(terminal);

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
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

        // ── Global keys ───────────────────────────────────────────────────────

        // Shift + O — toggle options panel (closes theme panel).
        if key.code == KeyCode::Char('O') && key.modifiers.is_empty() {
            app.show_options = !app.show_options;
            if app.show_options {
                app.show_themes = false;
            }
            continue;
        }

        // Shift + T — toggle theme panel (closes options panel).
        if key.code == KeyCode::Char('T') && key.modifiers.is_empty() {
            app.show_themes = !app.show_themes;
            if app.show_themes {
                app.show_options = false;
            }
            continue;
        }

        // Theme cycling — always available.
        if key.code == KeyCode::Char('t') && key.modifiers.is_empty() {
            app.next_theme();
            continue;
        }
        if key.code == KeyCode::Char('[') {
            app.prev_theme();
            continue;
        }

        // w — toggle single/dual pane (when options panel NOT open).
        if key.code == KeyCode::Char('w') && key.modifiers.is_empty() && !app.show_options {
            app.toggle_single_pane();
            continue;
        }

        // ── Options-panel-only keys ───────────────────────────────────────────
        if app.show_options {
            match key.code {
                // Shift + C — toggle cd-on-exit.
                KeyCode::Char('C') if key.modifiers.is_empty() => {
                    app.toggle_cd_on_exit();
                    continue;
                }
                // w — single pane (intercept before explorer gets it).
                KeyCode::Char('w') if key.modifiers.is_empty() => {
                    app.toggle_single_pane();
                    continue;
                }
                // e — cycle editor when panel is open.
                KeyCode::Char('e') if key.modifiers.is_empty() => {
                    app.cycle_editor();
                    continue;
                }
                _ => {}
            }
        }

        // ── e key when options panel is closed — open current file ────────────
        if key.code == KeyCode::Char('e') && key.modifiers.is_empty() && !app.show_options {
            if let Some(binary) = app.editor.binary() {
                let active = match app.dual.active_side {
                    DualPaneActive::Left => &app.dual.left,
                    DualPaneActive::Right => &app.dual.right,
                };
                if let Some(entry) = active.current_entry() {
                    if !entry.path.is_dir() {
                        let path = entry.path.clone();
                        let binary = binary.to_string();
                        open_in_editor(terminal, &binary, &path)?;
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
            app.status = "No editor set — open Options (Shift+O) and press e to pick one".into();
            continue;
        }

        // ── Delegate to dual pane ─────────────────────────────────────────────
        match app.dual.handle_key(key) {
            DualPaneOutcome::Selected(path) => {
                if path.is_dir() {
                    continue;
                }
                // If an editor is configured, open the file and resume.
                if let Some(binary) = app.editor.binary() {
                    let binary = binary.to_string();
                    open_in_editor(terminal, &binary, &path)?;
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
                // No editor — tell the user and stay in the TUI.
                app.status =
                    "No editor set — open Options (Shift+O) and press e to pick one".into();
                continue;
            }
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
            DualPaneOutcome::Dismissed => {
                // cd-on-exit: print the active pane's current directory.
                if app.cd_on_exit {
                    return Ok(Some(app.active_dir()));
                }
                return Ok(None);
            }
            DualPaneOutcome::Pending => {
                // Clear non-error status on navigation.
                if !app.status.starts_with("error") {
                    app.status.clear();
                }
            }
            DualPaneOutcome::Unhandled => {}
        }
    }
}

// ── Editor launch ─────────────────────────────────────────────────────────────

fn open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    binary: &str,
    path: &std::path::Path,
) -> io::Result<()> {
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    {
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
            let _ = cmd.status();
        }
        #[cfg(not(unix))]
        {
            let _ = std::process::Command::new(binary).arg(path).status();
        }
    }

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
    let theme = app.theme().clone();
    let area = frame.area();

    // Vertical: main area | nav hints (3) | action bar (3).
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let main_area = v[0];
    let nav_area = v[1];
    let action_area = v[2];

    // Horizontal: left pane | [right pane] | [side panel].
    let mut h_constraints = if app.single_pane {
        vec![Constraint::Min(0)]
    } else {
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    };
    let has_panel = app.show_options || app.show_themes;
    if has_panel {
        h_constraints.push(Constraint::Length(42));
    }
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(h_constraints)
        .split(main_area);

    // Active / inactive theme differentiation.
    let active_theme = theme.clone();
    let inactive_theme = theme.clone().accent(theme.dim).brand(theme.dim);
    let (left_theme, right_theme) = match app.dual.active_side {
        DualPaneActive::Left => (&active_theme, &inactive_theme),
        DualPaneActive::Right => (&inactive_theme, &active_theme),
    };

    render_themed(&mut app.dual.left, frame, h[0], left_theme);
    if !app.single_pane {
        render_themed(&mut app.dual.right, frame, h[1], right_theme);
    }

    if has_panel {
        let panel_area = *h.last().unwrap();
        if app.show_options {
            render_options(frame, panel_area, app, &theme);
        } else {
            render_themes(frame, panel_area, app, &theme);
        }
    }

    render_nav_bar(frame, nav_area, &theme);
    render_action_bar(frame, action_area, app, &theme);
}

// ── Nav hint bar ──────────────────────────────────────────────────────────────

fn render_nav_bar(frame: &mut Frame, area: Rect, theme: &Theme) {
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));

    let line = Line::from(vec![
        k("↑"),
        d("/"),
        k("k"),
        d(" up  "),
        k("↓"),
        d("/"),
        k("j"),
        d(" down  "),
        k("→"),
        d("/"),
        k("l"),
        d("/"),
        k("Enter"),
        d(" open  "),
        k("←"),
        d("/"),
        k("h"),
        d("/"),
        k("Bksp"),
        d(" up  "),
        k("/"),
        d(" search  "),
        k("s"),
        d(" sort  "),
        k("."),
        d(" hidden  "),
        k("Esc"),
        d(" dismiss"),
    ]);

    let bar = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(bar, area);
}

// ── Action bar ────────────────────────────────────────────────────────────────

fn render_action_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: status message or active pane indicator.
    let status_text = if app.status.is_empty() {
        format!(
            " Active: {}  cd-on-exit: {}",
            app.active_label(),
            if app.cd_on_exit { "on" } else { "off" }
        )
    } else {
        format!(" {}", app.status)
    };
    let status_colour = if app.status.starts_with("error") {
        theme.brand
    } else if app.status.is_empty() {
        theme.dim
    } else {
        theme.success
    };
    let left = Paragraph::new(Span::styled(
        status_text,
        Style::default().fg(status_colour),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(left, h[0]);

    // Right: global key hints.
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));

    let hints = Line::from(vec![
        k("Tab"),
        d(" pane  "),
        k("w"),
        d(" split  "),
        k("["),
        d("/"),
        k("t"),
        d(" theme  "),
        k("Shift + T"),
        d(" themes  "),
        k("Shift + O"),
        d(" options"),
    ]);
    let right = Paragraph::new(hints).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right, h[1]);
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

    // Slots:
    //  [0]  header              2 rows
    //  [1]  gap                 1 row
    //  [2]  "View" title        1 row
    //  [3]  View group          4 rows  (w + Tab = 2 inner rows)
    //  [4]  gap                 1 row
    //  [5]  "Toggles" title     1 row
    //  [6]  Toggles group       3 rows  (Shift+C = 1 inner row)
    //  [7]  gap                 1 row
    //  [8]  "Editor" title      1 row
    //  [9]  Editor group        3 rows  (e = 1 inner row)
    //  [10] slack
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // [0] header
            Constraint::Length(1), // [1] gap
            Constraint::Length(1), // [2] View title
            Constraint::Length(4), // [3] View group
            Constraint::Length(1), // [4] gap
            Constraint::Length(1), // [5] Toggles title
            Constraint::Length(3), // [6] Toggles group
            Constraint::Length(1), // [7] gap
            Constraint::Length(1), // [8] Editor title
            Constraint::Length(3), // [9] Editor group
            Constraint::Min(0),    // [10] slack
        ])
        .split(area);

    // Header: title on top border, close hint on bottom border.
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

    // Floating section title helper.
    let section_title = |frame: &mut Frame, slot: Rect, label: &str| {
        let dashes = "─".repeat((slot.width as usize).saturating_sub(label.len() + 2));
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {label} "), dim_style),
                Span::styled(dashes, dim_style),
            ])),
            slot,
        );
    };

    // Option row helper.
    let option_row = |key: &str, label: &str, value: Span<'static>| -> Line {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{key:<12}"), key_style),
            Span::styled(format!("{label:<14}"), label_style),
            value,
        ])
    };

    let bool_span = |on: bool| -> Span<'static> {
        if on {
            Span::styled("● on ", on_style)
        } else {
            Span::styled("○ off", off_style)
        }
    };

    // ── View group ────────────────────────────────────────────────────────────
    section_title(frame, slots[2], "View");
    let view_cell = Paragraph::new(vec![
        option_row("w", "single pane", bool_span(app.single_pane)),
        option_row(
            "Tab",
            "switch pane",
            Span::styled(app.active_label(), on_style),
        ),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(view_cell, slots[3]);

    // ── Toggles group ─────────────────────────────────────────────────────────
    section_title(frame, slots[5], "Toggles");
    let toggles_cell = Paragraph::new(vec![option_row(
        "Shift + C",
        "cd on exit",
        bool_span(app.cd_on_exit),
    )])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(toggles_cell, slots[6]);

    // ── Editor group ──────────────────────────────────────────────────────────
    section_title(frame, slots[8], "Editor");
    let editor_val_style = if app.editor == Editor::None {
        off_style
    } else {
        on_style
    };
    let editor_cell = Paragraph::new(vec![option_row(
        "e",
        "open with",
        Span::styled(app.editor.label(), editor_val_style),
    )])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(editor_cell, slots[9]);
}

// ── Theme panel ───────────────────────────────────────────────────────────────

fn render_themes(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.dim);
    let title_style = Style::default()
        .fg(theme.brand)
        .add_modifier(Modifier::BOLD);

    // Three-zone layout: controls header | list | description footer.
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header (border-only)
            Constraint::Min(0),    // scrollable list
            Constraint::Length(4), // description footer
        ])
        .split(area);

    // Header.
    let header = Block::default()
        .title(Span::styled(" 🎨 Themes ", title_style))
        .title_bottom(Line::from(vec![
            Span::styled(" [ ", key_style),
            Span::styled("prev  ", dim_style),
            Span::styled("t ", key_style),
            Span::styled("next", dim_style),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(header, v[0]);

    // Scrollable list.
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
            let marker = if is_active { "▶ " } else { "  " };
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

    // Description footer.
    let desc = Paragraph::new(format!("{}\n{}", app.theme_name(), app.theme_desc()))
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(desc, v[2]);
}
