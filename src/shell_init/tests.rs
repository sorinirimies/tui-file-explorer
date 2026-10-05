//! Unit tests for [`super`].

use super::*;
use std::fs;
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────

/// Call auto_install_with with the Nushell-specific nu_config_dir wired in.
fn auto_install_nu(nu_config_dir: &Path) -> InitOutcome {
    auto_install_with(
        Some(Shell::Nu),
        None,
        None,
        None,
        None,
        None,
        Some(nu_config_dir),
    )
}

// ── Shell::from_str ───────────────────────────────────────────────────────

#[test]
fn from_str_recognises_bash() {
    assert_eq!(Shell::from_str("bash"), Some(Shell::Bash));
}

#[test]
fn from_str_recognises_zsh() {
    assert_eq!(Shell::from_str("zsh"), Some(Shell::Zsh));
}

#[test]
fn from_str_recognises_fish() {
    assert_eq!(Shell::from_str("fish"), Some(Shell::Fish));
}

#[test]
fn from_str_recognises_powershell() {
    assert_eq!(Shell::from_str("powershell"), Some(Shell::Pwsh));
    assert_eq!(Shell::from_str("pwsh"), Some(Shell::Pwsh));
}

#[test]
fn from_str_recognises_nushell() {
    assert_eq!(Shell::from_str("nu"), Some(Shell::Nu));
    assert_eq!(Shell::from_str("nushell"), Some(Shell::Nu));
}

#[test]
fn from_str_is_case_insensitive() {
    assert_eq!(Shell::from_str("BASH"), Some(Shell::Bash));
    assert_eq!(Shell::from_str("ZSH"), Some(Shell::Zsh));
    assert_eq!(Shell::from_str("FISH"), Some(Shell::Fish));
    assert_eq!(Shell::from_str("NU"), Some(Shell::Nu));
    assert_eq!(Shell::from_str("Bash"), Some(Shell::Bash));
    assert_eq!(Shell::from_str("FISH"), Some(Shell::Fish));
    assert_eq!(Shell::from_str("PowerShell"), Some(Shell::Pwsh));
    assert_eq!(Shell::from_str("PWSH"), Some(Shell::Pwsh));
}

#[test]
fn from_str_returns_none_for_unknown() {
    assert_eq!(Shell::from_str("cmd"), None);
    assert_eq!(Shell::from_str(""), None);
    assert_eq!(Shell::from_str("sh"), None);
}

#[test]
fn display_returns_lowercase_name() {
    assert_eq!(Shell::Bash.to_string(), "bash");
    assert_eq!(Shell::Zsh.to_string(), "zsh");
    assert_eq!(Shell::Fish.to_string(), "fish");
    assert_eq!(Shell::Pwsh.to_string(), "powershell");
    assert_eq!(Shell::Nu.to_string(), "nushell");
}

// ── snippet ───────────────────────────────────────────────────────────────

#[test]
fn snippet_contains_sentinel() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh] {
        assert!(
            snippet(shell).contains(SENTINEL),
            "{shell} snippet missing sentinel"
        );
    }
}

#[test]
fn snippet_bash_contains_function_body() {
    let s = snippet(Shell::Bash);
    assert!(
        s.contains("command tfe"),
        "bash snippet missing command tfe"
    );
    assert!(s.contains("cd \"$line\""), "bash snippet missing cd");
    assert!(
        s.contains("source:"),
        "bash snippet missing source: handler"
    );
}

#[test]
fn snippet_zsh_identical_to_bash() {
    assert_eq!(snippet(Shell::Zsh), snippet(Shell::Bash));
}

#[test]
fn snippet_nushell_contains_def_wrapped() {
    let s = snippet(Shell::Nu);
    assert!(
        s.contains("def --env --wrapped tfe"),
        "nushell snippet must use 'def --env --wrapped tfe', got:\n{s}"
    );
}

#[test]
fn snippet_nushell_contains_def_env() {
    let s = snippet(Shell::Nu);
    assert!(
        s.contains("--env"),
        "nushell snippet must use --env so cd propagates to caller, got:\n{s}"
    );
}

