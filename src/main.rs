//! # tfe — two-pane terminal file explorer
//!
//! A keyboard-driven dual-pane file manager.
//!
//! ## Shell integration
//!
//! Add the wrapper function to your shell rc file so that dismissing `tfe`
//! automatically `cd`s the terminal to the pane's current directory:
//!
//! ```bash
//! # bash / zsh — put this in ~/.bashrc or ~/.zshrc
//! tfe() {
//!     local dir
//!     dir=$(command tfe "$@")
//!     [ -n "$dir" ] && cd "$dir"
//! }
//! ```
//!
//! ```fish
//! # fish — put this in ~/.config/fish/functions/tfe.fish
//! function tfe
//!     set dir (command tfe $argv)
//!     if test -n "$dir"
//!         cd $dir
//!     end
//! end
//! ```
//!
//! Other examples:
//!
//! ```bash
//! tfe | xargs -r $EDITOR           # open selected file in $EDITOR
//! tfe -e rs | pbcopy               # copy a .rs path to the clipboard
//! tfe --theme catppuccin-mocha     # choose a colour theme
//! tfe --single-pane                # start in single-pane mode
//! tfe --show-themes                # open theme panel on startup
//! tfe --list-themes                # list all themes and exit
//! tfe --editor nvim                # set editor for the e key
//! tfe --editor "code --wait"       # custom editor (quoted)
//! ```
//!
//! Exit codes:
//!   0 — path printed to stdout (file selected, or dismissed with a current dir)
//!   2 — bad arguments / I/O error

mod cli;
mod doctor;
mod info;
mod launch;
mod logger;
mod shell_init;
mod term;

use std::{
    io::{self, stdout, Write},
    path::PathBuf,
    process,
};

use clap::Parser;
use tui_file_explorer::{
    app::Editor, draw, load_state, resolve_output_path, resolve_theme_idx, try_save_state, App,
    AppOptions, AppState, PreviewState, Theme,
};

use cli::Cli;
use launch::{open_file_directly, spawn_editor, split_command, Stdio};
use logger::{vlog, Logger};
use term::TerminalGuard;

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("tfe: {e}");
        process::exit(2);
    }
}

fn run() -> io::Result<()> {
    let cli = Cli::parse();
    let mut log = Logger::new(cli.verbose);
    log_environment(&mut log);

    if run_utility_command(&cli, &mut log) {
        return Ok(());
    }

    // Auto-install the shell wrapper on first run. Runs before terminal setup
    // so stderr is still the real terminal; the notice is surfaced *after* the
    // TUI exits so the alternate screen cannot swallow it.
    vlog!(log, "running auto_install (shell wrapper)");
    let auto_install_outcome = shell_init::auto_install();
    vlog!(log, "auto_install outcome: {auto_install_outcome:?}");

    let themes = Theme::presets();
    vlog!(log, "loaded {} themes", themes.len());

    if cli.list_themes {
        print_themes(&themes);
        return Ok(());
    }

    vlog!(log, "loading persisted state");
    let saved = load_state();
    vlog!(
        log,
        "state: theme={:?} last_dir={:?} cd_on_exit={:?} editor={:?}",
        saved.theme,
        saved.last_dir,
        saved.cd_on_exit,
        saved.editor
    );

    let Some(options) = resolve_options(&cli, &saved, &themes, &mut log)? else {
        return Ok(()); // a file was opened directly in the editor
    };

    // The TUI renders on stderr, so stdout stays clean for the path that the
    // shell wrapper captures with `dir=$(command tfe)`.
    vlog!(log, "setting up terminal (backend=stderr)");
    let (image_picker, picker_desc) = term::build_image_picker();
    vlog!(log, "{picker_desc}");
    let mut guard = TerminalGuard::enter()?;

    vlog!(log, "creating App");
    let mut app = App::new(AppOptions {
        startup_log: log.take_buffer(),
        ..options
    });
    // Inject the pre-queried image picker so preview uses the best protocol.
    app.preview_state = PreviewState::with_picker(image_picker);
    restore_active_pane(&mut app, &saved);
    apply_user_keymap(&mut app, &mut log);

    app.log("app created, entering run loop".to_string());
    app.log(format!(
        "{} pane(s), {} entries in active pane",
        app.panes.len(),
        app.active_pane().entry_count()
    ));

    let result = run_loop(&mut guard, &mut app, &mut log);
    app.log(format!("run loop exited (result ok={})", result.is_ok()));
    if let Err(ref e) = result {
        app.log(format!("run loop error: {e}"));
    }

    // Restore before writing to stdout so the alternate screen is gone first.
    app.log("restoring terminal".to_string());
    drop(guard);
    if log.verbose() {
        eprintln!("[tfe] terminal restored");
    }
    result?;

    announce_shell_install(&auto_install_outcome, log.verbose());
    persist_state(&app, &saved, &log);
    emit_output(&app, &cli, log.verbose())
}

