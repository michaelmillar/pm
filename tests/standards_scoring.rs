use pm::standards::{evaluate_repo, StandardsConfig};
use std::fs;
use tempfile::TempDir;

#[test]
fn requirements_and_nice_to_haves_score() {
    let yaml = r#"
requirements:
  - name: README
    check: readme
nice_to_haves:
  - name: CI
    check: ci
"#;
    let cfg = StandardsConfig::from_str(yaml).unwrap();
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("README.md"), "# Test").unwrap();

    let report = evaluate_repo(dir.path(), &cfg).unwrap();
    assert_eq!(report.requirements_met, 1);
    assert_eq!(report.nice_to_haves_met, 0);
    assert_eq!(report.readiness_boost, 2);
}
