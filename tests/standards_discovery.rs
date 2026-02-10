use pm::cli_core::auto_score;
use pm::discovery::discover_projects;
use pm::scanner::scan_project;
use pm::store::Store;
use std::fs;
use tempfile::TempDir;

#[test]
fn discovery_applies_standards_boost() {
    let root = TempDir::new().unwrap();
    let repo = root.path().join("repo1");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(repo.join("README.md"), "# Repo").unwrap();

    let cfg = root.path().join("standards.yml");
    fs::write(
        &cfg,
        "requirements:\n  - name: README\n    check: readme\n",
    )
    .unwrap();
    unsafe {
        std::env::set_var("PM_STANDARDS_CONFIG", &cfg);
    }

    let scan = scan_project(repo.to_str().unwrap());
    let today = chrono::Local::now().date_naive();
    let base = auto_score(&scan, today, today).readiness as i32;

    let store = Store::open_in_memory().unwrap();
    discover_projects(&store, root.path()).unwrap();

    let proj = store.get_project_by_path(repo.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(proj.readiness as i32, base + 2);
}
