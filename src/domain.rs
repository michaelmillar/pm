#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Oss,
    Research,
    Game,
    Webapp,
    Study,
}

impl ProjectType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "oss" => ProjectType::Oss,
            "research" => ProjectType::Research,
            "game" => ProjectType::Game,
            "webapp" => ProjectType::Webapp,
            "study" => ProjectType::Study,
            _ => ProjectType::Oss,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Oss => "oss",
            ProjectType::Research => "research",
            ProjectType::Game => "game",
            ProjectType::Webapp => "webapp",
            ProjectType::Study => "study",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            ProjectType::Oss => "O",
            ProjectType::Research => "R",
            ProjectType::Game => "G",
            ProjectType::Webapp => "W",
            ProjectType::Study => "S",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            ProjectType::Oss => "Tool",
            ProjectType::Research => "Research",
            ProjectType::Game => "Game",
            ProjectType::Webapp => "Webapp",
            ProjectType::Study => "Study",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectState {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectAction {
    Push,
    Pivot,
    Kill,
    Groom,
    Integrate(String),
    Sustain,
    Repurpose,
    Observe,
}

#[derive(Debug, Clone)]
pub struct Thresholds {
    pub kill_fit: u8,
    pub kill_vel: u8,
    pub kill_sunk: i32,
    pub pivot_fit: u8,
    pub pivot_vel: u8,
    pub groom_fit: u8,
    pub groom_vel: u8,
    pub push_fit: u8,
    pub push_vel: u8,
    pub sustain_fit: u8,
    pub integrate_dist: u8,
    pub repurpose_lev: u8,
    pub repurpose_sunk: i32,
    pub ship_stage: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            kill_fit: 3,
            kill_vel: 3,
            kill_sunk: 30,
            pivot_fit: 3,
            pivot_vel: 5,
            groom_fit: 6,
            groom_vel: 3,
            push_fit: 6,
            push_vel: 6,
            sustain_fit: 6,
            integrate_dist: 3,
            repurpose_lev: 3,
            repurpose_sunk: 60,
            ship_stage: 4,
        }
    }
}

