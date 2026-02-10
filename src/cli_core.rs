use crate::domain::{ProjectState, ScanResult};
use chrono::NaiveDate;
use crate::store::Store;

#[derive(Debug, PartialEq)]
pub enum ScoreResult {
    Updated { id: i64 },
    NotFound { id: i64 },
    Invalid { field: &'static str },
}

#[derive(Debug, PartialEq)]
pub struct AutoScore {
    pub impact: u8,
    pub monetization: u8,
    pub readiness: u8,
}

pub fn auto_score(scan: &ScanResult, created_at: NaiveDate, today: NaiveDate) -> AutoScore {
    let age_days = (today - created_at).num_days().max(0) as u32;
    let age_boost = (age_days / 90) as i32;

    let readiness = if scan.total_tasks == 0 {
        if scan.last_commit_date.is_some() { 20 } else { 5 }
    } else {
        let ratio = (scan.completed_tasks as f32 / scan.total_tasks as f32) * 100.0;
        ratio.round() as i32
    };

    let planning = if scan.plan_files.is_empty() { 0 } else { 2 };
    let charter = match scan.charter_filled {
        Some((filled, total)) if total > 0 => {
            ((filled as f32 / total as f32) * 3.0).round() as i32
        }
        _ => 0,
    };

    let impact = (5 + planning + charter + age_boost).clamp(1, 10) as u8;
    let monetization = (5 + planning + age_boost).clamp(1, 10) as u8;
    let readiness = readiness.clamp(0, 100) as u8;

    AutoScore {
        impact,
        monetization,
        readiness,
    }
}

pub fn score_project(
    store: &Store,
    id: i64,
    impact: u8,
    monetization: u8,
    readiness: u8,
) -> rusqlite::Result<ScoreResult> {
    if !(1..=10).contains(&impact) {
        return Ok(ScoreResult::Invalid { field: "impact" });
    }
    if !(1..=10).contains(&monetization) {
        return Ok(ScoreResult::Invalid {
            field: "monetization",
        });
    }
    if readiness > 100 {
        return Ok(ScoreResult::Invalid { field: "readiness" });
    }

    let updated = store.update_scores(id, impact, monetization, readiness)?;
    if updated == 0 {
        return Ok(ScoreResult::NotFound { id });
    }
    store.update_state(id, ProjectState::Active)?;
    Ok(ScoreResult::Updated { id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn test_score_project_not_found() {
        let store = Store::open_in_memory().unwrap();
        let result = score_project(&store, 999, 5, 5, 50).unwrap();
        assert!(matches!(result, ScoreResult::NotFound { .. }));
    }

    #[test]
    fn test_score_project_invalid_readiness() {
        let store = Store::open_in_memory().unwrap();
        let result = score_project(&store, 1, 5, 5, 200).unwrap();
        assert!(matches!(
            result,
            ScoreResult::Invalid { field: "readiness" }
        ));
    }
}
