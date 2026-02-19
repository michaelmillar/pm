// Research scheduling and CLI integration with provider fallback.

use chrono::NaiveDate;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum ResearchFrequency {
    Daily,
    Weekly,
    Monthly,
    Never,
}

impl ResearchFrequency {
    pub fn days(&self) -> i64 {
        match self {
            ResearchFrequency::Daily => 1,
            ResearchFrequency::Weekly => 7,
            ResearchFrequency::Monthly => 30,
            ResearchFrequency::Never => i64::MAX,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "daily" => ResearchFrequency::Daily,
            "weekly" => ResearchFrequency::Weekly,
            "monthly" => ResearchFrequency::Monthly,
            _ => ResearchFrequency::Never,
        }
    }
}

pub fn is_research_due(researched_at: Option<&str>, freq: &ResearchFrequency) -> bool {
    if matches!(freq, ResearchFrequency::Never) {
        return false;
    }
    let Some(date_str) = researched_at else {
        return true; // never researched → due
    };
    let Ok(last) = NaiveDate::parse_from_str(date_str.trim_matches('"'), "%Y-%m-%d") else {
        return true;
    };
    let today = chrono::Local::now().date_naive();
    (today - last).num_days() >= freq.days()
}

pub fn load_frequency() -> ResearchFrequency {
    // Read from config file; default to weekly
    let config_path = research_config_path();
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("frequency=") {
                return ResearchFrequency::from_str(val.trim());
            }
        }
    }
    ResearchFrequency::Weekly
}

pub fn save_frequency(freq: &ResearchFrequency) {
    let path = research_config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let s = match freq {
        ResearchFrequency::Daily => "daily",
        ResearchFrequency::Weekly => "weekly",
        ResearchFrequency::Monthly => "monthly",
        ResearchFrequency::Never => "never",
    };
    let _ = std::fs::write(path, format!("frequency={}\n", s));
}

fn research_config_path() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("PM_RESEARCH_CONFIG") {
        return std::path::PathBuf::from(val);
    }
    dirs::config_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("pm")
        .join("research.conf")
}

pub fn detect_cut_losses(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("consider stopping") || lower.contains("cut losses")
}

/// Call the claude CLI to run competitive research for a project.
/// Returns the raw Claude output or an error message.
pub fn run_research_claude(project_name: &str, usp: &str) -> Result<String, String> {
    let prompt = format!(
        r#"Research the competitive landscape for this project and summarise your findings.

Project: {name}
USP: {usp}

Search for:
1. Existing products, tools, games, or services that solve the same problem
2. Recent market signals (forum discussions, reviews, launch announcements in the last few months)
3. Any gaps or opportunities in the current landscape

Format your response with exactly these three sections:

## Competitors
[List each competitor with: name, URL, one-line description, similarity level (high/medium/low)]

## Signals
[Recent market signals - forum posts, news, user demand indicators. If none found, write "None noted."]

## Assessment
Overall: [crowded/competitive/niche/novel] - one sentence.
Key differentiators needed:
- [bullet 1]
- [bullet 2]
Recommendation: [one paragraph - should the project continue, pivot, or consider stopping? Be direct.]"#,
        name = project_name,
        usp = usp,
    );

    run_with_fallback(&prompt, &["WebSearch", "WebFetch"])
}

/// Call the claude CLI to generate a diff between two research summaries.
pub fn run_diff_claude(project_name: &str, usp: &str, previous: &str, current: &str, previous_date: &str) -> Result<String, String> {
    let prompt = format!(
        r#"Compare these two competitive research summaries for the project '{name}' and show what changed.

USP: {usp}

PREVIOUS RESEARCH ({prev_date}):
{previous}

CURRENT RESEARCH (today):
{current}

Produce a diff showing what changed. Format your response with exactly these three sections:

## New entrants
[New competitors or tools that appeared since the last scan. If none, write "None noted."]

## Signals
[New market signals or trend changes since the last scan. If none, write "None noted."]

## Recommendation
[Based on changes in the landscape, what should the project owner do? Be direct.
End with one of:
- "continue"
- "pivot: [brief suggestion]"
- "consider stopping: [reason]"]"#,
        name = project_name,
        usp = usp,
        prev_date = previous_date,
        previous = previous,
        current = current,
    );

    run_with_fallback(&prompt, &["WebSearch", "WebFetch"])
}

