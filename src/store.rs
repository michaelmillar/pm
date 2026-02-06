use crate::domain::{Project, ProjectState};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result};

pub struct Store {
    conn: Connection,
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
                path TEXT
            );",
        )?;
        // Migration: add path column if missing
        let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN path TEXT", []);
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
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path
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
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path
             FROM projects WHERE state = 'active' ORDER BY name",
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
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path
             FROM projects WHERE state = 'inbox' ORDER BY created_at DESC",
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
            "SELECT id, name, state, impact, monetization, readiness, last_activity, created_at, soft_deadline, path
             FROM projects WHERE path IS NOT NULL AND state = 'active' ORDER BY name",
        )?;

        let mut projects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            projects.push(Self::row_to_project(row)?);
        }
        Ok(projects)
    }

    pub fn link_project(&self, id: i64, path: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
        Ok(())
    }

    pub fn update_from_scan(&self, id: i64, readiness: u8, last_activity: chrono::NaiveDate) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET readiness = ?1, last_activity = ?2 WHERE id = ?3",
            params![readiness, last_activity.to_string(), id],
        )?;
        Ok(())
    }

    pub fn update_scores(&self, id: i64, impact: u8, monetization: u8, readiness: u8) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET impact = ?1, monetization = ?2, readiness = ?3 WHERE id = ?4",
            params![impact, monetization, readiness, id],
        )?;
        Ok(())
    }

    pub fn update_state(&self, id: i64, state: ProjectState) -> Result<()> {
        let state_str = match state {
            ProjectState::Inbox => "inbox",
            ProjectState::Active => "active",
            ProjectState::Parked => "parked",
            ProjectState::Shipped => "shipped",
            ProjectState::Killed => "killed",
        };
        self.conn.execute(
            "UPDATE projects SET state = ?1 WHERE id = ?2",
            params![state_str, id],
        )?;
        Ok(())
    }

    pub fn touch_project(&self, id: i64) -> Result<()> {
        let today = chrono::Local::now().date_naive().to_string();
        self.conn.execute(
            "UPDATE projects SET last_activity = ?1 WHERE id = ?2",
            params![today, id],
        )?;
        Ok(())
    }

    fn row_to_project(row: &rusqlite::Row) -> Result<Project> {
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

        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            state,
            impact: row.get(3)?,
            monetization: row.get(4)?,
            readiness: row.get(5)?,
            last_activity: NaiveDate::parse_from_str(&last_activity, "%Y-%m-%d").unwrap(),
            created_at: NaiveDate::parse_from_str(&created_at, "%Y-%m-%d").unwrap(),
            soft_deadline: soft_deadline
                .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap()),
            path,
        })
    }
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
    fn test_update_state() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("test").unwrap();

        store.update_state(id, ProjectState::Active).unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.state, ProjectState::Active);
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
}
