//! Verbose start-up / debug logging for the `tfe` binary.

use std::{
    fs::File,
    io::Write,
    sync::OnceLock,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// Process-start instant used to stamp every log line.
static START: OnceLock<Instant> = OnceLock::new();

/// Collects log lines in memory (later handed to the in-TUI debug panel),
/// mirrors them to stderr, and optionally appends them to a file.
///
/// Does nothing unless `verbose` is set. Uses `\r\n` line endings so output
/// stays readable after `enable_raw_mode()`.
pub struct Logger {
    verbose: bool,
    buf: Vec<String>,
    file: Option<File>,
}

impl Logger {
    pub fn new(verbose: bool) -> Self {
        let file = if verbose { open_debug_log_file() } else { None };
        Self {
            verbose,
            buf: Vec::new(),
            file,
        }
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn log(&mut self, msg: impl AsRef<str>) {
        if !self.verbose {
            return;
        }
        let elapsed = START.get_or_init(Instant::now).elapsed().as_secs_f64();
        let line = format!("[tfe {elapsed:.3}s] {}", msg.as_ref());
        let _ = std::io::stderr().write_all(format!("{line}\r\n").as_bytes());
        self.write_file(&line);
        self.buf.push(line);
    }

    /// Append a raw line to the log file only (no stderr, no buffer).
    pub fn write_file(&mut self, line: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }

    /// Take the buffered lines (to seed `AppOptions::startup_log`).
    pub fn take_buffer(&mut self) -> Vec<String> {
        std::mem::take(&mut self.buf)
    }
}

/// `vlog!(logger, "fmt {}", args…)` — format lazily, only when verbose.
macro_rules! vlog {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.verbose() {
            $logger.log(format!($($arg)*));
        }
    };
}
pub(crate) use vlog;

fn open_debug_log_file() -> Option<File> {
    let path = std::env::temp_dir().join("tfe-debug.log");
    match File::options().create(true).append(true).open(&path) {
        Ok(mut f) => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let _ = writeln!(f, "\n--- tfe session {timestamp} ---");
            let _ = f.flush();
            eprintln!("[tfe] debug log: {}\r", path.display());
            Some(f)
        }
        Err(e) => {
            eprintln!("[tfe] warning: could not create log file: {e}\r");
            None
        }
    }
}
