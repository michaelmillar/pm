use pm::standards::StandardsConfig;

#[test]
fn load_standards_config() {
    let yaml = r#"
requirements:
  - name: README
    check: readme
nice_to_haves:
  - name: CI
    check: ci
languages:
  rust:
    requirements:
      - name: Cargo
        check: cargo_toml
"#;

    let cfg = StandardsConfig::from_str(yaml).unwrap();
    assert_eq!(cfg.requirements.len(), 1);
    assert_eq!(cfg.languages.get("rust").unwrap().requirements.len(), 1);
}
