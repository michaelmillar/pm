use pm::similarity::weighted_similarity;

#[test]
fn similarity_high_for_close_names() {
    let score = weighted_similarity("Budget Tracker", "budget_tracker", "", "", "", "", "", "");
    assert!(score >= 0.9);
}
