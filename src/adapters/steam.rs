use crate::domain::Project;
use super::FitSignalResult;

pub fn fetch_fit(project: &Project) -> Option<FitSignalResult> {
    let app_id = read_steam_app_id(project)?;
    let url = format!("https://store.steampowered.com/api/appdetails?appids={}", app_id);

    let response = ureq::get(&url)
        .set("User-Agent", "pm-scoring/0.1")
        .call()
        .ok()?;

    let body: serde_json::Value = response.into_json().ok()?;
    let app_data = body.get(&app_id)?.get("data")?;

    let total_reviews = app_data.get("recommendations")
        .and_then(|r| r.get("total"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);

    let raw = ((total_reviews as f64 + 1.0).log2() * 1.5).min(10.0) as u8;

    Some(FitSignalResult {
        raw_score: raw,
        source: format!("steam:app_id={} reviews={}", app_id, total_reviews),
    })
}

fn read_steam_app_id(project: &Project) -> Option<String> {
    let path = project.path.as_ref()?;
    let pm_toml = std::path::Path::new(path).join("pm.toml");
    let content = std::fs::read_to_string(&pm_toml).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("steam_app_id") {
            if let Some((_, value)) = trimmed.split_once('=') {
                let id = value.trim().trim_matches('"').trim_matches('\'');
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}
