use crate::charter;
use crate::domain::{ScanResult, TaskSource, TaskStatus};
use chrono::NaiveDate;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub fn scan_project(project_path: &str) -> ScanResult {
    let path = Path::new(project_path);

    let last_commit_date = get_last_commit_date(path);
    let commits = get_recent_commits(path);
    let changed_files = get_recent_commit_files(path);
    let progress = read_progress_file(path);
    let has_progress_file = path.join(".pm-progress").exists();
    let (plan_files, total_tasks, completed_tasks) =
        scan_plan_files(path, &commits, &changed_files, &progress);

    let charter_status = charter::check_charter(path);
    let charter_filled = if charter_status.exists {
        Some((charter_status.filled, charter_status.total))
    } else {
        None
    };

    ScanResult {
        total_tasks,
        completed_tasks,
        last_commit_date,
        plan_files,
        has_progress_file,
        charter_filled,
    }
}

fn get_last_commit_date(path: &Path) -> Option<NaiveDate> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ad", "--date=format:%Y-%m-%d"])
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

fn scan_plan_files(
    path: &Path,
    commits: &[String],
    changed_files: &[String],
    progress: &HashSet<(String, usize)>,
) -> (Vec<String>, usize, usize) {
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
            let name = match file_path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            plan_files.push(name.clone());

            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let (tasks, done) =
                    count_tasks_in_plan(&content, commits, changed_files, &name, progress);
                total_tasks += tasks;
                completed_tasks += done;
            }
        }
    }

    (plan_files, total_tasks, completed_tasks)
}

