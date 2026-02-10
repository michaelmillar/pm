use pm::standards::{write_report, RepoStandardsReport};
use tempfile::TempDir;

#[test]
fn writes_report_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("report.json");

    let report = RepoStandardsReport {
        name: "repo".to_string(),
        path: "/tmp/repo".to_string(),
        requirements_met: 1,
        nice_to_haves_met: 0,
        readiness_boost: 2,
        fixes: vec!["README.md".to_string()],
        missing: vec![],
    };

    write_report(&path, &[report]).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("repo"));
}
