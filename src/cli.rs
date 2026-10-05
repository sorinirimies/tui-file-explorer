//! Command-line interface for the `tfe` binary.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "tfe",
    version,
    about = "Keyboard-driven two-pane terminal file explorer",
    after_help = "\
SHELL INTEGRATION (cd on exit):\n\
  Step 1 — enable the feature (persisted across sessions):\n\
    tfe --cd\n\
\n\
  Step 2 — install the shell wrapper (one time):\n\
    tfe --init bash        # ~/.bashrc\n\
    tfe --init zsh         # ~/.zshrc\n\
    tfe --init fish        # ~/.config/fish/functions/tfe.fish\n\
    tfe --init powershell  # $PROFILE (Windows / cross-platform PowerShell)\n\
    tfe --init nushell     # <config-dir>/nushell/config.nu\n\
\n\
  After both steps, dismissing tfe with Esc/q cd's your terminal to the\n\
  directory you were browsing.  Works on macOS, Linux, and Windows.\n\
\n\
  Open selected file in $EDITOR:     command tfe | xargs -r $EDITOR\n\
  NUL-delimited output:              command tfe -0 | xargs -0 wc -l"
)]
pub struct Cli {
    /// Starting directory or file to open [default: current directory]
    ///
    /// If PATH is a directory, the TUI file explorer opens there.
    /// If PATH is a file and an editor is configured (--editor or persisted),
    /// the file is opened directly in that editor without launching the TUI.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Only show/select files with these extensions (repeatable: -e rs -e toml)
    #[arg(short, long = "ext", value_name = "EXT", action = clap::ArgAction::Append)]
    pub extensions: Vec<String>,

    /// Show file and folder size info (persisted; toggle at runtime with z)
    #[arg(long = "sizes", overrides_with = "no_sizes")]
    pub sizes: bool,

    /// Hide all file and folder size info (persisted)
    #[arg(long = "no-sizes", overrides_with = "sizes")]
    pub no_sizes: bool,

    /// Compute recursive folder sizes (persisted; toggle at runtime with Z).
    /// Disable on slow or huge filesystems — folders then show item counts.
    #[arg(long = "dir-sizes", overrides_with = "no_dir_sizes")]
    pub dir_sizes: bool,

    /// Do not compute recursive folder sizes (persisted); show item counts
    #[arg(long = "no-dir-sizes", overrides_with = "dir_sizes")]
    pub no_dir_sizes: bool,

    /// Send deleted entries to the OS trash / recycle bin (persisted; default)
    #[arg(long = "trash", overrides_with = "no_trash")]
    pub trash: bool,

    /// Delete permanently instead of using the trash (persisted)
    #[arg(long = "no-trash", overrides_with = "trash")]
    pub no_trash: bool,

    /// Show git change markers (M, A, ?, …) next to entries (persisted; toggle with v)
    #[arg(long = "git", overrides_with = "no_git")]
    pub git: bool,

    /// Hide git change markers (persisted)
    #[arg(long = "no-git", overrides_with = "git")]
    pub no_git: bool,

    /// Print a complete keymap.toml with the built-in key bindings and exit
    #[arg(long)]
    pub dump_keymap: bool,

    /// Show hidden (dot-file) entries on startup
    #[arg(short = 'H', long)]
    pub hidden: bool,

    /// Colour theme [default: persisted selection, or "default"]. Use --list-themes to see options.
    #[arg(short, long, value_name = "THEME")]
    pub theme: Option<String>,

    /// List all available themes and exit
    #[arg(long)]
    pub list_themes: bool,

    /// Open the theme panel on startup (toggle at runtime with T)
    #[arg(long)]
    pub show_themes: bool,

    /// Start in single-pane mode (toggle at runtime with w)
    #[arg(long)]
    pub single_pane: bool,

    /// Print the selected file's parent directory instead of the full path
    #[arg(long)]
    pub print_dir: bool,

    /// Terminate output with a NUL byte instead of a newline
    #[arg(short = '0', long)]
    pub null: bool,

    /// Install the shell wrapper for cd-on-exit integration and exit.
    /// Install the shell wrapper for `shell` (auto-detected when `None`).
    ///
    /// Appends the wrapper function to your rc file (creating it if needed)
    /// and prints instructions.  Supported shells: bash, zsh, fish, powershell, nushell.
    /// Examples: tfe --init zsh   tfe --init nushell   tfe --init powershell
    #[arg(long, value_name = "SHELL")]
    pub init: Option<String>,

    /// Enable cd-on-exit: on dismiss, print the active pane's current directory
    /// to stdout so the shell wrapper can cd to it.  The setting is persisted —
    /// run once to enable, run with --no-cd to disable.
    #[arg(long = "cd", overrides_with = "no_cd")]
    pub cd_on_exit: bool,

    /// Disable cd-on-exit (persisted).  Dismissing without a selection will
    /// print nothing and exit with code 1.
    #[arg(long = "no-cd", overrides_with = "cd_on_exit")]
    pub no_cd: bool,

    /// Print version, platform, and environment info, then exit.
    #[arg(long)]
    pub info: bool,

