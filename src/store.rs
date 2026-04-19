use crate::domain::{Project, ProjectState, ProjectType};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result};

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct StageEvent {
    pub project_id: i64,
    pub from_stage: u8,
    pub to_stage: u8,
    pub occurred_at: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PivotEvent {
    pub project_id: i64,
    pub occurred_at: String,
    pub reason: Option<String>,
}

const PROJECT_COLUMNS: &str =
    "id, name, state, project_type, stage, velocity, fit_signal, distinctness, leverage, \
     sunk_cost_days, pivot_count, last_activity, created_at, soft_deadline, path, \
     deleted_at, duplicate_of, possible_duplicate_score, research_summary, inbox_note, next_task";

impl Store {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    #[doc(hidden)]
    pub fn conn_for_test(&self) -> &Connection {
        &self.conn
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'active',
                project_type TEXT NOT NULL DEFAULT 'oss',
                stage INTEGER NOT NULL DEFAULT 0,
                velocity INTEGER,
                fit_signal INTEGER,
                distinctness INTEGER,
                leverage INTEGER,
                sunk_cost_days INTEGER,
                pivot_count INTEGER NOT NULL DEFAULT 0,
                last_activity TEXT NOT NULL,
                created_at TEXT NOT NULL,
                soft_deadline TEXT,
                path TEXT,
                deleted_at TEXT,
                duplicate_of INTEGER,
                possible_duplicate_score REAL,
                research_summary TEXT,
                inbox_note TEXT,
                next_task TEXT
            );

            CREATE TABLE IF NOT EXISTS stage_events (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES projects(id),
                from_stage INTEGER NOT NULL,
                to_stage INTEGER NOT NULL,
                occurred_at TEXT NOT NULL,
                reason TEXT
            );

            CREATE TABLE IF NOT EXISTS pivot_events (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES projects(id),
                occurred_at TEXT NOT NULL,
                reason TEXT
            );"
        )?;

        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN project_type TEXT NOT NULL DEFAULT 'oss'", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN stage INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN velocity INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN fit_signal INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN distinctness INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN leverage INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN sunk_cost_days INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN pivot_count INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN path TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN deleted_at TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN duplicate_of INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN possible_duplicate_score REAL", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN soft_deadline TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN next_task TEXT", []);

        Ok(())
    }

    pub fn add_project(&self, name: &str) -> Result<i64> {
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "INSERT INTO projects (name, state, project_type, last_activity, created_at)
             VALUES (?1, 'active', 'oss', ?2, ?2)",
            params![name, today],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_project(&self, id: i64) -> Result<Option<Project>> {
        let sql = format!("SELECT {} FROM projects WHERE id = ?1", PROJECT_COLUMNS);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::row_to_project(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE path = ?1 AND deleted_at IS NULL",
            PROJECT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![path])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::row_to_project(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_or_create_by_path(&self, name: &str, path: &str) -> Result<i64> {
        if let Some(project) = self.get_project_by_path(path)? {
            return Ok(project.id);
        }
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "INSERT INTO projects (name, state, project_type, last_activity, created_at, path)
             VALUES (?1, 'active', 'oss', ?2, ?2, ?3)",
            params![name, today, path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_active_projects(&self) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE state = 'active' AND deleted_at IS NULL AND duplicate_of IS NULL ORDER BY name",
            PROJECT_COLUMNS
        );
        self.query_projects(&sql, [])
    }

    pub fn list_linked_projects(&self) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE path IS NOT NULL AND state = 'active' AND deleted_at IS NULL AND duplicate_of IS NULL ORDER BY name",
            PROJECT_COLUMNS
        );
        self.query_projects(&sql, [])
    }

    pub fn list_deleted_projects(&self) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
            PROJECT_COLUMNS
        );
        self.query_projects(&sql, [])
    }

    pub fn list_projects_for_dedupe(&self) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE deleted_at IS NULL AND duplicate_of IS NULL",
            PROJECT_COLUMNS
        );
        self.query_projects(&sql, [])
    }

    pub fn list_possible_duplicates(&self, min_score: f32) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE deleted_at IS NULL AND duplicate_of IS NULL AND possible_duplicate_score >= ?1 ORDER BY possible_duplicate_score DESC, name",
            PROJECT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut projects = Vec::new();
        let mut rows = stmt.query(params![min_score])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn list_archived_projects(&self) -> Result<Vec<Project>> {
        let sql = format!(
            "SELECT {} FROM projects WHERE state = 'archived' AND deleted_at IS NULL ORDER BY name",
            PROJECT_COLUMNS
        );
        self.query_projects(&sql, [])
    }

    fn query_projects(&self, sql: &str, params: impl rusqlite::Params) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(sql)?;
        let mut projects = Vec::new();
        let mut rows = stmt.query(params)?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn update_state(&self, id: i64, state: ProjectState) -> Result<usize> {
        let state_str = match state {
            ProjectState::Active => "active",
            ProjectState::Archived => "archived",
        };
        self.conn.execute(
            "UPDATE projects SET state = ?1 WHERE id = ?2",
            params![state_str, id],
        )
    }

    pub fn update_stage(&self, id: i64, stage: u8) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET stage = ?1 WHERE id = ?2",
            params![stage, id],
        )
    }

    pub fn update_axis(&self, id: i64, column: &str, value: Option<u8>) -> Result<usize> {
        const VALID: &[&str] = &["velocity", "fit_signal", "distinctness", "leverage"];
        if !VALID.contains(&column) {
            return Err(rusqlite::Error::InvalidParameterName(column.to_string()));
        }
        let sql = format!("UPDATE projects SET {} = ?1 WHERE id = ?2", column);
        self.conn.execute(&sql, params![value, id])
    }

    pub fn update_sunk_cost(&self, id: i64, days: i32) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET sunk_cost_days = ?1 WHERE id = ?2",
            params![days, id],
        )
    }

    pub fn update_next_task(&self, id: i64, next_task: Option<&str>) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET next_task = ?1 WHERE id = ?2",
            params![next_task, id],
        )
    }

    pub fn update_research_summary(&self, id: i64, summary: &str) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET research_summary = ?1 WHERE id = ?2",
            params![summary, id],
        )
    }

    pub fn update_from_scan(&self, id: i64, last_activity: NaiveDate) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET last_activity = ?1 WHERE id = ?2",
            params![last_activity.to_string(), id],
        )
    }

    pub fn update_project_type(&self, id: i64, project_type: &ProjectType) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET project_type = ?1 WHERE id = ?2",
            params![project_type.as_str(), id],
        )
    }

    pub fn touch_project(&self, id: i64) -> Result<usize> {
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "UPDATE projects SET last_activity = ?1 WHERE id = ?2",
            params![today, id],
        )
    }

    pub fn rename_project(&self, id: i64, name: &str) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
    }

    pub fn link_project(&self, id: i64, path: &str) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET path = ?1 WHERE id = ?2",
            params![path, id],
        )
    }

    pub fn soft_delete(&self, id: i64) -> Result<usize> {
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "UPDATE projects SET deleted_at = ?1 WHERE id = ?2",
            params![today, id],
        )
    }

    pub fn restore(&self, id: i64) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET deleted_at = NULL WHERE id = ?1",
            params![id],
        )
    }

    pub fn mark_duplicate(&self, id: i64, original_id: i64) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET duplicate_of = ?1 WHERE id = ?2",
            params![original_id, id],
        )
    }

    pub fn mark_possible_duplicate(&self, id: i64, score: f32) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET possible_duplicate_score = ?1 WHERE id = ?2",
            params![score, id],
        )
    }

    pub fn purge_old_deleted(&self, days: i64) -> Result<usize> {
        let cutoff = (chrono::Local::now().date_naive() - chrono::Duration::days(days)).to_string();
        self.conn.execute(
            "DELETE FROM projects WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
            params![cutoff],
        )
    }

    pub fn record_stage_event(
        &self, project_id: i64, from_stage: u8, to_stage: u8, reason: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Local::now().naive_local().to_string();
        self.conn.execute(
            "INSERT INTO stage_events (project_id, from_stage, to_stage, occurred_at, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![project_id, from_stage, to_stage, now, reason],
        )?;
        Ok(())
    }

    pub fn record_pivot_event(&self, project_id: i64, reason: Option<&str>) -> Result<()> {
        let now = chrono::Local::now().naive_local().to_string();
        self.conn.execute(
            "INSERT INTO pivot_events (project_id, occurred_at, reason) VALUES (?1, ?2, ?3)",
            params![project_id, now, reason],
        )?;
        self.conn.execute(
            "UPDATE projects SET pivot_count = pivot_count + 1, stage = 1, fit_signal = NULL WHERE id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    pub fn get_pivot_count(&self, id: i64) -> Result<u32> {
        self.conn.query_row(
            "SELECT pivot_count FROM projects WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
    }

    pub fn list_stage_events(&self, project_id: i64) -> Result<Vec<StageEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT project_id, from_stage, to_stage, occurred_at, reason
             FROM stage_events WHERE project_id = ?1
             ORDER BY occurred_at LIMIT 1000"
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(StageEvent {
                project_id: row.get(0)?,
                from_stage: row.get(1)?,
                to_stage: row.get(2)?,
                occurred_at: row.get(3)?,
                reason: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_all_stage_events(&self) -> Result<Vec<StageEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT project_id, from_stage, to_stage, occurred_at, reason
             FROM stage_events ORDER BY occurred_at LIMIT 10000"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StageEvent {
                project_id: row.get(0)?,
                from_stage: row.get(1)?,
                to_stage: row.get(2)?,
                occurred_at: row.get(3)?,
                reason: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_all_pivot_events(&self) -> Result<Vec<PivotEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT project_id, occurred_at, reason
             FROM pivot_events ORDER BY occurred_at LIMIT 10000"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PivotEvent {
                project_id: row.get(0)?,
                occurred_at: row.get(1)?,
                reason: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn save_thresholds(&self, t: &crate::domain::Thresholds) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS calibration (
                key TEXT PRIMARY KEY,
                value INTEGER NOT NULL
            )"
        )?;
        let pairs: Vec<(&str, i32)> = vec![
            ("kill_fit", t.kill_fit as i32),
            ("kill_vel", t.kill_vel as i32),
            ("kill_sunk", t.kill_sunk),
            ("pivot_fit", t.pivot_fit as i32),
            ("pivot_vel", t.pivot_vel as i32),
            ("groom_fit", t.groom_fit as i32),
            ("groom_vel", t.groom_vel as i32),
            ("push_fit", t.push_fit as i32),
            ("push_vel", t.push_vel as i32),
            ("sustain_fit", t.sustain_fit as i32),
            ("integrate_dist", t.integrate_dist as i32),
            ("repurpose_lev", t.repurpose_lev as i32),
            ("repurpose_sunk", t.repurpose_sunk),
            ("ship_stage", t.ship_stage as i32),
        ];
        for (key, value) in pairs {
            self.conn.execute(
                "INSERT OR REPLACE INTO calibration (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
        Ok(())
    }

    pub fn load_thresholds(&self) -> Result<crate::domain::Thresholds> {
        let exists: bool = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='calibration'"
        )?.exists([])?;

        if !exists {
            return Ok(crate::domain::Thresholds::default());
        }

        let mut t = crate::domain::Thresholds::default();
        let mut stmt = self.conn.prepare("SELECT key, value FROM calibration")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "kill_fit" => t.kill_fit = value as u8,
                "kill_vel" => t.kill_vel = value as u8,
                "kill_sunk" => t.kill_sunk = value,
                "pivot_fit" => t.pivot_fit = value as u8,
                "pivot_vel" => t.pivot_vel = value as u8,
                "groom_fit" => t.groom_fit = value as u8,
                "groom_vel" => t.groom_vel = value as u8,
                "push_fit" => t.push_fit = value as u8,
                "push_vel" => t.push_vel = value as u8,
                "sustain_fit" => t.sustain_fit = value as u8,
                "integrate_dist" => t.integrate_dist = value as u8,
                "repurpose_lev" => t.repurpose_lev = value as u8,
                "repurpose_sunk" => t.repurpose_sunk = value,
                "ship_stage" => t.ship_stage = value as u8,
                _ => {}
            }
        }
        Ok(t)
    }

    pub fn migrate_scoring(&self) -> Result<i64> {
        self.conn.execute(
            "UPDATE projects SET project_type = 'webapp' WHERE project_type IN ('product', 'webapp', 'blog', 'consumer-app')",
            [],
        )?;
        self.conn.execute(
            "UPDATE projects SET project_type = 'oss' WHERE project_type IN ('library', 'open-core', 'personal-tool')",
            [],
        )?;
        self.conn.execute(
            "UPDATE projects SET project_type = 'research' WHERE project_type IN ('study')",
            [],
        )?;
        self.conn.execute(
            "UPDATE projects SET project_type = 'game' WHERE project_type IN ('game', 'games')",
            [],
        )?;
        self.conn.execute(
            "UPDATE projects SET state = 'active' WHERE state IN ('inbox', 'parked')",
            [],
        )?;
        self.conn.execute(
            "UPDATE projects SET state = 'archived' WHERE state IN ('shipped', 'killed')",
            [],
        )?;

        let has_readiness: bool = self.conn
            .prepare("SELECT readiness FROM projects LIMIT 0")
            .is_ok();

        if has_readiness {
            self.conn.execute(
                "UPDATE projects SET stage = CASE
                    WHEN readiness >= 80 THEN 4
                    WHEN readiness >= 60 THEN 3
                    WHEN readiness >= 30 THEN 2
                    WHEN readiness > 0 THEN 1
                    ELSE 0
                END WHERE stage = 0",
                [],
            )?;
        }

        self.conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get::<_, i64>(0))
    }

    fn row_to_project(row: &rusqlite::Row) -> Result<Project> {
        fn parse_date(value: &str) -> NaiveDate {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::Local::now().date_naive())
        }

        let state_str: String = row.get(2)?;
        let state = match state_str.as_str() {
            "active" | "inbox" | "parked" => ProjectState::Active,
            "archived" | "shipped" | "killed" => ProjectState::Archived,
            _ => ProjectState::Active,
        };

        let type_str: String = row.get::<_, Option<String>>(3)?
            .unwrap_or_else(|| "oss".to_string());
        let project_type = match type_str.as_str() {
            "oss" | "library" | "open-core" | "personal-tool" => ProjectType::Oss,
            "research" => ProjectType::Research,
            "game" | "games" => ProjectType::Game,
            "webapp" | "product" | "consumer-app" | "blog" => ProjectType::Webapp,
            "study" | "learning" => ProjectType::Study,
            _ => ProjectType::from_str(&type_str),
        };

        let last_activity: String = row.get(11)?;
        let created_at: String = row.get(12)?;
        let soft_deadline: Option<String> = row.get(13)?;
        let deleted_at: Option<String> = row.get(15)?;

        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            state,
            project_type,
            stage: row.get::<_, Option<u8>>(4)?.unwrap_or(0),
            velocity: row.get(5)?,
            fit_signal: row.get(6)?,
            distinctness: row.get(7)?,
            leverage: row.get(8)?,
            sunk_cost_days: row.get(9)?,
            pivot_count: row.get::<_, Option<u32>>(10)?.unwrap_or(0),
            last_activity: parse_date(&last_activity),
            created_at: parse_date(&created_at),
            soft_deadline: soft_deadline.map(|s| parse_date(&s)),
            path: row.get(14)?,
            deleted_at: deleted_at.map(|s| parse_date(&s)),
            duplicate_of: row.get(16)?,
            possible_duplicate_score: row.get(17)?,
            research_summary: row.get(18)?,
            inbox_note: row.get(19)?,
            next_task: row.get(20)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_project() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Test").unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.name, "Test");
        assert_eq!(p.state, ProjectState::Active);
        assert_eq!(p.project_type, ProjectType::Oss);
        assert_eq!(p.stage, 0);
    }

    #[test]
    fn update_axis_accepts_valid_columns() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Ax").unwrap();
        store.update_axis(id, "velocity", Some(8)).unwrap();
        store.update_axis(id, "fit_signal", Some(6)).unwrap();
        store.update_axis(id, "distinctness", Some(9)).unwrap();
        store.update_axis(id, "leverage", Some(7)).unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.velocity, Some(8));
        assert_eq!(p.fit_signal, Some(6));
        assert_eq!(p.distinctness, Some(9));
        assert_eq!(p.leverage, Some(7));
    }

    #[test]
    fn update_axis_rejects_invalid_column() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Bad").unwrap();
        assert!(store.update_axis(id, "not_real", Some(5)).is_err());
    }

    #[test]
    fn update_stage_persists() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Stage").unwrap();
        store.update_stage(id, 3).unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.stage, 3);
    }

    #[test]
    fn record_pivot_resets_stage_and_increments_count() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Pivot").unwrap();
        store.update_stage(id, 3).unwrap();
        store.record_pivot_event(id, Some("wrong direction")).unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.stage, 1);
        assert_eq!(p.pivot_count, 1);
        assert_eq!(p.fit_signal, None);
    }

    #[test]
    fn record_stage_event_persists() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("Ev").unwrap();
        store.record_stage_event(id, 0, 1, Some("first commit")).unwrap();
        let count: i64 = store.conn.query_row(
            "SELECT COUNT(*) FROM stage_events WHERE project_id = ?1",
            params![id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn state_update_round_trips() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("St").unwrap();
        store.update_state(id, ProjectState::Archived).unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.state, ProjectState::Archived);
        store.update_state(id, ProjectState::Active).unwrap();
        let p = store.get_project(id).unwrap().unwrap();
        assert_eq!(p.state, ProjectState::Active);
    }

    #[test]
    fn list_active_excludes_archived() {
        let store = Store::open_in_memory().unwrap();
        let a = store.add_project("Active").unwrap();
        let b = store.add_project("Archived").unwrap();
        store.update_state(b, ProjectState::Archived).unwrap();
        let active = store.list_active_projects().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, a);
    }

    #[test]
    fn get_or_create_by_path_deduplicates() {
        let store = Store::open_in_memory().unwrap();
        let id1 = store.get_or_create_by_path("foo", "/tmp/foo").unwrap();
        let id2 = store.get_or_create_by_path("foo", "/tmp/foo").unwrap();
        assert_eq!(id1, id2);
    }
}
