pub struct GitHubData {
    pub stars: Option<u64>,
    pub forks: Option<u64>,
    pub open_issues: Option<u64>,
    pub last_activity_at: Option<String>,
}

pub fn fetch(slug: &str) -> Result<GitHubData, String> {
    let url = format!("https://api.github.com/repos/{}", slug);
    let mut request = ureq::get(&url);

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.set("Authorization", &format!("Bearer {}", token));
    }

    let response = request
        .set("User-Agent", "pm-scoring/0.1")
        .call()
        .map_err(|e| format!("GitHub request failed for {}: {}", slug, e))?;

    let body: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("GitHub response parse failed: {}", e))?;

    Ok(GitHubData {
        stars: body.get("stargazers_count").and_then(|v| v.as_u64()),
        forks: body.get("forks_count").and_then(|v| v.as_u64()),
        open_issues: body.get("open_issues_count").and_then(|v| v.as_u64()),
        last_activity_at: body.get("pushed_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}

pub fn slug_from_remote(remote_url: &str) -> Option<String> {
    let trimmed = remote_url.trim().trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        return Some(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        return Some(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("http://github.com/") {
        return Some(rest.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_remote() {
        assert_eq!(
            slug_from_remote("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn parses_ssh_remote() {
        assert_eq!(
            slug_from_remote("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn rejects_unknown_host() {
        assert_eq!(slug_from_remote("https://gitlab.com/owner/repo"), None);
    }
}
