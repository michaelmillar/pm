use crate::cli_core;
use crate::similarity;
use crate::scanner;
use crate::standards;
use crate::store::Store;
use chrono::Local;
use std::error::Error;
use std::path::Path;
use std::collections::HashSet;

const AUTO_MERGE_THRESHOLD: f32 = 0.90;
const AUTO_NAME_THRESHOLD: f32 = 0.85;
const FLAG_THRESHOLD: f32 = 0.80;

const DEFAULT_IGNORED_FOLDERS: &[&str] = &[
    ".git",
    ".worktrees",
    "node_modules",
    "target",
    "_build",
    "deps",
    "docs",
    "vendor",
];

pub fn discover_projects(store: &Store, root: &Path) -> Result<(), Box<dyn Error>> {
    let standards_config = standards::StandardsConfig::load().ok();
    let mut standards_reports: Vec<standards::RepoStandardsReport> = Vec::new();
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
        let _today = Local::now().date_naive();
        let _project = match store.get_project(id)? {
            Some(p) => p,
            None => continue,
        };
        let mut readiness = cli_core::auto_readiness(&scan) as i32;
        if let Some(cfg) = &standards_config {
            if let Ok(report) = standards::evaluate_repo(&path, cfg) {
                readiness = (readiness + report.readiness_boost as i32).min(100);
                standards_reports.push(standards::RepoStandardsReport {
                    name: name.clone(),
                    path: path_str.clone(),
                    requirements_met: report.requirements_met,
                    nice_to_haves_met: report.nice_to_haves_met,
                    readiness_boost: report.readiness_boost,
                    fixes: report.fixes.clone(),
                    missing: report.missing.clone(),
                });
            }
        }
        store.update_readiness(id, readiness as u8)?;
    }

    if standards_config.is_some() && !standards_reports.is_empty() {
        let report_path = std::env::var("PM_STANDARDS_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| standards::default_report_path());
        let _ = standards::write_report(&report_path, &standards_reports);
    }

    Ok(())
}

pub fn list_nonrepo_folders(root: &Path) -> Vec<String> {
    let ignored: HashSet<&str> = DEFAULT_IGNORED_FOLDERS.iter().copied().collect();
    let mut results = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        if ignored.contains(name.as_str()) {
            continue;
        }
        if path.join(".git").is_dir() {
            continue;
        }
        results.push(name);
    }

    results.sort();
    results
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
