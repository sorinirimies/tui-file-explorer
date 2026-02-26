//! # basic — single-pane file explorer
//!
//! A fully self-contained Ratatui application that demonstrates the
//! `tui-file-explorer` library API:
//!
//! - [`FileExplorer::builder`] with an optional extension filter
//! - [`render_themed`] with a custom [`Theme`]
//! - All four [`ExplorerOutcome`] variants
//! - Incremental search (`/`), sort cycling (`s`), hidden-file toggle (`.`)
//!
//! ## Usage
//!
//! ```bash
//! # Browse everything
//! cargo run --example basic
//!
//! # Only .rs and .toml files are selectable
//! cargo run --example basic -- rs toml
//! ```
//!
//! On selection the chosen path is printed to stdout and the process exits 0.
//! On dismissal (`Esc` / `q`) the process exits 1.

use std::{
    io::{self, stdout},
    path::PathBuf,
    process,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, Theme};

fn main() {
    // Collect optional extension-filter args (e.g. `-- rs toml`).
    let extensions: Vec<String> = std::env::args().skip(1).collect();

    match run(extensions) {
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

fn run(extensions: Vec<String>) -> io::Result<Option<PathBuf>> {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    let mut explorer = FileExplorer::builder(start)
        .extension_filter(extensions)
        .build();

    // A subtle custom theme: keep the default palette but soften the brand
    // colour to violet so the example is visually distinct from the `tfe` binary.
    let theme = Theme::grape();

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut explorer, &theme);

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
    explorer: &mut FileExplorer,
    theme: &Theme,
) -> io::Result<Option<PathBuf>> {
    loop {
        terminal.draw(|frame| {
            render_themed(explorer, frame, frame.area(), theme);
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        match explorer.handle_key(key) {
            ExplorerOutcome::Selected(path) => return Ok(Some(path)),
            ExplorerOutcome::Dismissed => return Ok(None),
            ExplorerOutcome::Pending | ExplorerOutcome::Unhandled => {}
        }
    }
}
