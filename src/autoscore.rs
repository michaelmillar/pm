use crate::adapters;
use crate::domain::{Project, ProjectType, RepoSignals};
use crate::scanner;
use crate::scoring::{distinctness, leverage, velocity};
use crate::store::Store;
use std::path::Path;

pub fn score_all(store: &Store, projects: &[Project], fetch_remote: bool) {
    for project in projects {
        if let Err(e) = score_one(store, project, projects, fetch_remote) {
            eprintln!("scoring {} failed: {}", project.name, e);
        }
    }
}

fn score_one(store: &Store, project: &Project, all: &[Project], fetch_remote: bool) -> Result<(), String> {
    let path = match &project.path {
        Some(p) if Path::new(p).exists() => p.clone(),
        _ => return Ok(()),
    };
    let repo = Path::new(&path);
    let signals = scanner::scan_signals(repo);

    if let Some(vel) = velocity::compute(repo) {
        store.update_axis(project.id, "velocity", Some(vel.score))
            .map_err(|e| e.to_string())?;
        store.update_sunk_cost(project.id, vel.sunk_cost_days)
            .map_err(|e| e.to_string())?;
    }

    let dist = distinctness::compute(project, all);
    store.update_axis(project.id, "distinctness", Some(dist.score))
        .map_err(|e| e.to_string())?;

    let lev = leverage::compute(repo);
    store.update_axis(project.id, "leverage", Some(lev))
        .map_err(|e| e.to_string())?;

    if fetch_remote {
        if let Some(fit) = adapters::fetch_fit_signal(project) {
            store.update_axis(project.id, "fit_signal", Some(fit.raw_score))
                .map_err(|e| e.to_string())?;
        }
    }

    let detected_type = detect_project_type(&signals, repo);
    if detected_type != ProjectType::Oss && project.project_type == ProjectType::Oss {
        store.update_project_type(project.id, &detected_type)
            .map_err(|e| e.to_string())?;
    }

    let suggested = suggest_stage(&signals);
    if suggested > project.stage {
        store.update_stage(project.id, suggested)
            .map_err(|e| e.to_string())?;
        store.record_stage_event(
            project.id,
            project.stage,
            suggested,
            Some("auto-detected from repo signals"),
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn detect_project_type(signals: &RepoSignals, path: &Path) -> ProjectType {
    if signals.has_game_engine {
        return ProjectType::Game;
    }
    if signals.has_notebooks {
        return ProjectType::Research;
    }
    if signals.has_webapp_framework {
        return ProjectType::Webapp;
    }
    if let Some(parent) = path.parent() {
        let parent_name = parent.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if parent_name == "studying" || parent_name == "learning" {
            return ProjectType::Study;
        }
    }
    ProjectType::Oss
}

fn suggest_stage(signals: &RepoSignals) -> u8 {
    if signals.has_tags && signals.has_ci && signals.has_readme {
        return 4;
    }
    if signals.has_ci || (signals.has_tags && signals.has_tests) {
        return 3;
    }
    if signals.has_src && signals.has_readme && signals.has_tests {
        return 2;
    }
    if signals.has_src || signals.has_readme {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectState, ProjectType};
    use chrono::NaiveDate;

    fn make_project(id: i64, name: &str) -> Project {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        Project {
            id,
            name: name.to_string(),
            state: ProjectState::Active,
            project_type: ProjectType::Oss,
            stage: 0,
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
            research_summary: None,
            inbox_note: None,
            next_task: None,
        }
    }

    fn signals(f: impl FnOnce(&mut RepoSignals)) -> RepoSignals {
        let mut s = RepoSignals::default();
        f(&mut s);
        s
    }

    #[test]
    fn score_one_skips_no_path() {
        let store = Store::open_in_memory().unwrap();
        let p = make_project(1, "no-path");
        assert!(score_one(&store, &p, &[], false).is_ok());
    }

    #[test]
    fn stage_empty_is_zero() {
        assert_eq!(suggest_stage(&RepoSignals::default()), 0);
    }

    #[test]
    fn stage_src_only_is_one() {
        assert_eq!(suggest_stage(&signals(|s| s.has_src = true)), 1);
    }

    #[test]
    fn stage_src_readme_tests_is_two() {
        assert_eq!(suggest_stage(&signals(|s| {
            s.has_src = true;
            s.has_readme = true;
            s.has_tests = true;
        })), 2);
    }

    #[test]
    fn stage_with_ci_is_three() {
        assert_eq!(suggest_stage(&signals(|s| {
            s.has_src = true;
            s.has_readme = true;
            s.has_tests = true;
            s.has_ci = true;
        })), 3);
    }

    #[test]
    fn stage_tags_ci_readme_is_four() {
        assert_eq!(suggest_stage(&signals(|s| {
            s.has_src = true;
            s.has_readme = true;
            s.has_tests = true;
            s.has_ci = true;
            s.has_tags = true;
        })), 4);
    }

    fn dummy_path() -> &'static Path {
        Path::new("/tmp/test-project")
    }

    #[test]
    fn type_game_engine_detected() {
        assert_eq!(detect_project_type(&signals(|s| s.has_game_engine = true), dummy_path()), ProjectType::Game);
    }

    #[test]
    fn type_notebooks_detected_as_research() {
        assert_eq!(detect_project_type(&signals(|s| s.has_notebooks = true), dummy_path()), ProjectType::Research);
    }

    #[test]
    fn type_webapp_framework_detected() {
        assert_eq!(detect_project_type(&signals(|s| s.has_webapp_framework = true), dummy_path()), ProjectType::Webapp);
    }

    #[test]
    fn type_default_is_oss() {
        assert_eq!(detect_project_type(&RepoSignals::default(), dummy_path()), ProjectType::Oss);
    }

    #[test]
    fn game_engine_takes_priority_over_webapp() {
        assert_eq!(detect_project_type(&signals(|s| {
            s.has_game_engine = true;
            s.has_webapp_framework = true;
        }), dummy_path()), ProjectType::Game);
    }

    #[test]
    fn type_study_from_parent_dir() {
        let path = Path::new("/home/user/projects/studying/some-repo");
        assert_eq!(detect_project_type(&RepoSignals::default(), path), ProjectType::Study);
    }
}
