use pm::cli::cmd_status;
use pm::store::Store;
use std::fs;
use tempfile::TempDir;

#[test]
fn status_triggers_discovery() {
    let root = TempDir::new_in("/home/markw/projects").unwrap();
    fs::create_dir_all(root.path().join(".git")).unwrap();
    let repo_name = root.path().file_name().unwrap().to_string_lossy();

    let store = Store::open_in_memory().unwrap();
    cmd_status(&store);

    let inbox = store.list_inbox_projects().unwrap();
    let found = inbox.iter().any(|p| p.name == repo_name);
    assert!(found);
}
