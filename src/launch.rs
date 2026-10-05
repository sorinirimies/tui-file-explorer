//! External-editor launching for the `tfe` binary.
//!
//! One place owns the platform-specific details of spawning an editor so the
//! interactive path (editor opened from inside the TUI) and the direct path
//! (`tfe <file>`) cannot drift apart.

use std::{
    io,
    path::Path,
    process::{self, Command, ExitStatus},
};

use tui_file_explorer::app::Editor;

/// Split an editor command string into `(binary, extra_args)`.
///
/// A minimal whitespace split — no shell quoting — so `Custom("code --wait")`
/// becomes `("code", ["--wait"])`.
pub fn split_command(command: &str) -> (String, Vec<String>) {
    let mut parts = command.split_whitespace();
    let binary = parts.next().unwrap_or(command).to_string();
    (binary, parts.map(str::to_string).collect())
}

/// Where the editor's stdio should be attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stdio {
    /// Inherit this process's stdio.
    Inherit,
    /// Attach stdin/stdout/stderr to the controlling terminal (`/dev/tty`).
    ///
    /// Needed inside the shell wrapper (`dir=$(command tfe)`), where our own
    /// stdout is a pipe. A no-op on Windows.
    ControllingTty,
}

/// Spawn `binary extra_args… path` and wait for it to exit.
///
/// * Unix: runs the binary directly (aliases/functions are not available,
///   which is fine — editors are real executables).
/// * Windows: routes through `cmd /C` so `.cmd` / `.bat` shims (VS Code,
///   scoop/winget installs, …) resolve via `PATH`.
pub fn spawn_editor(
    binary: &str,
    extra_args: &[String],
    path: &Path,
    stdio: Stdio,
) -> io::Result<ExitStatus> {
    #[cfg(windows)]
    let mut cmd = {
        let _ = stdio;
        let mut c = Command::new("cmd");
        c.arg("/C").arg(binary);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(binary);

    cmd.args(extra_args).arg(path);

    #[cfg(not(windows))]
    if stdio == Stdio::ControllingTty {
        if let Ok(tty) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
        {
            // `try_clone` dup()s the descriptor — no `unsafe` required.
            cmd.stdin(tty.try_clone()?)
                .stdout(tty.try_clone()?)
                .stderr(tty);
        }
    }

    cmd.status()
}

/// Open `path` in `editor` without starting the TUI (`tfe <file>`).
///
/// Exits the process with the editor's status code on failure.
pub fn open_file_directly(path: &Path, editor: &Editor) -> io::Result<()> {
    let Some(binary_str) = editor.binary() else {
        let fname = path.file_name().unwrap_or_default().to_string_lossy();
        eprintln!("tfe: cannot open '{fname}' — no editor configured.");
        eprintln!();
        eprintln!("  Set an editor with one of:");
        eprintln!("    tfe --editor nvim          # use Neovim");
        eprintln!("    tfe --editor vim           # use Vim");
        eprintln!("    tfe --editor helix         # use Helix");
        eprintln!("    tfe --editor nano          # use Nano");
        eprintln!("    tfe --editor \"code --wait\"  # use VS Code");
        eprintln!();
        eprintln!("  Or pick one interactively: run tfe, then press Shift+E.");
        eprintln!("  The selection is persisted across sessions.");
        process::exit(2);
    };

    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let editor_label = editor.label();
    eprintln!("tfe: opening '{fname}' in {editor_label}…");

    let (binary, extra_args) = split_command(&binary_str);
    match spawn_editor(&binary, &extra_args, path, Stdio::Inherit) {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            eprintln!("tfe: {editor_label} exited with status {code}");
            process::exit(code);
        }
        Err(e) => {
            eprintln!("tfe: failed to launch '{binary}': {e}");
            eprintln!();
            eprintln!("  Make sure '{binary}' is installed and on your PATH.");
            eprintln!("  Change the editor with: tfe --editor <name>");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_command_plain_binary() {
        assert_eq!(split_command("nvim"), ("nvim".to_string(), vec![]));
    }

    #[test]
    fn split_command_keeps_extra_args() {
        assert_eq!(
            split_command("code --wait  --new-window"),
            (
                "code".to_string(),
                vec!["--wait".to_string(), "--new-window".to_string()]
            )
        );
    }

    #[test]
    fn split_command_empty_falls_back_to_input() {
        assert_eq!(split_command(""), (String::new(), vec![]));
    }

    #[cfg(unix)]
    #[test]
    fn spawn_editor_reports_exit_status() {
        let ok = spawn_editor("true", &[], Path::new("x"), Stdio::Inherit).unwrap();
        assert!(ok.success());
        let bad = spawn_editor("false", &[], Path::new("x"), Stdio::Inherit).unwrap();
        assert!(!bad.success());
    }

    #[cfg(unix)]
    #[test]
    fn spawn_editor_missing_binary_is_error() {
        assert!(spawn_editor(
            "definitely-not-an-editor-xyz",
            &[],
            Path::new("x"),
            Stdio::Inherit
        )
        .is_err());
    }
}