/// Call the claude CLI to verify a DOD criterion against a codebase.
pub fn run_verify_claude(
    project_name: &str,
    usp: &str,
    criterion_id: &str,
    criterion_desc: &str,
    scenario: &str,
    evidence_hint: Option<&str>,
    evidence_content: Option<&str>,
    file_tree: &str,
) -> Result<String, String> {
    let evidence_section = match (evidence_hint, evidence_content) {
        (Some(hint), Some(content)) => format!(
            "Evidence hint: {}\n\nEvidence file contents:\n```\n{}\n```",
            hint, content
        ),
        (Some(hint), None) => format!("Evidence hint: {} (file not found)", hint),
        _ => "No evidence hint provided.".to_string(),
    };

    let prompt = format!(
        r#"You are verifying a Definition of Done criterion for the Rust/software project '{name}'.

USP: {usp}

Criterion: [{id}] {desc}

{evidence}

Repository file tree (truncated):
{tree}

BDD Scenario to verify:
{scenario}

Based on the evidence above, verify whether this criterion has been implemented.
Respond in exactly this format (no other text):
VERDICT: pass|fail|inconclusive
RATIONALE: [one paragraph citing specific evidence from the files above]"#,
        name = project_name,
        usp = usp,
        id = criterion_id,
        desc = criterion_desc,
        evidence = evidence_section,
        tree = file_tree,
        scenario = scenario,
    );

    run_with_fallback(&prompt, &["Read", "Glob", "Grep"])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    Claude,
    Codex,
}

fn run_with_fallback(prompt: &str, tools: &[&str]) -> Result<String, String> {
    run_with_fallback_using(prompt, tools, |provider| match provider {
        Provider::Claude => run_claude_with_tools(prompt, tools),
        Provider::Codex => run_codex(prompt),
    })
}

fn run_with_fallback_using<F>(_prompt: &str, _tools: &[&str], mut runner: F) -> Result<String, String>
where
    F: FnMut(Provider) -> Result<String, String>,
{
    match runner(Provider::Claude) {
        Ok(output) => Ok(output),
        Err(claude_err) => match runner(Provider::Codex) {
            Ok(output) => {
                eprintln!(
                    "Warning: Claude failed ({}). Fell back to Codex.",
                    compact_error(&claude_err)
                );
                Ok(output)
            }
            Err(codex_err) => Err(format!(
                "claude failed: {}; codex fallback failed: {}",
                compact_error(&claude_err),
                compact_error(&codex_err)
            )),
        },
    }
}

fn compact_error(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn run_claude_with_tools(prompt: &str, tools: &[&str]) -> Result<String, String> {
    let allowed_tools = tools.join(",");
    let output = Command::new("claude")
        .args([
            "-p", prompt,
            "--model", "sonnet",
            "--allowedTools", &allowed_tools,
        ])
        .output()
        .map_err(|e| format!("Failed to run claude CLI: {}. Is it installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("claude CLI failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_codex(prompt: &str) -> Result<String, String> {
    let timeout = codex_timeout();
    let mut command = Command::new("codex");
    command.args([
        "exec",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        prompt,
    ]);
    let output = run_command_with_timeout(&mut command, timeout)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("codex CLI failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn codex_timeout() -> Duration {
    let secs = std::env::var("PM_CODEX_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(45);
    Duration::from_secs(secs)
}

fn run_command_with_timeout(command: &mut Command, timeout: Duration) -> Result<std::process::Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to run command: {}", e))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|e| format!("Failed collecting command output: {}", e));
            }
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("codex CLI timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Failed waiting on command: {}", e)),
        }
    }
}

/// Parse the VERDICT/RATIONALE response from claude verify output.
pub fn parse_verdict(output: &str) -> (String, Option<String>) {
    let mut verdict = "inconclusive".to_string();
    let mut rationale_lines: Vec<String> = Vec::new();
    let mut in_rationale = false;

    for line in output.lines() {
        if let Some(v) = line.strip_prefix("VERDICT:") {
            verdict = v.trim().to_lowercase();
            in_rationale = false;
        } else if let Some(r) = line.strip_prefix("RATIONALE:") {
            rationale_lines.push(r.trim().to_string());
            in_rationale = true;
        } else if in_rationale && !line.trim().is_empty() {
            rationale_lines.push(line.to_string());
        }
    }

    let rationale = if rationale_lines.is_empty() {
        None
    } else {
        Some(rationale_lines.join(" ").trim().to_string())
    };

    (verdict, rationale)
}


/// Load the developer profile from ~/.config/pm/profile.md.
/// Falls back to a formatted list of active project names + USPs.
pub fn load_profile(fallback_projects: Option<&[(String, Option<String>)]>) -> String {
    let profile_path = profile_path();
    if let Ok(content) = std::fs::read_to_string(&profile_path) {
        if !content.trim().is_empty() {
            return content;
        }
    }
    match fallback_projects {
        None | Some([]) => "No profile available.".to_string(),
        Some(projects) => {
            let lines: Vec<String> = projects.iter().map(|(name, usp)| {
                match usp {
                    Some(u) => format!("- {}: {}", name, u),
                    None => format!("- {}", name),
                }
            }).collect();
            format!("Active projects (as skills proxy):\n{}", lines.join("\n"))
        }
    }
}

fn profile_path() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("PM_PROFILE_PATH") {
        return std::path::PathBuf::from(val);
    }
    dirs::config_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("pm")
        .join("profile.md")
}

