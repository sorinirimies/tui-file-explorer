//! # editor_picker — live editor selection with a sectioned sidebar
//!
//! Demonstrates the editor-picker panel.  A narrow sidebar on the right lists
//! every supported editor split into two labelled sections:
//!
//! - **Terminal Editors** — `none`, `helix`, `nvim`, `vim`, `nano`, `micro`, `emacs`
//! - **IDEs & GUI Editors** — `sublime`, `vscode`, `zed`, `xcode`, `android-studio`,
//!   `rustrover`, `intellij`, `webstorm`, `pycharm`, `goland`, `clion`, `fleet`,
//!   `rubymine`, `phpstorm`, `rider`, `eclipse`
//!
//! The highlighted row (cursor) is tracked independently from the active
//! selection.  Pressing `Enter` confirms; `Esc` cancels without changing the
//! active editor.  A footer shows the CLI binary for the highlighted entry.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example editor_picker
//! ```
//!
//! ## Key bindings
//!
//! | Key           | Action                                        |
//! |---------------|-----------------------------------------------|
//! | `Shift + E`   | Toggle editor picker panel                    |
//! | `up`  / `k`   | Move cursor up   (panel or file list)         |
//! | `down` / `j`  | Move cursor down (panel or file list)         |
//! | `Enter`       | Confirm editor selection / descend into dir   |
//! | `Esc`         | Close panel / dismiss                         |
//! | `right` / `l` | Descend into directory                        |
//! | `left`  / `h` | Ascend to parent directory                    |
//! | `e`           | Open highlighted file in configured editor    |
//! | `q`           | Quit                                          |
//!
//! On selection the chosen path is printed to stdout and the process exits 0.
//! On dismissal the process exits 1.

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
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── Editor catalogue ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Editor {
    // Terminal editors
    #[default]
    None,
    Helix,
    Neovim,
    Vim,
    Nano,
    Micro,
    Emacs,
    // IDEs & GUI editors
    Sublime,
    VSCode,
    Zed,
    Xcode,
    AndroidStudio,
    RustRover,
    IntelliJIdea,
    WebStorm,
    PyCharm,
    GoLand,
    CLion,
    Fleet,
    RubyMine,
    PHPStorm,
    Rider,
    Eclipse,
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
            Editor::Emacs => "emacs",
            Editor::Sublime => "sublime",
            Editor::VSCode => "vscode",
            Editor::Zed => "zed",
            Editor::Xcode => "xcode",
            Editor::AndroidStudio => "android-studio",
            Editor::RustRover => "rustrover",
            Editor::IntelliJIdea => "intellij",
            Editor::WebStorm => "webstorm",
            Editor::PyCharm => "pycharm",
            Editor::GoLand => "goland",
            Editor::CLion => "clion",
            Editor::Fleet => "fleet",
            Editor::RubyMine => "rubymine",
            Editor::PHPStorm => "phpstorm",
            Editor::Rider => "rider",
            Editor::Eclipse => "eclipse",
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
            Editor::Emacs => Some("emacs"),
            Editor::Sublime => Some("subl"),
            Editor::VSCode => Some("code"),
            Editor::Zed => Some("zed"),
            Editor::Xcode => Some("xed"),
            Editor::AndroidStudio => Some("studio"),
            Editor::RustRover => Some("rustrover"),
            Editor::IntelliJIdea => Some("idea"),
            Editor::WebStorm => Some("webstorm"),
            Editor::PyCharm => Some("pycharm"),
            Editor::GoLand => Some("goland"),
            Editor::CLion => Some("clion"),
            Editor::Fleet => Some("fleet"),
            Editor::RubyMine => Some("rubymine"),
            Editor::PHPStorm => Some("phpstorm"),
            Editor::Rider => Some("rider"),
            Editor::Eclipse => Some("eclipse"),
        }
    }

    /// Index of the first IDE/GUI entry in `all()`.
    /// Everything before this index is a terminal editor.
    fn first_ide_idx() -> usize {
        7 // None, Helix, Neovim, Vim, Nano, Micro, Emacs
    }

    fn all() -> Vec<Editor> {
        vec![
            Editor::None,
            Editor::Helix,
            Editor::Neovim,
            Editor::Vim,
            Editor::Nano,
            Editor::Micro,
            Editor::Emacs,
            Editor::Sublime,
            Editor::VSCode,
            Editor::Zed,
            Editor::Xcode,
            Editor::AndroidStudio,
            Editor::RustRover,
            Editor::IntelliJIdea,
            Editor::WebStorm,
            Editor::PyCharm,
            Editor::GoLand,
            Editor::CLion,
            Editor::Fleet,
            Editor::RubyMine,
            Editor::PHPStorm,
            Editor::Rider,
            Editor::Eclipse,
        ]
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    explorer: FileExplorer,
    themes: Vec<(&'static str, &'static str, Theme)>,
    theme_idx: usize,
    editor: Editor,
    show_panel: bool,
    panel_idx: usize, // cursor within Editor::all(), independent of active editor
    status: String,
}

