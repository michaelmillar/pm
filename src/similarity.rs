use std::collections::HashSet;

fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            out.push(' ');
        }
    }
    out
}

fn tokenize(input: &str) -> HashSet<String> {
    normalize(input)
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f32;
    let union = (a.len() + b.len()) as f32 - intersection;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

pub fn token_similarity(a: &str, b: &str) -> f32 {
    let ta = tokenize(a);
    let tb = tokenize(b);
    jaccard(&ta, &tb)
}

pub fn weighted_similarity(
    name_a: &str,
    name_b: &str,
    title_a: &str,
    title_b: &str,
    snippet_a: &str,
    snippet_b: &str,
    desc_a: &str,
    desc_b: &str,
) -> f32 {
    let name = token_similarity(name_a, name_b);
    let title = token_similarity(title_a, title_b);
    let snippet = token_similarity(snippet_a, snippet_b);
    let desc = token_similarity(desc_a, desc_b);

    let mut total_weight = 0.0;
    let mut total_score = 0.0;

    total_weight += 0.40;
    total_score += name * 0.40;

    if !(title_a.trim().is_empty() && title_b.trim().is_empty()) {
        total_weight += 0.20;
        total_score += title * 0.20;
    }
    if !(snippet_a.trim().is_empty() && snippet_b.trim().is_empty()) {
        total_weight += 0.30;
        total_score += snippet * 0.30;
    }
    if !(desc_a.trim().is_empty() && desc_b.trim().is_empty()) {
        total_weight += 0.10;
        total_score += desc * 0.10;
    }

    if total_weight == 0.0 {
        0.0
    } else {
        total_score / total_weight
    }
}
