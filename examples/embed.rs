//! Embed the complete file explorer inside a host-owned Ratatui layout.

use std::io;

use crossterm::{
    event::{self, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use tui_file_explorer::{
    draw_in_with_options, execute_operation, App, AppCommand, AppOutcome, AppViewOptions,
    KeyBindings, OperationMode, PaneOptions,
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = run(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::builder()
        .pane(PaneOptions::new(std::env::current_dir()?))
        .key_bindings(KeyBindings::new().bind(
            KeyCode::F(2),
            KeyModifiers::NONE,
            AppCommand::ToggleThemePanel,
        ))
        .build()
        .map_err(io::Error::other)?;
    app.set_operation_mode(OperationMode::Deferred);
    let mut host_status = String::from("F2: theme panel | Esc: close");

    loop {
        terminal.draw(|frame| {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(26), Constraint::Min(30)])
                .split(frame.area());
            frame.render_widget(
                Paragraph::new(host_status.as_str()).block(
                    Block::default()
                        .title(" Host application ")
                        .borders(Borders::ALL),
                ),
                columns[0],
            );
            draw_in_with_options(
                &mut app,
                frame,
                columns[1],
                AppViewOptions {
                    minimum_pane_width: 30,
                    ..AppViewOptions::default()
                },
            );
        })?;

        let outcome = app.dispatch_event(event::read()?)?;
        match outcome {
            AppOutcome::Dismissed => break,
            AppOutcome::Selected(path) => {
                host_status = format!("Selected: {}", path.display());
            }
            AppOutcome::OpenEditor { path, editor } => {
                host_status = format!("Open {} with {}", path.display(), editor.label());
                app.take_editor_request();
            }
            AppOutcome::OperationRequested(request) => {
                // Real hosts normally execute this on a worker thread.
                let result = execute_operation(&request);
                app.apply_operation_result(result)
                    .map_err(io::Error::other)?;
            }
            AppOutcome::Changed(events) => {
                host_status = format!("{} state change(s)", events.len());
            }
            AppOutcome::Continue | AppOutcome::Ignored => {}
        }
    }
    Ok(())
}
