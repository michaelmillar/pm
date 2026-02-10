use pm::similarity::weighted_similarity;

#[test]
fn similarity_high_for_close_names() {
    let score = weighted_similarity("Accent Game", "accent_game", "", "", "", "", "", "");
    assert!(score >= 0.9);
}
