//! # tfe — two-pane terminal file explorer
//!
//! A keyboard-driven dual-pane file manager.
//!
//! ## Shell integration
//!
//! ```bash
//! tfe | xargs -r $EDITOR           # open selected file in $EDITOR
//! cd "$(tfe --print-dir)"          # cd into the directory of the selection
//! tfe -e rs | pbcopy               # copy a .rs path to the clipboard
//! tfe --theme catppuccin-mocha     # choose a colour theme
//! tfe --single-pane                # start in single-pane mode
//! tfe --show-themes                # open theme panel on startup
//! tfe --list-themes                # list all themes and exit
//! ```
//!
//! Exit codes:
//!   0 — a file was selected (path printed to stdout)
//!   1 — explorer was dismissed without a selection
//!   2 — bad arguments / I/O error

use std::{
    fs,
    io::{self, stdout, Write},
    path::{Path, PathBuf},
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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "tfe",
    version,
    about = "Keyboard-driven two-pane terminal file explorer",
    after_help = "\
SHELL INTEGRATION:\n\
  Open selected file in $EDITOR:     tfe | xargs -r $EDITOR\n\
  cd into directory of selection:    cd \"$(tfe --print-dir)\"\n\
  NUL-delimited output:              tfe -0 | xargs -0 wc -l"
)]
struct Cli {
    /// Starting directory [default: current directory]
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Only show/select files with these extensions (repeatable: -e rs -e toml)
    #[arg(short, long = "ext", value_name = "EXT", action = clap::ArgAction::Append)]
    extensions: Vec<String>,

    /// Show hidden (dot-file) entries on startup
    #[arg(short = 'H', long)]
    hidden: bool,

    /// Colour theme [default: default]. Use --list-themes to see options.
    #[arg(short, long, value_name = "THEME", default_value = "default")]
    theme: String,

    /// List all available themes and exit
    #[arg(long)]
    list_themes: bool,

    /// Open the theme panel on startup (toggle at runtime with T)
    #[arg(long)]
    show_themes: bool,

    /// Start in single-pane mode (toggle at runtime with w)
    #[arg(long)]
    single_pane: bool,

    /// Print the selected file's parent directory instead of the full path
    #[arg(long)]
    print_dir: bool,

    /// Terminate output with a NUL byte instead of a newline
    #[arg(short = '0', long)]
    null: bool,
}

// ── App state types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Left,
    Right,
}

