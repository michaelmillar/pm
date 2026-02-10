use crate::cli_core;
use crate::similarity;
use crate::scanner;
use crate::store::Store;
use chrono::Local;
use std::error::Error;
use std::path::Path;

const AUTO_MERGE_THRESHOLD: f32 = 0.90;
const AUTO_NAME_THRESHOLD: f32 = 0.85;
const FLAG_THRESHOLD: f32 = 0.80;

pub fn discover_projects(store: &Store, root: &Path) -> Result<(), Box<dyn Error>> {
    let existing_projects = store.list_projects_for_dedupe()?;
    let entries = std::fs::read_dir(root)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join(".git").is_dir() {
            continue;
        }

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        let path_str = path.to_string_lossy().to_string();
        let id = store.get_or_create_by_path(&name, &path_str)?;

        let new_profile = profile_from_repo(&name, &path);
        let mut best_match: Option<(i64, f32, f32)> = None;

        for project in &existing_projects {
            if project.id == id || project.duplicate_of.is_some() {
                continue;
            }
            let profile = profile_from_project(project);
            let score = similarity::weighted_similarity(
                &new_profile.name,
                &profile.name,
                &new_profile.readme_title,
                &profile.readme_title,
                &new_profile.readme_snippet,
                &profile.readme_snippet,
                &new_profile.description,
                &profile.description,
            );
            let name_score = similarity::token_similarity(&new_profile.name, &profile.name);
            let better = match best_match {
                Some((_, best_score, _)) => score > best_score,
                None => true,
            };
            if better {
                best_match = Some((project.id, score, name_score));
            }
        }

        if let Some((candidate_id, score, name_score)) = best_match {
            if score >= AUTO_MERGE_THRESHOLD && name_score >= AUTO_NAME_THRESHOLD {
                store.mark_duplicate(id, candidate_id)?;
            } else if score >= FLAG_THRESHOLD {
                store.mark_possible_duplicate(id, score)?;
            }
        }

        let scan = scanner::scan_project(&path_str);
        let today = Local::now().date_naive();
        let project = match store.get_project(id)? {
            Some(p) => p,
            None => continue,
        };
        let score = cli_core::auto_score(&scan, project.created_at, today);
        store.update_scores(id, score.impact, score.monetization, score.readiness)?;
    }

    Ok(())
}

#[derive(Debug, Default)]
struct RepoProfile {
    name: String,
    readme_title: String,
    readme_snippet: String,
    description: String,
}

fn profile_from_project(project: &crate::domain::Project) -> RepoProfile {
    if let Some(path) = &project.path {
        let repo_path = Path::new(path);
        if repo_path.exists() {
            return profile_from_repo(&project.name, repo_path);
        }
    }

    RepoProfile {
        name: project.name.clone(),
        ..RepoProfile::default()
    }
}

fn profile_from_repo(name: &str, path: &Path) -> RepoProfile {
    let (readme_title, readme_snippet) = read_readme(path);
    let description = read_description(path);
    RepoProfile {
        name: name.to_string(),
        readme_title,
        readme_snippet,
        description,
    }
}

fn read_readme(path: &Path) -> (String, String) {
    let mut readme_path = None;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_lowercase();
            if file_name.starts_with("readme") {
                readme_path = Some(entry.path());
                break;
            }
        }
    }

    let Some(readme_path) = readme_path else {
        return (String::new(), String::new());
    };

    let content = match std::fs::read_to_string(readme_path) {
        Ok(c) => c,
        Err(_) => return (String::new(), String::new()),
    };

    let mut lines: Vec<&str> = content
        .lines()
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

    if !lines.is_empty() {
        lines.remove(0);
    }

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

    let package = path.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&package) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("\"description\"") {
                if let Some((_, value)) = line.split_once(':') {
                    return value.trim().trim_matches('"').trim_end_matches(',').to_string();
                }
            }
        }
    }

    String::new()
}