#[derive(Debug, Clone)]
pub struct PivotIdea {
    pub name: String,
    pub usp: String,
    pub gap: String,
    pub fit: String,
}

pub fn parse_pivot_ideas(output: &str) -> Vec<PivotIdea> {
    let mut ideas = Vec::new();

    let blocks: Vec<&str> = output.split("---").collect();

    for block in blocks {
        let block = block.trim();
        if block.is_empty() { continue; }

        let mut name = String::new();
        let mut usp = String::new();
        let mut gap = String::new();
        let mut fit = String::new();

        for line in block.lines() {
            if let Some(v) = line.strip_prefix("NAME:") { name = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("USP:") { usp = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("GAP:") { gap = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("FIT:") { fit = v.trim().to_string(); }
        }

        if !name.is_empty() && !usp.is_empty() {
            ideas.push(PivotIdea { name, usp, gap, fit });
        }
    }

    ideas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_research_due_never_researched() {
        assert!(is_research_due(None, &ResearchFrequency::Weekly));
    }

    #[test]
    fn test_is_research_due_recent() {
        let today = chrono::Local::now().date_naive().to_string();
        assert!(!is_research_due(Some(&today), &ResearchFrequency::Weekly));
    }

    #[test]
    fn test_is_research_due_overdue() {
        let old = "2020-01-01";
        assert!(is_research_due(Some(old), &ResearchFrequency::Weekly));
    }

    #[test]
    fn test_is_research_due_never_frequency() {
        assert!(!is_research_due(None, &ResearchFrequency::Never));
    }

    #[test]
    fn test_detect_cut_losses_positive() {
        assert!(detect_cut_losses("You should consider stopping this project."));
        assert!(detect_cut_losses("Recommendation: cut losses and move on."));
    }

    #[test]
    fn test_detect_cut_losses_negative() {
        assert!(!detect_cut_losses("Looks promising, continue building."));
    }

    #[test]
    fn test_frequency_days() {
        assert_eq!(ResearchFrequency::Weekly.days(), 7);
        assert_eq!(ResearchFrequency::Monthly.days(), 30);
    }

    #[test]
    fn test_parse_verdict_pass() {
        let output = "VERDICT: pass\nRATIONALE: The test confirms exit code behaviour.";
        let (v, r) = parse_verdict(output);
        assert_eq!(v, "pass");
        assert!(r.unwrap().contains("exit code"));
    }

    #[test]
    fn test_parse_verdict_fail() {
        let output = "VERDICT: fail\nRATIONALE: No test found for this scenario.";
        let (v, _) = parse_verdict(output);
        assert_eq!(v, "fail");
    }

    #[test]
    fn test_parse_verdict_defaults_to_inconclusive() {
        let (v, _) = parse_verdict("No structured response here.");
        assert_eq!(v, "inconclusive");
    }

    #[test]
    fn test_fallback_uses_codex_when_claude_fails() {
        let mut seen = Vec::new();
        let result = run_with_fallback_using("prompt", &["Read"], |provider| {
            seen.push(provider);
            match provider {
                Provider::Claude => Err("claude CLI failed: out of credits".to_string()),
                Provider::Codex => Ok("ok from codex".to_string()),
            }
        })
        .expect("codex fallback should succeed");

        assert_eq!(result, "ok from codex");
        assert_eq!(seen, vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn test_fallback_keeps_claude_output_when_claude_succeeds() {
        let mut seen = Vec::new();
        let result = run_with_fallback_using("prompt", &["Read"], |provider| {
            seen.push(provider);
            match provider {
                Provider::Claude => Ok("ok from claude".to_string()),
                Provider::Codex => Ok("ok from codex".to_string()),
            }
        })
        .expect("claude should succeed");

        assert_eq!(result, "ok from claude");
        assert_eq!(seen, vec![Provider::Claude]);
    }

    #[test]
    fn test_fallback_errors_when_both_providers_fail() {
        let result = run_with_fallback_using("prompt", &["Read"], |provider| match provider {
            Provider::Claude => Err("claude boom".to_string()),
            Provider::Codex => Err("codex boom".to_string()),
        })
        .expect_err("both providers fail");

        assert!(result.contains("claude failed"));
        assert!(result.contains("codex fallback failed"));
    }

    #[test]
    fn test_codex_timeout_uses_env_when_set() {
        unsafe {
            std::env::set_var("PM_CODEX_TIMEOUT_SECS", "2");
        }
        assert_eq!(codex_timeout().as_secs(), 2);
        unsafe {
            std::env::remove_var("PM_CODEX_TIMEOUT_SECS");
        }
    }

    #[test]
    fn test_load_profile_reads_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let profile_path = tmp.path().join("profile.md");
        std::fs::write(&profile_path, "Languages: Rust\nDomains: gamedev\n").unwrap();
        unsafe { std::env::set_var("PM_PROFILE_PATH", &profile_path); }
        let profile = load_profile(None);
        unsafe { std::env::remove_var("PM_PROFILE_PATH"); }
        assert!(profile.contains("Rust"));
    }

    #[test]
    fn test_load_profile_falls_back_to_projects() {
        unsafe { std::env::set_var("PM_PROFILE_PATH", "/tmp/nonexistent-profile-xyz.md"); }
        let fallback = vec![
            ("patchwaste".to_string(), Some("CI check for Steam devs.".to_string())),
        ];
        let profile = load_profile(Some(&fallback));
        unsafe { std::env::remove_var("PM_PROFILE_PATH"); }
        assert!(profile.contains("patchwaste"));
    }

    #[test]
    fn test_parse_pivot_ideas_single() {
        let output = "---\nNAME: Constituency Engine\nUSP: Procedural UK constituencies for game devs.\nGAP: Competitors built full games, not engine layers.\nFIT: Rust library suits your gamedev background.\n---\n";
        let ideas = parse_pivot_ideas(output);
        assert_eq!(ideas.len(), 1);
        assert_eq!(ideas[0].name, "Constituency Engine");
        assert!(ideas[0].usp.contains("Procedural"));
        assert!(ideas[0].gap.contains("engine layers"));
        assert!(ideas[0].fit.contains("Rust"));
    }

    #[test]
    fn test_parse_pivot_ideas_multiple() {
        let output = "---\nNAME: Idea One\nUSP: Does A.\nGAP: Gap B.\nFIT: Fit C.\n---\nNAME: Idea Two\nUSP: Does D.\nGAP: Gap E.\nFIT: Fit F.\n---\n";
        let ideas = parse_pivot_ideas(output);
        assert_eq!(ideas.len(), 2);
        assert_eq!(ideas[0].name, "Idea One");
        assert_eq!(ideas[1].name, "Idea Two");
    }

    #[test]
    fn test_parse_pivot_ideas_empty() {
        let ideas = parse_pivot_ideas("No structured output here.");
        assert!(ideas.is_empty());
    }
}
