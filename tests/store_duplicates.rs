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
