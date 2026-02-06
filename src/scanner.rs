use crate::domain::ScanResult;
use chrono::NaiveDate;
use std::path::Path;
use std::process::Command;

pub fn scan_project(project_path: &str) -> ScanResult {
    let path = Path::new(project_path);

    let last_commit_date = get_last_commit_date(path);
    let (plan_files, total_tasks, completed_tasks) = scan_plan_files(path);

    ScanResult {
        total_tasks,
        completed_tasks,
        last_commit_date,
        plan_files,
    }
}

fn get_last_commit_date(path: &Path) -> Option<NaiveDate> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%Y-%m-%d"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let date_str = String::from_utf8_lossy(&output.stdout);
    let date_str = date_str.trim();
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

fn scan_plan_files(path: &Path) -> (Vec<String>, usize, usize) {
    let plans_dir = path.join("docs").join("plans");
    let mut plan_files = Vec::new();
    let mut total_tasks = 0;
    let mut completed_tasks = 0;

    if !plans_dir.exists() {
        return (plan_files, total_tasks, completed_tasks);
    }

    let entries = match std::fs::read_dir(&plans_dir) {
        Ok(e) => e,
        Err(_) => return (plan_files, total_tasks, completed_tasks),
    };

    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(name) = file_path.file_name() {
                plan_files.push(name.to_string_lossy().to_string());
            }

            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let (tasks, done) = count_tasks_in_plan(&content, path);
                total_tasks += tasks;
                completed_tasks += done;
            }
        }
    }

    (plan_files, total_tasks, completed_tasks)
}

fn count_tasks_in_plan(content: &str, project_path: &Path) -> (usize, usize) {
    let mut total = 0;
    let mut completed = 0;

    // Get commit messages for matching
    let commits = get_recent_commits(project_path);

    for line in content.lines() {
        // Match "### Task N:" headers
        if line.starts_with("### Task") && line.contains(':') {
            total += 1;

            // Extract task description after the colon
            if let Some(desc) = line.split(':').nth(1) {
                let desc_lower = desc.trim().to_lowercase();

                // Check if any commit message contains keywords from task description
                let keywords: Vec<&str> = desc_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .take(3)
                    .collect();

                if !keywords.is_empty() {
                    let is_done = commits.iter().any(|commit| {
                        let commit_lower = commit.to_lowercase();
                        keywords.iter().filter(|kw| commit_lower.contains(*kw)).count() >= 2
                    });

                    if is_done {
                        completed += 1;
                    }
                }
            }
        }
    }

    (total, completed)
}

fn get_recent_commits(path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-50", "--format=%s"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tasks_basic() {
        let content = r#"
# Plan

### Task 1: Set up project skeleton

Some content

### Task 2: Add domain logic

More content

### Task 3: Wire CLI
"#;
        let (total, _) = count_tasks_in_plan(content, Path::new("."));
        assert_eq!(total, 3);
    }
}
