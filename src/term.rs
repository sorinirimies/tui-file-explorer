//! Terminal lifecycle and image-protocol detection for the `tfe` binary.

use std::io::{self, Stderr};

use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// RAII owner of the TUI terminal.
///
/// The TUI renders on **stderr** (stdout is reserved for the path the shell
/// wrapper captures with `dir=$(command tfe)`). Raw mode and the alternate
/// screen are restored on [`Drop`], so the terminal is left usable on every
/// exit path — including errors and panics.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stderr>>,
    active: bool,
}

impl TerminalGuard {
    /// Enter raw mode + alternate screen and build the ratatui terminal.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        // Build the guard-less state first so a failure below still restores.
        let setup = (|| {
            execute!(io::stderr(), EnterAlternateScreen)?;
            // Defensively disable mouse capture: a previous TUI may have
            // crashed with tracking on, and some terminals (notably macOS)
            // persist it, flooding stdin with SGR sequences.
            execute!(io::stderr(), DisableMouseCapture)?;
            Terminal::new(CrosstermBackend::new(io::stderr()))
        })();
        match setup {
            Ok(terminal) => Ok(Self {
                terminal,
                active: true,
            }),
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = execute!(io::stderr(), LeaveAlternateScreen);
                Err(e)
            }
        }
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stderr>> {
        &mut self.terminal
    }

    /// Hand the real terminal to a child process (e.g. an editor).
    pub fn suspend(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }

    /// Take the terminal back after [`suspend`](Self::suspend).
    pub fn resume(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), EnterAlternateScreen);
        // The child may have enabled mouse tracking — switch it off so stray
        // events don't flood the event loop.
        let _ = execute!(self.terminal.backend_mut(), DisableMouseCapture);
        self.force_full_redraw();
    }

    /// Wipe the screen and make the next frame repaint every cell.
    ///
    /// Deliberately **not** `Terminal::clear()`: in current ratatui that first
    /// asks the terminal for the cursor position (`CSI 6n`) and returns early
    /// on any error — without clearing and without resetting the diff
    /// buffer. After a full-screen child such as Helix has run, that reply
    /// can be lost or late (it timed out here), so the screen kept the
    /// child's leftovers and only the few cells that changed were redrawn:
    /// a broken layout. A fresh `Terminal` has empty buffers, so the next
    /// draw repaints everything, and no cursor query is involved.
    fn force_full_redraw(&mut self) {
        use crossterm::{
            cursor::MoveTo,
            terminal::{Clear, ClearType},
        };
        let _ = execute!(
            self.terminal.backend_mut(),
            Clear(ClearType::All),
            MoveTo(0, 0)
        );
        if let Ok(fresh) = Terminal::new(CrosstermBackend::new(io::stderr())) {
            self.terminal = fresh;
        }
    }

    /// Restore the terminal now (idempotent; also run on drop).
    pub fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Build the image-preview picker without a stdin round-trip.
///
/// `Picker::from_query_stdio()` can leave a background thread blocked on
/// stdin that then swallows the user's first keystrokes (seen on macOS with
/// Ghostty). Instead we pick the protocol from the terminal's identity
/// ([`detect_image_protocol`]) and read the cell pixel size via the
/// `TIOCGWINSZ` ioctl — pure lookups that cannot hang. Unknown terminals, or
/// a non-tty stdout (the `$(command tfe)` wrapper), fall back to halfblocks.
///
/// Returns the picker plus a one-line description for the debug log.
pub fn build_image_picker() -> (ratatui_image::picker::Picker, String) {
    use ratatui_image::picker::{Picker, ProtocolType};

    let stdout_is_tty = crossterm::tty::IsTty::is_tty(&io::stdout());
    let term = std::env::var("TERM").ok();
    let term_program = std::env::var("TERM_PROGRAM").ok();
    let protocol = if stdout_is_tty {
        detect_image_protocol(term.as_deref(), term_program.as_deref(), |name| {
            std::env::var_os(name).is_some()
        })
    } else {
        ProtocolType::Halfblocks
    };
    let font_size = crossterm::terminal::window_size()
        .ok()
        .filter(|ws| ws.columns > 0 && ws.rows > 0 && ws.width > 0 && ws.height > 0)
        .map(|ws| (ws.width / ws.columns, ws.height / ws.rows));
    let desc = format!(
        "image protocol: stdout_is_tty={stdout_is_tty}, TERM={term:?}, \
         TERM_PROGRAM={term_program:?}, protocol={protocol:?}, font_size={font_size:?}"
    );

    let picker = match font_size {
        Some(fs) if protocol != ProtocolType::Halfblocks => {
            // `from_fontsize` is deprecated, but it is the only constructor
            // that takes a known cell size without the racy stdin probe.
            #[allow(deprecated)]
            let mut picker = Picker::from_fontsize(fs.into());
            picker.set_protocol_type(protocol);
            picker
        }
        _ => Picker::halfblocks(),
    };
    (picker, desc)
}