impl App {
    fn new() -> Self {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            explorer: FileExplorer::builder(start).build(),
            themes: Theme::all_presets(),
            theme_idx: 0,
            editor: Editor::default(),
            show_panel: true, // open on launch so the panel is visible immediately
            panel_idx: 0,
            status: String::new(),
        }
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx].2
    }

    fn open_panel(&mut self) {
        self.show_panel = true;
        // Sync cursor to the currently active editor.
        self.panel_idx = Editor::all()
            .iter()
            .position(|e| e == &self.editor)
            .unwrap_or(0);
    }

    fn panel_down(&mut self) {
        self.panel_idx = (self.panel_idx + 1) % Editor::all().len();
    }

    fn panel_up(&mut self) {
        let len = Editor::all().len();
        self.panel_idx = self.panel_idx.checked_sub(1).unwrap_or(len - 1);
    }

    fn confirm(&mut self) {
        self.editor = Editor::all()[self.panel_idx].clone();
        self.show_panel = false;
        self.status = format!("Editor set to \"{}\"", self.editor.label());
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
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

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
        terminal.draw(|f| draw(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(None);
        }

        // Shift+E — toggle panel.
        if key.code == KeyCode::Char('E') && key.modifiers.is_empty() {
            if app.show_panel {
                app.show_panel = false;
            } else {
                app.open_panel();
            }
            continue;
        }

        // Panel is focused — arrows/j/k navigate, Enter confirms, Esc cancels.
        if app.show_panel {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                    app.panel_down();
                }
                KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
                    app.panel_up();
                }
                KeyCode::Enter => app.confirm(),
                KeyCode::Esc => app.show_panel = false,
                _ => {}
            }
            continue;
        }

        // e — open current file in the configured editor.
        if key.code == KeyCode::Char('e') && key.modifiers.is_empty() {
            if let Some(binary) = app.editor.binary() {
                if let Some(entry) = app.explorer.current_entry() {
                    if !entry.path.is_dir() {
                        let path = entry.path.clone();
                        open_in_editor(terminal, binary, &path)?;
                        app.explorer.reload();
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        app.status = format!("Returned from {} -- {name}", app.editor.label());
                        continue;
                    }
                }
            } else {
                app.status = "No editor set -- press Shift+E to open the editor picker".to_string();
            }
            continue;
        }

        // Everything else goes to the file explorer.
        match app.explorer.handle_key(key) {
            ExplorerOutcome::Selected(path) => {
                if !path.is_dir() {
                    if let Some(binary) = app.editor.binary() {
                        open_in_editor(terminal, binary, &path)?;
                        app.explorer.reload();
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        app.status = format!("Returned from {} -- {name}", app.editor.label());
                        continue;
                    }
                    app.status =
                        "No editor set -- press Shift+E to open the editor picker".to_string();
                    continue;
                }
                return Ok(Some(path));
            }
            ExplorerOutcome::Dismissed => return Ok(None),
            _ => {}
        }
    }
}

