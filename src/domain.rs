#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Product,
    Game,
    Webapp,
    OpenCore,
    Study,
    Library,
    Blog,
}

impl ProjectType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "game" => ProjectType::Game,
            "webapp" => ProjectType::Webapp,
            "open-core" => ProjectType::OpenCore,
            "study" => ProjectType::Study,
            "library" => ProjectType::Library,
            "blog" => ProjectType::Blog,
            _ => ProjectType::Product,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Product => "product",
            ProjectType::Game => "game",
            ProjectType::Webapp => "webapp",
            ProjectType::OpenCore => "open-core",
            ProjectType::Study => "study",
            ProjectType::Library => "library",
            ProjectType::Blog => "blog",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            ProjectType::Product => "P",
            ProjectType::Game => "G",
            ProjectType::Webapp => "W",
            ProjectType::OpenCore => "OC",
            ProjectType::Study => "S",
            ProjectType::Library => "L",
            ProjectType::Blog => "B",
        }
    }

    pub fn weights(&self) -> (i32, i32, i32) {
        match self {
            ProjectType::Product => (3, 2, 2),
            ProjectType::Game =>    (3, 2, 3),
            ProjectType::Webapp =>  (3, 2, 2),
            ProjectType::OpenCore =>(3, 1, 3),
            ProjectType::Study =>   (3, 0, 0),
            ProjectType::Library => (3, 0, 2),
            ProjectType::Blog =>    (2, 0, 0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectState {
    Inbox,
    Active,
    Parked,
    Shipped,
    Killed,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub state: ProjectState,
    pub impact: u8,
    pub monetization: u8,
    pub readiness: u8,
    pub last_activity: chrono::NaiveDate,
    pub created_at: chrono::NaiveDate,
    pub soft_deadline: Option<chrono::NaiveDate>,
    pub path: Option<String>,
    pub deleted_at: Option<chrono::NaiveDate>,
    pub duplicate_of: Option<i64>,
    pub possible_duplicate_score: Option<f32>,
    pub cloneability: Option<u8>,
    pub uniqueness: Option<u8>,
    pub defensibility: Option<u8>,
    pub project_type: ProjectType,
    pub vibe: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub last_commit_date: Option<chrono::NaiveDate>,
    pub plan_files: Vec<String>,
    pub has_progress_file: bool,
    pub charter_filled: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskSource {
    Manual,
    Git,
    Pending,
}

#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub plan_file: String,
    pub task_number: usize,
    pub description: String,
    pub source: TaskSource,
}

impl Project {
    pub fn effective_defensibility(&self) -> u8 {
        if let Some(d) = self.defensibility {
            return d;
        }
        match (self.uniqueness, self.cloneability) {
            (Some(u), Some(c)) => u.max(c),
            (Some(u), None) => u,
            (None, Some(c)) => c,
            (None, None) => 5,
        }
    }

    /// Effective impact: blends research impact with personal vibe.
    /// If both set, averages them. If only one, uses that. Default 5.
    pub fn effective_impact(&self) -> u8 {
        match (self.impact, self.vibe) {
            (i, Some(v)) if i != 5 => ((i as u16 + v as u16 + 1) / 2) as u8,
            (i, None) if i != 5 => i,
            (_, Some(v)) => v,
            _ => 5,
        }
    }

    pub fn priority_score(&self, today: chrono::NaiveDate) -> i32 {
        let staleness_days = (today - self.last_activity).num_days() as i32;
        let staleness_penalty = staleness_days.min(30);
        let (impact_w, monet_w, def_w) = self.project_type.weights();

        (self.effective_impact() as i32 * impact_w)
            + (self.monetization as i32 * monet_w)
            + (self.effective_defensibility() as i32 * def_w)
            + (self.readiness as i32 / 10 * 4)
            - staleness_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_project(impact: u8, monetization: u8, readiness: u8, days_stale: i64) -> Project {
        let today = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
        Project {
            id: 1,
            name: "test".to_string(),
            state: ProjectState::Active,
            impact,
            monetization,
            readiness,
            uniqueness: None,
            last_activity: today - chrono::Duration::days(days_stale),
            created_at: today - chrono::Duration::days(30),
            soft_deadline: None,
            path: None,
            deleted_at: None,
            duplicate_of: None,
            possible_duplicate_score: None,
            cloneability: None,
            defensibility: None,
            project_type: ProjectType::Product,
            vibe: None,
        }
    }

    #[test]
    fn test_priority_score_weights_readiness_highest() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
        let high_readiness = make_project(5, 5, 90, 0);
        let low_readiness = make_project(5, 5, 20, 0);

        assert!(high_readiness.priority_score(today) > low_readiness.priority_score(today));
    }

    #[test]
    fn test_priority_score_penalizes_staleness() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
        let fresh = make_project(5, 5, 50, 0);
        let stale = make_project(5, 5, 50, 10);

        assert!(fresh.priority_score(today) > stale.priority_score(today));
    }

    #[test]
    fn test_priority_score_caps_staleness_penalty() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
        let very_stale = make_project(5, 5, 50, 100);

        let score = very_stale.priority_score(today);
        assert!(score > 0);
    }

    fn make_project_def(impact: u8, monetization: u8, readiness: u8, days_stale: i64, defensibility: Option<u8>) -> Project {
        let today = NaiveDate::from_ymd_opt(2026, 2, 20).unwrap();
        Project {
            id: 1,
            name: "test".to_string(),
            state: ProjectState::Active,
            impact,
            monetization,
            readiness,
            uniqueness: None,
            last_activity: today - chrono::Duration::days(days_stale),
            created_at: today - chrono::Duration::days(30),
            soft_deadline: None,
            path: None,
            deleted_at: None,
            duplicate_of: None,
            possible_duplicate_score: None,
            cloneability: None,
            defensibility,
            project_type: ProjectType::Product,
            vibe: None,
        }
    }

    #[test]
    fn test_priority_score_includes_defensibility() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 20).unwrap();
        let high_def = make_project_def(5, 5, 50, 0, Some(8));
        let low_def  = make_project_def(5, 5, 50, 0, Some(2));
        assert!(high_def.priority_score(today) > low_def.priority_score(today));
    }

    #[test]
    fn test_priority_score_ignores_monetisation_for_study() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
        let product = make_project(5, 10, 50, 0);
        let mut study = make_project(5, 10, 50, 0);
        study.project_type = ProjectType::Study;
        assert!(product.priority_score(today) > study.priority_score(today));
    }
}