// ── Start-up helpers ──────────────────────────────────────────────────────────

fn log_environment(log: &mut Logger) {
    vlog!(log, "version {}", env!("CARGO_PKG_VERSION"));
    vlog!(
        log,
        "os={} arch={}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    vlog!(log, "args: {:?}", std::env::args().collect::<Vec<_>>());
    vlog!(
        log,
        "$HOME={:?} $SHELL={:?}",
        std::env::var("HOME").ok(),
        std::env::var("SHELL").ok()
    );
    vlog!(
        log,
        "terminal size={:?} stderr_is_tty={}",
        crossterm::terminal::size().ok(),
        crossterm::tty::IsTty::is_tty(&io::stderr())
    );
}

/// Handle `--info`, `--doctor` and `--init`. Returns `true` when one ran and
/// the program should exit.
fn run_utility_command(cli: &Cli, log: &mut Logger) -> bool {
    if cli.info {
        info::print_info();
        return true;
    }
    if cli.doctor {
        doctor::run_doctor();
        return true;
    }
    if cli.dump_keymap {
        print!("{}", tui_file_explorer::default_keymap_toml());
        return true;
    }
    vlog!(log, "checking --init flag");
    let Some(ref shell_name) = cli.init else {
        return false;
    };
    let Some(shell) = shell_init::Shell::from_str(shell_name) else {
        eprintln!(
            "tfe: unrecognised shell '{shell_name}'. \
             Supported: bash, zsh, fish, powershell, nushell"
        );
        process::exit(2);
    };
    use shell_init::InitOutcome;
    match shell_init::install_or_print(Some(shell)) {
        InitOutcome::Installed(path) => {
            eprintln!("tfe: shell integration installed to {}", path.display());
            eprintln!("  Activate now : source {}", path.display());
            eprintln!("  Or just open a new terminal window.");
        }
        InitOutcome::AlreadyInstalled(path) => {
            eprintln!(
                "tfe: shell integration already present in {}",
                path.display()
            );
        }
        // Diagnostic already printed by install_or_print.
        InitOutcome::PrintedToStdout | InitOutcome::UnknownShell => {}
    }
    true
}

fn print_themes(themes: &[(&'static str, Theme)]) {
    for (name, _) in themes {
        println!("{name}");
    }
}

/// Merge CLI flags, persisted state and defaults into [`AppOptions`].
///
/// Priority everywhere: explicit CLI flag > persisted value > default.
/// Returns `Ok(None)` when PATH named a file that was opened directly in the
/// editor (no TUI needed).
fn resolve_options(
    cli: &Cli,
    saved: &AppState,
    themes: &[(&'static str, Theme)],
    log: &mut Logger,
) -> io::Result<Option<AppOptions>> {
    let theme_name = cli
        .theme
        .clone()
        .or_else(|| saved.theme.clone())
        .unwrap_or_else(|| "default".to_string());
    let theme_idx = resolve_theme_idx(&theme_name, themes);
    vlog!(log, "resolved theme: '{theme_name}' (index {theme_idx})");

    let (cli_editor, cli_path) = cli.editor_and_path();
    let editor = match (&cli_editor, &saved.editor) {
        (Some(raw), _) => Editor::from_key(raw).unwrap_or_else(|| Editor::Custom(raw.clone())),
        (None, Some(raw)) => Editor::from_key(raw).unwrap_or_default(),
        (None, None) => Editor::default(),
    };
    vlog!(
        log,
        "editor: {} (binary: {:?})",
        editor.label(),
        editor.binary()
    );

    // PATH may be a file: `tfe myfile.rs` is then a quick "open in editor".
    vlog!(log, "resolving start directory (cli_path={cli_path:?})");
    let start_dir = match cli_path {
        Some(ref p) => {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            if c.is_dir() {
                c
            } else if c.is_file() {
                vlog!(log, "cli path is file, opening directly: {}", c.display());
                open_file_directly(&c, &editor)?;
                return Ok(None);
            } else {
                eprintln!("tfe: {p:?} — no such file or directory");
                process::exit(2);
            }
        }
        None => saved
            .last_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
    };
    vlog!(log, "start_dir: {}", start_dir.display());

    let right_start_dir = saved
        .last_dir_right
        .clone()
        .unwrap_or_else(|| start_dir.clone());

    // An explicit CLI path always starts with the classic 2-pane layout;
    // otherwise prefer the persisted multi-pane list, falling back to the
    // legacy left/right pair for older state files.
    let pane_dirs: Vec<PathBuf> = match (&cli_path, &saved.pane_dirs) {
        (None, Some(dirs)) => dirs.clone(),
        _ => vec![start_dir, right_start_dir],
    };
    vlog!(log, "pane_dirs: {pane_dirs:?}");

    // Plain bool flags cannot tell "not passed" from "passed as false", so for
    // these `true` on the CLI always wins; the paired flags (sizes, cd) use
    // explicit on/off overrides instead.
    let show_hidden = cli.hidden || saved.show_hidden.unwrap_or(false);
    let single_pane = cli.single_pane || saved.single_pane.unwrap_or(false);
    let show_sizes = cli.sizes_override().or(saved.show_sizes).unwrap_or(true);
    let show_dir_sizes = cli
        .dir_sizes_override()
        .or(saved.show_dir_sizes)
        .unwrap_or(true);
    // Safer default for the CLI: trash when this build supports it.
    let use_trash = cli.trash_override().or(saved.use_trash).unwrap_or(true);
    let show_git = cli.git_override().or(saved.show_git).unwrap_or(false);
    let cd_on_exit = if cli.cd_on_exit {
        true
    } else if cli.no_cd {
        false
    } else {
        saved.cd_on_exit.unwrap_or(true)
    };
    vlog!(
        log,
        "show_hidden={show_hidden} show_sizes={show_sizes} show_dir_sizes={show_dir_sizes} use_trash={use_trash} show_git={show_git} \
         single_pane={single_pane} cd_on_exit={cd_on_exit}"
    );

    Ok(Some(AppOptions {
        pane_dirs,
        extensions: cli.extensions.clone(),
        show_hidden,
        show_sizes,
        show_dir_sizes,
        use_trash,
        show_git,
        // Never let a slow disk freeze the TUI.
        async_loading: true,
        bookmarks: saved.bookmarks.clone().unwrap_or_default(),
        theme_idx,
        show_theme_panel: cli.show_themes,
        single_pane,
        sort_mode: saved.sort_mode.unwrap_or_default(),
        cd_on_exit,
        editor,
        verbose: cli.verbose,
        startup_log: Vec::new(),
    }))
}

/// Load `keymap.toml` (if present) and apply it. Problems are reported on
/// stderr and never abort startup.
fn apply_user_keymap(app: &mut App, log: &mut Logger) {
    match tui_file_explorer::load_keymap_config() {
        Ok(Some(cfg)) => {
            for w in &cfg.warnings {
                eprintln!("tfe: keymap.toml: {w}");
            }
            vlog!(
                log,
                "applied keymap.toml ({} warning(s))",
                cfg.warnings.len()
            );
            app.apply_keymap_config(&cfg);
        }
        Ok(None) => {}
        Err(e) => eprintln!("tfe: ignoring keymap.toml: {e}"),
    }
}

/// Restore the persisted active pane: prefer the explicit index, falling back
/// to the legacy `"left"`/`"right"` string of older state files.
fn restore_active_pane(app: &mut App, saved: &AppState) {
    if let Some(idx) = saved.active_pane_idx {
        if idx < app.panes.len() {
            app.active_idx = idx;
        }
    } else if let Some(ref pane_str) = saved.active_pane {
        app.active_idx = match pane_str.as_str() {
            "right" => 1.min(app.panes.len().saturating_sub(1)),
            _ => 0,
        };
    }
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop(guard: &mut TerminalGuard, app: &mut App, log: &mut Logger) -> io::Result<()> {
    loop {
        guard.terminal().draw(|frame| draw(app, frame))?;

        if app.handle_event()? {
            app.log("handle_event signalled exit".to_string());
            log.write_file("[tfe] handle_event signalled exit");
            return Ok(());
        }

        // `handle_event` sets the editor request on `e` / Enter-on-file. It is
        // served here because we own the terminal that must be torn down.
        if let Some((path, editor)) = app.take_editor_request() {
            launch_editor_from_tui(guard, app, log, &path, &editor);
        }
    }
}

/// Suspend the TUI, run the editor on `path`, then resume and reload panes.
fn launch_editor_from_tui(
    guard: &mut TerminalGuard,
    app: &mut App,
    log: &mut Logger,
    path: &std::path::Path,
    editor: &Editor,
) {
    // `Editor::None` should never request an editor; discard defensively.
    let Some(binary_str) = editor.binary() else {
        return;
    };
    let label = editor.label().to_string();
    let msg = format!(
        "launching editor '{label}' ({binary_str}) for {}",
        path.display()
    );
    app.log(msg.clone());
    log.write_file(&format!("[tfe] {msg}"));

    let (binary, extra_args) = split_command(&binary_str);

    guard.suspend();
    // Explicit /dev/tty: the editor must get the real terminal even when our
    // stdout is a pipe (the `dir=$(command tfe)` wrapper).
    let status = spawn_editor(&binary, &extra_args, path, Stdio::ControllingTty);
    guard.resume();

    // A bulk rename edited externally is applied now.
    app.editor_finished(path);

    app.log("reloading panes after editor exit".to_string());
    for p in &mut app.panes {
        p.reload();
    }

    app.status_msg = match status {
        Ok(s) if s.success() => {
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            format!("Returned from {label} \u{2014} {fname}")
        }
        Ok(s) => format!("Editor exited with status {}", s.code().unwrap_or(-1)),
        Err(e) => format!("Error launching '{binary}': {e}"),
    };
}

// ── Shutdown helpers ──────────────────────────────────────────────────────────

/// If the shell wrapper was freshly installed, emit a `source:<path>`
/// directive on stdout *before* any cd path so the wrapper sources the rc
/// file in the parent shell, and tell the user on stderr.
fn announce_shell_install(outcome: &shell_init::InitOutcome, verbose: bool) {
    if let shell_init::InitOutcome::Installed(rc_path) = outcome {
        if verbose {
            eprintln!("[tfe] emitting source directive for {}", rc_path.display());
        }
        shell_init::emit_source_directive(rc_path);
        eprintln!("tfe: shell integration installed to {}", rc_path.display());
        eprintln!("  The wrapper function has been sourced into this session automatically.");
        eprintln!("  cd-on-exit is now active — no restart needed.");
    }
}

/// Persist the full application state on clean exit.
///
/// `pane_dirs` is the source of truth; `last_dir` / `last_dir_right` are still
/// written so older `tfe` binaries see a sensible 2-pane layout. In
/// single-pane mode the hidden panes were not navigated this session, so the
/// previously persisted values are kept rather than clobbered.
fn persist_state(app: &App, saved: &AppState, log: &Logger) {
    let pane_dirs: Vec<PathBuf> = if app.single_pane {
        saved
            .pane_dirs
            .clone()
            .unwrap_or_else(|| app.panes.iter().map(|p| p.current_dir.clone()).collect())
    } else {
        app.panes.iter().map(|p| p.current_dir.clone()).collect()
    };
    let last_dir_right = if app.single_pane {
        saved.last_dir_right.clone()
    } else {
        pane_dirs.get(1).cloned()
    };

    if log.verbose() {
        eprintln!("[tfe] saving state");
    }
    let state = AppState {
        theme: Some(app.theme_name().to_string()),
        last_dir: Some(app.panes[0].current_dir.clone()),
        last_dir_right,
        sort_mode: Some(app.panes[0].sort_mode),
        show_hidden: Some(app.panes[0].show_hidden),
        show_sizes: Some(app.panes[0].show_sizes),
        show_dir_sizes: Some(app.panes[0].show_dir_sizes),
        use_trash: Some(app.use_trash),
        show_git: Some(app.panes[0].show_git),
        bookmarks: Some(app.bookmarks.clone()),
        single_pane: Some(app.single_pane),
        // Captures CLI flags and any in-TUI toggle (O panel / C key).
        cd_on_exit: Some(app.cd_on_exit),
        editor: Some(app.editor.to_key()),
        active_pane: Some(if app.active_idx == 0 { "left" } else { "right" }.to_string()),
        pane_dirs: Some(pane_dirs),
        active_pane_idx: Some(app.active_idx),
    };
    // Best-effort, but never silent: a failed save is a warning, not a crash.
    if let Err(e) = try_save_state(&state) {
        eprintln!("tfe: warning: could not save state: {e}");
    }
}

/// Write the result path to stdout for the shell wrapper.
///
/// * file selected          → the path (or its parent with `--print-dir`)
/// * dismissed + cd_on_exit → the active pane's directory (wrapper `cd`s)
/// * dismissed otherwise    → nothing, exit status 1
fn emit_output(app: &App, cli: &Cli, verbose: bool) -> io::Result<()> {
    let output = match &app.selected {
        Some(path) => {
            if verbose {
                eprintln!("[tfe] file selected: {}", path.display());
            }
            resolve_output_path(path.clone(), cli.print_dir)
        }
        None if app.cd_on_exit => {
            if verbose {
                eprintln!("[tfe] dismissed with cd_on_exit=true");
            }
            app.active_pane().current_dir.clone()
        }
        None => {
            if verbose {
                eprintln!("[tfe] dismissed without selection, cd_on_exit=false, exit 1");
            }
            process::exit(1);
        }
    };
    let mut out = stdout();
    write!(out, "{}", output.display())?;
    out.write_all(if cli.null { b"\0" } else { b"\n" })?;
    out.flush()
}
