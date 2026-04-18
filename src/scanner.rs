use crate::domain::{RepoSignals, ScanResult};
use chrono::NaiveDate;
use std::path::Path;
use std::process::Command;

pub fn scan_project(project_path: &str) -> ScanResult {
    let path = Path::new(project_path);
    let last_commit_date = get_last_commit_date(path);
    ScanResult { last_commit_date }
}

pub fn scan_signals(path: &Path) -> RepoSignals {
    let has_src = path.join("src").is_dir();
    let has_readme = has_file_prefix(path, "readme");
    let has_tests = path.join("tests").is_dir()
        || path.join("test").is_dir()
        || path.join("__tests__").is_dir()
        || path.join("spec").is_dir();
    let has_ci = path.join(".github/workflows").is_dir()
        || path.join(".gitlab-ci.yml").exists()
        || path.join("Jenkinsfile").exists()
        || path.join(".circleci").is_dir();
    let has_license = has_file_prefix(path, "licen");
    let has_changelog = has_file_prefix(path, "changelog")
        || has_file_prefix(path, "changes");
    let has_cargo_toml = path.join("Cargo.toml").exists();
    let has_package_json = path.join("package.json").exists();
    let has_game_engine = path.join("project.godot").exists()
        || has_bevy_dep(path)
        || path.join("Assets").is_dir() && path.join("ProjectSettings").is_dir();
    let has_notebooks = has_extension_in_dir(path, "ipynb");
    let has_webapp_framework = detect_webapp_framework(path);
    let (has_tags, tag_count) = count_git_tags(path);
    let contributor_count = count_contributors(path);

    RepoSignals {
        has_src,
        has_readme,
        has_tests,
        has_ci,
        has_tags,
        tag_count,
        has_license,
        has_changelog,
        has_cargo_toml,
        has_package_json,
        has_game_engine,
        has_notebooks,
        has_webapp_framework,
        contributor_count,
    }
}

fn has_file_prefix(path: &Path, prefix: &str) -> bool {
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.starts_with(prefix) {
            return true;
        }
    }
    false
}

fn has_extension_in_dir(path: &Path, ext: &str) -> bool {
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        if let Some(e) = entry.path().extension() {
            if e.to_string_lossy().eq_ignore_ascii_case(ext) {
                return true;
            }
        }
    }
    false
}

fn has_bevy_dep(path: &Path) -> bool {
    let cargo = path.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(cargo) {
        return content.contains("bevy");
    }
    false
}

fn detect_webapp_framework(path: &Path) -> bool {
    let pkg = path.join("package.json");
    if let Ok(content) = std::fs::read_to_string(pkg) {
        let frameworks = ["react", "vue", "svelte", "next", "nuxt", "angular", "express", "fastify"];
        let lower = content.to_lowercase();
        if frameworks.iter().any(|f| lower.contains(f)) {
            return true;
        }
    }
    let cargo = path.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(cargo) {
        let frameworks = ["leptos", "yew", "dioxus"];
        let in_deps = extract_deps_section(&content);
        if frameworks.iter().any(|f| in_deps.contains(f)) {
            return true;
        }
    }
    false
}

fn extract_deps_section(toml: &str) -> String {
    let mut in_deps = false;
    let mut result = String::new();
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if in_deps {
            result.push_str(&trimmed.to_lowercase());
            result.push('\n');
        }
    }
    result
}

fn count_git_tags(path: &Path) -> (bool, usize) {
    let output = Command::new("git")
        .args(["tag", "--list"])
        .current_dir(path)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let count = String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            (count > 0, count)
        }
        _ => (false, 0),
    }
}

pub fn extract_next_task(repo: &Path) -> Option<String> {
    for fname in ["PLAN.md", "TODO.md", "ROADMAP.md", "plan.md", "todo.md", "roadmap.md"] {
        let p = repo.join(fname);
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Some(item) = first_unchecked_item(&content) {
                return Some(item);
            }
        }
    }
    None
}

fn first_unchecked_item(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("- [ ]") || lower.starts_with("* [ ]") {
            let rest = trimmed[5..].trim();
            if !rest.is_empty() {
                let truncated: String = rest.chars().take(120).collect();
                return Some(truncated);
            }
        }
    }
    None
}

fn count_contributors(path: &Path) -> usize {
    let output = Command::new("git")
        .args(["shortlog", "-sn", "--no-merges", "HEAD"])
        .current_dir(path)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count()
        }
        _ => 0,
    }
}

pub fn get_last_commit_date(path: &Path) -> Option<NaiveDate> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ad", "--date=format:%Y-%m-%d"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let date_str = String::from_utf8_lossy(&output.stdout);
    NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d").ok()
}

pub fn get_recent_commits(path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-200", "--format=%s"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_git_repo(path: &Path) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(path)
                .status()
                .expect("git command failed");
        };
        run(&["init"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test User"]);
        fs::write(path.join("README.md"), "init").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "initial commit"]);
    }

    #[test]
    fn scan_returns_last_commit_date() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        let result = scan_project(tmp.path().to_str().unwrap());
        assert!(result.last_commit_date.is_some());
    }

    #[test]
    fn scan_no_git_returns_none() {
        let tmp = TempDir::new().unwrap();
        let result = scan_project(tmp.path().to_str().unwrap());
        assert!(result.last_commit_date.is_none());
    }
}
