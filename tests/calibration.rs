use pm::domain::{ProjectAction, Thresholds};
use pm::scoring::calibrate;
use pm::store::Store;

#[test]
fn list_stage_events_empty_for_fresh_db() {
    let store = Store::open_in_memory().unwrap();
    let id = store.add_project("Test").unwrap();
    let events = store.list_stage_events(id).unwrap();
    assert!(events.is_empty());
}

#[test]
fn list_stage_events_returns_recorded_events() {
    let store = Store::open_in_memory().unwrap();
    let id = store.add_project("Test").unwrap();
    store.record_stage_event(id, 0, 1, Some("first commit")).unwrap();
    store.record_stage_event(id, 1, 2, Some("prototype works")).unwrap();
    let events = store.list_stage_events(id).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].from_stage, 0);
    assert_eq!(events[0].to_stage, 1);
    assert_eq!(events[1].from_stage, 1);
    assert_eq!(events[1].to_stage, 2);
}

#[test]
fn list_all_stage_events_crosses_projects() {
    let store = Store::open_in_memory().unwrap();
    let a = store.add_project("A").unwrap();
    let b = store.add_project("B").unwrap();
    store.record_stage_event(a, 0, 1, None).unwrap();
    store.record_stage_event(b, 0, 2, None).unwrap();
    let events = store.list_all_stage_events().unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn list_all_pivot_events_returns_pivots() {
    let store = Store::open_in_memory().unwrap();
    let id = store.add_project("Pivoted").unwrap();
    store.update_stage(id, 3).unwrap();
    store.record_pivot_event(id, Some("wrong direction")).unwrap();
    let events = store.list_all_pivot_events().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reason, Some("wrong direction".to_string()));
}

fn make_scored_project(store: &Store, name: &str, vel: u8, fit: u8, stage: u8) -> i64 {
    let id = store.add_project(name).unwrap();
    store.update_axis(id, "velocity", Some(vel)).unwrap();
    store.update_axis(id, "fit_signal", Some(fit)).unwrap();
    store.update_stage(id, stage).unwrap();
    id
}

#[test]
fn default_thresholds_match_hardcoded_behaviour() {
    let store = Store::open_in_memory().unwrap();
    let id = make_scored_project(&store, "test", 8, 7, 2);
    let p = store.get_project(id).unwrap().unwrap();
    let default_action = p.action_recommendation(None);
    let threshold_action = p.action_with_thresholds(&Thresholds::default(), None);
    assert_eq!(default_action, threshold_action);
}

#[test]
fn custom_thresholds_change_recommendation() {
    let store = Store::open_in_memory().unwrap();
    let id = make_scored_project(&store, "edge", 4, 4, 1);
    let p = store.get_project(id).unwrap().unwrap();

    let default_action = p.action_recommendation(None);
    assert_eq!(default_action, ProjectAction::Observe);

    let mut t = Thresholds::default();
    t.pivot_fit = 5;
    t.pivot_vel = 4;
    let calibrated_action = p.action_with_thresholds(&t, None);
    assert_eq!(calibrated_action, ProjectAction::Pivot);
}

#[test]
fn calibrate_returns_defaults_with_few_events() {
    let store = Store::open_in_memory().unwrap();
    let id = make_scored_project(&store, "sparse", 5, 5, 2);
    store.record_stage_event(id, 0, 1, None).unwrap();

    let projects = store.list_active_projects().unwrap();
    let stage_events = store.list_all_stage_events().unwrap();
    let pivot_events = store.list_all_pivot_events().unwrap();

    let result = calibrate::compute_thresholds(&projects, &stage_events, &pivot_events);
    assert_eq!(result.thresholds.kill_sunk, Thresholds::default().kill_sunk);
    assert!(result.adjustments[0].contains("insufficient"));
}

#[test]
fn calibrate_adjusts_kill_sunk_from_missed_archives() {
    let store = Store::open_in_memory().unwrap();

    for i in 0..6 {
        let id = make_scored_project(&store, &format!("stale-{}", i), 1, 1, 1);
        store.update_sunk_cost(id, 25).unwrap();
        store.record_stage_event(id, 1, 0, Some("archived")).unwrap();
    }
    for i in 0..5 {
        let id = make_scored_project(&store, &format!("active-{}", i), 8, 7, 3);
        store.record_stage_event(id, 2, 3, None).unwrap();
    }

    let projects = store.list_active_projects().unwrap();
    let stage_events = store.list_all_stage_events().unwrap();
    let pivot_events = store.list_all_pivot_events().unwrap();

    let result = calibrate::compute_thresholds(&projects, &stage_events, &pivot_events);
    assert!(
        result.thresholds.kill_sunk < Thresholds::default().kill_sunk,
        "kill_sunk should decrease when projects were archived below threshold, got {}",
        result.thresholds.kill_sunk
    );
    assert!(result.adjustments.iter().any(|a| a.contains("kill_sunk")));
}

#[test]
fn calibrate_adjusts_pivot_vel_from_missed_pivots() {
    let store = Store::open_in_memory().unwrap();

    for i in 0..6 {
        let id = make_scored_project(&store, &format!("pivoted-{}", i), 4, 1, 2);
        store.record_stage_event(id, 2, 2, None).unwrap();
        store.record_pivot_event(id, Some("wrong direction")).unwrap();
    }
    for i in 0..5 {
        let id = make_scored_project(&store, &format!("steady-{}", i), 8, 8, 3);
        store.record_stage_event(id, 2, 3, None).unwrap();
    }

    let projects = store.list_active_projects().unwrap();
    let stage_events = store.list_all_stage_events().unwrap();
    let pivot_events = store.list_all_pivot_events().unwrap();

    let result = calibrate::compute_thresholds(&projects, &stage_events, &pivot_events);
    assert!(
        result.thresholds.pivot_vel < Thresholds::default().pivot_vel,
        "pivot_vel should decrease when pivots happened below threshold, got {}",
        result.thresholds.pivot_vel
    );
    assert!(result.adjustments.iter().any(|a| a.contains("pivot_vel")));
}

#[test]
fn thresholds_persist_and_load() {
    let store = Store::open_in_memory().unwrap();
    let mut t = Thresholds::default();
    t.kill_sunk = 20;
    t.pivot_vel = 3;
    store.save_thresholds(&t).unwrap();

    let loaded = store.load_thresholds().unwrap();
    assert_eq!(loaded.kill_sunk, 20);
    assert_eq!(loaded.pivot_vel, 3);
    assert_eq!(loaded.push_fit, Thresholds::default().push_fit);
}

#[test]
fn load_thresholds_returns_defaults_without_table() {
    let store = Store::open_in_memory().unwrap();
    let loaded = store.load_thresholds().unwrap();
    assert_eq!(loaded.kill_sunk, Thresholds::default().kill_sunk);
}

#[test]
fn calibrated_thresholds_affect_status_output() {
    let store = Store::open_in_memory().unwrap();
    let id = make_scored_project(&store, "edge-case", 4, 4, 1);
    let p = store.get_project(id).unwrap().unwrap();

    assert_eq!(p.action_recommendation(None), ProjectAction::Observe);

    let mut t = Thresholds::default();
    t.pivot_fit = 5;
    t.pivot_vel = 4;
    store.save_thresholds(&t).unwrap();

    let loaded = store.load_thresholds().unwrap();
    let action = p.action_with_thresholds(&loaded, None);
    assert_eq!(action, ProjectAction::Pivot);
}
