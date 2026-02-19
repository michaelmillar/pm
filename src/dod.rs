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
}