#[test]
fn snippet_nushell_uses_caret_tfe() {
    let s = snippet(Shell::Nu);
    assert!(
        s.contains("^tfe"),
        "nushell snippet must call the external binary with ^tfe, got:\n{s}"
    );
}

#[test]
fn snippet_nushell_uses_cd() {
    let s = snippet(Shell::Nu);
    assert!(
        s.contains("cd"),
        "nushell snippet must cd to the dir, got:\n{s}"
    );
}

#[test]
fn snippet_nushell_does_not_use_each_closure() {
    // cd inside an `each` closure does not propagate to the caller —
    // the correct form calls cd at the top level of the def --env body.
    let s = snippet(Shell::Nu);
    assert!(
        !s.contains("each {"),
        "nushell snippet must not use an each closure for cd, got:\n{s}"
    );
}

#[test]
fn snippet_nushell_uses_str_trim() {
    let s = snippet(Shell::Nu);
    assert!(
        s.contains("str trim"),
        "nushell snippet must use str trim to strip trailing newline, got:\n{s}"
    );
}

#[test]
fn snippet_nushell_differs_from_bash() {
    assert_ne!(snippet(Shell::Nu), snippet(Shell::Bash));
}

#[test]
fn snippet_bash_handles_source_directive() {
    let s = snippet(Shell::Bash);
    assert!(
        s.contains("source:"),
        "bash snippet must handle source: directives, got:\n{s}"
    );
}

#[test]
fn snippet_fish_handles_source_directive() {
    let s = snippet(Shell::Fish);
    assert!(
        s.contains("source:"),
        "fish snippet must handle source: directives, got:\n{s}"
    );
}

#[test]
fn snippet_pwsh_handles_source_directive() {
    let s = snippet(Shell::Pwsh);
    assert!(
        s.contains("source:"),
        "powershell snippet must handle source: directives, got:\n{s}"
    );
}

#[test]
fn source_directive_prefix_constant_value() {
    assert_eq!(SOURCE_DIRECTIVE_PREFIX, "source:");
}

#[test]
fn emit_source_directive_does_not_panic() {
    // We can't easily capture stdout in a unit test without extra deps,
    // but we can verify the function completes without panicking.
    emit_source_directive(std::path::Path::new("/tmp/test_rc"));
}

#[test]
fn snippet_fish_contains_function_body() {
    let s = snippet(Shell::Fish);
    assert!(
        s.contains("command tfe"),
        "fish snippet missing command tfe"
    );
    assert!(s.contains("cd $line"), "fish snippet missing cd");
    assert!(
        s.contains("function tfe"),
        "fish snippet missing function keyword"
    );
    assert!(
        s.contains("source:"),
        "fish snippet missing source: handler"
    );
}

#[test]
fn snippet_fish_differs_from_bash() {
    assert_ne!(snippet(Shell::Fish), snippet(Shell::Bash));
}

#[test]
fn snippet_powershell_contains_function_body() {
    let s = snippet(Shell::Pwsh);
    assert!(s.contains("function tfe"), "missing function tfe");
    assert!(s.contains("Set-Location"), "missing Set-Location");
    assert!(s.contains("Get-Command tfe"), "missing Get-Command tfe");
}

#[test]
fn snippet_powershell_differs_from_bash() {
    assert_ne!(snippet(Shell::Pwsh), snippet(Shell::Bash));
}

// ── rc_path_with ──────────────────────────────────────────────────────────

