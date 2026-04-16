use crate::domain::ScanResult;
use chrono::NaiveDate;
use std::path::Path;
use std::process::Command;

pub fn scan_project(project_path: &str) -> ScanResult {
    let path = Path::new(project_path);
    let last_commit_date = get_last_commit_date(path);
    ScanResult { last_commit_date }
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