impl ProjectAction {
    pub fn label(&self) -> &str {
        match self {
            ProjectAction::Push => "PUSH",
            ProjectAction::Pivot => "PIVOT",
            ProjectAction::Kill => "KILL",
            ProjectAction::Groom => "GROOM",
            ProjectAction::Integrate(_) => "INTEGRATE",
            ProjectAction::Sustain => "SUSTAIN",
            ProjectAction::Repurpose => "REPURPOSE",
            ProjectAction::Observe => "OBSERVE",
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            ProjectAction::Integrate(name) => Some(name),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub state: ProjectState,
    pub project_type: ProjectType,
    pub stage: u8,
    pub velocity: Option<u8>,
    pub fit_signal: Option<u8>,
    pub distinctness: Option<u8>,
    pub leverage: Option<u8>,
    pub sunk_cost_days: Option<i32>,
    pub pivot_count: u32,
    pub last_activity: chrono::NaiveDate,
    pub created_at: chrono::NaiveDate,
    pub soft_deadline: Option<chrono::NaiveDate>,
    pub path: Option<String>,
    pub deleted_at: Option<chrono::NaiveDate>,
    pub duplicate_of: Option<i64>,
    pub possible_duplicate_score: Option<f32>,
    pub research_summary: Option<String>,
    pub inbox_note: Option<String>,
    pub next_task: Option<String>,
}

impl Project {
    pub fn stage_contribution(&self) -> i32 {
        self.stage as i32 * 20
    }

    pub fn mean_axes(&self) -> f32 {
        let axes = [self.velocity, self.fit_signal, self.distinctness, self.leverage];
        let (sum, count) = axes.iter().fold((0u32, 0u32), |(s, c), ax| {
            match ax {
                Some(v) => (s + *v as u32, c + 1),
                None => (s, c),
            }
        });
        if count == 0 { 0.0 } else { sum as f32 / count as f32 }
    }

    pub fn axis_values(&self) -> [Option<u8>; 4] {
        [self.velocity, self.fit_signal, self.distinctness, self.leverage]
    }

    pub fn axis_coverage(&self) -> f32 {
        let count = [self.velocity, self.fit_signal, self.distinctness, self.leverage]
            .iter()
            .filter(|a| a.is_some())
            .count();
        count as f32 / 4.0
    }

    pub fn priority_score(&self, today: chrono::NaiveDate) -> i32 {
        let stage_base = (self.stage_contribution() as f32 * self.axis_coverage()) as i32;
        let base = stage_base + self.mean_axes() as i32;
        (base - self.staleness_penalty(today)).max(0)
    }

    pub fn staleness_penalty(&self, today: chrono::NaiveDate) -> i32 {
        if self.stage >= 2 {
            return 0;
        }
        let days = (today - self.last_activity).num_days().max(0) as i32;
        let raw = (days - 30).max(0) / 7;
        raw.min(10)
    }

    pub fn action_recommendation(&self, nearest_neighbour: Option<&str>) -> ProjectAction {
        self.action_with_thresholds(&Thresholds::default(), nearest_neighbour)
    }

    pub fn action_with_thresholds(&self, t: &Thresholds, nearest_neighbour: Option<&str>) -> ProjectAction {
        let fit = self.fit_signal;
        let vel = self.velocity;
        let dist = self.distinctness;
        let lev = self.leverage;
        let sunk = self.sunk_cost_days.unwrap_or(0);

        if let (Some(f), Some(v)) = (fit, vel) {
            if f < t.kill_fit && v < t.kill_vel && sunk > t.kill_sunk {
                return ProjectAction::Kill;
            }
            if f < t.pivot_fit && v >= t.pivot_vel {
                return ProjectAction::Pivot;
            }
            if f >= t.groom_fit && v < t.groom_vel && self.stage < t.ship_stage {
                return ProjectAction::Groom;
            }
            if f >= t.push_fit && v >= t.push_vel && self.stage < t.ship_stage {
                return ProjectAction::Push;
            }
        }
        if let Some(f) = fit {
            if f >= t.sustain_fit && self.stage >= t.ship_stage {
                return ProjectAction::Sustain;
            }
        }
        if let Some(d) = dist {
            if d < t.integrate_dist {
                let target = nearest_neighbour.unwrap_or("unknown").to_string();
                return ProjectAction::Integrate(target);
            }
        }
        if let (Some(l), true) = (lev, sunk > t.repurpose_sunk) {
            if l < t.repurpose_lev {
                return ProjectAction::Repurpose;
            }
        }
        if fit.is_none() {
            if let Some(v) = vel {
                if v >= t.push_vel && self.stage < t.ship_stage {
                    return ProjectAction::Push;
                }
                if v < t.groom_vel && self.stage >= 2 {
                    return ProjectAction::Groom;
                }
            }
        }
        ProjectAction::Observe
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub last_commit_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Default)]
pub struct RepoSignals {
    pub has_src: bool,
    pub has_readme: bool,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_tags: bool,
    pub tag_count: usize,
    pub has_license: bool,
    pub has_changelog: bool,
    pub has_cargo_toml: bool,
    pub has_package_json: bool,
    pub has_game_engine: bool,
    pub has_notebooks: bool,
    pub has_webapp_framework: bool,
    pub contributor_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_project(stage: u8, velocity: Option<u8>, fit: Option<u8>, dist: Option<u8>, lev: Option<u8>) -> Project {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        Project {
            id: 1,
            name: "test".to_string(),
            state: ProjectState::Active,
            project_type: ProjectType::Oss,
            stage,
            velocity,
            fit_signal: fit,
            distinctness: dist,
            leverage: lev,
            sunk_cost_days: Some(45),
            pivot_count: 0,
            last_activity: today,
            created_at: today - chrono::Duration::days(90),
            soft_deadline: None,
            path: None,
            deleted_at: None,
            duplicate_of: None,
            possible_duplicate_score: None,
            research_summary: None,
            inbox_note: None,
            next_task: None,
        }
    }

    #[test]
    fn stage_contribution_is_stage_times_twenty() {
        for s in 0..=5 {
            let p = make_project(s, None, None, None, None);
            assert_eq!(p.stage_contribution(), s as i32 * 20);
        }
    }

    #[test]
    fn mean_axes_averages_non_none() {
        let p = make_project(0, Some(8), None, Some(6), None);
        assert!((p.mean_axes() - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mean_axes_all_none_returns_zero() {
        let p = make_project(0, None, None, None, None);
        assert!((p.mean_axes()).abs() < f32::EPSILON);
    }

    #[test]
    fn priority_score_is_stage_plus_axes() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let p = make_project(3, Some(8), Some(6), Some(7), Some(9));
        assert_eq!(p.priority_score(today), 67);
    }

    #[test]
    fn staleness_penalty_zero_above_stage_two() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let mut p = make_project(2, None, None, None, None);
        p.last_activity = today - chrono::Duration::days(100);
        assert_eq!(p.staleness_penalty(today), 0);
    }

    #[test]
    fn staleness_penalty_kicks_in_below_stage_two() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let mut p = make_project(1, None, None, None, None);
        p.last_activity = today - chrono::Duration::days(44);
        assert_eq!(p.staleness_penalty(today), 2);
    }

    #[test]
    fn staleness_penalty_capped_at_ten() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let mut p = make_project(0, None, None, None, None);
        p.last_activity = today - chrono::Duration::days(365);
        assert_eq!(p.staleness_penalty(today), 10);
    }

    #[test]
    fn action_kill_when_low_fit_low_velocity_high_sunk() {
        let mut p = make_project(1, Some(2), Some(1), Some(8), Some(5));
        p.sunk_cost_days = Some(60);
        assert_eq!(p.action_recommendation(None), ProjectAction::Kill);
    }

    #[test]
    fn action_pivot_when_low_fit_but_active() {
        let p = make_project(1, Some(7), Some(2), Some(8), Some(5));
        assert_eq!(p.action_recommendation(None), ProjectAction::Pivot);
    }

    #[test]
    fn action_integrate_when_low_distinctness() {
        let p = make_project(2, Some(5), Some(5), Some(2), Some(5));
        assert_eq!(
            p.action_recommendation(Some("ward")),
            ProjectAction::Integrate("ward".to_string())
        );
    }

    #[test]
    fn action_push_when_high_fit_high_velocity_pre_ship() {
        let p = make_project(2, Some(8), Some(7), Some(8), Some(6));
        assert_eq!(p.action_recommendation(None), ProjectAction::Push);
    }

    #[test]
    fn action_groom_when_high_fit_low_velocity_pre_ship() {
        let p = make_project(3, Some(1), Some(8), Some(8), Some(6));
        assert_eq!(p.action_recommendation(None), ProjectAction::Groom);
    }

    #[test]
    fn action_sustain_when_high_fit_post_ship() {
        let p = make_project(4, Some(3), Some(7), Some(8), Some(6));
        assert_eq!(p.action_recommendation(None), ProjectAction::Sustain);
    }

    #[test]
    fn action_repurpose_when_low_leverage_high_sunk() {
        let mut p = make_project(1, Some(5), Some(5), Some(8), Some(2));
        p.sunk_cost_days = Some(90);
        assert_eq!(p.action_recommendation(None), ProjectAction::Repurpose);
    }

    #[test]
    fn action_observe_default() {
        let p = make_project(2, Some(4), Some(4), Some(7), Some(5));
        assert_eq!(p.action_recommendation(None), ProjectAction::Observe);
    }

    #[test]
    fn action_push_when_high_velocity_no_fit() {
        let p = make_project(1, Some(9), None, Some(8), Some(5));
        assert_eq!(p.action_recommendation(None), ProjectAction::Push);
    }

    #[test]
    fn action_groom_when_low_velocity_no_fit() {
        let p = make_project(2, Some(1), None, Some(8), Some(5));
        assert_eq!(p.action_recommendation(None), ProjectAction::Groom);
    }

    #[test]
    fn action_observe_when_all_none() {
        let p = make_project(0, None, None, None, None);
        assert_eq!(p.action_recommendation(None), ProjectAction::Observe);
    }

    #[test]
    fn coverage_factor_scales_stage_bonus() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let no_axes = make_project(3, None, None, None, None);
        assert_eq!(no_axes.priority_score(today), 0);

        let one_axis = make_project(3, Some(8), None, None, None);
        assert_eq!(one_axis.priority_score(today), 23);

        let two_axes = make_project(3, Some(8), Some(6), None, None);
        assert_eq!(two_axes.priority_score(today), 37);

        let all_axes = make_project(3, Some(8), Some(6), Some(7), Some(9));
        assert_eq!(all_axes.priority_score(today), 67);
    }

    #[test]
    fn priority_score_clamps_to_zero() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let mut p = make_project(0, None, None, None, None);
        p.last_activity = today - chrono::Duration::days(365);
        assert_eq!(p.priority_score(today), 0);
    }

    #[test]
    fn project_type_round_trips() {
        for t in [ProjectType::Oss, ProjectType::Research, ProjectType::Game, ProjectType::Webapp, ProjectType::Study] {
            assert_eq!(ProjectType::from_str(t.as_str()), t);
        }
    }
}