/// Tear down the TUI, run the editor, then restore the TUI.
fn open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    binary: &str,
    path: &std::path::Path,
) -> io::Result<()> {
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = std::process::Command::new(binary).arg(path).status();
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

    // Horizontal split: file explorer (fill) | editor panel (42 cols, when open).
    let chunks = if app.show_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(42)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    // Optional status bar at the bottom of the explorer column.
    let (explorer_area, status_area) = if app.status.is_empty() {
        (chunks[0], None)
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(chunks[0]);
        (rows[0], Some(rows[1]))
    };

    render_themed(&mut app.explorer, frame, explorer_area, &theme);

    if app.show_panel {
        render_editor_panel(frame, chunks[1], app, &theme);
    }

    if let Some(slot) = status_area {
        let para = Paragraph::new(Span::styled(
            format!(" {}", app.status),
            Style::default().fg(theme.success),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.dim)),
        );
        frame.render_widget(para, slot);
    }
}

// ── Editor panel ──────────────────────────────────────────────────────────────

fn render_editor_panel(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let on_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg);
    let subtitle_style = Style::default().fg(theme.dim);
    let title_style = Style::default()
        .fg(theme.brand)
        .add_modifier(Modifier::BOLD);

    let editors = Editor::all();
    let first_ide = Editor::first_ide_idx();
    let terminal_editors = &editors[..first_ide];
    let ide_editors = &editors[first_ide..];

    // ── Layout ───────────────────────────────────────────────────────────────
    // [0] hints header (border-only, 2 rows)
    // [1] gap
    // [2] "Terminal Editors" section title
    // [3] Terminal Editors bordered cell
    // [4] gap
    // [5] "IDEs & GUI Editors" section title
    // [6] IDEs bordered cell
    // [7] gap
    // [8] footer (3 rows)
    // [9] slack
    let terminal_cell_h = terminal_editors.len() as u16 + 2;
    let ide_cell_h = ide_editors.len() as u16 + 2;

    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(terminal_cell_h),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(ide_cell_h),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // ── Section title helper ──────────────────────────────────────────────────
    let section_title = |frame: &mut Frame, slot: Rect, label: &str| {
        let dashes = "─".repeat((slot.width as usize).saturating_sub(label.len() + 2));
        let para = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), subtitle_style),
            Span::styled(dashes, subtitle_style),
        ]));
        frame.render_widget(para, slot);
    };

    // ── Editor row helper ─────────────────────────────────────────────────────
    let editor_row = |editor: &Editor, idx: usize| -> Line {
        let is_highlighted = idx == app.panel_idx;
        let is_selected = editor == &app.editor;
        let marker = if is_highlighted { "\u{25BA} " } else { "   " };
        let check = if is_selected { "\u{2713} " } else { "  " };
        Line::from(vec![
            Span::styled(
                marker,
                Style::default().fg(if is_highlighted {
                    theme.brand
                } else {
                    theme.dim
                }),
            ),
            Span::styled(
                check,
                if is_selected {
                    on_style
                } else {
                    subtitle_style
                },
            ),
            Span::styled(
                format!("{:<16}", editor.label()),
                if is_highlighted {
                    key_style
                } else {
                    label_style
                },
            ),
        ])
    };

    // ── Hints header ─────────────────────────────────────────────────────────
    let header = Block::default()
        .title(Span::styled(" Editor ", title_style))
        .title_bottom(Line::from(vec![
            Span::styled(" Shift + E ", key_style),
            Span::styled("close", subtitle_style),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(header, slots[0]);

    // ── Terminal Editors cell ─────────────────────────────────────────────────
    section_title(frame, slots[2], "Terminal Editors");

    let terminal_rows: Vec<Line> = terminal_editors
        .iter()
        .enumerate()
        .map(|(i, ed)| editor_row(ed, i))
        .collect();
    let terminal_cell = Paragraph::new(terminal_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(terminal_cell, slots[3]);

    // ── IDEs & GUI Editors cell ───────────────────────────────────────────────
    section_title(frame, slots[5], "IDEs & GUI Editors");

    let ide_rows: Vec<Line> = ide_editors
        .iter()
        .enumerate()
        .map(|(i, ed)| editor_row(ed, first_ide + i))
        .collect();
    let ide_cell = Paragraph::new(ide_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(ide_cell, slots[6]);

    // ── Footer — binary of the highlighted editor ─────────────────────────────
    let hi_editor = &editors[app.panel_idx];
    let footer_text = if *hi_editor == Editor::None {
        "none  --  no editor".to_string()
    } else {
        format!(
            "{}  ->  {}",
            hi_editor.label(),
            hi_editor.binary().unwrap_or("")
        )
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(footer, slots[8]);
}
