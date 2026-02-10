use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::TempDir;
use std::fs;

#[test]
fn inbox_shows_untracked_folders() {
    let root = TempDir::new().unwrap();
    fs::create_dir_all(root.path().join("notes")).unwrap();
    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_ROOT", root.path());
    cmd.args(["inbox"]).assert().success().stdout(contains("Untracked folders"));
}
