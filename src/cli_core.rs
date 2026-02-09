use crate::domain::ProjectState;
use crate::store::Store;

#[derive(Debug, PartialEq)]
pub enum ScoreResult {
    Updated { id: i64 },
    NotFound { id: i64 },
    Invalid { field: &'static str },
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
}
