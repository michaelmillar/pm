use pm::domain::ProjectState;
use pm::store::Store;

#[test]
fn move_project_to_inbox() {
    let store = Store::open_in_memory().unwrap();
    let id = store.add_project("Test").unwrap();
    store.update_state(id, ProjectState::Active).unwrap();

    let updated = store.move_to_inbox(id).unwrap();
    assert_eq!(updated, 1);

    let project = store.get_project(id).unwrap().unwrap();
    assert_eq!(project.state, ProjectState::Inbox);
}
