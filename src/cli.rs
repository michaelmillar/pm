use crate::charter;
use crate::domain::{ProjectState, TaskSource};
use crate::scanner;
use crate::store::Store;
use chrono::Local;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pm")]
#[command(about = "Personal project manager - tells you what to work on")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Shows what you should work on next
    Next,
    /// Quick status overview of all active projects
    Status,
    /// Mark a project task as done
    Done {
        /// Project ID
        id: i64,
    },
    /// Add a new project to inbox
    Add {
        /// Project name
        name: String,
    },
    /// Score a project (sets impact, monetization, readiness)
    Score {
        /// Project ID
        id: i64,
        /// Impact score 1-10
        #[arg(short, long)]
        impact: u8,
        /// Monetization score 1-10
        #[arg(short, long)]
        monetization: u8,
        /// Readiness percentage 0-100
        #[arg(short, long)]
        readiness: u8,
    },
    /// Show your top 3 priority projects
    Throne,
    /// Explain why the algorithm picked what it picked
    Why,
    /// List projects in inbox
    Inbox,
    /// Link a project to a codebase directory
    Link {
        /// Project ID
        id: i64,
        /// Path to the project directory
        path: String,
    },
    /// Scan linked projects for progress from git and plan files
    Scan,
    /// Show detailed info for a project
    Show {
        /// Project ID
        id: i64,
    },
    /// Remove a project (soft delete, recoverable for 30 days)
    Remove {
        /// Project ID
        id: i64,
    },
    /// List deleted projects (recoverable within 30 days)
    Trash,
    /// Restore a deleted project
    Restore {
        /// Project ID
        id: i64,
    },
    /// Rename a project
    Rename {
        /// Project ID
        id: i64,
        /// New name
        name: String,
    },
    /// Park a project (pause with reason)
    Park {
        /// Project ID
        id: i64,
        /// Reason for parking
        reason: String,
    },
    /// List all tasks for a project with their status
    Tasks {
        /// Project ID
        id: i64,
    },
    /// Generate or check a project charter
    Charter {
        /// Project ID
        id: i64,
        /// Overwrite existing charter
        #[arg(long)]
        force: bool,
    },
    /// Mark a task as done in the local progress file
    Mark {
        /// Project ID
        id: i64,
        /// Task number to mark done
        task_number: usize,
        /// Plan file name (defaults to most recent)
        #[arg(long)]
        plan: Option<String>,
    },
}

pub fn run() {
    let cli = Cli::parse();
    let store = open_store();

    match cli.command {
        Commands::Next => cmd_next(&store),
        Commands::Status => cmd_status(&store),
        Commands::Done { id } => cmd_done(&store, id),
        Commands::Add { name } => cmd_add(&store, &name),
        Commands::Score {
            id,
            impact,
            monetization,
            readiness,
        } => {
            cmd_score(&store, id, impact, monetization, readiness)
        }
        Commands::Throne => cmd_throne(&store),
        Commands::Why => cmd_why(&store),
        Commands::Inbox => cmd_inbox(&store),
        Commands::Link { id, path } => cmd_link(&store, id, &path),
        Commands::Scan => cmd_scan(&store),
        Commands::Show { id } => cmd_show(&store, id),
        Commands::Remove { id } => cmd_remove(&store, id),
        Commands::Trash => cmd_trash(&store),
        Commands::Restore { id } => cmd_restore(&store, id),
        Commands::Rename { id, name } => cmd_rename(&store, id, &name),
        Commands::Park { id, reason } => cmd_park(&store, id, &reason),
        Commands::Tasks { id } => cmd_tasks(&store, id),
        Commands::Charter { id, force } => cmd_charter(&store, id, force),
        Commands::Mark {
            id,
            task_number,
            plan,
        } => cmd_mark(&store, id, task_number, plan),
    }
}

fn open_store() -> Store {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pm");
    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("pm.db");
    Store::open(&db_path).expect("Failed to open database")
}

