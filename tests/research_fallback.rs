use std::{
    env,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use pm::research::run_research_claude;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn write_exec(path: &Path, body: &str) {
    fs::write(path, body).expect("write script");
    let mut perms = fs::metadata(path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn prepend_path(dir: &Path) -> Option<String> {
    let old = env::var("PATH").ok();
    let mut joined = dir.to_string_lossy().to_string();
    if let Some(ref p) = old {
        joined.push(':');
        joined.push_str(p);
    }
    unsafe {
        env::set_var("PATH", joined);
    }
    old
}

fn restore_path(old: Option<String>) {
    match old {
        Some(value) => unsafe {
            env::set_var("PATH", value);
        },
        None => unsafe {
            env::remove_var("PATH");
        },
    }
}

fn script_path(bin_dir: &Path, name: &str) -> PathBuf {
    bin_dir.join(name)
}

#[test]
fn uses_claude_when_claude_succeeds() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let tmp = TempDir::new().expect("tmp");
    let bin_dir = tmp.path();

    write_exec(
        &script_path(bin_dir, "claude"),
        "#!/bin/sh\necho 'claude ok'\nexit 0\n",
    );
    write_exec(
        &script_path(bin_dir, "codex"),
        "#!/bin/sh\necho 'codex should not run' >&2\nexit 77\n",
    );

    let old = prepend_path(bin_dir);
    let result = run_research_claude("proj", "usp").expect("claude should succeed");
    restore_path(old);

    assert!(result.contains("claude ok"));
}

#[test]
fn falls_back_to_codex_when_claude_fails() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let tmp = TempDir::new().expect("tmp");
    let bin_dir = tmp.path();

    write_exec(
        &script_path(bin_dir, "claude"),
        "#!/bin/sh\necho 'token limit' >&2\nexit 1\n",
    );
    write_exec(
        &script_path(bin_dir, "codex"),
        "#!/bin/sh\necho 'codex ok'\nexit 0\n",
    );

    let old = prepend_path(bin_dir);
    let result = run_research_claude("proj", "usp").expect("codex fallback should succeed");
    restore_path(old);

    assert!(result.contains("codex ok"));
}

#[test]
fn returns_combined_error_when_both_fail() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let tmp = TempDir::new().expect("tmp");
    let bin_dir = tmp.path();

    write_exec(
        &script_path(bin_dir, "claude"),
        "#!/bin/sh\necho 'claude fail' >&2\nexit 1\n",
    );
    write_exec(
        &script_path(bin_dir, "codex"),
        "#!/bin/sh\necho 'codex fail' >&2\nexit 1\n",
    );

    let old = prepend_path(bin_dir);
    let err = run_research_claude("proj", "usp").expect_err("both should fail");
    restore_path(old);

    assert!(err.contains("claude failed"));
    assert!(err.contains("codex fallback failed"));
}

#[test]
fn codex_fallback_times_out_fast() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let tmp = TempDir::new().expect("tmp");
    let bin_dir = tmp.path();

    write_exec(
        &script_path(bin_dir, "claude"),
        "#!/bin/sh\necho 'claude fail' >&2\nexit 1\n",
    );
    write_exec(
        &script_path(bin_dir, "codex"),
        "#!/bin/sh\nsleep 5\necho 'late' >&2\nexit 0\n",
    );

    let old_path = prepend_path(bin_dir);
    let old_timeout = env::var("PM_CODEX_TIMEOUT_SECS").ok();
    unsafe {
        env::set_var("PM_CODEX_TIMEOUT_SECS", "1");
    }

    let err = run_research_claude("proj", "usp").expect_err("codex should time out");

    match old_timeout {
        Some(value) => unsafe { env::set_var("PM_CODEX_TIMEOUT_SECS", value) },
        None => unsafe { env::remove_var("PM_CODEX_TIMEOUT_SECS") },
    }
    restore_path(old_path);

    assert!(err.contains("timed out"));
}
