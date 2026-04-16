use pm::discovery::discover_projects;
use pm::store::Store;
use std::fs;
use tempfile::TempDir;

#[test]
fn discover_only_git_repos() {
    let root = TempDir::new().unwrap();
    let repo = root.path().join("repo1");
    let other = root.path().join("plain");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&other).unwrap();

    let store = Store::open_in_memory().unwrap();
    discover_projects(&store, root.path()).unwrap();

    let projects = store.list_active_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "repo1");
}
