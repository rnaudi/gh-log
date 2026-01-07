use assert_cmd::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("gh-log").unwrap();
    let output = cmd.arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_invalid_date_format() {
    let mut cmd = Command::cargo_bin("gh-log").unwrap();
    let output = cmd.arg("--month").arg("2025/11").output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!(stderr);
}
