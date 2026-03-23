//! # full — complete tui-file-explorer showcase
//!
//! Demonstrates every major feature of the library by using the library's own
//! [`App`], [`AppOptions`], [`Editor`], and [`draw`] directly.
//!
//! - Dual-pane file browsing with active/inactive theme differentiation
//! - Options panel (toggles, sort, editor) toggled with `Shift + O`
//! - Theme panel with live preview toggled with `Shift + T` / `[` / `t`
//! - Editor panel toggled with `Shift + E`
//! - File opening — tears down TUI, opens file, restores TUI
//! - Navigation hint bar and action/status bar at the bottom
//! - `cd`-on-exit: prints the active pane's directory on dismiss
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example full
//! ```

use std::{
    io::{self, stderr},
    path::PathBuf,
    process,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_file_explorer::{draw, App, AppOptions, Editor};

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
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    let mut app = App::new(AppOptions {
        left_dir: start.clone(),
        right_dir: start,
        editor: Editor::Helix,
        ..AppOptions::default()
    });

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

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    app: &mut App,
) -> io::Result<Option<PathBuf>> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;

        // Check if the app wants to open a file in an editor.
        if let Some(path) = app.open_with_editor.take() {
            if let Some(binary) = app.editor.binary() {
                open_in_editor(terminal, &binary, &path)?;
                app.left.reload();
                app.right.reload();
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                app.notify(format!("returned from {fname}"));
            }
            continue;
        }

        // Check if the app has selected a path to exit with.
        if let Some(path) = app.selected.take() {
            return Ok(Some(path));
        }

        let Event::Key(_) = event::read()? else {
            continue;
        };

        // Re-read the event — we peeked above just for the type check.
        // Use handle_event which processes all keys and returns true on exit.
        if app.handle_event()? {
            // App signalled exit.
            let exit_path = if app.cd_on_exit {
                Some(app.active_pane().current_dir.clone())
            } else {
                app.selected.clone()
            };
            return Ok(exit_path);
        }
    }
}

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

    #[cfg(unix)]
    {
        use std::os::unix::io::{FromRawFd, IntoRawFd};
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty");
        let mut cmd = std::process::Command::new(binary);
        cmd.arg(path);
        if let Ok(tty_file) = tty {
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

    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    let _ = terminal.clear();
    Ok(())
}
