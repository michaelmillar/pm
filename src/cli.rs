use crate::domain::ProjectState;
use crate::scanner;
use crate::store::Store;
use chrono::Local;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    store.touch_project(id).unwrap();
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
    store.update_scores(id, impact, monetization, readiness).unwrap();
    store.update_state(id, ProjectState::Active).unwrap();
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

    store.link_project(id, &final_path).unwrap();
    println!("Linked project {} to {}", id, final_path);
    println!("Run 'pm scan' to detect progress from git and plan files.");
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

    store.soft_delete(id).unwrap();
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

    store.restore(id).unwrap();
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
    store.rename_project(id, name).unwrap();
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

    store.update_state(id, ProjectState::Parked).unwrap();
    println!("Parked '{}'. Reason: {}", project.name, reason);
    println!("Revive with: pm score {} -i <impact> -m <money> -r <readiness>", id);
}
