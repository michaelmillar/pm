use pm::standards::{evaluate_repo, StandardsConfig};
use tempfile::TempDir;

#[test]
fn autofix_creates_missing_readme() {
    let yaml = r#"
requirements:
  - name: README
    check: readme
"#;
    let cfg = StandardsConfig::from_str(yaml).unwrap();
    let dir = TempDir::new().unwrap();

    let report = evaluate_repo(dir.path(), &cfg).unwrap();
    assert!(report.fixes.contains(&"README.md".to_string()));
    assert!(dir.path().join("README.md").exists());
}
