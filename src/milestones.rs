use std::path::Path;

#[derive(Debug, Clone)]
pub struct Milestone {
    pub name: String,
    pub target: Option<String>,
    pub items: Vec<MilestoneItem>,
}

#[derive(Debug, Clone)]
pub struct MilestoneItem {
    pub done: bool,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct MilestoneFile {
    pub milestones: Vec<Milestone>,
}

impl MilestoneFile {
    /// Compute readiness as percentage of all checkbox items done across all milestones.
    pub fn readiness(&self) -> u8 {
        let total: usize = self.milestones.iter().map(|m| m.items.len()).sum();
        if total == 0 {
            return 0;
        }
        let done: usize = self
            .milestones
            .iter()
            .flat_map(|m| &m.items)
            .filter(|i| i.done)
            .count();
        ((done as f64 / total as f64) * 100.0).round().min(100.0) as u8
    }

    /// Return a short summary for the nearest milestone with incomplete items.
    /// Format: "Show HN 60%" or None if all done.
    pub fn target_summary(&self) -> Option<String> {
        for m in &self.milestones {
            let total = m.items.len();
            if total == 0 {
                continue;
            }
            let done = m.items.iter().filter(|i| i.done).count();
            if done < total {
                let pct = ((done as f64 / total as f64) * 100.0).round() as u8;
                return Some(format!("{} {}%", m.name, pct));
            }
        }
        None
    }
}

/// Load and parse MILESTONES.md from a project directory.
pub fn load_milestones(project_path: &Path) -> Option<MilestoneFile> {
    let md_path = project_path.join("MILESTONES.md");
    let content = std::fs::read_to_string(md_path).ok()?;
    Some(parse_milestones(&content))
}

/// Parse MILESTONES.md content into structured data.
pub fn parse_milestones(content: &str) -> MilestoneFile {
    let mut milestones = Vec::new();
    let mut current: Option<Milestone> = None;

    for line in content.lines() {
        if line.starts_with("## ") {
            if let Some(m) = current.take() {
                milestones.push(m);
            }
            current = Some(Milestone {
                name: line[3..].trim().to_string(),
                target: None,
                items: Vec::new(),
            });
        } else if let Some(ref mut m) = current {
            let trimmed = line.trim();
            if let Some(date) = trimmed.strip_prefix("target:") {
                m.target = Some(date.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("- [x]").or_else(|| trimmed.strip_prefix("- [X]")) {
                m.items.push(MilestoneItem {
                    done: true,
                    label: rest.trim().to_string(),
                });
            } else if let Some(rest) = trimmed.strip_prefix("- [ ]") {
                m.items.push(MilestoneItem {
                    done: false,
                    label: rest.trim().to_string(),
                });
            }
        }
    }

    if let Some(m) = current {
        milestones.push(m);
    }

    MilestoneFile { milestones }
}

/// Generate a scaffold MILESTONES.md template for a project.
pub fn scaffold_template(project_name: &str) -> String {
    format!(
        r#"# {name} Milestones

## Show HN
target: 2026-06-01

- [ ] Core binary works end to end
- [ ] README with build/test/install
- [ ] Landing page with demo GIF
- [ ] Free tier available without sign-up
- [ ] Show HN post drafted

## v1.0 GA
target: 2026-12-01

- [ ] All Show HN feedback addressed
- [ ] Documentation site live
- [ ] Billing integration
"#,
        name = project_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let mf = parse_milestones("");
        assert!(mf.milestones.is_empty());
        assert_eq!(mf.readiness(), 0);
    }

    #[test]
    fn parse_single_milestone_all_done() {
        let content = "# My Project\n\n## Alpha\ntarget: 2026-04-01\n\n- [x] Task A\n- [x] Task B\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.milestones.len(), 1);
        assert_eq!(mf.milestones[0].name, "Alpha");
        assert_eq!(mf.milestones[0].target, Some("2026-04-01".to_string()));
        assert_eq!(mf.milestones[0].items.len(), 2);
        assert_eq!(mf.readiness(), 100);
    }

    #[test]
    fn parse_mixed_checkboxes() {
        let content = "## Show HN\n- [x] Done one\n- [ ] Not done\n- [x] Done two\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.readiness(), 67); // 2/3 = 66.67 rounds to 67
    }

    #[test]
    fn parse_multiple_milestones() {
        let content = "## Alpha\n- [x] A1\n- [x] A2\n\n## Beta\n- [ ] B1\n- [ ] B2\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.milestones.len(), 2);
        assert_eq!(mf.readiness(), 50); // 2/4
    }

    #[test]
    fn target_summary_shows_first_incomplete() {
        let content = "## Alpha\n- [x] A1\n- [x] A2\n\n## Beta\n- [x] B1\n- [ ] B2\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.target_summary(), Some("Beta 50%".to_string()));
    }

    #[test]
    fn target_summary_none_when_all_done() {
        let content = "## Alpha\n- [x] A1\n\n## Beta\n- [x] B1\n";
        let mf = parse_milestones(content);
        assert!(mf.target_summary().is_none());
    }

    #[test]
    fn readiness_zero_for_no_items() {
        let content = "## Empty milestone\ntarget: 2026-01-01\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.readiness(), 0);
    }

    #[test]
    fn parse_uppercase_x() {
        let content = "## M1\n- [X] Done with capital X\n- [ ] Not done\n";
        let mf = parse_milestones(content);
        assert_eq!(mf.milestones[0].items[0].done, true);
        assert_eq!(mf.milestones[0].items[1].done, false);
    }

    #[test]
    fn scaffold_produces_parseable_content() {
        let template = scaffold_template("testproject");
        let mf = parse_milestones(&template);
        assert_eq!(mf.milestones.len(), 2);
        assert_eq!(mf.milestones[0].name, "Show HN");
        assert_eq!(mf.milestones[1].name, "v1.0 GA");
        assert_eq!(mf.readiness(), 0); // all items unchecked
    }

    #[test]
    fn load_milestones_from_disk() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = "## M1\n- [x] Done\n- [ ] Todo\n";
        std::fs::write(tmp.path().join("MILESTONES.md"), content).unwrap();
        let mf = load_milestones(tmp.path()).unwrap();
        assert_eq!(mf.readiness(), 50);
    }

    #[test]
    fn load_milestones_returns_none_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_milestones(tmp.path()).is_none());
    }
}
