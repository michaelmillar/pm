use crate::domain::{Project, ProjectState};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result};

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct ResearchRecord {
    pub summary: String,
    pub previous: Option<String>,
    pub researched_at: Option<String>,
    pub consecutive_flags: i64,
}

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

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'inbox',
                impact INTEGER NOT NULL DEFAULT 5,
                monetization INTEGER NOT NULL DEFAULT 5,
                readiness INTEGER NOT NULL DEFAULT 0,
                last_activity TEXT NOT NULL,
                created_at TEXT NOT NULL,
                soft_deadline TEXT,
                path TEXT,
                deleted_at TEXT,
                duplicate_of INTEGER,
                possible_duplicate_score REAL,
                cloneability INTEGER
            );",
        )?;
        // Migrations
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN path TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN deleted_at TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN duplicate_of INTEGER", []);
        let _ = self
            .conn
            .execute("ALTER TABLE projects ADD COLUMN possible_duplicate_score REAL", []);
        let _ = self
            .conn
            .execute("ALTER TABLE projects ADD COLUMN cloneability INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN research_summary TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN research_previous TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN researched_at TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN research_consecutive_flags INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN inbox_note TEXT", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN uniqueness INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN defensibility INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN project_type TEXT NOT NULL DEFAULT 'product'", []);
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN vibe INTEGER", []);
        Ok(())
    }

    pub fn add_project(&self, name: &str) -> Result<i64> {
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "INSERT INTO projects (name, last_activity, created_at) VALUES (?1, ?2, ?2)",
            params![name, today],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_project(&self, id: i64) -> Result<Option<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::row_to_project(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_active_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE state = 'active' AND deleted_at IS NULL AND duplicate_of IS NULL ORDER BY name",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn list_inbox_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE state = 'inbox' AND deleted_at IS NULL AND duplicate_of IS NULL ORDER BY created_at DESC",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn list_linked_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE path IS NOT NULL AND state = 'active' AND deleted_at IS NULL AND duplicate_of IS NULL ORDER BY name",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE path = ?1 AND deleted_at IS NULL",
        )?;

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
            "INSERT INTO projects (name, state, impact, monetization, readiness, last_activity, created_at, path, project_type)
             VALUES (?1, 'inbox', 5, 5, 0, ?2, ?2, ?3, 'product')",
            params![name, today, path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn link_project(&self, id: i64, path: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
        Ok(count)
    }

    pub fn update_from_scan(
        &self,
        id: i64,
        readiness: u8,
        last_activity: chrono::NaiveDate,
    ) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET readiness = ?1, last_activity = ?2 WHERE id = ?3",
            params![readiness, last_activity.to_string(), id],
        )?;
        Ok(count)
    }

    pub fn update_scores(
        &self,
        id: i64,
        impact: u8,
        monetization: u8,
        readiness: u8,
    ) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET impact = ?1, monetization = ?2, readiness = ?3 WHERE id = ?4",
            params![impact, monetization, readiness, id],
        )?;
        Ok(count)
    }

    pub fn update_readiness(&self, id: i64, readiness: u8) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET readiness = ?1 WHERE id = ?2",
            params![readiness, id],
        )?;
        Ok(count)
    }

    pub fn update_state(&self, id: i64, state: ProjectState) -> Result<usize> {
        let state_str = match state {
            ProjectState::Inbox => "inbox",
            ProjectState::Active => "active",
            ProjectState::Parked => "parked",
            ProjectState::Shipped => "shipped",
            ProjectState::Killed => "killed",
        };
        let count = self.conn.execute(
            "UPDATE projects SET state = ?1 WHERE id = ?2",
            params![state_str, id],
        )?;
        Ok(count)
    }

    pub fn mark_duplicate(&self, id: i64, original_id: i64) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET duplicate_of = ?1 WHERE id = ?2",
            params![original_id, id],
        )?;
        Ok(count)
    }

    pub fn mark_possible_duplicate(&self, id: i64, score: f32) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET possible_duplicate_score = ?1 WHERE id = ?2",
            params![score, id],
        )?;
        Ok(count)
    }

    pub fn list_possible_duplicates(&self, min_score: f32) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE deleted_at IS NULL AND duplicate_of IS NULL AND possible_duplicate_score >= ?1
             ORDER BY possible_duplicate_score DESC, name",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query(params![min_score])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn move_to_inbox(&self, id: i64) -> Result<usize> {
        self.update_state(id, ProjectState::Inbox)
    }

    pub fn touch_project(&self, id: i64) -> Result<usize> {
        let today = chrono::Local::now().date_naive().to_string();
        let count = self.conn.execute(
            "UPDATE projects SET last_activity = ?1 WHERE id = ?2",
            params![today, id],
        )?;
        Ok(count)
    }

    pub fn rename_project(&self, id: i64, name: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(count)
    }

    pub fn set_inbox_note(&self, id: i64, note: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET inbox_note = ?1 WHERE id = ?2",
            params![note, id],
        )?;
        Ok(())
    }

    pub fn get_inbox_note(&self, id: i64) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT inbox_note FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => row.get(0),
            None => Ok(None),
        }
    }

    fn row_to_project(row: &rusqlite::Row) -> Result<Project> {
        fn parse_date(value: &str) -> NaiveDate {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::Local::now().date_naive())
        }

        let state_str: String = row.get(2)?;
        let state = match state_str.as_str() {
            "inbox" => ProjectState::Inbox,
            "active" => ProjectState::Active,
            "parked" => ProjectState::Parked,
            "shipped" => ProjectState::Shipped,
            "killed" => ProjectState::Killed,
            _ => ProjectState::Inbox,
        };

        let last_activity: String = row.get(6)?;
        let created_at: String = row.get(7)?;
        let soft_deadline: Option<String> = row.get(8)?;
        let path: Option<String> = row.get(9)?;
        let deleted_at: Option<String> = row.get(10)?;
        let duplicate_of: Option<i64> = row.get(11)?;
        let possible_duplicate_score: Option<f32> = row.get(12)?;
        let cloneability: Option<u8> = row.get(13)?;
        let uniqueness: Option<u8> = row.get(14)?;
        let defensibility: Option<u8> = row.get::<_, Option<u8>>(15).unwrap_or(None);
        let project_type = {
            let type_str: String = row.get::<_, Option<String>>(16).unwrap_or(None).unwrap_or_else(|| "product".to_string());
            crate::domain::ProjectType::from_str(&type_str)
        };
        let vibe: Option<u8> = row.get::<_, Option<u8>>(17).unwrap_or(None);

        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            state,
            impact: row.get(3)?,
            monetization: row.get(4)?,
            readiness: row.get(5)?,
            last_activity: parse_date(&last_activity),
            created_at: parse_date(&created_at),
            soft_deadline: soft_deadline
                .map(|s| parse_date(&s)),
            path,
            deleted_at: deleted_at
                .map(|s| parse_date(&s)),
            duplicate_of,
            possible_duplicate_score,
            cloneability,
            uniqueness,
            defensibility,
            project_type,
            vibe,
        })
    }

    pub fn update_assessment(
        &self,
        id: i64,
        impact: u8,
        monetization: u8,
        cloneability: Option<u8>,
        uniqueness: Option<u8>,
    ) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET impact = ?1, monetization = ?2, cloneability = ?3, uniqueness = ?4 WHERE id = ?5",
            params![impact, monetization, cloneability, uniqueness, id],
        )?;
        Ok(count)
    }

    pub fn update_defensibility(&self, id: i64, defensibility: Option<u8>) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET defensibility = ?1 WHERE id = ?2",
            params![defensibility, id],
        )
    }

    pub fn update_project_type(&self, id: i64, project_type: &crate::domain::ProjectType) -> Result<usize> {
        self.conn.execute(
            "UPDATE projects SET project_type = ?1 WHERE id = ?2",
            params![project_type.as_str(), id],
        )
    }

    pub fn update_vibe(&self, id: i64, vibe: Option<u8>) -> Result<usize> {
        self.conn.execute("UPDATE projects SET vibe = ?1 WHERE id = ?2", params![vibe, id])
    }

    pub fn soft_delete(&self, id: i64) -> Result<usize> {
        let today = chrono::Local::now().date_naive().to_string();
        let count = self.conn.execute(
            "UPDATE projects SET deleted_at = ?1 WHERE id = ?2",
            params![today, id],
        )?;
        Ok(count)
    }

    pub fn restore(&self, id: i64) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE projects SET deleted_at = NULL WHERE id = ?1",
            params![id],
        )?;
        Ok(count)
    }

    pub fn list_deleted_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn list_projects_for_dedupe(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path, deleted_at, duplicate_of, possible_duplicate_score, cloneability, uniqueness, defensibility, project_type, vibe
             FROM projects WHERE deleted_at IS NULL AND duplicate_of IS NULL",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn purge_old_deleted(&self, days: i64) -> Result<usize> {
        let cutoff = (chrono::Local::now().date_naive() - chrono::Duration::days(days)).to_string();
        let count = self.conn.execute(
            "DELETE FROM projects WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
            params![cutoff],
        )?;
        Ok(count)
    }

    pub fn save_research(&self, id: i64, summary: &str) -> Result<()> {
        let today = chrono::Local::now().naive_local().to_string();

        // Detect cut-losses signal
        let flags_increment = if detect_cut_losses(summary) { 1i64 } else { 0i64 };

        // Reset consecutive flags if no signal, else increment
        self.conn.execute(
            "UPDATE projects SET
                research_previous = research_summary,
                research_summary = ?1,
                researched_at = ?2,
                research_consecutive_flags = CASE
                    WHEN ?3 = 1 THEN research_consecutive_flags + 1
                    ELSE 0
                END
             WHERE id = ?4",
            params![summary, today, flags_increment, id],
        )?;
        Ok(())
    }

    pub fn get_research(&self, id: i64) -> Result<Option<ResearchRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT research_summary, research_previous, researched_at, research_consecutive_flags
             FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let summary: Option<String> = row.get(0)?;
                match summary {
                    None => Ok(None),
                    Some(s) => Ok(Some(ResearchRecord {
                        summary: s,
                        previous: row.get(1)?,
                        researched_at: row.get(2)?,
                        consecutive_flags: row.get(3)?,
                    })),
                }
            }
            None => Ok(None),
        }
    }
}

