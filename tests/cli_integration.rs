use std::process::Command;

fn tfe_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tfe"))
}

#[test]
fn info_flag_exits_zero() {
    let output = tfe_bin().arg("--info").output().unwrap();
    assert!(output.status.success());
    // --info writes to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tfe "));
    assert!(stderr.contains("Platform"));
    assert!(stderr.contains("Environment"));
    assert!(stderr.contains("Binary"));
    assert!(stderr.contains("Terminal"));
    assert!(stderr.contains("Shell detection"));
    assert!(stderr.contains("Config"));
}

#[test]
fn doctor_flag_exits_zero() {
    let output = tfe_bin().arg("--doctor").output().unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tfe doctor"));
    assert!(stderr.contains("Platform"));
    assert!(stderr.contains("Summary"));
}

#[test]
fn version_flag_exits_zero() {
    let output = tfe_bin().arg("--version").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn info_stdout_is_empty() {
    // --info should not write to stdout (to avoid shell wrapper issues)
    let output = tfe_bin().arg("--info").output().unwrap();
    assert!(
        output.stdout.is_empty(),
        "--info should not write to stdout"
    );
}

#[test]
fn doctor_stdout_is_empty() {
    let output = tfe_bin().arg("--doctor").output().unwrap();
    assert!(
        output.stdout.is_empty(),
        "--doctor should not write to stdout"
    );
}
