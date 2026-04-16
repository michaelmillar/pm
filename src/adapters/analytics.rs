use crate::domain::Project;
use super::FitSignalResult;

pub fn fetch_fit(project: &Project) -> Option<FitSignalResult> {
    let config = read_analytics_config(project)?;

    match config.analytics_type.as_str() {
        "plausible" => fetch_plausible(&config),
        "umami" => fetch_umami(&config),
        _ => None,
    }
}

struct AnalyticsConfig {
    analytics_type: String,
    base_url: String,
    site_id: String,
    api_key: Option<String>,
}

fn read_analytics_config(project: &Project) -> Option<AnalyticsConfig> {
    let path = project.path.as_ref()?;
    let pm_toml = std::path::Path::new(path).join("pm.toml");
    let content = std::fs::read_to_string(&pm_toml).ok()?;

    let mut analytics_type = String::new();
    let mut base_url = String::new();
    let mut site_id = String::new();
    let mut api_key_env = String::new();
    let mut in_analytics = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[analytics]" {
            in_analytics = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_analytics = false;
            continue;
        }
        if !in_analytics {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let k = key.trim();
            let v = value.trim().trim_matches('"').trim_matches('\'');
            match k {
                "type" => analytics_type = v.to_string(),
                "base_url" => base_url = v.to_string(),
                "site_id" => site_id = v.to_string(),
                "api_key_env" => api_key_env = v.to_string(),
                _ => {}
            }
        }
    }

    if analytics_type.is_empty() || base_url.is_empty() || site_id.is_empty() {
        return None;
    }

    let api_key = if api_key_env.is_empty() {
        None
    } else {
        std::env::var(&api_key_env).ok()
    };

    Some(AnalyticsConfig { analytics_type, base_url, site_id, api_key })
}

fn fetch_plausible(config: &AnalyticsConfig) -> Option<FitSignalResult> {
    let url = format!(
        "{}/api/v1/stats/aggregate?site_id={}&period=30d&metrics=visitors",
        config.base_url, config.site_id
    );

    let mut req = ureq::get(&url);
    if let Some(ref key) = config.api_key {
        req = req.set("Authorization", &format!("Bearer {}", key));
    }

    let response = req.call().ok()?;
    let body: serde_json::Value = response.into_json().ok()?;

    let visitors = body.get("results")
        .and_then(|r| r.get("visitors"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let raw = ((visitors as f64 + 1.0).log2() * 2.0).min(10.0) as u8;

    Some(FitSignalResult {
        raw_score: raw,
        source: format!("plausible:visitors_30d={}", visitors),
    })
}

fn fetch_umami(config: &AnalyticsConfig) -> Option<FitSignalResult> {
    let now = chrono::Local::now();
    let start = (now - chrono::Duration::days(30)).timestamp_millis();
    let end = now.timestamp_millis();

    let url = format!(
        "{}/api/websites/{}/stats?startAt={}&endAt={}",
        config.base_url, config.site_id, start, end
    );

    let mut req = ureq::get(&url);
    if let Some(ref key) = config.api_key {
        req = req.set("Authorization", &format!("Bearer {}", key));
    }

    let response = req.call().ok()?;
    let body: serde_json::Value = response.into_json().ok()?;

    let visitors = body.get("uniques")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let raw = ((visitors as f64 + 1.0).log2() * 2.0).min(10.0) as u8;

    Some(FitSignalResult {
        raw_score: raw,
        source: format!("umami:uniques_30d={}", visitors),
    })
}
