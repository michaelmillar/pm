use crate::domain::{Project, Thresholds};
use crate::store::{StageEvent, PivotEvent};

pub struct CalibrationResult {
    pub thresholds: Thresholds,
    pub adjustments: Vec<String>,
    pub event_count: usize,
}

pub fn compute_thresholds(
    projects: &[Project],
    stage_events: &[StageEvent],
    pivot_events: &[PivotEvent],
) -> CalibrationResult {
    let defaults = Thresholds::default();
    let event_count = stage_events.len() + pivot_events.len();

    if event_count < 10 {
        return CalibrationResult {
            thresholds: defaults,
            adjustments: vec!["insufficient events (<10), using defaults".to_string()],
            event_count,
        };
    }

    let mut t = defaults.clone();
    let mut adjustments = Vec::new();

    let kill_misses = count_unpredicted_kills(&defaults, projects, stage_events);
    if kill_misses > 0 {
        let old = t.kill_sunk;
        t.kill_sunk = (t.kill_sunk - 5).max(14);
        if t.kill_sunk != old {
            adjustments.push(format!(
                "kill_sunk {} -> {} ({} projects archived without KILL recommendation)",
                old, t.kill_sunk, kill_misses
            ));
        }
    }

    let pivot_misses = count_unpredicted_pivots(&defaults, projects, pivot_events);
    if pivot_misses > 0 {
        let old = t.pivot_vel;
        t.pivot_vel = (t.pivot_vel - 1).max(2);
        if t.pivot_vel != old {
            adjustments.push(format!(
                "pivot_vel {} -> {} ({} pivots happened below old velocity threshold)",
                old, t.pivot_vel, pivot_misses
            ));
        }
    }

    let groom_misses = count_stalled_high_fit(&defaults, projects, stage_events);
    if groom_misses > 0 {
        let old = t.groom_fit;
        t.groom_fit = (t.groom_fit - 1).max(3);
        if t.groom_fit != old {
            adjustments.push(format!(
                "groom_fit {} -> {} ({} high-fit projects stalled without GROOM recommendation)",
                old, t.groom_fit, groom_misses
            ));
        }
    }

    if adjustments.is_empty() {
        adjustments.push("thresholds match historical decisions, no changes".to_string());
    }

    CalibrationResult {
        thresholds: t,
        adjustments,
        event_count,
    }
}

fn count_unpredicted_kills(
    t: &Thresholds,
    projects: &[Project],
    events: &[StageEvent],
) -> usize {
    let archived_ids: Vec<i64> = events.iter()
        .filter(|e| e.reason.as_deref() == Some("archived") || e.to_stage == 0)
        .map(|e| e.project_id)
        .collect();

    projects.iter()
        .filter(|p| archived_ids.contains(&p.id))
        .filter(|p| {
            p.action_with_thresholds(t, None) != crate::domain::ProjectAction::Kill
        })
        .count()
}

fn count_unpredicted_pivots(
    t: &Thresholds,
    projects: &[Project],
    events: &[PivotEvent],
) -> usize {
    let pivoted_ids: Vec<i64> = events.iter()
        .map(|e| e.project_id)
        .collect();

    projects.iter()
        .filter(|p| pivoted_ids.contains(&p.id))
        .filter(|p| {
            p.action_with_thresholds(t, None) != crate::domain::ProjectAction::Pivot
        })
        .count()
}

fn count_stalled_high_fit(
    t: &Thresholds,
    projects: &[Project],
    events: &[StageEvent],
) -> usize {
    let promoted_ids: Vec<i64> = events.iter()
        .filter(|e| e.to_stage > e.from_stage)
        .map(|e| e.project_id)
        .collect();

    projects.iter()
        .filter(|p| {
            p.fit_signal.map(|f| f >= t.groom_fit).unwrap_or(false)
                && p.velocity.map(|v| v < t.groom_vel).unwrap_or(false)
                && !promoted_ids.contains(&p.id)
        })
        .filter(|p| {
            p.action_with_thresholds(t, None) != crate::domain::ProjectAction::Groom
        })
        .count()
}