impl Pane {
    fn other(self) -> Self {
        match self {
            Pane::Left => Pane::Right,
            Pane::Right => Pane::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipOp {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
struct ClipboardItem {
    path: PathBuf,
    op: ClipOp,
}

impl ClipboardItem {
    fn icon(&self) -> &'static str {
        match self.op {
            ClipOp::Copy => "\u{1F4CB}", // 📋
            ClipOp::Cut => "\u{2702} ",  // ✂
        }
    }
    fn label(&self) -> &'static str {
        match self.op {
            ClipOp::Copy => "Copy",
            ClipOp::Cut => "Cut ",
        }
    }
}

#[derive(Debug)]
enum Modal {
    DeleteConfirm {
        path: PathBuf,
    },
    OverwriteConfirm {
        src: PathBuf,
        dst: PathBuf,
        is_cut: bool,
    },
}

// ── App ───────────────────────────────────────────────────────────────────────

struct App {
    left: FileExplorer,
    right: FileExplorer,
    active: Pane,
    clipboard: Option<ClipboardItem>,
    themes: Vec<(&'static str, &'static str, Theme)>,
    theme_idx: usize,
    show_theme_panel: bool,
    single_pane: bool,
    modal: Option<Modal>,
    selected: Option<PathBuf>,
    status_msg: String,
}

impl App {
    fn new(
        start_dir: PathBuf,
        extensions: Vec<String>,
        show_hidden: bool,
        theme_idx: usize,
        show_theme_panel: bool,
        single_pane: bool,
    ) -> Self {
        let left = FileExplorer::builder(start_dir.clone())
            .extension_filter(extensions.clone())
            .show_hidden(show_hidden)
            .build();
        let right = FileExplorer::builder(start_dir)
            .extension_filter(extensions)
            .show_hidden(show_hidden)
            .build();
        Self {
            left,
            right,
            active: Pane::Left,
            clipboard: None,
            themes: Theme::all_presets(),
            theme_idx,
            show_theme_panel,
            single_pane,
            modal: None,
            selected: None,
            status_msg: String::new(),
        }
    }

    fn active_pane(&self) -> &FileExplorer {
        match self.active {
            Pane::Left => &self.left,
            Pane::Right => &self.right,
        }
    }

    fn active_pane_mut(&mut self) -> &mut FileExplorer {
        match self.active {
            Pane::Left => &mut self.left,
            Pane::Right => &mut self.right,
        }
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx].2
    }

    fn theme_name(&self) -> &str {
        self.themes[self.theme_idx].0
    }

    fn theme_desc(&self) -> &str {
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

    // ── File operations ───────────────────────────────────────────────────────

    fn yank(&mut self, op: ClipOp) {
        if let Some(entry) = self.active_pane().current_entry() {
            let label = entry.name.clone();
            self.clipboard = Some(ClipboardItem {
                path: entry.path.clone(),
                op,
            });
            let (verb, hint) = if op == ClipOp::Copy {
                ("Copied", "paste a copy")
            } else {
                ("Cut", "move it")
            };
            self.status_msg = format!("{verb} '{label}' — press p in other pane to {hint}");
        }
    }

    fn paste(&mut self) {
        let Some(clip) = self.clipboard.clone() else {
            self.status_msg = "Nothing in clipboard.".into();
            return;
        };

        let dst_dir = self.active_pane().current_dir.clone();
        let file_name = match clip.path.file_name() {
            Some(n) => n.to_owned(),
            None => {
                self.status_msg = "Cannot paste: clipboard path has no filename.".into();
                return;
            }
        };
        let dst = dst_dir.join(&file_name);

        // Don't paste into the same location for Cut
        if clip.op == ClipOp::Cut && clip.path.parent() == Some(&dst_dir) {
            self.status_msg = "Source and destination are the same — skipped.".into();
            return;
        }

        if dst.exists() {
            self.modal = Some(Modal::OverwriteConfirm {
                src: clip.path,
                dst,
                is_cut: clip.op == ClipOp::Cut,
            });
        } else {
            self.do_paste(&clip.path, &dst, clip.op == ClipOp::Cut);
        }
    }

    fn do_paste(&mut self, src: &Path, dst: &Path, is_cut: bool) {
        let result = if src.is_dir() {
            copy_dir_all(src, dst)
        } else {
            fs::copy(src, dst).map(|_| ())
        };

        match result {
            Ok(()) => {
                if is_cut {
                    let _ = if src.is_dir() {
                        fs::remove_dir_all(src)
                    } else {
                        fs::remove_file(src)
                    };
                    self.clipboard = None;
                }
                self.left.reload();
                self.right.reload();
                self.status_msg = format!(
                    "{} '{}'",
                    if is_cut { "Moved" } else { "Pasted" },
                    dst.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    fn prompt_delete(&mut self) {
        if let Some(entry) = self.active_pane().current_entry() {
            self.modal = Some(Modal::DeleteConfirm {
                path: entry.path.clone(),
            });
        }
    }

    fn confirm_delete(&mut self, path: &Path) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match result {
            Ok(()) => {
                self.left.reload();
                self.right.reload();
                self.status_msg = format!("Deleted '{name}'");
            }
            Err(e) => {
                self.status_msg = format!("Delete failed: {e}");
            }
        }
    }

    // ── Event handling ────────────────────────────────────────────────────────

    fn handle_event(&mut self) -> io::Result<bool> {
        let Event::Key(key) = event::read()? else {
            return Ok(false);
        };

        // Always handle Ctrl-C
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }

        // ── Modal intercepts all input ────────────────────────────────────────
        if let Some(modal) = self.modal.take() {
            match &modal {
                Modal::DeleteConfirm { path } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let p = path.clone();
                        self.confirm_delete(&p);
                    }
                    _ => self.status_msg = "Delete cancelled.".into(),
                },
                Modal::OverwriteConfirm { src, dst, is_cut } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let (s, d, cut) = (src.clone(), dst.clone(), *is_cut);
                        self.do_paste(&s, &d, cut);
                    }
                    _ => self.status_msg = "Paste cancelled.".into(),
                },
            }
            return Ok(false);
        }

        // ── Global keys (always active) ───────────────────────────────────────
        match key.code {
            // Cycle theme forward
            KeyCode::Char('t') if key.modifiers.is_empty() => {
                self.next_theme();
                return Ok(false);
            }
            // Cycle theme backward
            KeyCode::Char('[') => {
                self.prev_theme();
                return Ok(false);
            }
            // Toggle theme panel
            KeyCode::Char('T') => {
                self.show_theme_panel = !self.show_theme_panel;
                return Ok(false);
            }
            // Switch pane
            KeyCode::Tab => {
                self.active = self.active.other();
                return Ok(false);
            }
            // Toggle single/two-pane
            KeyCode::Char('w') if key.modifiers.is_empty() => {
                self.single_pane = !self.single_pane;
                return Ok(false);
            }
            // Copy
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                self.yank(ClipOp::Copy);
                return Ok(false);
            }
            // Cut
            KeyCode::Char('x') if key.modifiers.is_empty() => {
                self.yank(ClipOp::Cut);
                return Ok(false);
            }
            // Paste
            KeyCode::Char('p') if key.modifiers.is_empty() => {
                self.paste();
                return Ok(false);
            }
            // Delete
            KeyCode::Char('d') if key.modifiers.is_empty() => {
                self.prompt_delete();
                return Ok(false);
            }
            _ => {}
        }

