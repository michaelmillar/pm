use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use pm::store::Store;
use tempfile::TempDir;

#[test]
fn inbox_shows_possible_duplicates() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("pm.db");
    let store = Store::open(&db_path).unwrap();

    let _a = store.add_project("Accent Game").unwrap();
    let b = store.add_project("accent_game").unwrap();
    store.mark_possible_duplicate(b, 0.85).unwrap();

    let mut cmd = cargo_bin_cmd!("pm");
    cmd.env("PM_DATA_DIR", dir.path());
    cmd.args(["inbox"]).assert().success().stdout(contains("Possible duplicates"));
}
