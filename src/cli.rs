use crate::domain::ProjectState;
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
