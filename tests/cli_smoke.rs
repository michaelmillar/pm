use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_pm_add_and_status() {
    let tmp = tempfile::TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("add").arg("Test Project");
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("status");
    cmd.assert().success()
        .stdout(predicate::str::contains("Test Project"));
}

#[test]
fn test_status_shows_new_columns() {
    let tmp = tempfile::TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("add").arg("Alpha Project");
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("#"))
        .stdout(predicate::str::contains("Action"))
        .stdout(predicate::str::contains("Stage"))
        .stdout(predicate::str::contains("Score"))
        .stdout(predicate::str::contains("Alpha Project"));
}

#[test]
fn test_status_accepts_sort_flag() {
    let tmp = tempfile::TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("add").arg("Alpha Project");
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.args(["status", "--sort", "name"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alpha Project"));
}