        // ── Delegate to active pane explorer ─────────────────────────────────
        // Clear any previous non-error status when navigating
        let outcome = self.active_pane_mut().handle_key(key);
        match outcome {
            ExplorerOutcome::Selected(path) => {
                self.selected = Some(path);
                return Ok(true);
            }
            ExplorerOutcome::Dismissed => return Ok(true),
            ExplorerOutcome::Pending => {
                if self.status_msg.starts_with("Error") || self.status_msg.starts_with("Delete") {
                    // keep error messages visible
                } else {
                    self.status_msg.clear();
                }
            }
            ExplorerOutcome::Unhandled => {}
        }

        Ok(false)
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(app: &mut App, frame: &mut Frame) {
    let theme = app.theme().clone();
    let full = frame.area();

    // Vertical: main | action_bar
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(full);

    let main_area = v_chunks[0];
    let action_area = v_chunks[1];

    // Horizontal: left | right? | theme_panel?
    let mut h_constraints = vec![];
    if app.single_pane {
        h_constraints.push(Constraint::Min(0));
    } else {
        h_constraints.push(Constraint::Percentage(50));
        h_constraints.push(Constraint::Percentage(50));
    }
    if app.show_theme_panel {
        h_constraints.push(Constraint::Length(32));
    }
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(h_constraints)
        .split(main_area);

    // ── Panes ─────────────────────────────────────────────────────────────────
    let active_theme = theme.clone();
    let inactive_theme = theme.clone().accent(theme.dim).brand(theme.dim);

    let (left_theme, right_theme) = match app.active {
        Pane::Left => (&active_theme, &inactive_theme),
        Pane::Right => (&inactive_theme, &active_theme),
    };

    render_themed(&mut app.left, frame, h_chunks[0], left_theme);

    if !app.single_pane {
        render_themed(&mut app.right, frame, h_chunks[1], right_theme);
    }

    // ── Theme panel ───────────────────────────────────────────────────────────
    if app.show_theme_panel {
        let panel_area = h_chunks[h_chunks.len() - 1];
        render_theme_panel(frame, panel_area, app);
    }

    // ── Action bar ────────────────────────────────────────────────────────────
    render_action_bar(frame, action_area, app, &theme);

    // ── Modal overlay ─────────────────────────────────────────────────────────
    if let Some(modal) = &app.modal {
        render_modal(frame, full, modal, &theme);
    }
}

fn render_theme_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    // Layout: [controls (3)] | [list (fill)] | [desc (4)]
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    // Controls header
    let controls = Paragraph::new(Line::from(vec![
        Span::styled(" [ ", Style::default().fg(theme.dim)),
        Span::styled("prev", Style::default().fg(theme.accent)),
        Span::styled("    ", Style::default().fg(theme.dim)),
        Span::styled("t ", Style::default().fg(theme.accent)),
        Span::styled("next", Style::default().fg(theme.accent)),
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

    // Theme list
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

    // Description footer
    let desc_text = format!("{}\n{}", app.theme_name(), app.theme_desc());
    let desc = Paragraph::new(desc_text)
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(desc, v[2]);
}

fn render_action_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    // Split: [left clipboard/status (fill)] | [right hints (fill)]
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: clipboard info (if any), else status message, else active pane indicator
    if let Some(clip) = &app.clipboard {
        let name = clip.path.file_name().unwrap_or_default().to_string_lossy();
        let line = Line::from(vec![
            Span::styled(
                format!(" {} {}: ", clip.icon(), clip.label()),
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let left_bar = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.brand)),
        );
        frame.render_widget(left_bar, h[0]);
    } else {
        let status = if app.status_msg.is_empty() {
            let active = match app.active {
                Pane::Left => "left",
                Pane::Right => "right",
            };
            format!(" Active pane: {active}")
        } else {
            format!(" {}", app.status_msg)
        };
        let status_color =
            if app.status_msg.starts_with("Error") || app.status_msg.starts_with("Delete failed") {
                theme.brand
            } else {
                theme.success
            };
        let left_bar = Paragraph::new(Span::styled(status, Style::default().fg(status_color)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.dim)),
            );
        frame.render_widget(left_bar, h[0]);
    }

    // Right: global key hints
    let hints = Line::from(render_action_bar_spans(theme));
    let right_bar = Paragraph::new(hints).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right_bar, h[1]);
}

