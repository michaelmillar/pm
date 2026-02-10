use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "/home/markw/projects/pm-standards.yml";
pub const DEFAULT_REPORT_PATH: &str = "/home/markw/projects/.pm-standards-report.json";

#[derive(Debug, Deserialize)]
pub struct StandardsConfig {
    #[serde(default)]
    pub requirements: Vec<Check>,
    #[serde(default)]
    pub nice_to_haves: Vec<Check>,
    #[serde(default)]
    pub languages: HashMap<String, LanguageChecks>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageChecks {
    #[serde(default)]
    pub requirements: Vec<Check>,
    #[serde(default)]
    pub nice_to_haves: Vec<Check>,
}

#[derive(Debug, Deserialize)]
pub struct Check {
    pub name: String,
    pub check: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct StandardsReport {
    pub requirements_met: usize,
    pub nice_to_haves_met: usize,
    pub readiness_boost: u8,
    pub fixes: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RepoStandardsReport {
    pub name: String,
    pub path: String,
    pub requirements_met: usize,
    pub nice_to_haves_met: usize,
    pub readiness_boost: u8,
    pub fixes: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug)]
pub enum StandardsError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
}

impl StandardsConfig {
    pub fn from_str(input: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(input)
    }

    pub fn load() -> Result<Self, StandardsError> {
        let path = std::env::var("PM_STANDARDS_CONFIG")
            .unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
        load_from_path(Path::new(&path))
    }
}

pub fn load_from_path(path: &Path) -> Result<StandardsConfig, StandardsError> {
    let content = std::fs::read_to_string(path).map_err(StandardsError::Io)?;
    StandardsConfig::from_str(&content).map_err(StandardsError::Yaml)
}

pub fn write_report(path: &Path, reports: &[RepoStandardsReport]) -> Result<(), StandardsError> {
    let content = serde_json::to_string_pretty(reports).map_err(StandardsError::Json)?;
    std::fs::write(path, content).map_err(StandardsError::Io)
}

pub fn evaluate_repo(path: &Path, cfg: &StandardsConfig) -> Result<StandardsReport, StandardsError> {
    let mut report = StandardsReport::default();

    apply_checks(path, &cfg.requirements, true, &mut report);
    apply_checks(path, &cfg.nice_to_haves, false, &mut report);

    for language in detect_languages(path) {
        if let Some(lang_checks) = cfg.languages.get(language) {
            apply_checks(path, &lang_checks.requirements, true, &mut report);
            apply_checks(path, &lang_checks.nice_to_haves, false, &mut report);
        }
    }

    let boost = (report.requirements_met * 2 + report.nice_to_haves_met) as u8;
    report.readiness_boost = boost.min(20);

    Ok(report)
}

fn apply_checks(path: &Path, checks: &[Check], required: bool, report: &mut StandardsReport) {
    for check in checks {
        if check_path(path, &check.check) {
            if required {
                report.requirements_met += 1;
            } else {
                report.nice_to_haves_met += 1;
            }
        } else if let Some(fix) = try_fix(path, &check.check) {
            report.fixes.push(fix);
            if required {
                report.requirements_met += 1;
            } else {
                report.nice_to_haves_met += 1;
            }
        } else {
            report.missing.push(check.name.clone());
        }
    }
}

fn detect_languages(path: &Path) -> Vec<&'static str> {
    let mut langs = Vec::new();
    if path_exists(path, "Cargo.toml") {
        langs.push("rust");
    }
    if path_exists(path, "mix.exs") {
        langs.push("elixir");
    }
    if path_exists(path, "package.json") {
        langs.push("js");
    }
    langs
}

fn check_path(base: &Path, check: &str) -> bool {
    match check {
        "readme" => path_exists(base, "README.md"),
        "license" => path_exists(base, "LICENSE"),
        "agents_md" => path_exists(base, "AGENTS.md"),
        "docs_dir" => base.join("docs").is_dir(),
        "docs_plans_dir" => base.join("docs").join("plans").is_dir(),
        "ci" | "ci_config" => base.join(".github").join("workflows").is_dir(),
        "tests_dir" => base.join("tests").is_dir(),
        "cargo_toml" => path_exists(base, "Cargo.toml"),
        "mix_exs" => path_exists(base, "mix.exs"),
        "package_json" => path_exists(base, "package.json"),
        _ => false,
    }
}

fn try_fix(base: &Path, check: &str) -> Option<String> {
    match check {
        "readme" => ensure_readme(base).ok()?,
        "license" => ensure_license(base).ok()?,
        "agents_md" => ensure_agents(base).ok()?,
        "docs_dir" => ensure_dir(base.join("docs")).ok()?,
        "docs_plans_dir" => ensure_dir(base.join("docs").join("plans")).ok()?,
        "tests_dir" => ensure_dir(base.join("tests")).ok()?,
        _ => return None,
    };
    Some(check_file_label(check))
}

fn check_file_label(check: &str) -> String {
    match check {
        "readme" => "README.md",
        "license" => "LICENSE",
        "agents_md" => "AGENTS.md",
        "docs_dir" => "docs/",
        "docs_plans_dir" => "docs/plans/",
        "tests_dir" => "tests/",
        other => other,
    }
    .to_string()
}

fn ensure_readme(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join("README.md");
    if path.exists() {
        return Ok(());
    }
    std::fs::write(
        path,
        "# Project\n\n## Ethos/Mission\n- Problem: \n- Audience: \n- Promise: \n- Principles: \n\n## Build/Test\n- ",
    )
}

fn ensure_license(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join("LICENSE");
    if path.exists() {
        return Ok(());
    }
    std::fs::write(path, "")
}

fn ensure_agents(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join("AGENTS.md");
    if path.exists() {
        return Ok(());
    }
    std::fs::write(path, "")
}

fn ensure_dir(path: PathBuf) -> Result<(), std::io::Error> {
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(path)
}

fn path_exists(base: &Path, name: &str) -> bool {
    let path = PathBuf::from(base).join(name);
    path.exists()
}
