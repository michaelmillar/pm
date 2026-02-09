use assert_cmd::Command;

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