fn render_action_bar_spans<'a>(theme: &'a Theme) -> Vec<Span<'a>> {
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));
    vec![
        k("Tab"),
        d(" pane  "),
        k("y"),
        d(" copy  "),
        k("x"),
        d(" cut  "),
        k("p"),
        d(" paste  "),
        k("d"),
        d(" del  "),
        k("["),
        d(" prev  "),
        k("t"),
        d(" next  "),
        k("T"),
        d(" pane  "),
        k("w"),
        d(" split"),
    ]
}

// ── Modal ─────────────────────────────────────────────────────────────────────

fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal, theme: &Theme) {
    let (title, body, hint) = match modal {
        Modal::DeleteConfirm { path } => (
            " Confirm Delete ",
            format!(
                "Delete '{}' ?",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            "  y  yes    any key  cancel  ",
        ),
        Modal::OverwriteConfirm { dst, .. } => (
            " Confirm Overwrite ",
            format!(
                "'{}' already exists. Overwrite?",
                dst.file_name().unwrap_or_default().to_string_lossy()
            ),
            "  y  yes    any key  cancel  ",
        ),
    };

    let w = (body.len() as u16 + 6).max(40).min(area.width - 4);
    let h = 7u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal_area = Rect::new(x, y, w, h);

    frame.render_widget(Clear, modal_area);

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(modal_area);

    let outer = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.brand));
    frame.render_widget(outer, modal_area);

    let body_para = Paragraph::new(Span::styled(
        body,
        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(body_para, v[0]);

    let hint_para = Paragraph::new(Line::from(vec![
        Span::styled(
            "  y",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  confirm    ", Style::default().fg(theme.dim)),
        Span::styled(
            "any key",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  cancel  ", Style::default().fg(theme.dim)),
    ]))
    .alignment(Alignment::Center);
    let _ = hint; // suppress unused
    frame.render_widget(hint_para, v[2]);
}

// ── File system helpers ───────────────────────────────────────────────────────

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ── Theme resolution ──────────────────────────────────────────────────────────

fn resolve_theme_idx(name: &str, themes: &[(&str, &str, Theme)]) -> usize {
    let key = name.to_lowercase().replace('-', " ");
    for (i, (n, _, _)) in themes.iter().enumerate() {
        if n.to_lowercase().replace('-', " ") == key {
            return i;
        }
    }
    eprintln!(
        "tfe: unknown theme {:?} — falling back to default. \
         Run `tfe --list-themes` to see options.",
        name
    );
    0
}

// ── Output ────────────────────────────────────────────────────────────────────

fn emit_path(path: &Path, null: bool) -> io::Result<()> {
    let mut out = stdout().lock();
    write!(out, "{}", path.display())?;
    out.write_all(if null { b"\0" } else { b"\n" })?;
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

    let themes = Theme::all_presets();

    // --list-themes
    if cli.list_themes {
        let max = themes.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);
        println!("{:<width$}  DESCRIPTION", "THEME", width = max);
        println!("{}", "\u{2500}".repeat(max + 52));
        for (name, desc, _) in &themes {
            println!("{name:<width$}  {desc}", width = max);
        }
        return Ok(());
    }

    let theme_idx = resolve_theme_idx(&cli.theme, &themes);

    let start_dir = match cli.path {
        Some(ref p) => {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            if c.is_dir() {
                c
            } else {
                eprintln!("tfe: {:?} is not a directory", p);
                process::exit(2);
            }
        }
        None => std::env::current_dir()?,
    };

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(
        start_dir,
        cli.extensions,
        cli.hidden,
        theme_idx,
        cli.show_themes,
        cli.single_pane,
    );

    let result = run_loop(&mut terminal, &mut app);

    // Always restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    result?;

    match app.selected {
        Some(path) => {
            let output = if cli.print_dir {
                path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
            } else {
                path
            };
            emit_path(&output, cli.null)?;
            // exit 0 implicit
        }
        None => process::exit(1),
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;
        if app.handle_event()? {
            break;
        }
    }
    Ok(())
}
