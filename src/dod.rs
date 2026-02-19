// Definition of Done — file parser, writer, and rollup logic.

use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub enum CriterionStatus {
    Pending,
    Pass { date: NaiveDate, rationale: Option<String> },
    Fail { date: NaiveDate, rationale: Option<String> },
    Inconclusive { date: NaiveDate, rationale: Option<String> },
}

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: String,
    pub description: String,
    pub evidence: Option<String>,
    pub scenario: String,
    pub automated: CriterionStatus,
    pub human: CriterionStatus,
}

#[derive(Debug, Clone)]
pub struct DodFile {
    pub project_name: String,
    pub usp: String,
    pub criteria: Vec<Criterion>,
}

impl CriterionStatus {
    pub fn is_done(&self) -> bool {
        matches!(self, CriterionStatus::Pass { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            CriterionStatus::Pending => "pending",
            CriterionStatus::Pass { .. } => "pass",
            CriterionStatus::Fail { .. } => "fail",
            CriterionStatus::Inconclusive { .. } => "inconclusive",
        }
    }
}

/// Parse a status line value like "pending", "pass — 2026-02-19", "fail — 2026-02-19"
pub fn parse_status(value: &str) -> CriterionStatus {
    let value = value.trim();
    if value == "pending" {
        return CriterionStatus::Pending;
    }

    // "pass — 2026-02-19" or "pass — 2026-02-19\n> rationale"
    let (keyword, rest) = if let Some(pos) = value.find(" — ") {
        (&value[..pos], value[pos + " — ".len()..].trim())
    } else if let Some(pos) = value.find(" - ") {
        (&value[..pos], value[pos + " - ".len()..].trim())
    } else {
        return CriterionStatus::Pending;
    };

    let date = NaiveDate::parse_from_str(rest, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());

    match keyword {
        "pass" => CriterionStatus::Pass { date, rationale: None },
        "fail" => CriterionStatus::Fail { date, rationale: None },
        "inconclusive" => CriterionStatus::Inconclusive { date, rationale: None },
        _ => CriterionStatus::Pending,
    }
}

pub fn parse_dod(content: &str) -> Result<DodFile, String> {
    let mut project_name = String::new();
    let mut usp_lines: Vec<String> = Vec::new();
    let mut criteria: Vec<Criterion> = Vec::new();

    #[derive(PartialEq)]
    enum State {
        Header,
        Usp,
        CriterionBody,
        Scenario,
        Done,
    }

    let mut state = State::Header;
    let mut current: Option<CriterionInProgress> = None;

    struct CriterionInProgress {
        id: String,
        description: String,
        evidence: Option<String>,
        scenario_lines: Vec<String>,
        automated_raw: String,
        automated_rationale: Vec<String>,
        human_raw: String,
        human_rationale: Vec<String>,
        in_automated_rationale: bool,
        in_human_rationale: bool,
    }

    impl CriterionInProgress {
        fn new(id: &str, description: &str) -> Self {
            Self {
                id: id.to_string(),
                description: description.to_string(),
                evidence: None,
                scenario_lines: Vec::new(),
                automated_raw: "pending".to_string(),
                automated_rationale: Vec::new(),
                human_raw: "pending".to_string(),
                human_rationale: Vec::new(),
                in_automated_rationale: false,
                in_human_rationale: false,
            }
        }
        fn to_criterion(self) -> Criterion {
            let rationale_str = |lines: Vec<String>| -> Option<String> {
                if lines.is_empty() { None } else { Some(lines.join("\n")) }
            };
            let mut auto_status = parse_status(&self.automated_raw);
            let mut human_status = parse_status(&self.human_raw);
            // attach rationale
            if let Some(r) = rationale_str(self.automated_rationale) {
                match &mut auto_status {
                    CriterionStatus::Pass { rationale, .. }
                    | CriterionStatus::Fail { rationale, .. }
                    | CriterionStatus::Inconclusive { rationale, .. } => *rationale = Some(r),
                    _ => {}
                }
            }
            if let Some(r) = rationale_str(self.human_rationale) {
                match &mut human_status {
                    CriterionStatus::Pass { rationale, .. }
                    | CriterionStatus::Fail { rationale, .. }
                    | CriterionStatus::Inconclusive { rationale, .. } => *rationale = Some(r),
                    _ => {}
                }
            }
            Criterion {
                id: self.id,
                description: self.description,
                evidence: self.evidence,
                scenario: self.scenario_lines.join("\n").trim().to_string(),
                automated: auto_status,
                human: human_status,
            }
        }
    }

    fn flush(current: &mut Option<CriterionInProgress>, criteria: &mut Vec<Criterion>) {
        if let Some(c) = current.take() {
            criteria.push(c.to_criterion());
        }
    }

    for line in content.lines() {
        // Project name from H1: "# example-app — Definition of Done"
        if project_name.is_empty() && line.starts_with("# ") {
            let title = &line[2..];
            project_name = if let Some(pos) = title.find(" — ") {
                title[..pos].trim().to_string()
            } else {
                title.trim().to_string()
            };
            continue;
        }

        // Section headings
        if line.starts_with("## USP") {
            state = State::Usp;
            continue;
        }

        // Criterion heading: "## [C1] Description"
        if line.starts_with("## [") {
            flush(&mut current, &mut criteria);
            if let Some(bracket_end) = line.find(']') {
                let id = line[4..bracket_end].to_string(); // "C1"
                let description = line[bracket_end + 1..].trim().to_string();
                current = Some(CriterionInProgress::new(&id, &description));
                state = State::CriterionBody;
            }
            continue;
        }

        if line == "---" {
            if state == State::Usp {
                state = State::Done;
            }
            continue;
        }

        match state {
            State::Usp => {
                if !line.trim().is_empty() {
                    usp_lines.push(line.to_string());
                }
            }
            State::CriterionBody | State::Scenario => {
                if let Some(ref mut c) = current {
                    // Evidence
                    if let Some(ev) = line.strip_prefix("**Evidence:**") {
                        c.evidence = Some(ev.trim().to_string());
                        c.in_automated_rationale = false;
                        c.in_human_rationale = false;
                        continue;
                    }
                    // Scenario start
                    if line.starts_with("**Scenario:**") {
                        state = State::Scenario;
                        c.in_automated_rationale = false;
                        c.in_human_rationale = false;
                        continue;
                    }
                    // Automated status
                    if let Some(val) = line.strip_prefix("**Automated:**") {
                        c.automated_raw = val.trim().to_string();
                        c.in_automated_rationale = true;
                        c.in_human_rationale = false;
                        state = State::CriterionBody;
                        continue;
                    }
                    // Human status
                    if let Some(val) = line.strip_prefix("**Human:**") {
                        c.human_raw = val.trim().to_string();
                        c.in_human_rationale = true;
                        c.in_automated_rationale = false;
                        state = State::CriterionBody;
                        continue;
                    }
                    // Rationale lines ("> text")
                    if let Some(r) = line.strip_prefix("> ") {
                        if c.in_human_rationale {
                            c.human_rationale.push(r.to_string());
                        } else if c.in_automated_rationale {
                            c.automated_rationale.push(r.to_string());
                        }
                        continue;
                    }
                    // Scenario body lines
                    if state == State::Scenario {
                        if !line.trim().is_empty() {
                            c.scenario_lines.push(line.to_string());
                        }
                        continue;
                    }
                    // Non-rationale content resets rationale mode
                    if !line.trim().is_empty() && !line.starts_with('>') {
                        c.in_automated_rationale = false;
                        c.in_human_rationale = false;
                    }
                }
            }
            _ => {}
        }
    }

    flush(&mut current, &mut criteria);

    Ok(DodFile {
        project_name,
        usp: usp_lines.join("\n").trim().to_string(),
        criteria,
    })
}

pub fn write_dod(dod: &DodFile) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} — Definition of Done\n", dod.project_name));
    out.push_str("\n## USP\n");
    out.push_str(&dod.usp);
    out.push_str("\n\n---\n");

    for criterion in &dod.criteria {
        out.push_str(&format!(
            "\n## [{}] {}\n\n",
            criterion.id, criterion.description
        ));
        if let Some(ref ev) = criterion.evidence {
            out.push_str(&format!("**Evidence:** {}\n\n", ev));
        }
        out.push_str("**Scenario:**\n");
        out.push_str(&criterion.scenario);
        out.push_str("\n\n");
        out.push_str(&format_status_line("Automated", &criterion.automated));
        out.push_str(&format_status_line("Human", &criterion.human));
    }

    out
}

