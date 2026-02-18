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
            assessment_stale: false,
        },
        Some(a) => {
            let stale = is_assessment_stale(&a.researched_at);
            RoadmapScores {
                readiness,
                impact: Some(a.impact),
                monetization: Some(a.monetization),
                cloneability: a.cloneability,
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

pub fn scaffold_template(project_name: &str) -> String {
    format!(
        r#"project: {name}
assessment:
  impact: 7
  monetization: 7
  cloneability: 6
  researched_at: "{today}"
  reasoning: |
    Impact N: Describe the audience and pain point solved.
    Monetization N: Describe how it makes money.
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
}