fn cmd_next(store: &Store) {
    let projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects. Add one with: pm add \"project name\"");
        return;
    }

    let today = Local::now().date_naive();
    let mut scored: Vec<_> = projects.iter().map(|p| (p, p.priority_score(today))).collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    let (top, score) = scored[0];
    let days_stale = (today - top.last_activity).num_days();

    println!("┌─────────────────────────────────────────────────┐");
    println!("│  WORK ON: {:<37} │", truncate(&top.name, 37));
    println!(
        "│  WHY: {}% ready, score {}, {} days stale{} │",
        top.readiness,
        score,
        days_stale,
        " ".repeat(10_usize.saturating_sub(format!("{}{}{}", top.readiness, score, days_stale).len()))
    );
    println!("└─────────────────────────────────────────────────┘");
}

fn cmd_status(store: &Store) {
    let projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects.");
        return;
    }

    let today = Local::now().date_naive();
    println!("Active projects:\n");
    for p in &projects {
        let days_stale = (today - p.last_activity).num_days();
        let bar = progress_bar(p.readiness as usize, 10);
        println!(
            "  [{}] {} {} {}%  ({} days)",
            p.id,
            truncate(&p.name, 20),
            bar,
            p.readiness,
            days_stale
        );
    }
}

fn cmd_done(store: &Store, id: i64) {
    let count = store.touch_project(id).unwrap();
    if count == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Marked progress on project {}. Keep shipping!", id);
}

fn cmd_add(store: &Store, name: &str) {
    let id = store.add_project(name).unwrap();
    println!("Added project '{}' to inbox (id: {})", name, id);
    println!(
        "Score it with: pm score {} -i <impact> -m <monetization> -r <readiness>",
        id
    );
}

fn cmd_score(store: &Store, id: i64, impact: u8, monetization: u8, readiness: u8) {
    let updated = store.update_scores(id, impact, monetization, readiness).unwrap();
    if updated == 0 {
        println!("Project {} not found", id);
        return;
    }
    let updated = store.update_state(id, ProjectState::Active).unwrap();
    if updated == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Updated project {} and moved to active.", id);
}

fn cmd_throne(store: &Store) {
    let projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects for the throne.");
        return;
    }

    let today = Local::now().date_naive();
    let mut scored: Vec<_> = projects.iter().map(|p| (p, p.priority_score(today))).collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    println!("THRONE (top priorities):\n");
    for (i, (p, score)) in scored.iter().take(3).enumerate() {
        let bar = progress_bar(p.readiness as usize, 10);
        println!(
            "  {}. {} {} {}%  (score: {})",
            i + 1,
            truncate(&p.name, 25),
            bar,
            p.readiness,
            score
        );
    }
}

fn cmd_why(store: &Store) {
    println!("Priority formula: (impact*3) + (monetization*2) + (readiness/10*4) - staleness_days");
    println!("\nReadiness weighted highest -> finishers win.");
    println!("Staleness penalized -> untouched projects drop.");

    let projects = store.list_active_projects().unwrap();
    if !projects.is_empty() {
        let today = Local::now().date_naive();
        let mut scored: Vec<_> = projects.iter().map(|p| (p, p.priority_score(today))).collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((top, score)) = scored.first() {
            println!("\nTop pick '{}' breakdown:", top.name);
            println!("  impact({})x3 = {}", top.impact, top.impact as i32 * 3);
            println!(
                "  monetization({})x2 = {}",
                top.monetization,
                top.monetization as i32 * 2
            );
            println!(
                "  readiness({}/10)x4 = {}",
                top.readiness,
                (top.readiness as i32 / 10) * 4
            );
            let stale = (today - top.last_activity).num_days() as i32;
            println!("  staleness penalty = -{}", stale.min(30));
            println!("  TOTAL = {}", score);
        }
    }
}

