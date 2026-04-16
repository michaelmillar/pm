use std::path::Path;

pub fn compute(path: &Path) -> u8 {
    let dep_count = count_deps(path);
    let (test_files, source_files) = count_files(path);
    let test_ratio = if source_files > 0 {
        test_files as f32 / source_files as f32
    } else {
        0.0
    };

    let dep_penalty = match dep_count {
        0..=5 => 0,
        6..=10 => 1,
        11..=20 => 2,
        _ => 3,
    };

    let test_bonus = (test_ratio * 5.0).min(4.0) as i32;
    let raw = 7 - dep_penalty + test_bonus;
    raw.clamp(0, 10) as u8
}

fn count_deps(path: &Path) -> usize {
    if let Some(n) = count_cargo_deps(path) {
        return n;
    }
    if let Some(n) = count_package_json_deps(path) {
        return n;
    }
    0
}

fn count_cargo_deps(path: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(path.join("Cargo.toml")).ok()?;
    let mut in_deps = false;
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if in_deps && !trimmed.is_empty() && trimmed.contains('=') {
            count += 1;
        }
    }
    Some(count)
}

fn count_package_json_deps(path: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(path.join("package.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let deps = v.get("dependencies").and_then(|d| d.as_object()).map(|d| d.len()).unwrap_or(0);
    let dev_deps = v.get("devDependencies").and_then(|d| d.as_object()).map(|d| d.len()).unwrap_or(0);
    Some(deps + dev_deps)
}

fn count_files(path: &Path) -> (usize, usize) {
    let mut test_files = 0;
    let mut source_files = 0;

    let tests_dir = path.join("tests");
    if tests_dir.is_dir() {
        test_files += count_rs_files(&tests_dir);
    }

    let src_dir = path.join("src");
    if src_dir.is_dir() {
        source_files += count_rs_files(&src_dir);
    }

    let test_dir_js = path.join("__tests__");
    if test_dir_js.is_dir() {
        test_files += count_js_files(&test_dir_js);
    }

    (test_files, source_files)
}

fn count_rs_files(dir: &Path) -> usize {
    walkdir(dir, &["rs"])
}

fn count_js_files(dir: &Path) -> usize {
    walkdir(dir, &["ts", "js", "tsx", "jsx"])
}

fn walkdir(dir: &Path, extensions: &[&str]) -> usize {
    let mut count = 0;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with('.') && name != "target" && name != "node_modules" {
                count += walkdir(&path, extensions);
            }
        } else if let Some(ext) = path.extension() {
            if extensions.iter().any(|e| ext == *e) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn empty_dir_scores_neutral() {
        let tmp = TempDir::new().unwrap();
        let score = compute(tmp.path());
        assert_eq!(score, 7);
    }

    #[test]
    fn many_deps_reduces_score() {
        let tmp = TempDir::new().unwrap();
        let mut cargo = "[dependencies]\n".to_string();
        for i in 0..25 {
            cargo.push_str(&format!("dep{} = \"1\"\n", i));
        }
        fs::write(tmp.path().join("Cargo.toml"), cargo).unwrap();
        let score = compute(tmp.path());
        assert!(score <= 5, "25 deps should lower score, got {}", score);
    }

    #[test]
    fn good_test_ratio_increases_score() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[dependencies]\nfoo = \"1\"\n").unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::create_dir_all(tmp.path().join("tests")).unwrap();
        for i in 0..3 {
            fs::write(tmp.path().join("src").join(format!("m{}.rs", i)), "").unwrap();
        }
        for i in 0..3 {
            fs::write(tmp.path().join("tests").join(format!("t{}.rs", i)), "").unwrap();
        }
        let score = compute(tmp.path());
        assert!(score >= 8, "1:1 test ratio should score high, got {}", score);
    }
}
