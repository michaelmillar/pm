pub fn auto_readiness_from_scan(has_commits: bool) -> u8 {
    if has_commits { 20 } else { 5 }
}
