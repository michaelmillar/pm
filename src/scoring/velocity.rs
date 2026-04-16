use chrono::NaiveDate;
use std::path::Path;
use std::process::Command;

pub struct VelocityResult {
    pub score: u8,
    pub sunk_cost_days: i32,
}

pub fn compute(path: &Path) -> Option<VelocityResult> {
    let today = chrono::Local::now().date_naive();
    let commits_14d = count_recent_commits(path, 14)?;
    let first_date = read_first_commit_date(path)?;
    let sunk_cost_days = (today - first_date).num_days().max(0) as i32;

    let cadence_penalty = cadence_variance_penalty(path);
    let raw = (commits_14d as i32 * 2).min(10) - cadence_penalty;
    let score = raw.clamp(0, 10) as u8;

    Some(VelocityResult { score, sunk_cost_days })
}

fn count_recent_commits(path: &Path, days: u32) -> Option<usize> {
    let output = Command::new("git")
        .args(["log", "--oneline", &format!("--since={} days ago", days)])
        .current_dir(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.lines().filter(|l| !l.is_empty()).count())
}

fn read_first_commit_date(path: &Path) -> Option<NaiveDate> {
    let output = Command::new("git")
        .args(["log", "--reverse", "--format=%ad", "--date=format:%Y-%m-%d", "-1"])
        .current_dir(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

fn cadence_variance_penalty(path: &Path) -> i32 {
    let output = Command::new("git")
        .args(["log", "--format=%ad", "--date=format:%Y-%m-%d", "-30"])
        .current_dir(path)
        .output();

    let dates: Vec<NaiveDate> = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| NaiveDate::parse_from_str(l.trim(), "%Y-%m-%d").ok())
                .collect()
        }
        _ => return 0,
    };

    if dates.len() < 3 {
        return 0;
    }

    let intervals: Vec<f64> = dates.windows(2)
        .map(|w| (w[0] - w[1]).num_days().abs() as f64)
        .collect();

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let variance = intervals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / intervals.len() as f64;
    let stdev = variance.sqrt();

    if stdev > 7.0 { 2 } else if stdev > 3.0 { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo_with_commits(path: &Path, count: usize) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(path)
                .output()
                .unwrap();
        };
        run(&["init"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        for i in 0..count {
            fs::write(path.join(format!("f{}.txt", i)), format!("{}", i)).unwrap();
            run(&["add", "."]);
            run(&["commit", "-m", &format!("commit {}", i)]);
        }
    }

    #[test]
    fn velocity_for_active_repo() {
        let tmp = TempDir::new().unwrap();
        init_repo_with_commits(tmp.path(), 5);
        let result = compute(tmp.path()).unwrap();
        assert!(result.score >= 8, "5 commits in 14d should score high, got {}", result.score);
        assert!(result.sunk_cost_days >= 0);
    }

    #[test]
    fn velocity_none_for_non_repo() {
        let tmp = TempDir::new().unwrap();
        assert!(compute(tmp.path()).is_none());
    }

    #[test]
    fn first_commit_date_works() {
        let tmp = TempDir::new().unwrap();
        init_repo_with_commits(tmp.path(), 3);
        let date = read_first_commit_date(tmp.path());
        assert!(date.is_some());
    }
}
