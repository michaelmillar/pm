use pm::store::Store;

#[test]
fn mark_project_duplicate() {
    let store = Store::open_in_memory().unwrap();
    let id1 = store.add_project("A").unwrap();
    let id2 = store.add_project("B").unwrap();

    store.mark_duplicate(id2, id1).unwrap();
    let dup = store.get_project(id2).unwrap().unwrap();

    assert_eq!(dup.duplicate_of, Some(id1));
}

#[test]
fn list_inbox_excludes_duplicates() {
    let store = Store::open_in_memory().unwrap();
    let id1 = store.add_project("Original").unwrap();
    let id2 = store.add_project("Dup").unwrap();

    store.mark_duplicate(id2, id1).unwrap();

    let inbox = store.list_inbox_projects().unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].id, id1);
}