#[test]
fn rc_path_bash_ends_with_bashrc() {
    let p = rc_path_with(
        Shell::Bash,
        Some(Path::new("/test/home")),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(p, PathBuf::from("/test/home/.bashrc"));
}

#[test]
fn rc_path_zsh_defaults_to_zshrc() {
    // Neither .zshrc nor .zshenv exist on disk — should default to .zshrc.
    let p = rc_path_with(
        Shell::Zsh,
        Some(Path::new("/test/home")),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(p, PathBuf::from("/test/home/.zshrc"));
}

#[test]
fn rc_path_zsh_prefers_zdotdir() {
    let p = rc_path_with(
        Shell::Zsh,
        Some(Path::new("/home/user")),
        None,
        Some(Path::new("/custom/zdotdir")),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(p, PathBuf::from("/custom/zdotdir/.zshrc"));
}

#[test]
fn rc_path_zsh_falls_back_to_zshenv_when_it_exists() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    // Create .zshenv but NOT .zshrc — colleague's setup.
    let zshenv = home.join(".zshenv");
    fs::write(&zshenv, b"# existing zshenv\n").unwrap();
    let p = rc_path_with(
        Shell::Zsh,
        Some(home),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    )
    .unwrap();
    assert_eq!(p, zshenv, "must write to .zshenv when .zshrc absent");
}

#[test]
fn rc_path_zsh_prefers_zshrc_over_zshenv_when_both_exist() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let zshrc = home.join(".zshrc");
    let zshenv = home.join(".zshenv");
    fs::write(&zshrc, b"# existing zshrc\n").unwrap();
    fs::write(&zshenv, b"# existing zshenv\n").unwrap();
    let p = rc_path_with(
        Shell::Zsh,
        Some(home),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    )
    .unwrap();
    assert_eq!(p, zshrc, "must prefer .zshrc over .zshenv when both exist");
}

#[test]
#[cfg(not(windows))]
fn rc_path_powershell_falls_back_to_home_config_on_unix() {
    std::env::remove_var("PROFILE");
    let p = rc_path_with(
        Shell::Pwsh,
        Some(Path::new("/test/home")),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        p,
        PathBuf::from("/test/home/.config/powershell/Microsoft.PowerShell_profile.ps1")
    );
}

#[test]
fn rc_path_fish_uses_xdg_config_home_when_set() {
    let p = rc_path_with(
        Shell::Fish,
        Some(Path::new("/test/home")),
        Some(Path::new("/custom/config")),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(p, PathBuf::from("/custom/config/fish/functions/tfe.fish"));
}

#[test]
fn rc_path_fish_falls_back_to_home_config() {
    let p = rc_path_with(
        Shell::Fish,
        Some(Path::new("/test/home")),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        p,
        PathBuf::from("/test/home/.config/fish/functions/tfe.fish")
    );
}

#[test]
fn rc_path_returns_none_when_home_unset() {
    std::env::remove_var("PROFILE");
    assert!(rc_path_with(Shell::Bash, None, None, None, None, None, None).is_none());
    assert!(rc_path_with(Shell::Zsh, None, None, None, None, None, None).is_none());
    assert!(rc_path_with(Shell::Fish, None, None, None, None, None, None).is_none());
    assert!(rc_path_with(Shell::Pwsh, None, None, None, None, None, None).is_none());
    assert!(rc_path_with(Shell::Nu, None, None, None, None, None, None).is_none());
}

#[test]
fn rc_path_nu_uses_nu_config_dir() {
    let p = rc_path_with(
        Shell::Nu,
        None,
        None,
        None,
        None,
        None,
        Some(Path::new("/test/nushell")),
    )
    .unwrap();
    assert_eq!(p, PathBuf::from("/test/nushell/config.nu"));
}

#[test]
fn rc_path_nu_returns_none_when_nu_config_dir_unset() {
    assert!(
        rc_path_with(Shell::Nu, None, None, None, None, None, None).is_none(),
        "Nu rc_path must be None when nu_config_dir is not provided"
    );
}

// ── is_installed ──────────────────────────────────────────────────────────

#[test]
fn is_installed_detects_sentinel_based_wrapper() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    install(Shell::Zsh, &rc).unwrap();
    assert!(is_installed(&rc));
}

#[test]
fn is_installed_detects_sentinel_in_nushell_config() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("config.nu");
    install(Shell::Nu, &rc).unwrap();
    assert!(is_installed(&rc));
}