fn cmd_inbox(store: &Store) {
    let projects = store.list_inbox_projects().unwrap();
    if projects.is_empty() {
        println!("Inbox empty. Add ideas with: pm add \"idea\"");
        return;
    }

    println!("INBOX ({} items):\n", projects.len());
    for p in &projects {
        println!("  [{}] {}", p.id, p.name);
    }
    println!("\nScore items to move to active: pm score <id> -i <1-10> -m <1-10> -r <0-100>");
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn progress_bar(percent: usize, width: usize) -> String {
    let filled = (percent * width) / 100;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn cmd_link(store: &Store, id: i64, path: &str) {
    // Resolve to absolute path
    let abs_path = std::fs::canonicalize(path);
    let final_path = match abs_path {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => {
            println!("Error: path '{}' does not exist", path);
            return;
        }
    };

    // Verify it's a directory
    if !std::path::Path::new(&final_path).is_dir() {
        println!("Error: '{}' is not a directory", final_path);
        return;
    }

    let updated = store.link_project(id, &final_path).unwrap();
    if updated == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Linked project {} to {}", id, final_path);
    println!("Run 'pm scan' to detect progress from git and plan files.");
    println!("Run 'pm charter {}' to generate a project charter.", id);
}

fn cmd_scan(store: &Store) {
    let projects = store.list_linked_projects().unwrap();
    if projects.is_empty() {
        println!("No linked projects. Use 'pm link <id> <path>' to link a project to its codebase.");
        return;
    }

    println!("Scanning {} linked projects...\n", projects.len());

    for p in &projects {
        if let Some(ref path) = p.path {
            print!("  {} ", p.name);

            let result = scanner::scan_project(path);

            // Calculate readiness from task completion
            // Only override manual scores if there's actual progress detected
            let scan_readiness = if result.total_tasks > 0 {
                ((result.completed_tasks as f32 / result.total_tasks as f32) * 100.0) as u8
            } else {
                0
            };

            // Preserve manual scores: only update if scan shows progress OR no manual score exists
            let readiness = if result.completed_tasks > 0 || p.readiness == 0 {
                scan_readiness
            } else {
                p.readiness // Keep manual score when scan finds 0 completed
            };

            // Update last activity from git if available
            let last_activity = result.last_commit_date.unwrap_or(p.last_activity);

            // Update the project
            store.update_from_scan(p.id, readiness, last_activity).unwrap();

            // Report
            if result.total_tasks > 0 {
                println!(
                    "-> {}/{} tasks done ({}%), last commit: {}",
                    result.completed_tasks,
                    result.total_tasks,
                    readiness,
                    last_activity
                );
            } else {
                println!("-> no plan tasks found, last commit: {}", last_activity);
            }

            if !result.plan_files.is_empty() {
                println!("     plans: {}", result.plan_files.join(", "));
            }
            if result.has_progress_file {
                println!("     .pm-progress: active");
            }
            match result.charter_filled {
                Some((filled, total)) if filled == total => {
                    println!("     charter: complete");
                }
                Some((filled, total)) => {
                    println!("     charter: {}/{} filled (pm charter {})", filled, total, p.id);
                }
                None => {
                    println!("     charter: missing (pm charter {})", p.id);
                }
            }
            if result.plan_files.len() >= 3 && result.total_tasks >= 15 {
                println!(
                    "     hint: {} tasks across {} plans — consider splitting (pm tasks {})",
                    result.total_tasks,
                    result.plan_files.len(),
                    p.id
                );
            }
        }
    }

    println!("\nDone. Run 'pm throne' to see updated priorities.");
}

fn cmd_show(store: &Store, id: i64) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    let today = Local::now().date_naive();
    let days_stale = (today - project.last_activity).num_days();
    let score = project.priority_score(today);

    println!("Project: {}", project.name);
    println!("ID: {}", project.id);
    println!("State: {:?}", project.state);
    println!();
    println!("Scores:");
    println!("  Impact:       {}/10", project.impact);
    println!("  Monetization: {}/10", project.monetization);
    println!("  Readiness:    {}%", project.readiness);
    println!("  Priority:     {} (computed)", score);
    println!();
    println!("Dates:");
    println!("  Created:       {}", project.created_at);
    println!("  Last activity: {} ({} days ago)", project.last_activity, days_stale);
    if let Some(deadline) = project.soft_deadline {
        println!("  Soft deadline: {}", deadline);
    }
    println!();
    if let Some(ref path) = project.path {
        println!("Linked to: {}", path);

        // Show scan info
        let result = scanner::scan_project(path);
        if result.total_tasks > 0 {
            println!("Plan progress: {}/{} tasks", result.completed_tasks, result.total_tasks);
        }
        if !result.plan_files.is_empty() {
            println!("Plan files:");
            for f in &result.plan_files {
                println!("  - {}", f);
            }
        }
        if result.has_progress_file {
            println!(".pm-progress: active");
        }
        match result.charter_filled {
            Some((filled, total)) if filled == total => {
                println!("Charter: complete");
            }
            Some((filled, total)) => {
                println!("Charter: {}/{} sections filled (pm charter {})", filled, total, id);
            }
            None => {
                println!("Charter: missing (pm charter {})", id);
            }
        }
    } else {
        println!("Not linked to codebase. Use: pm link {} <path>", id);
    }
}

fn cmd_remove(store: &Store, id: i64) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    if project.deleted_at.is_some() {
        println!("Project '{}' is already deleted.", project.name);
        return;
    }

    // Generate random confirmation phrase
    let phrases = [
        "red fox", "blue moon", "green leaf", "dark sky", "cold wind",
        "warm sun", "deep lake", "tall tree", "fast car", "slow boat",
    ];
    let phrase = phrases[std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as usize % phrases.len()];

    println!("You are about to delete: {}", project.name);
    println!("This is a soft delete - recoverable for 30 days via 'pm restore {}'", id);
    println!();
    println!("To confirm, type: {}", phrase);
    print!("> ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        println!("Failed to read input. Aborting.");
        return;
    }

    if input.trim() != phrase {
        println!("Confirmation failed. Project NOT deleted.");
        return;
    }

    let deleted = store.soft_delete(id).unwrap();
    if deleted == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Deleted '{}'. Recoverable for 30 days with: pm restore {}", project.name, id);

    // Purge projects deleted more than 30 days ago
    let purged = store.purge_old_deleted(30).unwrap();
    if purged > 0 {
        println!("(Permanently removed {} project(s) deleted over 30 days ago)", purged);
    }
}

