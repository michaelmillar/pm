use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Roadmap {
    pub project: String,
    pub assessment: Option<Assessment>,
    pub components: Option<Vec<Component>>,
    pub phases: Vec<Phase>,
}

#[derive(Debug, Deserialize)]
pub struct Assessment {
    pub impact: u8,
    pub monetization: u8,
    pub cloneability: Option<u8>,
    pub uniqueness: Option<u8>,
    pub defensibility: Option<u8>,
    pub researched_at: String,
    pub reasoning: Option<String>,
    pub signals: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Component {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct Phase {
    pub id: String,
    pub label: String,
    pub weight: f64,
    pub component: Option<String>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: String,
    pub label: String,
    pub done: bool,
}

pub struct RoadmapScores {
    pub readiness: u8,
    pub impact: Option<u8>,
    pub monetization: Option<u8>,
    pub cloneability: Option<u8>,
    pub uniqueness: Option<u8>,
    pub defensibility: Option<u8>,
    pub assessment_stale: bool,
}

pub fn load_roadmap(project_path: &std::path::Path) -> Option<Roadmap> {
    let yaml_path = project_path.join("docs").join("roadmap.yaml");
    let content = std::fs::read_to_string(yaml_path).ok()?;
    serde_yaml::from_str(&content).ok()
}

pub fn compute_readiness(roadmap: &Roadmap) -> u8 {
    let mut total = 0.0f64;
    for phase in &roadmap.phases {
        if phase.tasks.is_empty() {
            continue;
        }
        let done = phase.tasks.iter().filter(|t| t.done).count() as f64;
        let all = phase.tasks.len() as f64;
        total += phase.weight * (done / all);
    }
    (total * 100.0).round().min(100.0) as u8
}

pub fn extract_scores(roadmap: &Roadmap) -> RoadmapScores {
    let readiness = compute_readiness(roadmap);
    match &roadmap.assessment {
        None => RoadmapScores {
            readiness,
            impact: None,
            monetization: None,
            cloneability: None,
            uniqueness: None,
            defensibility: None,
            assessment_stale: false,
        },
        Some(a) => {
            let stale = is_assessment_stale(&a.researched_at);
            RoadmapScores {
                readiness,
                impact: Some(a.impact),
                monetization: Some(a.monetization),
                cloneability: a.cloneability,
                uniqueness: a.uniqueness,
                defensibility: a.defensibility,
                assessment_stale: stale,
            }
        }
    }
}

pub fn load_scores(project_path: &std::path::Path) -> Option<RoadmapScores> {
    let roadmap = load_roadmap(project_path)?;
    Some(extract_scores(&roadmap))
}

pub fn validate_weights(roadmap: &Roadmap) -> Option<f64> {
    let sum: f64 = roadmap.phases.iter().map(|p| p.weight).sum();
    if (sum - 1.0).abs() > 0.001 {
        Some(sum)
    } else {
        None
    }
}

pub fn is_assessment_stale(researched_at: &str) -> bool {
    let Ok(date) = chrono::NaiveDate::parse_from_str(researched_at, "%Y-%m-%d") else {
        return false;
    };
    let today = chrono::Local::now().date_naive();
    (today - date).num_days() > 90
}

/// Update the four axis scores and `researched_at` in the project's `docs/roadmap.yaml` in-place.
/// Preserves all other content (phases, reasoning, signals) unchanged.
/// Inserts missing fields (`uniqueness`, `cloneability`, `researched_at`) if absent.
pub fn patch_assessment_scores(
    project_path: &std::path::Path,
    impact: u8,
    monetization: u8,
    cloneability: Option<u8>,
    uniqueness: Option<u8>,
    defensibility: Option<u8>,
) -> Result<(), String> {
    let yaml_path = project_path.join("docs").join("roadmap.yaml");
    let content = std::fs::read_to_string(&yaml_path)
        .map_err(|e| format!("Could not read roadmap.yaml: {}", e))?;

    let today = chrono::Local::now().date_naive().to_string();
    let mut out: Vec<String> = Vec::new();
    let mut in_assessment = false;
    let mut found_uniqueness = false;
    let mut found_cloneability = false;
    let mut found_defensibility = false;
    let mut found_researched_at = false;
    let mut last_score_idx: usize = 0;

    for line in content.lines() {
        if line.starts_with("assessment:") {
            in_assessment = true;
            out.push(line.to_string());
            continue;
        }
        // Leave the assessment block when we see a non-blank, non-indented line
        if in_assessment && !line.starts_with("  ") && !line.trim().is_empty() {
            in_assessment = false;
        }
        if in_assessment {
            if line.starts_with("  impact:") {
                out.push(format!("  impact: {}", impact));
                last_score_idx = out.len() - 1;
                continue;
            }
            if line.starts_with("  monetization:") {
                out.push(format!("  monetization: {}", monetization));
                last_score_idx = out.len() - 1;
                continue;
            }
            if line.starts_with("  cloneability:") {
                found_cloneability = true;
                if let Some(c) = cloneability {
                    out.push(format!("  cloneability: {}", c));
                } else {
                    out.push(line.to_string());
                }
                last_score_idx = out.len() - 1;
                continue;
            }
            if line.starts_with("  uniqueness:") {
                found_uniqueness = true;
                if let Some(u) = uniqueness {
                    out.push(format!("  uniqueness: {}", u));
                } else {
                    out.push(line.to_string());
                }
                last_score_idx = out.len() - 1;
                continue;
            }
            if line.starts_with("  defensibility:") {
                found_defensibility = true;
                if let Some(d) = defensibility {
                    out.push(format!("  defensibility: {}", d));
                } else {
                    out.push(line.to_string());
                }
                last_score_idx = out.len() - 1;
                continue;
            }
            if line.starts_with("  researched_at:") {
                // Insert any missing score fields before researched_at
                if !found_uniqueness {
                    if let Some(u) = uniqueness {
                        out.push(format!("  uniqueness: {}", u));
                        last_score_idx = out.len() - 1;
                        found_uniqueness = true;
                    }
                }
                if !found_cloneability {
                    if let Some(c) = cloneability {
                        out.push(format!("  cloneability: {}", c));
                        last_score_idx = out.len() - 1;
                        found_cloneability = true;
                    }
                }
                if !found_defensibility {
                    if let Some(d) = defensibility {
                        out.push(format!("  defensibility: {}", d));
                        last_score_idx = out.len() - 1;
                        found_defensibility = true;
                    }
                }
                out.push(format!("  researched_at: \"{}\"", today));
                found_researched_at = true;
                continue;
            }
        }
        out.push(line.to_string());
    }

    // If researched_at was absent, insert all missing fields after the last score line
    if !found_researched_at {
        if !found_uniqueness {
            if let Some(u) = uniqueness {
                out.insert(last_score_idx + 1, format!("  uniqueness: {}", u));
                last_score_idx += 1;
            }
        }
        if !found_cloneability {
            if let Some(c) = cloneability {
                out.insert(last_score_idx + 1, format!("  cloneability: {}", c));
                last_score_idx += 1;
            }
        }
        if !found_defensibility {
            if let Some(d) = defensibility {
                out.insert(last_score_idx + 1, format!("  defensibility: {}", d));
                last_score_idx += 1;
            }
        }
        out.insert(last_score_idx + 1, format!("  researched_at: \"{}\"", today));
    }

    let mut new_content = out.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }

    std::fs::write(&yaml_path, new_content)
        .map_err(|e| format!("Could not write roadmap.yaml: {}", e))?;

    Ok(())
}

pub fn scaffold_template(project_name: &str) -> String {
    format!(
        r#"project: {name}
assessment:
  impact: 7
  monetization: 7
  uniqueness: 7
  cloneability: 6
  researched_at: "{today}"
  reasoning: |
    Impact N: Describe the audience and pain point solved.
    Monetization N: Describe how it makes money.
    Uniqueness N: Describe how this differs from existing solutions.
    Cloneability N: Describe how hard it is to replicate the value.
  signals:
    - "Evidence of market demand or competitor pricing"

phases:
  - id: core
    label: Core MVP
    weight: 0.60
    tasks:
      - id: task-1
        label: First task description
        done: false

  - id: hardening
    label: Hardening
    weight: 0.25
    tasks:
      - id: coverage
        label: Test coverage and edge cases
        done: false

  - id: polish
    label: Polish & Release
    weight: 0.15
    tasks:
      - id: docs
        label: Public documentation
        done: false
"#,
        name = project_name,
        today = chrono::Local::now().date_naive(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roadmap_from_str(yaml: &str) -> Roadmap {
        serde_yaml::from_str(yaml).expect("valid yaml")
    }

    #[test]
    fn compute_readiness_empty_phases_is_zero() {
        let r = roadmap_from_str("project: x\nphases: []");
        assert_eq!(compute_readiness(&r), 0);
    }

    #[test]
    fn compute_readiness_all_done_single_phase() {
        let yaml = "
project: x
phases:
  - id: core
    label: Core
    weight: 1.0
    tasks:
      - id: t1
        label: Task 1
        done: true
      - id: t2
        label: Task 2
        done: true
";
        let r = roadmap_from_str(yaml);
        assert_eq!(compute_readiness(&r), 100);
    }

    #[test]
    fn compute_readiness_weighted_sum() {
        let yaml = "
project: x
phases:
  - id: a
    label: A
    weight: 0.60
    tasks:
      - {id: a1, label: A1, done: true}
      - {id: a2, label: A2, done: true}
  - id: b
    label: B
    weight: 0.40
    tasks:
      - {id: b1, label: B1, done: false}
      - {id: b2, label: B2, done: false}
";
        let r = roadmap_from_str(yaml);
        assert_eq!(compute_readiness(&r), 60);
    }

    #[test]
    fn compute_readiness_empty_phase_contributes_zero() {
        let yaml = "
project: x
phases:
  - id: a
    label: A
    weight: 0.60
    tasks: []
  - id: b
    label: B
    weight: 0.40
    tasks:
      - {id: b1, label: B1, done: true}
";
        let r = roadmap_from_str(yaml);
        assert_eq!(compute_readiness(&r), 40);
    }

    #[test]
    fn load_scores_extracts_assessment_block() {
        let yaml = "
project: x
assessment:
  impact: 8
  monetization: 7
  cloneability: 6
  researched_at: \"2026-02-01\"
  reasoning: |
    Test
  signals:
    - \"signal 1\"
phases: []
";
        let r = roadmap_from_str(yaml);
        let scores = extract_scores(&r);
        assert_eq!(scores.impact, Some(8));
        assert_eq!(scores.monetization, Some(7));
        assert_eq!(scores.cloneability, Some(6));
        assert!(!scores.assessment_stale);
    }

    #[test]
    fn validate_weights_warns_on_bad_sum() {
        let yaml = "
project: x
phases:
  - {id: a, label: A, weight: 0.6, tasks: []}
  - {id: b, label: B, weight: 0.6, tasks: []}
";
        let r = roadmap_from_str(yaml);
        assert!(validate_weights(&r).is_some());
    }

    #[test]
    fn validate_weights_passes_on_correct_sum() {
        let yaml = "
project: x
phases:
  - {id: a, label: A, weight: 0.6, tasks: []}
  - {id: b, label: B, weight: 0.4, tasks: []}
";
        let r = roadmap_from_str(yaml);
        assert!(validate_weights(&r).is_none());
    }

    #[test]
    fn patch_assessment_scores_updates_all_fields() {
        let tmp = tempfile::TempDir::new().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let yaml = "project: test\nassessment:\n  impact: 7\n  monetization: 7\n  cloneability: 6\n  researched_at: \"2026-02-18\"\n  reasoning: |\n    Old reasoning.\n  signals:\n    - \"old signal\"\nphases: []\n";
        std::fs::write(docs.join("roadmap.yaml"), yaml).unwrap();

        patch_assessment_scores(tmp.path(), 6, 4, Some(3), Some(8), None).unwrap();

        let result = std::fs::read_to_string(docs.join("roadmap.yaml")).unwrap();
        assert!(result.contains("  impact: 6"), "impact not updated");
        assert!(result.contains("  monetization: 4"), "monetization not updated");
        assert!(result.contains("  cloneability: 3"), "cloneability not updated");
        assert!(result.contains("  uniqueness: 8"), "uniqueness not inserted");
        let today = chrono::Local::now().date_naive().to_string();
        assert!(result.contains(&format!("  researched_at: \"{}\"", today)), "researched_at not updated");
        assert!(result.contains("Old reasoning."), "reasoning lost");
        assert!(result.contains("old signal"), "signals lost");
        // Must still parse as valid YAML
        serde_yaml::from_str::<Roadmap>(&result).expect("result should be valid YAML");
    }

    #[test]
    fn patch_assessment_scores_inserts_uniqueness_before_researched_at() {
        let tmp = tempfile::TempDir::new().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let yaml = "project: test\nassessment:\n  impact: 7\n  monetization: 7\n  cloneability: 6\n  researched_at: \"2026-02-18\"\nphases: []\n";
        std::fs::write(docs.join("roadmap.yaml"), yaml).unwrap();

        patch_assessment_scores(tmp.path(), 7, 7, Some(6), Some(8), None).unwrap();

        let result = std::fs::read_to_string(docs.join("roadmap.yaml")).unwrap();
        let u_pos = result.find("  uniqueness: 8").expect("uniqueness missing");
        let r_pos = result.find("  researched_at:").expect("researched_at missing");
        assert!(u_pos < r_pos, "uniqueness should appear before researched_at");
    }

    #[test]
    fn test_extract_scores_reads_uniqueness() {
        let yaml = r#"
project: test
assessment:
  impact: 8
  monetization: 7
  uniqueness: 6
  cloneability: 5
  researched_at: "2026-02-20"
phases: []
"#;
        let roadmap: Roadmap = serde_yaml::from_str(yaml).unwrap();
        let scores = extract_scores(&roadmap);
        assert_eq!(scores.uniqueness, Some(6));
    }
}
