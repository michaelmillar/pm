use crate::domain::Project;
use crate::similarity;
use std::path::Path;

pub struct DistinctnessResult {
    pub score: u8,
    pub nearest_name: Option<String>,
    pub nearest_similarity: f32,
}

pub fn compute(target: &Project, all_active: &[Project]) -> DistinctnessResult {
    let target_profile = profile_for(target);
    let mut max_sim = 0.0f32;
    let mut nearest: Option<String> = None;

    for other in all_active {
        if other.id == target.id || other.duplicate_of.is_some() {
            continue;
        }
        let other_profile = profile_for(other);
        let sim = similarity::weighted_similarity(
            &target_profile.0, &other_profile.0,
            &target_profile.1, &other_profile.1,
            &target_profile.2, &other_profile.2,
            &target_profile.3, &other_profile.3,
        );
        if sim > max_sim {
            max_sim = sim;
            nearest = Some(other.name.clone());
        }
    }

    let raw = ((1.0 - max_sim) * 10.0) as u8;
    let score = raw.min(10);

    DistinctnessResult {
        score,
        nearest_name: if max_sim > 0.3 { nearest } else { None },
        nearest_similarity: max_sim,
    }
}

fn profile_for(p: &Project) -> (String, String, String, String) {
    let name = p.name.clone();
    let (title, snippet) = p.path.as_ref()
        .map(|path| read_readme(Path::new(path)))
        .unwrap_or_default();
    let desc = p.path.as_ref()
        .map(|path| read_description(Path::new(path)))
        .unwrap_or_default();
    (name, title, snippet, desc)
}

fn read_readme(path: &Path) -> (String, String) {
    let readme_path = path.join("README.md");
    let content = match std::fs::read_to_string(&readme_path) {
        Ok(c) => c,
        Err(_) => return (String::new(), String::new()),
    };
    let mut lines: Vec<&str> = content.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return (String::new(), String::new());
    }
    let title = if lines[0].starts_with('#') {
        lines[0].trim_start_matches('#').trim().to_string()
    } else {
        lines[0].to_string()
    };
    lines.remove(0);
    let snippet = lines.into_iter().take(5).collect::<Vec<_>>().join(" ");
    (title, snippet)
}

fn read_description(path: &Path) -> String {
    let cargo = path.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo) {
        let mut in_package = false;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_package = line == "[package]";
                continue;
            }
            if in_package && line.starts_with("description") {
                if let Some((_, value)) = line.split_once('=') {
                    return value.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectState, ProjectType};
    use chrono::NaiveDate;

    fn make(id: i64, name: &str) -> Project {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        Project {
            id,
            name: name.to_string(),
            state: ProjectState::Active,
            project_type: ProjectType::Oss,
            stage: 2,
            velocity: None,
            fit_signal: None,
            distinctness: None,
            leverage: None,
            sunk_cost_days: None,
            pivot_count: 0,
            last_activity: today,
            created_at: today,
            soft_deadline: None,
            path: None,
            deleted_at: None,
            duplicate_of: None,
            possible_duplicate_score: None,
        }
    }

    #[test]
    fn identical_names_score_low() {
        let target = make(1, "budget tracker");
        let others = vec![make(2, "budget tracker")];
        let result = compute(&target, &others);
        assert!(result.score <= 2, "identical names should be low distinctness, got {}", result.score);
        assert!(result.nearest_name.is_some());
    }

    #[test]
    fn completely_different_scores_high() {
        let target = make(1, "quantum flux capacitor");
        let others = vec![make(2, "banana smoothie recipe")];
        let result = compute(&target, &others);
        assert!(result.score >= 8, "different names should score high, got {}", result.score);
    }

    #[test]
    fn solo_project_scores_max() {
        let target = make(1, "only project");
        let result = compute(&target, &[]);
        assert_eq!(result.score, 10);
        assert!(result.nearest_name.is_none());
    }
}