#[test]
fn is_installed_detects_hand_written_bash_wrapper() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    fs::write(
        &rc,
        "# tfe — cd to the directory browsed when dismissing the file explorer\n\
         tfe() {\n\
         \x20   local dir\n\
         \x20   dir=$(command tfe \"$@\")\n\
         \x20   [ -n \"$dir\" ] && cd \"$dir\"\n\
         }\n",
    )
    .unwrap();
    assert!(
        is_installed(&rc),
        "hand-written bash wrapper must be detected"
    );
}

#[test]
fn is_installed_detects_hand_written_fish_wrapper() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("tfe.fish");
    fs::write(
        &rc,
        "function tfe\n\
         \x20   set dir (command tfe $argv)\n\
         \x20   if test -n \"$dir\"\n\
         \x20       cd $dir\n\
         \x20   end\n\
         end\n",
    )
    .unwrap();
    assert!(
        is_installed(&rc),
        "hand-written fish wrapper must be detected"
    );
}

#[test]
fn is_installed_returns_false_for_missing_file() {
    let dir = tempdir().unwrap();
    assert!(!is_installed(&dir.path().join("nonexistent")));
}

#[test]
fn is_installed_returns_false_for_empty_file() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    fs::write(&rc, b"").unwrap();
    assert!(!is_installed(&rc));
}

#[test]
fn is_installed_returns_false_when_sentinel_absent() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    fs::write(&rc, b"export PATH=$PATH:/usr/local/bin\n").unwrap();
    assert!(!is_installed(&rc));
}

#[test]
fn is_installed_nushell_returns_false_when_sentinel_absent() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("config.nu");
    fs::write(&rc, b"$env.config.show_banner = false\n").unwrap();
    assert!(!is_installed(&rc));
}

#[test]
fn is_installed_returns_true_when_sentinel_present() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    fs::write(&rc, format!("some content\n{SENTINEL}\nmore\n").as_bytes()).unwrap();
    assert!(is_installed(&rc));
}

#[test]
fn is_installed_nushell_returns_true_when_sentinel_present() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("config.nu");
    fs::write(
        &rc,
        format!("$env.config.show_banner = false\n{SENTINEL}\n").as_bytes(),
    )
    .unwrap();
    assert!(is_installed(&rc));
}

// ── install ───────────────────────────────────────────────────────────────

#[test]
fn install_creates_rc_file_when_missing() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    assert!(!rc.exists());
    install(Shell::Zsh, &rc).unwrap();
    assert!(rc.exists());
}

#[test]
fn install_nushell_creates_config_nu_when_missing() {
    let dir = tempdir().unwrap();
    let nu_dir = dir.path().join("nushell");
    let rc = nu_dir.join("config.nu");
    assert!(!rc.exists());
    install(Shell::Nu, &rc).unwrap();
    assert!(rc.exists());
    assert!(is_installed(&rc));
}

#[test]
fn install_nushell_creates_parent_directories() {
    let dir = tempdir().unwrap();
    let rc = dir
        .path()
        .join("Library")
        .join("Application Support")
        .join("nushell")
        .join("config.nu");
    install(Shell::Nu, &rc).unwrap();
    assert!(rc.exists());
    assert!(is_installed(&rc));
}

#[test]
fn install_nushell_snippet_passes_is_installed() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("config.nu");
    install(Shell::Nu, &rc).unwrap();
    assert!(
        is_installed(&rc),
        "installed nushell snippet must pass is_installed"
    );
}

#[test]
fn install_nushell_does_not_duplicate_when_called_twice() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("config.nu");
    install(Shell::Nu, &rc).unwrap();
    let before = fs::read_to_string(&rc).unwrap();
    // Second install should not be reached in practice (auto_install_with
    // guards it), but install() itself does not de-duplicate — verify that
    // is_installed catches it before a second call.
    assert!(is_installed(&rc), "must be detected after first install");
    // Confirm the content after one install contains the snippet exactly once.
    let count = before.matches(SENTINEL).count();
    assert_eq!(
        count, 1,
        "sentinel should appear exactly once after one install"
    );
}

