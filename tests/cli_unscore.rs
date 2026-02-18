use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn unscore_moves_project_to_inbox() {
    let dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["add", "Test Project"]).assert().success();

    // Activate via roadmap instead of pm score
    std::fs::create_dir_all(project_dir.path().join("docs")).unwrap();
    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["link", "1", project_dir.path().to_str().unwrap()]).assert().success();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["roadmap", "1"]).assert().success();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["unscore", "1"]).assert().success()
        .stdout(contains("moved to inbox"));
}
