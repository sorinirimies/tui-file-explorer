//! Unit tests for [`super`].

use super::*;
use std::path::PathBuf;

// ── DoctorReport output helpers ───────────────────────────────────────

#[test]
fn new_report_starts_empty() {
    let r = DoctorReport::new();
    assert_eq!(r.oks, 0);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 0);
    assert!(r.lines.is_empty());
}

#[test]
fn pass_increments_oks() {
    let mut r = DoctorReport::new();
    r.pass("good");
    assert_eq!(r.oks, 1);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 0);
}

#[test]
fn warn_increments_warnings() {
    let mut r = DoctorReport::new();
    r.warn("hmm", "fix it");
    assert_eq!(r.oks, 0);
    assert_eq!(r.warnings, 1);
    assert_eq!(r.failures, 0);
}

#[test]
fn fail_increments_failures() {
    let mut r = DoctorReport::new();
    r.fail("bad", "fix it");
    assert_eq!(r.oks, 0);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 1);
}

#[test]
fn pass_line_contains_ok_marker() {
    let mut r = DoctorReport::new();
    r.pass("looks good");
    assert!(r.output().contains("[ok]"));
    assert!(r.output().contains("looks good"));
}

#[test]
fn warn_line_contains_warn_marker() {
    let mut r = DoctorReport::new();
    r.warn("not ideal", "try this");
    let out = r.output();
    assert!(out.contains("[warn]"));
    assert!(out.contains("not ideal"));
    assert!(out.contains("-> try this"));
}

#[test]
fn fail_line_contains_fail_marker() {
    let mut r = DoctorReport::new();
    r.fail("broken", "do this");
    let out = r.output();
    assert!(out.contains("[FAIL]"));
    assert!(out.contains("broken"));
    assert!(out.contains("-> do this"));
}

#[test]
fn warn_with_empty_hint_omits_arrow() {
    let mut r = DoctorReport::new();
    r.warn("minor issue", "");
    let out = r.output();
    assert!(out.contains("[warn]"));
    assert!(!out.contains("->"));
}

#[test]
fn fail_with_empty_hint_omits_arrow() {
    let mut r = DoctorReport::new();
    r.fail("major issue", "");
    let out = r.output();
    assert!(out.contains("[FAIL]"));
    assert!(!out.contains("->"));
}

#[test]
fn heading_appears_in_output() {
    let mut r = DoctorReport::new();
    r.heading("My Section");
    assert!(r.output().contains("My Section"));
}

#[test]
fn detail_appears_indented() {
    let mut r = DoctorReport::new();
    r.detail("some detail");
    assert!(r.output().contains("         some detail"));
}

#[test]
fn write_to_produces_newline_separated_output() {
    let mut r = DoctorReport::new();
    r.heading("H");
    r.pass("ok");
    let mut buf = Vec::new();
    r.write_to(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("H\n"));
    assert!(s.contains("[ok]   ok\n"));
}

#[test]
fn multiple_checks_accumulate_counters() {
    let mut r = DoctorReport::new();
    r.pass("a");
    r.pass("b");
    r.warn("c", "");
    r.fail("d", "");
    assert_eq!(r.oks, 2);
    assert_eq!(r.warnings, 1);
    assert_eq!(r.failures, 1);
}

// ── check_platform ────────────────────────────────────────────────────

#[test]
fn check_platform_always_passes() {
    let mut r = DoctorReport::new();
    r.check_platform("macos", "aarch64", "unix");
    assert_eq!(r.oks, 1);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 0);
    let out = r.output();
    assert!(out.contains("Platform"));
    assert!(out.contains("macos aarch64 (unix)"));
}

// ── check_binary ──────────────────────────────────────────────────────

#[test]
fn check_binary_exe_found_cargo_on_path() {
    let mut r = DoctorReport::new();
    let exe = PathBuf::from("/usr/local/bin/tfe");
    let cargo = PathBuf::from("/home/user/.cargo/bin");
    let dirs = vec![PathBuf::from("/usr/bin"), cargo.clone()];
    r.check_binary(Some(&exe), Some(&cargo), &dirs);
    assert_eq!(r.oks, 2);
    assert_eq!(r.warnings, 0);
}

#[test]
fn check_binary_exe_missing() {
    let mut r = DoctorReport::new();
    r.check_binary(None, None, &[]);
    assert_eq!(r.oks, 0);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("could not resolve"));
}

#[test]
fn check_binary_cargo_not_on_path() {
    let mut r = DoctorReport::new();
    let exe = PathBuf::from("/usr/local/bin/tfe");
    let cargo = PathBuf::from("/home/user/.cargo/bin");
    let dirs = vec![PathBuf::from("/usr/bin")];
    r.check_binary(Some(&exe), Some(&cargo), &dirs);
    assert_eq!(r.oks, 1);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("NOT on $PATH"));
}

// ── check_environment ─────────────────────────────────────────────────

#[test]
fn check_environment_all_set() {
    let mut r = DoctorReport::new();
    let cwd = PathBuf::from("/home/user");
    r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
    assert_eq!(r.oks, 3);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 0);
}

