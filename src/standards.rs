use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> PathBuf {
    dirs::config_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pm")
        .join("standards.yml")
}

pub fn default_report_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pm")
        .join("standards-report.json")
}

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
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_config_path());
        load_from_path(&path)
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
        "charter" => path_exists(base, "CHARTER.md"),
        "dod" => path_exists(base, "DOD.md"),
        "gitleaks" => path_exists(base, ".gitleaks.toml"),
        "no_secrets" => !has_leaked_secrets(base),
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
        "charter" => ensure_charter(base).ok()?,
        "dod" => ensure_dod(base).ok()?,
        "gitleaks" => ensure_gitleaks(base).ok()?,
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
        "charter" => "CHARTER.md",
        "dod" => "DOD.md",
        "gitleaks" => ".gitleaks.toml",
        "no_secrets" => "(secret scan)",
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

fn ensure_charter(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join("CHARTER.md");
    if path.exists() {
        return Ok(());
    }
    let name = base
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());
    std::fs::write(
        path,
        format!(
            r#"# {name}

## USP (one-liner)
<!-- What is this and why does it matter, in one sentence? -->


## USP (expanded)
<!-- Three lines: problem, audience, unique angle -->
- Problem:
- Audience:
- Unique angle:

## Target audience


## Key principles

"#
        ),
    )
}

fn ensure_dod(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join("DOD.md");
    if path.exists() {
        return Ok(());
    }
    let name = base
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());
    std::fs::write(
        path,
        format!(
            r#"# Definition of Done: {name}

USP: <!-- paste one-liner from CHARTER.md -->

## Criteria

### 1. Core value delivered
- scenario: A user can [describe primary workflow]
- automated: pending
- human: pending

### 2. Test coverage meets threshold
- scenario: cargo test / npm test passes, coverage >= 80%
- automated: pending
- human: pending

### 3. No credential leaks
- scenario: gitleaks scan returns zero findings on full history
- automated: pending
- human: pending

### 4. Documentation complete
- scenario: README has build/test instructions, CHARTER.md filled in
- automated: pending
- human: pending
"#
        ),
    )
}

fn ensure_gitleaks(base: &Path) -> Result<(), std::io::Error> {
    let path = base.join(".gitleaks.toml");
    if path.exists() {
        return Ok(());
    }
    std::fs::write(
        path,
        r#"title = "gitleaks config"

[extend]
# Uses the default gitleaks rules (API keys, tokens, passwords, etc.)
# See https://github.com/gitleaks/gitleaks

[allowlist]
description = "project-specific allowlist"
paths = [
    '''(^|/)vendor/''',
    '''(^|/)node_modules/''',
    '''(^|/)target/''',
    '''\.lock$''',
]
"#,
    )
}

/// Scan tracked files for common secret patterns.
/// Returns true if any likely secrets are found.
fn has_leaked_secrets(base: &Path) -> bool {
    // Only scan if it is a git repo so we can use git ls-files
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(base)
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let file_list = String::from_utf8_lossy(&output.stdout);
    for entry in file_list.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Skip binary-looking extensions and vendored paths
        if entry.ends_with(".lock")
            || entry.contains("node_modules/")
            || entry.contains("vendor/")
            || entry.contains("target/")
            || entry.ends_with(".png")
            || entry.ends_with(".jpg")
            || entry.ends_with(".wasm")
            || entry.ends_with(".db")
        {
            continue;
        }
        // Flag .env files that are tracked (should never be committed)
        let basename = entry.rsplit('/').next().unwrap_or(entry);
        if basename == ".env"
            || basename == ".env.local"
            || basename == ".env.production"
            || basename == "credentials.json"
            || basename == "secrets.yml"
            || basename == "secrets.yaml"
        {
            return true;
        }
        // Read file and scan for high-confidence patterns
        let full_path = base.join(entry);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            if scan_content_for_secrets(&content) {
                return true;
            }
        }
    }
    false
}

fn scan_content_for_secrets(content: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        // AWS access key IDs (always start with AKIA)
        if trimmed.contains("AKIA") && trimmed.len() > 20 {
            return true;
        }
        // OpenAI / Anthropic / GitHub tokens
        for prefix in &["sk-ant-", "sk-proj-", "ghp_", "ghs_", "gho_", "github_pat_"] {
            if trimmed.contains(prefix) {
                return true;
            }
        }
        // Generic patterns: long hex/base64 assigned to key-like variables
        let lower = trimmed.to_lowercase();
        if (lower.contains("api_key") || lower.contains("apikey") || lower.contains("secret_key") || lower.contains("password"))
            && (trimmed.contains('=') || trimmed.contains(':'))
            && !trimmed.contains("env(") && !trimmed.contains("env!(")
            && !trimmed.contains("${") && !trimmed.contains("process.env")
            && !trimmed.contains("std::env") && !trimmed.contains("os.environ")
        {
            // Check if there is an actual value (not empty, not a placeholder)
            if let Some(val_part) = trimmed.split(['=', ':']).nth(1) {
                let val = val_part.trim().trim_matches('"').trim_matches('\'').trim();
                if val.len() > 8
                    && !val.starts_with("your-")
                    && !val.starts_with("xxx")
                    && !val.starts_with("TODO")
                    && !val.starts_with("CHANGE")
                    && !val.contains("example")
                    && !val.is_empty()
                {
                    return true;
                }
            }
        }
    }
    false
}
