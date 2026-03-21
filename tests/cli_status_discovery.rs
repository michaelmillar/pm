use pm::cli::cmd_status;
use pm::store::Store;
use std::fs;
use tempfile::TempDir;

#[test]
fn status_triggers_discovery() {
    let parent = TempDir::new().unwrap();
    let repo_dir = parent.path().join("my_project");
    fs::create_dir_all(repo_dir.join(".git")).unwrap();

    let store = Store::open_in_memory().unwrap();
    unsafe { std::env::set_var("PM_ROOT", parent.path()); }
    cmd_status(&store);
    unsafe { std::env::remove_var("PM_ROOT"); }

    let inbox = store.list_inbox_projects().unwrap();
    let found = inbox.iter().any(|p| p.name == "my_project");
    assert!(found);
}
