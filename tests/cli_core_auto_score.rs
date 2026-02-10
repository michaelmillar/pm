use chrono::NaiveDate;
use pm::cli_core::auto_score;
use pm::domain::ScanResult;

#[test]
fn auto_score_bounds_and_defaults() {
    let scan = ScanResult {
        total_tasks: 0,
        completed_tasks: 0,
        last_commit_date: None,
        plan_files: Vec::new(),
        has_progress_file: false,
        charter_filled: None,
    };

    let today = NaiveDate::from_ymd_opt(2026, 2, 10).unwrap();
    let created_at = NaiveDate::from_ymd_opt(2025, 2, 10).unwrap();

    let score = auto_score(&scan, created_at, today);
    assert!((1..=10).contains(&score.impact));
    assert!((1..=10).contains(&score.monetization));
    assert!((0..=100).contains(&score.readiness));
}
