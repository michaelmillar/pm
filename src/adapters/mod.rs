pub mod github;
pub mod research;
pub mod steam;
pub mod analytics;

use crate::domain::{Project, ProjectType};

#[derive(Debug, Clone)]
pub struct FitSignalResult {
    pub raw_score: u8,
    pub source: String,
}

pub fn fetch_fit_signal(project: &Project) -> Option<FitSignalResult> {
    match project.project_type {
        ProjectType::Oss => fetch_oss_fit(project),
        ProjectType::Research => research::fetch_fit(project),
        ProjectType::Game => steam::fetch_fit(project),
        ProjectType::Webapp => analytics::fetch_fit(project),
    }
}

fn fetch_oss_fit(project: &Project) -> Option<FitSignalResult> {
    let path = project.path.as_ref()?;
    let remote = read_git_remote(std::path::Path::new(path))?;
    let slug = github::slug_from_remote(&remote)?;
    let signal = github::fetch(&slug).ok()?;
    let stars = signal.stars.unwrap_or(0);
    let raw = (((stars as f64 + 1.0).log2()) * 1.5).min(10.0) as u8;
    Some(FitSignalResult {
        raw_score: raw,
        source: format!("github:{} stars={}", slug, stars),
    })
}

fn read_git_remote(path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