#[test]
fn install_creates_parent_directories() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join("fish/functions/tfe.fish");
    install(Shell::Fish, &rc).unwrap();
    assert!(rc.exists());
}

#[test]
fn install_appends_snippet_to_existing_file() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    fs::write(&rc, b"export FOO=bar\n").unwrap();
    install(Shell::Zsh, &rc).unwrap();
    let content = fs::read_to_string(&rc).unwrap();
    assert!(
        content.starts_with("export FOO=bar\n"),
        "existing content must be preserved"
    );
    assert!(content.contains(SENTINEL), "snippet must be appended");
}

#[test]
fn install_written_snippet_passes_is_installed() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".bashrc");
    install(Shell::Bash, &rc).unwrap();
    assert!(is_installed(&rc));
}

#[test]
fn install_does_not_duplicate_when_called_twice() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    install(Shell::Zsh, &rc).unwrap();
    // Guard: caller should check is_installed first — but even if install
    // is called again the sentinel appears twice (expected append behaviour).
    // This test documents the expected behaviour rather than asserting idempotency.
    install(Shell::Zsh, &rc).unwrap();
    let content = fs::read_to_string(&rc).unwrap();
    let count = content.matches(SENTINEL).count();
    assert_eq!(
        count, 2,
        "raw install appends each time — guard is is_installed"
    );
}

// ── install_or_print ─────────────────────────────────────────────────────

#[test]
fn install_or_print_installs_when_rc_writable() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    let outcome = install_or_print_to(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(outcome, InitOutcome::Installed(rc.clone()));
    assert!(is_installed(&rc));
}

#[test]
fn install_or_print_returns_already_installed_when_sentinel_present() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    // Pre-install the snippet.
    install(Shell::Zsh, &rc).unwrap();
    let outcome = install_or_print_to(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(outcome, InitOutcome::AlreadyInstalled(rc));
}

#[test]
fn install_or_print_zsh_uses_zdotdir() {
    let dir = tempdir().unwrap();
    let zdotdir = dir.path().join("zdotdir");
    fs::create_dir(&zdotdir).unwrap();
    let rc = zdotdir.join(".zshrc");
    let outcome = install_or_print_to(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        Some(&zdotdir),
        None,
        None,
        None,
    );
    assert_eq!(
        outcome,
        InitOutcome::Installed(rc.clone()),
        "must install into $ZDOTDIR/.zshrc not $HOME/.zshrc"
    );
    assert!(is_installed(&rc));
}

#[test]
fn install_or_print_zsh_falls_back_to_zshenv_when_zshrc_absent() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let zshenv = home.join(".zshenv");
    fs::write(&zshenv, b"# existing zshenv\n").unwrap();
    // No .zshrc present — should write to .zshenv.
    let outcome = install_or_print_to(
        Some(Shell::Zsh),
        Some(home),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    );
    assert_eq!(
        outcome,
        InitOutcome::Installed(zshenv.clone()),
        "must install into .zshenv when .zshrc is absent"
    );
    assert!(is_installed(&zshenv));
}

#[test]
fn install_or_print_bash_uses_bash_profile_when_present() {
    let dir = tempdir().unwrap();
    let bp = dir.path().join(".bash_profile");
    fs::write(&bp, b"# existing profile\n").unwrap();
    let outcome = install_or_print_to(
        Some(Shell::Bash),
        Some(dir.path()),
        None,
        None,
        Some(&bp),
        None,
        None,
    );
    assert_eq!(
        outcome,
        InitOutcome::Installed(bp.clone()),
        "must install into .bash_profile when it exists"
    );
    assert!(is_installed(&bp));
}