fn format_status_line(label: &str, status: &CriterionStatus) -> String {
    let mut out = String::new();
    match status {
        CriterionStatus::Pending => {
            out.push_str(&format!("**{}:** pending\n", label));
        }
        CriterionStatus::Pass { date, rationale } => {
            out.push_str(&format!("**{}:** pass — {}\n", label, date));
            if let Some(r) = rationale {
                for line in r.lines() {
                    out.push_str(&format!("> {}\n", line));
                }
            }
        }
        CriterionStatus::Fail { date, rationale } => {
            out.push_str(&format!("**{}:** fail — {}\n", label, date));
            if let Some(r) = rationale {
                for line in r.lines() {
                    out.push_str(&format!("> {}\n", line));
                }
            }
        }
        CriterionStatus::Inconclusive { date, rationale } => {
            out.push_str(&format!("**{}:** inconclusive — {}\n", label, date));
            if let Some(r) = rationale {
                for line in r.lines() {
                    out.push_str(&format!("> {}\n", line));
                }
            }
        }
    }
    out
}

/// Returns (complete, total). A criterion is complete when both automated and human are Pass.
pub fn rollup(dod: &DodFile) -> (usize, usize) {
    let total = dod.criteria.len();
    let complete = dod.criteria.iter().filter(|c| {
        c.automated.is_done() && c.human.is_done()
    }).count();
    (complete, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_status_is_not_done() {
        assert!(!CriterionStatus::Pending.is_done());
    }

    #[test]
    fn test_pass_status_is_done() {
        let s = CriterionStatus::Pass {
            date: NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(),
            rationale: None,
        };
        assert!(s.is_done());
    }

    #[test]
    fn test_fail_status_is_not_done() {
        let s = CriterionStatus::Fail {
            date: NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(),
            rationale: None,
        };
        assert!(!s.is_done());
    }

    #[test]
    fn test_parse_status_pending() {
        assert_eq!(parse_status("pending"), CriterionStatus::Pending);
    }

    #[test]
    fn test_parse_status_pass() {
        let s = parse_status("pass — 2026-02-19");
        assert!(s.is_done());
        assert_eq!(s.label(), "pass");
    }

    #[test]
    fn test_parse_status_fail() {
        let s = parse_status("fail — 2026-02-19");
        assert_eq!(s.label(), "fail");
    }

    #[test]
    fn test_parse_status_inconclusive() {
        let s = parse_status("inconclusive — 2026-02-19");
        assert_eq!(s.label(), "inconclusive");
    }

    #[test]
    fn test_parse_status_unknown_falls_back_to_pending() {
        let s = parse_status("garbage text");
        assert_eq!(s, CriterionStatus::Pending);
    }

    #[test]
    fn test_parse_dod_full() {
        let content = r#"# example-app — Definition of Done

## USP
CI check that integrates into a Steam game dev's workflow.

---

## [C1] CI binary exits non-zero on high-impact changes

**Evidence:** `tests/integration/ci_exit_codes.rs`

**Scenario:**
Given a Steam game repo
When the example-app CI check runs
Then it exits non-zero

**Automated:** pending
**Human:** pending
"#;
        let dod = parse_dod(content).unwrap();
        assert_eq!(dod.project_name, "example-app");
        assert_eq!(dod.criteria.len(), 1);
        let c = &dod.criteria[0];
        assert_eq!(c.id, "C1");
        assert_eq!(c.description, "CI binary exits non-zero on high-impact changes");
        assert!(c.evidence.is_some());
        assert!(c.scenario.contains("Given a Steam game repo"));
        assert_eq!(c.automated, CriterionStatus::Pending);
        assert_eq!(c.human, CriterionStatus::Pending);
    }

    #[test]
    fn test_parse_dod_with_pass_status_and_rationale() {
        let content = r#"# wonk — Definition of Done

## USP
UK political simulator.

---

## [C1] Player can win an election

**Evidence:** `src/game/election.rs`

**Scenario:**
Given the player has high approval
When they call an election
Then they win

**Automated:** pass — 2026-02-19
> Confirmed in election_win_test.
**Human:** fail — 2026-02-19
> Felt unfair as a player.
"#;
        let dod = parse_dod(content).unwrap();
        let c = &dod.criteria[0];
        assert_eq!(c.automated.label(), "pass");
        assert_eq!(c.human.label(), "fail");
        if let CriterionStatus::Pass { rationale, .. } = &c.automated {
            assert!(rationale.as_deref().unwrap_or("").contains("Confirmed"));
        }
    }

    #[test]
    fn test_parse_dod_multiple_criteria() {
        let content = r#"# foo — Definition of Done

## USP
Does something.

---

## [C1] First criterion

**Evidence:** `src/a.rs`

**Scenario:**
Given X
When Y
Then Z

**Automated:** pending
**Human:** pending

---

## [C2] Second criterion

**Evidence:** `src/b.rs`

**Scenario:**
Given A
When B
Then C

**Automated:** pending
**Human:** pending
"#;
        let dod = parse_dod(content).unwrap();
        assert_eq!(dod.criteria.len(), 2);
        assert_eq!(dod.criteria[0].id, "C1");
        assert_eq!(dod.criteria[1].id, "C2");
    }

    #[test]
    fn test_write_dod_round_trip() {
        let content = r#"# example-app — Definition of Done

## USP
CI check for Steam devs.

---

## [C1] CI exits non-zero on high-impact changes

**Evidence:** `tests/integration/ci_exit_codes.rs`

**Scenario:**
Given a Steam game repo
When the check runs
Then it exits non-zero

**Automated:** pending
**Human:** pending
"#;
        let dod = parse_dod(content).unwrap();
        let written = write_dod(&dod);
        let reparsed = parse_dod(&written).unwrap();
        assert_eq!(reparsed.project_name, dod.project_name);
        assert_eq!(reparsed.criteria.len(), 1);
        assert_eq!(reparsed.criteria[0].id, "C1");
        assert_eq!(reparsed.criteria[0].automated, CriterionStatus::Pending);
    }

    #[test]
    fn test_write_dod_pass_with_rationale() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        let dod = DodFile {
            project_name: "test".to_string(),
            usp: "Does something useful.".to_string(),
            criteria: vec![Criterion {
                id: "C1".to_string(),
                description: "Feature works".to_string(),
                evidence: Some("`src/feature.rs`".to_string()),
                scenario: "Given X\nWhen Y\nThen Z".to_string(),
                automated: CriterionStatus::Pass {
                    date: today,
                    rationale: Some("Evidence found in src/feature.rs.".to_string()),
                },
                human: CriterionStatus::Pending,
            }],
        };
        let written = write_dod(&dod);
        assert!(written.contains("pass — 2026-02-19"));
        assert!(written.contains("> Evidence found in src/feature.rs."));
        assert!(written.contains("**Human:** pending"));
    }

    #[test]
    fn test_rollup_counts_complete() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        let dod = DodFile {
            project_name: "test".to_string(),
            usp: "x".to_string(),
            criteria: vec![
                Criterion {
                    id: "C1".to_string(),
                    description: "a".to_string(),
                    evidence: None,
                    scenario: "".to_string(),
                    automated: CriterionStatus::Pass { date: today, rationale: None },
                    human: CriterionStatus::Pass { date: today, rationale: None },
                },
                Criterion {
                    id: "C2".to_string(),
                    description: "b".to_string(),
                    evidence: None,
                    scenario: "".to_string(),
                    automated: CriterionStatus::Pass { date: today, rationale: None },
                    human: CriterionStatus::Pending,
                },
            ],
        };
        let (complete, total) = rollup(&dod);
        assert_eq!(total, 2);
        assert_eq!(complete, 1); // only C1 has both pass
    }
}
