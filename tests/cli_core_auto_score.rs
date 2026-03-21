use pm::cli_core::auto_readiness;
use pm::domain::ScanResult;

#[test]
fn auto_readiness_bounds_and_defaults() {
    let scan = ScanResult {
        total_tasks: 0,
        completed_tasks: 0,
        last_commit_date: None,
        plan_files: Vec::new(),
        has_progress_file: false,
        charter_filled: None,
    };

    let readiness = auto_readiness(&scan);
    assert!((0..=100).contains(&readiness));
}