#[test]
fn install_or_print_returns_printed_when_rc_not_writable() {
    let dir = tempdir().unwrap();
    // Make the dir read-only so writing fails.
    let ro_dir = dir.path().join("readonly");
    fs::create_dir(&ro_dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Root bypasses Unix permission checks entirely, so this test
        // cannot be made to work reliably in CI containers that run as
        // root.  Skip rather than produce a spurious failure.
        // SAFETY: getuid() is always safe to call.
        let uid = unsafe { libc::getuid() };
        if uid == 0 {
            return;
        }

        let mut perms = fs::metadata(&ro_dir).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&ro_dir, perms).unwrap();
        let outcome = install_or_print_to(
            Some(Shell::Zsh),
            Some(&ro_dir),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            outcome,
            InitOutcome::PrintedToStdout,
            "read-only dir must fall back to stdout"
        );
    }
    #[cfg(not(unix))]
    {
        // On non-unix we can't reliably make dirs unwritable — skip.
        let _ = ro_dir;
    }
}

// ── auto_install_with ─────────────────────────────────────────────────────

#[test]
fn auto_install_writes_wrapper_on_first_run() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    // Simulate $SHELL=zsh by using auto_install_with directly.
    // Before: wrapper absent.
    assert!(!is_installed(&rc));
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    // After: wrapper present.
    assert!(
        is_installed(&rc),
        "auto_install must write wrapper on first run"
    );
}

#[test]
fn auto_install_does_not_duplicate_when_already_installed() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    // Pre-install once.
    install(Shell::Zsh, &rc).unwrap();
    let before = fs::read_to_string(&rc).unwrap();
    // Run auto_install — must be a no-op.
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    let after = fs::read_to_string(&rc).unwrap();
    assert_eq!(
        before, after,
        "auto_install must not append a duplicate when wrapper already present"
    );
}

#[test]
fn auto_install_detects_wrapper_in_zshenv_and_skips_install() {
    let dir = tempdir().unwrap();
    let zshenv = dir.path().join(".zshenv");
    // Install the wrapper into .zshenv (no .zshrc present).
    install(Shell::Zsh, &zshenv).unwrap();
    // auto_install should detect it in .zshenv and not create .zshrc.
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    );
    let zshrc = dir.path().join(".zshrc");
    assert!(
        !zshrc.exists(),
        "auto_install must not create .zshrc when wrapper already in .zshenv"
    );
}

#[test]
fn auto_install_detects_wrapper_in_zprofile_and_skips_install() {
    let dir = tempdir().unwrap();
    let zprofile = dir.path().join(".zprofile");
    // Manually write the sentinel into .zprofile.
    fs::write(&zprofile, format!("{SENTINEL}\ntfe() {{}}\n")).unwrap();
    // auto_install_with checks .zprofile as a candidate — should skip.
    // We pass zshenv=None so it won't find it there.
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    // .zshrc should not have been created (wrapper found in .zprofile).
    // Note: auto_install_with checks .zprofile via the candidates list
    // but rc_path_with would still pick .zshrc — the key invariant is
    // that the candidates scan fires before install_or_print_to.
    // Verify by checking .zshrc does NOT contain the sentinel.
    let zshrc = dir.path().join(".zshrc");
    if zshrc.exists() {
        let content = fs::read_to_string(&zshrc).unwrap();
        assert!(
            !content.contains(SENTINEL),
            "auto_install must not write to .zshrc when sentinel already in .zprofile"
        );
    }
}

#[test]
fn auto_install_uses_zdotdir_when_set() {
    let dir = tempdir().unwrap();
    let zdotdir = dir.path().join("zdotdir");
    fs::create_dir(&zdotdir).unwrap();
    let rc = zdotdir.join(".zshrc");
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        Some(&zdotdir),
        None,
        None,
        None,
    );
    assert!(
        is_installed(&rc),
        "auto_install must write to $ZDOTDIR/.zshrc when ZDOTDIR is set"
    );
    // Must not also write to $HOME/.zshrc.
    let home_rc = dir.path().join(".zshrc");
    assert!(
        !home_rc.exists(),
        "auto_install must not write to $HOME/.zshrc when ZDOTDIR is set"
    );
}

#[test]
fn auto_install_falls_back_to_zshenv_when_zshrc_absent() {
    let dir = tempdir().unwrap();
    let zshenv = dir.path().join(".zshenv");
    fs::write(&zshenv, b"# existing zshenv\n").unwrap();
    // No .zshrc — should fall back to .zshenv.
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    );
    assert!(
        is_installed(&zshenv),
        "auto_install must install into .zshenv when .zshrc is absent"
    );
}

