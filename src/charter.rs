use std::path::Path;

#[derive(Debug, Clone)]
pub struct CharterStatus {
    pub exists: bool,
    pub filled: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharterAction {
    Created,
    AlreadyExists(usize, usize),
    Overwritten,
}

pub const CHARTER_TEMPLATE: &str = r#"# {PROJECT_NAME} — Project Charter

## Vision & USP

_TODO: One sentence describing what this project does and why it's different from alternatives._

## Goals

_TODO: 3-5 concrete, measurable goals for the project._

## Definition of Done

_TODO: What "shipped" looks like — the minimum bar for calling this complete._

## Business Justification

_TODO: Why this project deserves time over everything else on the list._

## Monetization Path

_TODO: How this turns into revenue (direct sales, funnel, portfolio piece, etc.)._

## Quantified Impact

_TODO: Numbers — expected users, revenue, time saved, or other measurable outcomes._

## Target Audience

_TODO: Who specifically will use this? Be narrow enough to be useful._

## Time Estimates

_TODO: Rough effort estimates for each milestone._

- **v0.1 (MVP):**
- **v1.0 (Launchable):**
- **v2.0 (Growth):**

## Rubric

_TODO: Self-check — score yourself honestly before starting._

- [ ] I can explain the USP in one sentence
- [ ] At least one goal is measurable within 30 days
- [ ] Definition of done is concrete enough to test
- [ ] Business justification doesn't rely on "it would be cool"
- [ ] Monetization path has at least one realistic step
- [ ] Target audience is a real group I can reach
- [ ] Time estimates account for integration and testing
"#;

const TOTAL_SECTIONS: usize = 9;

pub fn generate_charter(path: &Path, name: &str, force: bool) -> Result<CharterAction, String> {
    let charter_path = path.join("docs").join("CHARTER.md");

    if charter_path.exists() && !force {
        let status = check_charter(path);
        return Ok(CharterAction::AlreadyExists(status.filled, status.total));
    }

    let docs_dir = path.join("docs");
    if !docs_dir.exists() {
        std::fs::create_dir_all(&docs_dir)
            .map_err(|e| format!("Failed to create docs/: {}", e))?;
    }

    let content = CHARTER_TEMPLATE.replace("{PROJECT_NAME}", name);
    std::fs::write(&charter_path, content)
        .map_err(|e| format!("Failed to write CHARTER.md: {}", e))?;

    if force && charter_path.exists() {
        Ok(CharterAction::Overwritten)
    } else {
        Ok(CharterAction::Created)
    }
}

pub fn check_charter(path: &Path) -> CharterStatus {
    let charter_path = path.join("docs").join("CHARTER.md");

    if !charter_path.exists() {
        return CharterStatus {
            exists: false,
            filled: 0,
            total: TOTAL_SECTIONS,
        };
    }

    let content = match std::fs::read_to_string(&charter_path) {
        Ok(c) => c,
        Err(_) => {
            return CharterStatus {
                exists: true,
                filled: 0,
                total: TOTAL_SECTIONS,
            }
        }
    };

    let mut filled = 0;
    let mut in_section = false;
    let mut section_has_todo = false;

    for line in content.lines() {
        if line.starts_with("## ") {
            if in_section && !section_has_todo {
                filled += 1;
            }
            in_section = true;
            section_has_todo = false;
        } else if in_section && line.contains("_TODO:") {
            section_has_todo = true;
        }
    }

    // Count the last section
    if in_section && !section_has_todo {
        filled += 1;
    }

    CharterStatus {
        exists: true,
        filled,
        total: TOTAL_SECTIONS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_charter_missing() {
        let tmp = TempDir::new().unwrap();
        let status = check_charter(tmp.path());
        assert!(!status.exists);
        assert_eq!(status.filled, 0);
        assert_eq!(status.total, TOTAL_SECTIONS);
    }

    #[test]
    fn test_generate_charter_creates_file() {
        let tmp = TempDir::new().unwrap();
        let result = generate_charter(tmp.path(), "TestProject", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CharterAction::Created);
        assert!(tmp.path().join("docs").join("CHARTER.md").exists());
    }

    #[test]
    fn test_generate_charter_already_exists() {
        let tmp = TempDir::new().unwrap();
        generate_charter(tmp.path(), "TestProject", false).unwrap();
        let result = generate_charter(tmp.path(), "TestProject", false).unwrap();
        match result {
            CharterAction::AlreadyExists(filled, total) => {
                assert_eq!(filled, 0);
                assert_eq!(total, TOTAL_SECTIONS);
            }
            _ => panic!("Expected AlreadyExists"),
        }
    }

    #[test]
    fn test_generate_charter_force_overwrites() {
        let tmp = TempDir::new().unwrap();
        generate_charter(tmp.path(), "TestProject", false).unwrap();
        let result = generate_charter(tmp.path(), "TestProject", true).unwrap();
        assert_eq!(result, CharterAction::Overwritten);
    }

    #[test]
    fn test_check_charter_unfilled() {
        let tmp = TempDir::new().unwrap();
        generate_charter(tmp.path(), "Test", false).unwrap();
        let status = check_charter(tmp.path());
        assert!(status.exists);
        assert_eq!(status.filled, 0);
        assert_eq!(status.total, TOTAL_SECTIONS);
    }

    #[test]
    fn test_check_charter_partially_filled() {
        let tmp = TempDir::new().unwrap();
        let docs = tmp.path().join("docs");
        fs::create_dir_all(&docs).unwrap();

        let content = r#"# Test — Project Charter

## Vision & USP

This project does X and is better because Y.

## Goals

_TODO: 3-5 concrete, measurable goals for the project._

## Definition of Done

The MVP ships when users can sign up and create a widget.

## Business Justification

_TODO: Why this project deserves time over everything else on the list._

## Monetization Path

_TODO: How this turns into revenue._

## Quantified Impact

_TODO: Numbers._

## Target Audience

_TODO: Who specifically will use this?_

## Time Estimates

_TODO: Rough effort estimates._

## Rubric

_TODO: Self-check._
"#;
        fs::write(docs.join("CHARTER.md"), content).unwrap();

        let status = check_charter(tmp.path());
        assert!(status.exists);
        assert_eq!(status.filled, 2); // Vision & USP + Definition of Done
        assert_eq!(status.total, TOTAL_SECTIONS);
    }

    #[test]
    fn test_check_charter_all_filled() {
        let tmp = TempDir::new().unwrap();
        let docs = tmp.path().join("docs");
        fs::create_dir_all(&docs).unwrap();

        let content = r#"# Test — Project Charter

## Vision & USP

A real vision here.

## Goals

1. Goal one
2. Goal two

## Definition of Done

Ship it.

## Business Justification

Makes money.

## Monetization Path

Direct sales.

## Quantified Impact

1000 users in 90 days.

## Target Audience

Developers who need X.

## Time Estimates

- v0.1: 2 weeks
- v1.0: 6 weeks

## Rubric

- [x] All checked
"#;
        fs::write(docs.join("CHARTER.md"), content).unwrap();

        let status = check_charter(tmp.path());
        assert!(status.exists);
        assert_eq!(status.filled, TOTAL_SECTIONS);
        assert_eq!(status.total, TOTAL_SECTIONS);
    }

    #[test]
    fn test_template_has_project_name_placeholder() {
        assert!(CHARTER_TEMPLATE.contains("{PROJECT_NAME}"));
    }

    #[test]
    fn test_generate_charter_substitutes_name() {
        let tmp = TempDir::new().unwrap();
        generate_charter(tmp.path(), "MyProject", false).unwrap();
        let content = fs::read_to_string(tmp.path().join("docs").join("CHARTER.md")).unwrap();
        assert!(content.contains("MyProject"));
        assert!(!content.contains("{PROJECT_NAME}"));
    }
}
