use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn unscore_moves_project_to_inbox() {
    let dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["add", "Test Project"]).assert().success();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["score", "1", "-i", "5", "-m", "5", "-r", "10"]).assert().success();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["unscore", "1"]).assert().success()
        .stdout(contains("moved to inbox"));
}