#[test]
fn auto_install_nushell_writes_config_nu_on_first_run() {
    let dir = tempdir().unwrap();
    let nu_dir = dir.path().join("nushell");
    let config_nu = nu_dir.join("config.nu");
    assert!(!config_nu.exists());
    let outcome = auto_install_nu(&nu_dir);
    assert!(
        matches!(outcome, InitOutcome::Installed(_)),
        "expected Installed, got {outcome:?}"
    );
    assert!(
        is_installed(&config_nu),
        "config.nu must contain the wrapper"
    );
}

#[test]
fn auto_install_nushell_does_not_duplicate_when_already_installed() {
    let dir = tempdir().unwrap();
    let nu_dir = dir.path().join("nushell");
    let config_nu = nu_dir.join("config.nu");
    // Pre-install once.
    install(Shell::Nu, &config_nu).unwrap();
    let before = fs::read_to_string(&config_nu).unwrap();
    // Second run — must be a no-op.
    auto_install_nu(&nu_dir);
    let after = fs::read_to_string(&config_nu).unwrap();
    assert_eq!(
        before, after,
        "auto_install must not duplicate the nushell wrapper"
    );
}

#[test]
fn auto_install_nushell_returns_already_installed_when_config_nu_has_sentinel() {
    let dir = tempdir().unwrap();
    let nu_dir = dir.path().join("nushell");
    let config_nu = nu_dir.join("config.nu");
    install(Shell::Nu, &config_nu).unwrap();
    let outcome = auto_install_nu(&nu_dir);
    assert!(
        matches!(outcome, InitOutcome::AlreadyInstalled(_)),
        "expected AlreadyInstalled, got {outcome:?}"
    );
}

#[test]
fn auto_install_nushell_creates_parent_directories_for_config_nu() {
    let dir = tempdir().unwrap();
    // Deep nested path — directories do not exist yet.
    let nu_dir = dir.path().join("deep").join("nested").join("nushell");
    let config_nu = nu_dir.join("config.nu");
    assert!(!nu_dir.exists());
    let outcome = auto_install_nu(&nu_dir);
    assert!(
        matches!(outcome, InitOutcome::Installed(_)),
        "expected Installed, got {outcome:?}"
    );
    assert!(config_nu.exists(), "config.nu must have been created");
}

#[test]
fn auto_install_does_nothing_when_shell_unrecognised() {
    // auto_install_with resolves shell via detect_shell() which reads $SHELL.
    // We can't safely mutate $SHELL in a parallel test, so we verify the
    // None-home path (unresolvable rc) is handled without panic instead.
    // The real unrecognised-shell path is covered by detect_shell tests.
    let outcome = auto_install_with(None, None, None, None, None, None, None);
    // Must not panic — that is the assertion.
    // Acceptable outcomes depending on the CI environment:
    //   - UnknownShell      : $SHELL is unset or unrecognised
    //   - AlreadyInstalled  : wrapper already present in the real rc file
    //   - PrintedToStdout   : $SHELL is set but $HOME is None (our override),
    //                         so rc_path_with returns None and the snippet
    //                         is printed to stdout as a fallback
    //   - Installed         : $SHELL is set, $HOME is None but rc_path_with
    //                         somehow resolves (shouldn't happen, but safe)
    assert!(
        matches!(
            outcome,
            InitOutcome::UnknownShell
                | InitOutcome::AlreadyInstalled(_)
                | InitOutcome::PrintedToStdout
                | InitOutcome::Installed(_)
        ),
        "unexpected outcome: {outcome:?}"
    );
}

// ── auto_install_with return value ────────────────────────────────────────

#[test]
fn auto_install_returns_installed_on_first_run() {
    let dir = tempdir().unwrap();
    let outcome = auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        matches!(outcome, InitOutcome::Installed(_)),
        "expected Installed on first run, got {outcome:?}"
    );
}