fn cmd_trash(store: &Store) {
    let projects = store.list_deleted_projects().unwrap();
    if projects.is_empty() {
        println!("Trash is empty.");
        return;
    }

    let today = Local::now().date_naive();
    println!("TRASH ({} items):\n", projects.len());

    for p in &projects {
        if let Some(deleted_at) = p.deleted_at {
            let days_ago = (today - deleted_at).num_days();
            let days_left = 30 - days_ago;
            println!(
                "  [{}] {} (deleted {} days ago, {} days to restore)",
                p.id, p.name, days_ago, days_left.max(0)
            );
        }
    }

    println!("\nRestore with: pm restore <id>");
}

fn cmd_restore(store: &Store, id: i64) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    if project.deleted_at.is_none() {
        println!("Project '{}' is not deleted.", project.name);
        return;
    }

    let restored = store.restore(id).unwrap();
    if restored == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Restored '{}'.", project.name);
}

fn cmd_rename(store: &Store, id: i64, name: &str) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    let old_name = project.name.clone();
    let renamed = store.rename_project(id, name).unwrap();
    if renamed == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Renamed '{}' -> '{}'", old_name, name);
}

fn cmd_park(store: &Store, id: i64, reason: &str) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    if project.state == ProjectState::Parked {
        println!("Project '{}' is already parked.", project.name);
        return;
    }

    let updated = store.update_state(id, ProjectState::Parked).unwrap();
    if updated == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Parked '{}'. Reason: {}", project.name, reason);
    println!("Revive with: pm score {} -i <impact> -m <money> -r <readiness>", id);
}

