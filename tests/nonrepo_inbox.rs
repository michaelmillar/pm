use pm::discovery::list_nonrepo_folders;
use std::fs;
use tempfile::TempDir;

#[test]
fn lists_nonrepo_folders() {
    let root = TempDir::new().unwrap();
    fs::create_dir_all(root.path().join("repo/.git")).unwrap();
    fs::create_dir_all(root.path().join("notes")).unwrap();

    let list = list_nonrepo_folders(root.path());
    assert_eq!(list, vec!["notes".to_string()]);
}
