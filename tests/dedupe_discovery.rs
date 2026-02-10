use pm::discovery::discover_projects;
use pm::store::Store;
use std::fs;
use tempfile::TempDir;

#[test]
fn discovery_marks_duplicate() {
    let root = TempDir::new().unwrap();
    let repo = root.path().join("accent_game");
    fs::create_dir_all(repo.join(".git")).unwrap();

    let store = Store::open_in_memory().unwrap();
    let existing = store.add_project("Accent Game").unwrap();

    discover_projects(&store, root.path()).unwrap();

    let dup = store.get_project_by_path(repo.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(dup.duplicate_of, Some(existing));
}