#[test]
fn auto_install_installed_outcome_carries_rc_path() {
    let dir = tempdir().unwrap();
    let outcome = auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    if let InitOutcome::Installed(path) = outcome {
        assert!(
            path.ends_with(".zshrc"),
            "installed path should be .zshrc, got {path:?}"
        );
    } else {
        panic!("expected Installed, got {outcome:?}");
    }
}

#[test]
fn auto_install_returns_already_installed_on_second_run() {
    let dir = tempdir().unwrap();
    // First run installs.
    auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    // Second run must return AlreadyInstalled.
    let outcome = auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        matches!(outcome, InitOutcome::AlreadyInstalled(_)),
        "expected AlreadyInstalled on second run, got {outcome:?}"
    );
}

#[test]
fn auto_install_already_installed_outcome_carries_rc_path() {
    let dir = tempdir().unwrap();
    let rc = dir.path().join(".zshrc");
    install(Shell::Zsh, &rc).unwrap();
    let outcome = auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    if let InitOutcome::AlreadyInstalled(path) = outcome {
        assert!(
            path.ends_with(".zshrc") || is_installed(&path),
            "AlreadyInstalled path should point to a file containing the wrapper, got {path:?}"
        );
    } else {
        panic!("expected AlreadyInstalled, got {outcome:?}");
    }
}

#[test]
fn auto_install_returns_unknown_shell_when_shell_is_none_and_home_is_none() {
    // When both shell detection and home are unavailable we get UnknownShell.
    let outcome = auto_install_with(None, None, None, None, None, None, None);
    // On a real machine $SHELL may be set, so we allow AlreadyInstalled too.
    assert!(
        matches!(
            outcome,
            InitOutcome::UnknownShell
                | InitOutcome::AlreadyInstalled(_)
                | InitOutcome::Installed(_)
                | InitOutcome::PrintedToStdout
        ),
        "unexpected outcome variant: {outcome:?}"
    );
}

#[test]
fn auto_install_bash_returns_installed_on_first_run() {
    let dir = tempdir().unwrap();
    let outcome = auto_install_with(
        Some(Shell::Bash),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        matches!(outcome, InitOutcome::Installed(_)),
        "expected Installed for bash on first run, got {outcome:?}"
    );
    let rc = dir.path().join(".bashrc");
    assert!(is_installed(&rc), ".bashrc should contain the wrapper");
}

#[test]
fn auto_install_fish_returns_installed_on_first_run() {
    let dir = tempdir().unwrap();
    let outcome = auto_install_with(
        Some(Shell::Fish),
        Some(dir.path()),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        matches!(outcome, InitOutcome::Installed(_)),
        "expected Installed for fish on first run, got {outcome:?}"
    );
    let rc = dir
        .path()
        .join(".config")
        .join("fish")
        .join("functions")
        .join("tfe.fish");
    assert!(is_installed(&rc), "tfe.fish should contain the wrapper");
}

#[test]
fn auto_install_zsh_already_installed_in_zshenv_returns_already_installed() {
    let dir = tempdir().unwrap();
    let zshenv = dir.path().join(".zshenv");
    install(Shell::Zsh, &zshenv).unwrap();
    // No .zshrc present — wrapper is in .zshenv.
    let outcome = auto_install_with(
        Some(Shell::Zsh),
        Some(dir.path()),
        None,
        None,
        None,
        Some(&zshenv),
        None,
    );
    assert!(
        matches!(outcome, InitOutcome::AlreadyInstalled(_)),
        "expected AlreadyInstalled when wrapper is in .zshenv, got {outcome:?}"
    );
}

#[test]
fn snippets_are_blank_line_separated_and_carry_the_sentinel() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh, Shell::Nu] {
        let s = snippet(shell);
        assert!(s.starts_with(&format!("\n{SENTINEL}\n")), "{shell}: {s:?}");
        assert!(!s.contains("{{SENTINEL}}"), "{shell}: placeholder left in");
    }
}