fn count_tasks_in_plan(
    content: &str,
    commits: &[String],
    changed_files: &[String],
    plan_name: &str,
    progress: &HashSet<(String, usize)>,
) -> (usize, usize) {
    let mut total = 0;
    let mut completed = 0;

    for line in content.lines() {
        if line.starts_with("### Task") && line.contains(':') {
            total += 1;

            // Check manual progress first
            if progress.contains(&(plan_name.to_string(), total)) {
                completed += 1;
                continue;
            }

            // Extract task description after the colon
            if let Some(desc) = line.split(':').nth(1) {
                let desc_lower = desc.trim().to_lowercase();

                let keywords: Vec<&str> = desc_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .take(5)
                    .collect();

                if keywords.is_empty() {
                    continue;
                }

                // Check commit subjects
                let is_done = commits.iter().any(|commit| {
                    let commit_lower = commit.to_lowercase();
                    keywords.iter().filter(|kw| commit_lower.contains(*kw)).count() >= 2
                });

                if is_done {
                    completed += 1;
                    continue;
                }

                // Check changed file paths
                let file_match = changed_files.iter().any(|f| {
                    let f_lower = f.to_lowercase();
                    keywords.iter().filter(|kw| f_lower.contains(*kw)).count() >= 2
                });

                if file_match {
                    completed += 1;
                }
            }
        }
    }

    (total, completed)
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

pub fn get_recent_commit_files(path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["log", "--name-only", "--pretty=format:", "-200"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn read_progress_file(path: &Path) -> HashSet<(String, usize)> {
    let progress_path = path.join(".pm-progress");
    let mut set = HashSet::new();

    let content = match std::fs::read_to_string(&progress_path) {
        Ok(c) => c,
        Err(_) => return set,
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((plan, num_str)) = line.rsplit_once(':') {
            if let Ok(num) = num_str.parse::<usize>() {
                set.insert((plan.to_string(), num));
            }
        }
    }

    set
}

pub fn list_tasks(project_path: &Path) -> Vec<TaskStatus> {
    let plans_dir = project_path.join("docs").join("plans");
    let mut tasks = Vec::new();

    if !plans_dir.exists() {
        return tasks;
    }

    let commits = get_recent_commits(project_path);
    let changed_files = get_recent_commit_files(project_path);
    let progress = read_progress_file(project_path);

    let mut plan_files: Vec<_> = match std::fs::read_dir(&plans_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return tasks,
    };
    plan_files.sort_by_key(|e| e.file_name());

    for entry in &plan_files {
        let file_path = entry.path();
        let plan_name = match file_path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut task_num = 0;
        for line in content.lines() {
            if line.starts_with("### Task") && line.contains(':') {
                task_num += 1;
                let desc = line
                    .split(':')
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                // Determine source
                let source = if progress.contains(&(plan_name.clone(), task_num)) {
                    TaskSource::Manual
                } else {
                    let desc_lower = desc.to_lowercase();
                    let keywords: Vec<&str> = desc_lower
                        .split_whitespace()
                        .filter(|w| w.len() > 2)
                        .take(5)
                        .collect();

                    let git_match = if keywords.is_empty() {
                        false
                    } else {
                        let commit_match = commits.iter().any(|commit| {
                            let commit_lower = commit.to_lowercase();
                            keywords
                                .iter()
                                .filter(|kw| commit_lower.contains(*kw))
                                .count()
                                >= 2
                        });
                        let file_match = changed_files.iter().any(|f| {
                            let f_lower = f.to_lowercase();
                            keywords.iter().filter(|kw| f_lower.contains(*kw)).count() >= 2
                        });
                        commit_match || file_match
                    };

                    if git_match {
                        TaskSource::Git
                    } else {
                        TaskSource::Pending
                    }
                };

                tasks.push(TaskStatus {
                    plan_file: plan_name.clone(),
                    task_number: task_num,
                    description: desc,
                    source,
                });
            }
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_git_repo(path: &Path) {
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(path)
                .status()
                .expect("git command failed");
            assert!(status.success());
        };

        run(&["init"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test User"]);

        fs::write(path.join("README.md"), "init").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "initial commit"]);
    }

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
        let commits = Vec::new();
        let changed_files = Vec::new();
        let progress = HashSet::new();
        let (total, _) = count_tasks_in_plan(content, &commits, &changed_files, "plan.md", &progress);
        assert_eq!(total, 3);
    }

    #[test]
    fn test_keyword_matching_wider() {
        let content = "### Task 1: Add CLI FAQ support";
        let commits = vec!["add faq page to cli docs".to_string()];
        let changed_files = Vec::new();
        let progress = HashSet::new();
        let (total, completed) =
            count_tasks_in_plan(content, &commits, &changed_files, "plan.md", &progress);
        assert_eq!(total, 1);
        // "add", "cli", "faq" are all > 2 chars, commit has "add" and "faq" and "cli"
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_file_path_matching() {
        let content = "### Task 1: Add domain logic module";
        let commits = Vec::new();
        let changed_files = vec!["src/domain/logic.rs".to_string()];
        let progress = HashSet::new();
        let (total, completed) =
            count_tasks_in_plan(content, &commits, &changed_files, "plan.md", &progress);
        assert_eq!(total, 1);
        // "domain" and "logic" match in file path
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_progress_file_overrides() {
        let content = r#"### Task 1: Something obscure
### Task 2: Another thing"#;
        let commits = Vec::new();
        let changed_files = Vec::new();
        let mut progress = HashSet::new();
        progress.insert(("plan.md".to_string(), 1));
        let (total, completed) =
            count_tasks_in_plan(content, &commits, &changed_files, "plan.md", &progress);
        assert_eq!(total, 2);
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_read_progress_file_empty() {
        let set = read_progress_file(Path::new("/nonexistent/path"));
        assert!(set.is_empty());
    }

    #[test]
    fn test_get_last_commit_date_returns_date() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        let date = get_last_commit_date(tmp.path());
        assert!(date.is_some());
    }

    #[test]
    fn test_get_recent_commit_files_contains_file() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());

        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/feature.rs"), "fn x() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature module"])
            .current_dir(tmp.path())
            .status()
            .unwrap();

        let files = get_recent_commit_files(tmp.path());
        assert!(files.iter().any(|f| f.contains("src/feature.rs")));
    }

    #[test]
    fn test_scan_project_detects_plan_and_progress() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());

        let plans_dir = tmp.path().join("docs").join("plans");
        fs::create_dir_all(&plans_dir).unwrap();
        let plan = r#"### Task 1: Add widget pipeline
### Task 2: Add reporting view
"#;
        fs::write(plans_dir.join("plan.md"), plan).unwrap();
        fs::write(tmp.path().join(".pm-progress"), "plan.md:1\n").unwrap();

        let result = scan_project(tmp.path().to_str().unwrap());
        assert_eq!(result.total_tasks, 2);
        assert_eq!(result.completed_tasks, 1);
        assert!(result.has_progress_file);
        assert_eq!(result.plan_files.len(), 1);
    }

    #[test]
    fn test_list_tasks_sources() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());

        let plans_dir = tmp.path().join("docs").join("plans");
        fs::create_dir_all(&plans_dir).unwrap();
        let plan = r#"### Task 1: Add widget pipeline
### Task 2: Add reporting view
"#;
        fs::write(plans_dir.join("plan.md"), plan).unwrap();
        fs::write(tmp.path().join(".pm-progress"), "plan.md:1\n").unwrap();

        fs::write(tmp.path().join("reporting.txt"), "done").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add reporting view"])
            .current_dir(tmp.path())
            .status()
            .unwrap();

        let tasks = list_tasks(tmp.path());
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].source, TaskSource::Manual);
        assert_eq!(tasks[1].source, TaskSource::Git);
    }
}
