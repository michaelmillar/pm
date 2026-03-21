use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

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
    cmd.assert().success();
}

#[test]
fn test_status_hides_naming_suggestions_and_shows_table() {
    let tmp = tempfile::TempDir::new().unwrap();
    let repo = tempfile::TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("docs/plans")).unwrap();
    fs::write(repo.path().join("README.md"), "alpha platform").unwrap();
    fs::write(repo.path().join("docs/plans/plan.md"), "### Task 1: Build alpha\n").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("add").arg("Alpha Project");
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.args(["link", "1", &repo.path().to_string_lossy()]);
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.args(["activate", "1"]);
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ID  S  T  Project"))
        .stdout(predicate::str::contains("Next"))
        .stdout(predicate::str::contains("Naming suggestions").not())
        .stdout(predicate::str::contains("Def"))
        .stdout(predicate::str::contains("impact:").not())
        .stdout(predicate::str::contains("(run 'pm research").not());
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
    cmd.args(["activate", "1"]);
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    cmd.env("PM_DATA_DIR", tmp.path());
    cmd.args(["status", "--sort", "name"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ID  S  T  Project"))
        .stdout(predicate::str::contains("Next"));
}
