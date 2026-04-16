use crate::adapters;
use crate::domain::Project;
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

    if project.stage == 0 {
        if let Some(suggested) = suggest_stage(repo) {
            if suggested > project.stage {
                store.record_stage_event(
                    project.id,
                    project.stage,
                    suggested,
                    Some("autoscore suggestion"),
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

fn suggest_stage(path: &Path) -> Option<u8> {
    let has_readme = path.join("README.md").exists();
    let has_src = path.join("src").is_dir();
    let has_tests = path.join("tests").is_dir();

    if !has_src && !has_readme {
        return Some(0);
    }
    if has_src && has_readme && has_tests {
        return Some(2);
    }
    if has_src || has_readme {
        return Some(1);
    }
    Some(0)
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
        }
    }

    #[test]
    fn score_one_skips_no_path() {
        let store = Store::open_in_memory().unwrap();
        let p = make_project(1, "no-path");
        assert!(score_one(&store, &p, &[], false).is_ok());
    }

    #[test]
    fn suggest_stage_empty_dir_is_zero() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(suggest_stage(tmp.path()), Some(0));
    }

    #[test]
    fn suggest_stage_with_src_and_readme_is_one() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("README.md"), "hello").unwrap();
        assert_eq!(suggest_stage(tmp.path()), Some(1));
    }

    #[test]
    fn suggest_stage_with_tests_is_two() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::create_dir(tmp.path().join("tests")).unwrap();
        std::fs::write(tmp.path().join("README.md"), "hello").unwrap();
        assert_eq!(suggest_stage(tmp.path()), Some(2));
    }
}
