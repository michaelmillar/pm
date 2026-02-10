use std::collections::HashMap;

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "into", "about", "this", "that", "your", "project",
    "game", "tool", "app", "repo", "task", "plan", "docs",
];

pub fn suggest_names(name: &str, readme: &str, plans: &str) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let text = format!("{}\n{}\n{}", name, readme, plans);
    for raw in text.split(|c: char| !c.is_alphanumeric()) {
        let word = raw.trim().to_lowercase();
        if word.len() < 3 {
            continue;
        }
        if STOPWORDS.contains(&word.as_str()) {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut suggestions = Vec::new();
    for (word, _) in ranked.into_iter().take(3) {
        suggestions.push(to_title(&word));
    }

    while suggestions.len() < 3 {
        let fallback = match suggestions.len() {
            0 => name.to_string(),
            1 => format!("{} Labs", name),
            _ => format!("{} Studio", name),
        };
        if !suggestions.contains(&fallback) {
            suggestions.push(fallback);
        }
    }

    suggestions.truncate(3);
    suggestions
}

fn to_title(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}