#[test]
fn check_environment_home_missing_is_fail() {
    let mut r = DoctorReport::new();
    let cwd = PathBuf::from("/tmp");
    r.check_environment(None, Some("/bin/zsh"), Some(&cwd));
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("[FAIL]"));
    assert!(r.output().contains("home directory is not set"));
}

#[test]
fn check_environment_shell_missing_is_warn_on_unix_pass_on_windows() {
    let mut r = DoctorReport::new();
    let cwd = PathBuf::from("/tmp");
    r.check_environment(Some("/home/user"), None, Some(&cwd));
    if cfg!(windows) {
        // On Windows $SHELL is never set; the check accepts this as normal.
        assert_eq!(r.warnings, 0);
        assert_eq!(r.oks, 3); // home + shell-ok + cwd
    } else {
        assert_eq!(r.warnings, 1);
        assert!(r.output().contains("$SHELL is not set"));
    }
}

#[test]
fn check_environment_cwd_missing_is_fail() {
    let mut r = DoctorReport::new();
    r.check_environment(Some("/home/user"), Some("/bin/zsh"), None);
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("cannot read working directory"));
}

// ── check_terminal ────────────────────────────────────────────────────

#[test]
fn check_terminal_good_size_and_tty() {
    let mut r = DoctorReport::new();
    r.check_terminal(Some((120, 40)), true);
    assert_eq!(r.oks, 2);
    assert_eq!(r.warnings, 0);
    assert_eq!(r.failures, 0);
}

#[test]
fn check_terminal_small_size_is_warn() {
    let mut r = DoctorReport::new();
    r.check_terminal(Some((30, 5)), true);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("very small"));
}

#[test]
fn check_terminal_no_size_is_fail() {
    let mut r = DoctorReport::new();
    r.check_terminal(None, true);
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("could not query terminal size"));
}

#[test]
fn check_terminal_not_tty_is_warn() {
    let mut r = DoctorReport::new();
    r.check_terminal(Some((80, 24)), false);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("stderr is NOT a tty"));
}

#[test]
fn check_terminal_small_and_not_tty_both_count() {
    let mut r = DoctorReport::new();
    r.check_terminal(Some((20, 5)), false);
    assert_eq!(r.warnings, 2);
    assert_eq!(r.oks, 0);
}

// ── check_shell_integration ───────────────────────────────────────────

#[test]
fn check_shell_no_detection_is_warn() {
    let mut r = DoctorReport::new();
    r.check_shell_integration(None, None, false, false);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("could not detect shell"));
}

#[test]
fn check_shell_detected_wrapper_installed() {
    let mut r = DoctorReport::new();
    let rc = PathBuf::from("/home/user/.zshrc");
    r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, true);
    assert_eq!(r.oks, 3); // detected + rc file + wrapper installed
    assert_eq!(r.failures, 0);
}

#[test]
fn check_shell_detected_wrapper_missing_is_fail() {
    let mut r = DoctorReport::new();
    let rc = PathBuf::from("/home/user/.zshrc");
    r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, false);
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("wrapper NOT found"));
}

#[test]
fn check_shell_detected_rc_missing_is_warn() {
    let mut r = DoctorReport::new();
    let rc = PathBuf::from("/home/user/.zshrc");
    r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), false, false);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("rc file does not exist yet"));
}

#[test]
fn check_shell_detected_no_rc_path_is_fail() {
    let mut r = DoctorReport::new();
    r.check_shell_integration(Some(shell_init::Shell::Bash), None, false, false);
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("could not determine rc file path"));
}

// ── check_config ──────────────────────────────────────────────────────

#[test]
fn check_config_no_state_path_is_fail() {
    let mut r = DoctorReport::new();
    r.check_config(None, false, None);
    assert_eq!(r.failures, 1);
    assert!(r.output().contains("could not determine state file path"));
}

#[test]
fn check_config_fresh_install_is_warn() {
    let mut r = DoctorReport::new();
    let sp = PathBuf::from("/home/user/.config/tfe/state");
    r.check_config(Some(&sp), false, None);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("state file does not exist yet"));
}

#[test]
fn check_config_existing_state_passes() {
    let mut r = DoctorReport::new();
    let sp = PathBuf::from("/home/user/.config/tfe/state");
    let state = tui_file_explorer::AppState {
        theme: Some("nord".into()),
        cd_on_exit: Some(true),
        ..Default::default()
    };
    r.check_config(Some(&sp), true, Some(&state));
    assert_eq!(r.oks, 2); // state file + exists
    assert_eq!(r.failures, 0);
    let out = r.output();
    assert!(out.contains("theme         = nord"));
    assert!(out.contains("cd_on_exit    = true"));
}