    /// Run diagnostic checks (environment, shell integration, config,
    /// terminal, editor) and exit.  Each check is assessed with
    /// pass / warn / fail indicators and actionable advice.
    #[arg(long)]
    pub doctor: bool,

    /// Enable verbose mode: print startup diagnostics to stderr, write all
    /// logs to a file (path printed at launch), and show a debug log panel
    /// inside the TUI.  Use `tail -f /tmp/tfe-debug.log` from another
    /// terminal for real-time monitoring.
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Editor to open when pressing `e` on a file.
    ///
    /// Accepted values: helix (hx), nvim, vim, nano, micro, or any binary
    /// name / path.  Overrides the persisted setting for this session only.
    ///
    /// Examples:
    ///   tfe --editor nvim
    ///   tfe --editor vim
    ///   tfe --editor "code --wait"
    #[arg(long, value_name = "EDITOR")]
    pub editor: Option<String>,
}

/// Editor names `--editor` legitimately accepts. Anything else that also
/// exists as a file on disk was almost certainly meant as the PATH argument.
const KNOWN_EDITORS: &[&str] = &[
    "none",
    "helix",
    "hx",
    "nvim",
    "vim",
    "nano",
    "micro",
    "emacs",
    "vscode",
    "code",
    "zed",
    "xcode",
    "android-studio",
    "rustrover",
    "intellij",
    "webstorm",
    "pycharm",
    "goland",
    "clion",
    "fleet",
    "sublime",
    "rubymine",
    "phpstorm",
    "rider",
    "eclipse",
];

impl Cli {
    /// Resolve `(--editor, PATH)`, fixing the `tfe --editor file.txt` misparse.
    ///
    /// clap hands `file.txt` to `--editor` and leaves PATH empty. When the
    /// `--editor` value is not a known editor, exists as a file, and no PATH
    /// was given, treat it as the PATH instead.
    pub fn editor_and_path(&self) -> (Option<String>, Option<PathBuf>) {
        match (&self.editor, &self.path) {
            (Some(val), None) => {
                let known = KNOWN_EDITORS.iter().any(|n| n.eq_ignore_ascii_case(val));
                if !known && std::path::Path::new(val).is_file() {
                    (None, Some(PathBuf::from(val)))
                } else {
                    (Some(val.clone()), None)
                }
            }
            _ => (self.editor.clone(), self.path.clone()),
        }
    }

    /// `Some(true/false)` when `--sizes` / `--no-sizes` was given.
    pub fn sizes_override(&self) -> Option<bool> {
        flag_pair(self.sizes, self.no_sizes)
    }

    /// `Some(true/false)` when `--git` / `--no-git` was given.
    pub fn git_override(&self) -> Option<bool> {
        flag_pair(self.git, self.no_git)
    }

    /// `Some(true/false)` when `--trash` / `--no-trash` was given.
    pub fn trash_override(&self) -> Option<bool> {
        flag_pair(self.trash, self.no_trash)
    }

    /// `Some(true/false)` when `--dir-sizes` / `--no-dir-sizes` was given.
    pub fn dir_sizes_override(&self) -> Option<bool> {
        flag_pair(self.dir_sizes, self.no_dir_sizes)
    }
}

fn flag_pair(on: bool, off: bool) -> Option<bool> {
    match (on, off) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(std::iter::once("tfe").chain(args.iter().copied()))
    }

    #[test]
    fn size_flags_default_to_no_override() {
        let c = parse(&[]);
        assert_eq!(c.sizes_override(), None);
        assert_eq!(c.dir_sizes_override(), None);
    }

    #[test]
    fn no_sizes_flag_disables() {
        assert_eq!(parse(&["--no-sizes"]).sizes_override(), Some(false));
        assert_eq!(parse(&["--sizes"]).sizes_override(), Some(true));
    }

    #[test]
    fn last_size_flag_wins() {
        assert_eq!(
            parse(&["--no-sizes", "--sizes"]).sizes_override(),
            Some(true)
        );
        assert_eq!(
            parse(&["--dir-sizes", "--no-dir-sizes"]).dir_sizes_override(),
            Some(false)
        );
    }

    #[test]
    fn trash_flags_override() {
        assert_eq!(parse(&[]).trash_override(), None);
        assert_eq!(parse(&["--trash"]).trash_override(), Some(true));
        assert_eq!(parse(&["--no-trash"]).trash_override(), Some(false));
    }

    #[test]
    fn git_flags_override() {
        assert_eq!(parse(&[]).git_override(), None);
        assert_eq!(parse(&["--git"]).git_override(), Some(true));
        assert_eq!(parse(&["--git", "--no-git"]).git_override(), Some(false));
    }

    #[test]
    fn known_editor_is_not_mistaken_for_path() {
        let c = parse(&["--editor", "nvim"]);
        assert_eq!(c.editor_and_path(), (Some("nvim".into()), None));
    }

    #[test]
    fn editor_value_that_is_a_file_becomes_path() {
        let dir = std::env::temp_dir().join("tfe-cli-test-file.txt");
        std::fs::write(&dir, "x").unwrap();
        let c = parse(&["--editor", dir.to_str().unwrap()]);
        assert_eq!(c.editor_and_path(), (None, Some(dir.clone())));
        let _ = std::fs::remove_file(dir);
    }
}