/// Pick the terminal graphics protocol for image previews from the terminal's
/// *identity* alone — no escape-sequence probe, no stdin round-trip.
///
/// The usual API, [`ratatui_image::picker::Picker::from_query_stdio`], writes
/// capability queries to stdout and blocks a background thread on stdin waiting
/// for the reply.  Terminals that don't answer (or answer in an unexpected
/// shape — observed on macOS/Ghostty) make the probe time out *and* leave that
/// thread alive; it then races the TUI event loop for stdin and swallows the
/// user's first keypresses, so the app looks like it never opened.
///
/// We avoid the probe completely by mapping well-known terminals to the
/// protocol they implement, defaulting to the always-safe half-blocks renderer
/// for anything we don't recognise.  `env_present` reports whether a given
/// environment variable is set, keeping this function pure and testable.
fn detect_image_protocol(
    term: Option<&str>,
    term_program: Option<&str>,
    env_present: impl Fn(&str) -> bool,
) -> ratatui_image::picker::ProtocolType {
    use ratatui_image::picker::ProtocolType;

    // Terminal-specific marker variables are the most reliable signal because
    // they are set by the terminal itself; when present they are unambiguous.
    // kitty & Ghostty speak the kitty graphics protocol; Konsole implements it
    // too.  iTerm2 and WezTerm speak the iTerm2 inline-image protocol.
    const KITTY_ENV_MARKERS: &[&str] = &[
        "KITTY_WINDOW_ID",       // kitty
        "GHOSTTY_RESOURCES_DIR", // ghostty
        "KONSOLE_VERSION",       // Konsole (kitty graphics)
    ];
    const ITERM2_ENV_MARKERS: &[&str] = &[
        "WEZTERM_PANE",       // WezTerm
        "WEZTERM_EXECUTABLE", // WezTerm
        "ITERM_SESSION_ID",   // iTerm2
    ];
    if KITTY_ENV_MARKERS.iter().any(|m| env_present(m)) {
        return ProtocolType::Kitty;
    }
    if ITERM2_ENV_MARKERS.iter().any(|m| env_present(m)) {
        return ProtocolType::Iterm2;
    }

    // TERM_PROGRAM identifies a handful of graphics-capable terminals.
    if let Some(tp) = term_program {
        match tp {
            "ghostty" => return ProtocolType::Kitty,
            "WezTerm" | "iTerm.app" => return ProtocolType::Iterm2,
            _ => {}
        }
    }

    // TERM substrings catch the common cases (e.g. kitty sets
    // `TERM=xterm-kitty`, ghostty `xterm-ghostty`, foot `foot`).
    if let Some(t) = term {
        let t = t.to_ascii_lowercase();
        if t.contains("kitty") || t.contains("ghostty") {
            return ProtocolType::Kitty;
        }
        if t.contains("wezterm") {
            return ProtocolType::Iterm2;
        }
        // `foot` implements sixel.
        if t.contains("foot") {
            return ProtocolType::Sixel;
        }
    }

    ProtocolType::Halfblocks
}

#[cfg(test)]
mod tests {
    use super::detect_image_protocol;
    use ratatui_image::picker::ProtocolType;

    /// Helper: no environment markers present.
    fn no_env(_: &str) -> bool {
        false
    }

    #[test]
    fn apple_terminal_falls_back_to_halfblocks() {
        // Regression: Apple_Terminal has no graphics protocol — must never be
        // probed and must render via halfblocks.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("Apple_Terminal"), no_env),
            ProtocolType::Halfblocks,
        );
    }

    #[test]
    fn warp_and_unknown_terminals_use_halfblocks() {
        // Warp and other silent terminals previously hung the app on startup.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("WarpTerminal"), no_env),
            ProtocolType::Halfblocks,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), None, no_env),
            ProtocolType::Halfblocks,
        );
        // No TERM and no TERM_PROGRAM at all (e.g. a bare pty) must be safe too.
        assert_eq!(
            detect_image_protocol(None, None, no_env),
            ProtocolType::Halfblocks,
        );
    }

    #[test]
    fn kitty_is_detected_via_term_and_env() {
        assert_eq!(
            detect_image_protocol(Some("xterm-kitty"), None, no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), None, |name| name
                == "KITTY_WINDOW_ID"),
            ProtocolType::Kitty,
        );
    }

    #[test]
    fn ghostty_uses_kitty_protocol() {
        // The macOS/Ghostty regression: Ghostty must be recognised (kitty
        // graphics) without a stdin probe.
        assert_eq!(
            detect_image_protocol(Some("xterm-256color"), Some("ghostty"), no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(Some("xterm-ghostty"), None, no_env),
            ProtocolType::Kitty,
        );
        assert_eq!(
            detect_image_protocol(None, None, |name| name == "GHOSTTY_RESOURCES_DIR"),
            ProtocolType::Kitty,
        );
    }

    #[test]
    fn wezterm_and_iterm_use_iterm2_protocol() {
        for tp in ["WezTerm", "iTerm.app"] {
            assert_eq!(
                detect_image_protocol(Some("xterm-256color"), Some(tp), no_env),
                ProtocolType::Iterm2,
                "expected {tp} to use the iTerm2 protocol"
            );
        }
        assert_eq!(
            detect_image_protocol(Some("wezterm"), None, no_env),
            ProtocolType::Iterm2,
        );
    }

    #[test]
    fn env_markers_map_to_expected_protocols() {
        for marker in [
            "KITTY_WINDOW_ID",
            "GHOSTTY_RESOURCES_DIR",
            "KONSOLE_VERSION",
        ] {
            assert_eq!(
                detect_image_protocol(None, None, |name| name == marker),
                ProtocolType::Kitty,
                "expected env marker {marker} to map to kitty"
            );
        }
        for marker in ["WEZTERM_PANE", "WEZTERM_EXECUTABLE", "ITERM_SESSION_ID"] {
            assert_eq!(
                detect_image_protocol(None, None, |name| name == marker),
                ProtocolType::Iterm2,
                "expected env marker {marker} to map to iTerm2"
            );
        }
    }

    #[test]
    fn foot_uses_sixel() {
        assert_eq!(
            detect_image_protocol(Some("foot"), None, no_env),
            ProtocolType::Sixel,
        );
    }
}