#[test]
fn check_config_default_state_shows_defaults() {
    let mut r = DoctorReport::new();
    let sp = PathBuf::from("/tmp/state");
    let state = tui_file_explorer::AppState::default();
    r.check_config(Some(&sp), true, Some(&state));
    let out = r.output();
    assert!(out.contains("theme         = (default)"));
    assert!(out.contains("cd_on_exit    = (default -> true)"));
    assert!(out.contains("editor        = (default -> none)"));
}

// ── check_editor ──────────────────────────────────────────────────────

#[test]
fn check_editor_configured_and_on_path() {
    let mut r = DoctorReport::new();
    r.check_editor("helix", Some("hx"), true);
    assert_eq!(r.oks, 2);
    assert_eq!(r.warnings, 0);
    let out = r.output();
    assert!(out.contains("helix"));
    assert!(out.contains("hx"));
}

#[test]
fn check_editor_configured_but_not_on_path() {
    let mut r = DoctorReport::new();
    r.check_editor("helix", Some("hx"), false);
    assert_eq!(r.oks, 1);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("not found on $PATH"));
}

#[test]
fn check_editor_not_configured() {
    let mut r = DoctorReport::new();
    r.check_editor("none", None, false);
    assert_eq!(r.oks, 0);
    assert_eq!(r.warnings, 1);
    assert!(r.output().contains("no editor configured"));
}

// ── summary ───────────────────────────────────────────────────────────

#[test]
fn summary_all_pass() {
    let mut r = DoctorReport::new();
    r.pass("a");
    r.pass("b");
    r.summary();
    let out = r.output();
    assert!(out.contains("2 passed, 0 warning(s), 0 failure(s)"));
    assert!(out.contains("Everything looks good!"));
}

#[test]
fn summary_with_warnings() {
    let mut r = DoctorReport::new();
    r.pass("a");
    r.warn("b", "hint");
    r.summary();
    let out = r.output();
    assert!(out.contains("1 passed, 1 warning(s), 0 failure(s)"));
    assert!(out.contains("No failures!"));
}

#[test]
fn summary_with_failures() {
    let mut r = DoctorReport::new();
    r.pass("a");
    r.fail("b", "hint");
    r.summary();
    let out = r.output();
    assert!(out.contains("1 passed, 0 warning(s), 1 failure(s)"));
    assert!(out.contains("Fix the items marked [FAIL]"));
}

// ── Integration: full report ──────────────────────────────────────────

#[test]
fn full_report_healthy_system() {
    let mut r = DoctorReport::new();
    let exe = PathBuf::from("/usr/local/bin/tfe");
    let cargo = PathBuf::from("/home/user/.cargo/bin");
    let dirs = vec![cargo.clone()];
    let cwd = PathBuf::from("/home/user");
    let rc = PathBuf::from("/home/user/.zshrc");
    let sp = PathBuf::from("/home/user/.config/tfe/state");
    let state = tui_file_explorer::AppState {
        theme: Some("dracula".into()),
        cd_on_exit: Some(true),
        ..Default::default()
    };

    r.check_platform("macos", "aarch64", "unix");
    r.check_binary(Some(&exe), Some(&cargo), &dirs);
    r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
    r.check_terminal(Some((120, 40)), true);
    r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), true, true);
    r.check_config(Some(&sp), true, Some(&state));
    r.check_editor("helix", Some("hx"), true);
    r.summary();

    assert_eq!(r.failures, 0);
    assert_eq!(r.warnings, 0);
    assert!(r.oks >= 10);
    assert!(r.output().contains("Everything looks good!"));
}

#[test]
fn full_report_fresh_install() {
    let mut r = DoctorReport::new();
    let exe = PathBuf::from("/home/user/.cargo/bin/tfe");
    let cargo = PathBuf::from("/home/user/.cargo/bin");
    let dirs = vec![cargo.clone()];
    let cwd = PathBuf::from("/home/user");
    let rc = PathBuf::from("/home/user/.zshrc");
    let sp = PathBuf::from("/home/user/.config/tfe/state");

    r.check_platform("macos", "aarch64", "unix");
    r.check_binary(Some(&exe), Some(&cargo), &dirs);
    r.check_environment(Some("/home/user"), Some("/bin/zsh"), Some(&cwd));
    r.check_terminal(Some((80, 24)), true);
    r.check_shell_integration(Some(shell_init::Shell::Zsh), Some(&rc), false, false);
    r.check_config(Some(&sp), false, None);
    r.check_editor("none", None, false);
    r.summary();

    assert_eq!(r.failures, 0);
    assert!(r.warnings >= 3); // rc missing, state missing, no editor
    assert!(r.output().contains("No failures!"));
}

#[test]
fn full_report_broken_install() {
    let mut r = DoctorReport::new();

    r.check_platform("linux", "x86_64", "unix");
    r.check_binary(None, None, &[]);
    r.check_environment(None, None, None);
    r.check_terminal(None, false);
    r.check_shell_integration(None, None, false, false);
    r.check_config(None, false, None);
    r.check_editor("none", None, false);
    r.summary();

    assert!(r.failures >= 4); // HOME, cwd, terminal size, state path
    assert!(r.output().contains("Fix the items marked [FAIL]"));
}
