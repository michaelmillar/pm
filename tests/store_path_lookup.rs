use pm::store::Store;

#[test]
fn store_gets_or_creates_by_path() {
    let store = Store::open_in_memory().unwrap();
    let path = "/tmp/example";

    let id1 = store.get_or_create_by_path("Example", path).unwrap();
    let id2 = store.get_or_create_by_path("Example", path).unwrap();

    assert_eq!(id1, id2);
}
