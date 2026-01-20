use assert_cmd::cargo;
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_cli_help_short_flag() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("-h").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_view_help() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("view").arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_print_help() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("print").arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_view_invalid_date_format() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd
        .arg("view")
        .arg("--month")
        .arg("2025/11")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!(stderr);
}

#[test]
fn test_print_invalid_date_format() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd
        .arg("print")
        .arg("--month")
        .arg("2025/11")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!(stderr);
}

#[test]
fn test_missing_subcommand() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!(stderr);
}

#[test]
fn test_zsh_completion() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("completions").arg("zsh").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    insta::assert_snapshot!(stdout);
}

#[test]
fn test_bash_completion() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("completions").arg("bash").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    insta::assert_snapshot!(stdout);
}

#[test]
fn test_fish_completion() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("completions").arg("fish").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    insta::assert_snapshot!(stdout);
}

#[test]
fn test_powershell_completion() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("completions").arg("powershell").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    insta::assert_snapshot!(stdout);
}

#[test]
fn test_completions_help() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd.arg("completions").arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_completions_invalid_shell() {
    let mut cmd = Command::new(cargo::cargo_bin!("gh-log"));
    let output = cmd
        .arg("completions")
        .arg("invalid-shell")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!(stderr);
}