fn cmd_tasks(store: &Store, id: i64) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' is not linked to a codebase.", project.name);
            println!("Link with: pm link {} <path>", id);
            return;
        }
    };

    let tasks = scanner::list_tasks(Path::new(&path));
    if tasks.is_empty() {
        println!("No plan tasks found for '{}'.", project.name);
        println!("Add plans in docs/plans/*.md with '### Task N: description' headers.");
        return;
    }

    println!("Tasks for: {}\n", project.name);

    let mut current_plan = String::new();
    for t in &tasks {
        if t.plan_file != current_plan {
            if !current_plan.is_empty() {
                println!();
            }
            println!("  {}", t.plan_file);
            current_plan = t.plan_file.clone();
        }

        let marker = match t.source {
            TaskSource::Manual => "x",
            TaskSource::Git => "~",
            TaskSource::Pending => " ",
        };
        println!("    [{}] Task {}: {}", marker, t.task_number, t.description);
    }

    println!("\nLegend: [x] = marked done, [~] = detected from git, [ ] = pending");
    println!("Mark tasks: pm mark {} <task-number>", id);

    // Suggest splitting if project looks unwieldy
    let plan_count = {
        let mut seen = std::collections::HashSet::new();
        for t in &tasks {
            seen.insert(&t.plan_file);
        }
        seen.len()
    };
    if plan_count >= 3 && tasks.len() >= 15 {
        println!();
        println!(
            "Hint: {} has {} tasks across {} plans. Consider splitting into",
            project.name,
            tasks.len(),
            plan_count
        );
        println!("separate projects that can be stitched together later.");
        println!("Use 'pm add \"sub-project\"' and link each to its own repo/branch.");
    }
}

fn cmd_charter(store: &Store, id: i64, force: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' is not linked to a codebase.", project.name);
            println!("Link with: pm link {} <path>", id);
            return;
        }
    };

    match charter::generate_charter(Path::new(&path), &project.name, force) {
        Ok(charter::CharterAction::Created) => {
            println!("Created docs/CHARTER.md for '{}'.", project.name);
            println!("Fill in the 9 sections, then run 'pm charter {}' to check progress.", id);
        }
        Ok(charter::CharterAction::AlreadyExists(filled, total)) => {
            if filled == total {
                println!("Charter: complete ({}/{})", filled, total);
            } else {
                println!("Charter: {}/{} sections filled", filled, total);
            }
            println!("Use --force to regenerate: pm charter {} --force", id);
        }
        Ok(charter::CharterAction::Overwritten) => {
            println!("Overwritten docs/CHARTER.md for '{}'.", project.name);
            println!("Fill in the 9 sections, then run 'pm charter {}' to check progress.", id);
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn cmd_mark(store: &Store, id: i64, task_number: usize, plan: Option<String>) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };

    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' is not linked to a codebase.", project.name);
            return;
        }
    };

    let project_path = Path::new(&path);

    // Determine plan file
    let plan_file = match plan {
        Some(p) => p,
        None => {
            // Find the most recent plan file (last alphabetically — works with date prefixes)
            let plans_dir = project_path.join("docs").join("plans");
            if !plans_dir.exists() {
                println!("No plans directory found at docs/plans/");
                return;
            }
            let mut plans: Vec<String> = match std::fs::read_dir(&plans_dir) {
                Ok(entries) => entries
                    .flatten()
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "md")
                            .unwrap_or(false)
                    })
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .collect(),
                Err(_) => {
                    println!("Could not read plans directory.");
                    return;
                }
            };
            plans.sort();
            match plans.last() {
                Some(p) => p.clone(),
                None => {
                    println!("No plan files found in docs/plans/");
                    return;
                }
            }
        }
    };

    // Validate task number exists
    let tasks = scanner::list_tasks(project_path);
    let valid = tasks
        .iter()
        .any(|t| t.plan_file == plan_file && t.task_number == task_number);
    if !valid {
        println!(
            "Task {} not found in {}. Run 'pm tasks {}' to see available tasks.",
            task_number, plan_file, id
        );
        return;
    }

    // Check if already marked
    let progress = scanner::read_progress_file(project_path);
    if progress.contains(&(plan_file.clone(), task_number)) {
        println!("Task {} in {} is already marked done.", task_number, plan_file);
        return;
    }

    // Append to .pm-progress
    let progress_path = project_path.join(".pm-progress");
    let first_creation = !progress_path.exists();

    let entry = format!("{}:{}\n", plan_file, task_number);
    let mut content = if progress_path.exists() {
        std::fs::read_to_string(&progress_path).unwrap_or_default()
    } else {
        "# Manually marked tasks\n".to_string()
    };
    content.push_str(&entry);
    std::fs::write(&progress_path, content).unwrap();

    println!(
        "Marked task {} done in {} for '{}'.",
        task_number, plan_file, project.name
    );

    if first_creation {
        println!("\nTip: Add .pm-progress to your .gitignore:");
        println!("  echo .pm-progress >> {}/.gitignore", path);
    }
}
