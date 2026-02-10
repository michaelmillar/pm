use crate::cli_core;
use crate::scanner;
use crate::store::Store;
use chrono::Local;
use std::error::Error;
use std::path::Path;

pub fn discover_projects(store: &Store, root: &Path) -> Result<(), Box<dyn Error>> {
    let entries = std::fs::read_dir(root)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join(".git").is_dir() {
            continue;
        }

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        let path_str = path.to_string_lossy().to_string();
        let id = store.get_or_create_by_path(&name, &path_str)?;

        let scan = scanner::scan_project(&path_str);
        let today = Local::now().date_naive();
        let project = match store.get_project(id)? {
            Some(p) => p,
            None => continue,
        };
        let score = cli_core::auto_score(&scan, project.created_at, today);
        store.update_scores(id, score.impact, score.monetization, score.readiness)?;
    }

    Ok(())
}
