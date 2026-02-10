use pm::naming::suggest_names;

#[test]
fn naming_suggests_three() {
    let names = suggest_names("Accent Game", "# Accent\nGame about accents", "Task: Improve accent game");
    assert_eq!(names.len(), 3);
}