fn detect_cut_losses(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("consider stopping") || lower.contains("cut losses")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_project() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("test project").unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.name, "test project");
        assert_eq!(project.state, ProjectState::Inbox);
    }

    #[test]
    fn test_update_scores() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("test").unwrap();

        store.update_scores(id, 8, 7, 60).unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.impact, 8);
        assert_eq!(project.monetization, 7);
        assert_eq!(project.readiness, 60);
    }

    #[test]
    fn test_update_scores_returns_not_found() {
        let store = Store::open_in_memory().unwrap();
        let result = store.update_scores(999, 8, 7, 60).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_update_state() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("test").unwrap();

        store.update_state(id, ProjectState::Active).unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.state, ProjectState::Active);
    }

    #[test]
    fn test_update_state_returns_not_found() {
        let store = Store::open_in_memory().unwrap();
        let count = store.update_state(999, ProjectState::Active).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_list_active_projects() {
        let store = Store::open_in_memory().unwrap();

        let id1 = store.add_project("active1").unwrap();
        let id2 = store.add_project("active2").unwrap();
        let _id3 = store.add_project("inbox_only").unwrap();

        store.update_state(id1, ProjectState::Active).unwrap();
        store.update_state(id2, ProjectState::Active).unwrap();

        let active = store.list_active_projects().unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_touch_project_returns_not_found() {
        let store = Store::open_in_memory().unwrap();
        let count = store.touch_project(999).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_row_to_project_handles_bad_dates() {
        let store = Store::open_in_memory().unwrap();
        store
            .conn
            .execute(
                "INSERT INTO projects (name, last_activity, created_at) VALUES (?1, ?2, ?3)",
                params!["bad", "not-a-date", "not-a-date"],
            )
            .unwrap();

        let project = store.get_project(1).unwrap().unwrap();
        assert_eq!(project.name, "bad");
    }

    #[test]
    fn test_schema_has_deleted_at_column() {
        let store = Store::open_in_memory().unwrap();
        let mut stmt = store.conn.prepare("PRAGMA table_info(projects)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert!(cols.contains(&"deleted_at".to_string()));
    }

    #[test]
    fn test_list_inbox_projects_excludes_deleted() {
        let store = Store::open_in_memory().unwrap();
        let id1 = store.add_project("inbox1").unwrap();
        let _id2 = store.add_project("inbox2").unwrap();

        store.soft_delete(id1).unwrap();

        let inbox = store.list_inbox_projects().unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].name, "inbox2");
    }

    #[test]
    fn test_list_linked_projects_requires_active() {
        let store = Store::open_in_memory().unwrap();
        let id1 = store.add_project("linked-active").unwrap();
        let id2 = store.add_project("linked-inbox").unwrap();

        store.update_state(id1, ProjectState::Active).unwrap();
        store.link_project(id1, "/tmp/linked-active").unwrap();
        store.link_project(id2, "/tmp/linked-inbox").unwrap();

        let linked = store.list_linked_projects().unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].name, "linked-active");
    }

    #[test]
    fn test_soft_delete_and_restore() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("to-delete").unwrap();

        store.soft_delete(id).unwrap();
        let deleted = store.list_deleted_projects().unwrap();
        assert_eq!(deleted.len(), 1);

        store.restore(id).unwrap();
        let deleted = store.list_deleted_projects().unwrap();
        assert_eq!(deleted.len(), 0);
    }

    #[test]
    fn test_purge_old_deleted() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("old-deleted").unwrap();
        store.soft_delete(id).unwrap();

        let old_date = (chrono::Local::now().date_naive() - chrono::Duration::days(40)).to_string();
        store
            .conn
            .execute(
                "UPDATE projects SET deleted_at = ?1 WHERE id = ?2",
                params![old_date, id],
            )
            .unwrap();

        let purged = store.purge_old_deleted(30).unwrap();
        assert_eq!(purged, 1);
    }

    #[test]
    fn test_save_and_get_research() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("research-test").unwrap();

        store.save_research(id, "Summary of findings.").unwrap();

        let r = store.get_research(id).unwrap().unwrap();
        assert_eq!(r.summary, "Summary of findings.");
        assert!(r.researched_at.is_some());
        assert!(r.previous.is_none());
    }

    #[test]
    fn test_save_research_rotates_previous() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("rotate-test").unwrap();

        store.save_research(id, "First summary.").unwrap();
        store.save_research(id, "Second summary.").unwrap();

        let r = store.get_research(id).unwrap().unwrap();
        assert_eq!(r.summary, "Second summary.");
        assert_eq!(r.previous.as_deref(), Some("First summary."));
    }

    #[test]
    fn test_get_research_none_for_unknown() {
        let store = Store::open_in_memory().unwrap();
        let result = store.get_research(9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_research_none_when_never_researched() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("no-research").unwrap();
        let result = store.get_research(id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_set_and_get_inbox_note() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("pivot-idea").unwrap();

        store.set_inbox_note(id, "A tool that does X uniquely.").unwrap();

        let note = store.get_inbox_note(id).unwrap();
        assert_eq!(note.as_deref(), Some("A tool that does X uniquely."));
    }

    #[test]
    fn test_get_inbox_note_none_when_unset() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("no-note").unwrap();
        let note = store.get_inbox_note(id).unwrap();
        assert!(note.is_none());
    }

    #[test]
    fn test_set_inbox_note_overwrites() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("rewrite").unwrap();
        store.set_inbox_note(id, "First note.").unwrap();
        store.set_inbox_note(id, "Second note.").unwrap();
        let note = store.get_inbox_note(id).unwrap();
        assert_eq!(note.as_deref(), Some("Second note."));
    }

    #[test]
    fn test_schema_has_uniqueness_column() {
        let store = Store::open_in_memory().unwrap();
        let mut stmt = store.conn.prepare("PRAGMA table_info(projects)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert!(cols.contains(&"uniqueness".to_string()));
    }

    #[test]
    fn test_update_assessment_stores_uniqueness() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("test").unwrap();

        store.update_assessment(id, 8, 7, Some(6), Some(5)).unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.impact, 8);
        assert_eq!(project.monetization, 7);
        assert_eq!(project.cloneability, Some(6));
        assert_eq!(project.uniqueness, Some(5));
    }
}
