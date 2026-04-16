use crate::domain::Project;
use super::FitSignalResult;

pub fn fetch_fit(project: &Project) -> Option<FitSignalResult> {
    let query = build_query(project)?;
    let url = format!(
        "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit=3&fields=citationCount,year,externalIds",
        urlenc(&query)
    );

    let response = ureq::get(&url)
        .set("User-Agent", "pm-scoring/0.1")
        .call()
        .ok()?;

    let body: serde_json::Value = response.into_json().ok()?;
    let papers = body.get("data")?.as_array()?;

    if papers.is_empty() {
        return Some(FitSignalResult { raw_score: 0, source: "s2:no papers found".to_string() });
    }

    let best = papers.iter()
        .max_by_key(|p| p.get("citationCount").and_then(|c| c.as_u64()).unwrap_or(0))?;

    let citations = best.get("citationCount").and_then(|c| c.as_u64()).unwrap_or(0);
    let has_arxiv = best.get("externalIds")
        .and_then(|ids| ids.get("ArXiv"))
        .is_some();

    let raw = ((citations as f64 + 1.0).log2() * 2.0 + if has_arxiv { 3.0 } else { 0.0 }).min(10.0) as u8;

    Some(FitSignalResult {
        raw_score: raw,
        source: format!("s2:citations={} arxiv={}", citations, has_arxiv),
    })
}

fn build_query(project: &Project) -> Option<String> {
    let path = project.path.as_ref()?;
    let readme = std::path::Path::new(path).join("README.md");
    if let Ok(content) = std::fs::read_to_string(&readme) {
        for line in content.lines().take(5) {
            if line.starts_with("# ") {
                return Some(line.trim_start_matches('#').trim().to_string());
            }
        }
    }
    Some(project.name.clone())
}

fn urlenc(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            c.to_string()
        } else {
            format!("%{:02X}", c as u8)
        }
    }).collect()
}
