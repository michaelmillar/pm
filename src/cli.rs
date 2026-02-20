use crate::charter;
use crate::discovery;
use crate::dod;
use crate::roadmap;
use crate::research;
use crate::domain::{ProjectState, TaskSource};
use crate::naming;
use crate::scanner;
use crate::standards;
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
    /// Scaffold or update docs/roadmap.yaml for a project
    Roadmap {
        /// Project ID
        id: i64,
        /// Add a component: name and relative path
        #[arg(long, value_names = ["NAME", "PATH"], num_args = 2)]
        add_component: Option<Vec<String>>,
        /// Overwrite existing roadmap.yaml
        #[arg(long)]
        force: bool,
    },
    /// Run or view competitive research for a project
    Research {
        /// Project ID (omit with --scheduled to scan all overdue projects)
        id: Option<i64>,
        /// Show last diff without re-running
        #[arg(long)]
        diff: bool,
        /// Show full current summary without re-running
        #[arg(long)]
        full: bool,
        /// Force re-run even if not due
        #[arg(long)]
        refresh: bool,
        /// Run research for all overdue active projects
        #[arg(long)]
        scheduled: bool,
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
    /// Move a project back to inbox
    Unscore {
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
    /// Interactive human sign-off for DOD criteria
    Signoff {
        /// Project ID
        id: i64,
        /// Show all criteria including unverified ones
        #[arg(long)]
        all: bool,
    },
    /// Run automated DOD verification via Claude
    Verify {
        /// Project ID
        id: i64,
        /// Re-run all criteria including already-passed ones
        #[arg(long)]
        all: bool,
        /// Run only this criterion (e.g. C1)
        #[arg(long)]
        criterion: Option<String>,
    },
    /// Initialise or show a project's Definition of Done
    Dod {
        /// Project ID
        id: i64,
        /// Show DOD status instead of initialising
        #[arg(long)]
        show: bool,
        /// Overwrite existing DOD.md
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
    /// Suggest novel pivot ideas when a project is competed out
    Pivot {
        /// Project ID
        id: i64,
        /// Number of ideas to generate (default 3)
        #[arg(long, default_value = "3")]
        count: usize,
        /// Re-run even if pivot was run recently
        #[arg(long)]
        refresh: bool,
    },
}

pub fn run() {
    let cli = Cli::parse();
    let store = open_store();

    // Startup: nudge if any project is overdue for research
    check_research_due_notice(&store);

    match cli.command {
        Commands::Next => cmd_next(&store),
        Commands::Status => cmd_status(&store),
        Commands::Done { id } => cmd_done(&store, id),
        Commands::Add { name } => cmd_add(&store, &name),
        Commands::Roadmap { id, add_component, force } => cmd_roadmap(&store, id, add_component, force),
        Commands::Research { id, diff, full, refresh, scheduled } => {
            cmd_research(&store, id, diff, full, refresh, scheduled)
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
        Commands::Unscore { id } => cmd_unscore(&store, id),
        Commands::Rename { id, name } => cmd_rename(&store, id, &name),
        Commands::Park { id, reason } => cmd_park(&store, id, &reason),
        Commands::Tasks { id } => cmd_tasks(&store, id),
        Commands::Charter { id, force } => cmd_charter(&store, id, force),
        Commands::Signoff { id, all } => cmd_signoff(&store, id, all),
        Commands::Verify { id, all, criterion } => cmd_verify(&store, id, all, criterion),
        Commands::Dod { id, show, force } => cmd_dod(&store, id, show, force),
        Commands::Mark {
            id,
            task_number,
            plan,
        } => cmd_mark(&store, id, task_number, plan),
        Commands::Pivot { id, count, refresh } => cmd_pivot(&store, id, count, refresh),
    }
}

fn open_store() -> Store {
    let data_dir = resolve_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("pm.db");
    Store::open(&db_path).expect("Failed to open database")
}

fn resolve_data_dir() -> PathBuf {
    if let Ok(val) = std::env::var("PM_DATA_DIR") {
        return PathBuf::from(val);
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pm")
}

fn enrich_with_roadmap(project: &mut crate::domain::Project) {
    let Some(ref path) = project.path else { return; };
    let Some(scores) = roadmap::load_scores(std::path::Path::new(path)) else { return; };
    project.readiness = scores.readiness;
    if let Some(v) = scores.impact { project.impact = v; }
    if let Some(v) = scores.monetization { project.monetization = v; }
    project.cloneability = scores.cloneability;
    project.uniqueness = scores.uniqueness;
}

fn cmd_next(store: &Store) {
    let mut projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects. Add one with: pm add \"project name\"");
        return;
    }
    for p in &mut projects { enrich_with_roadmap(p); }

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

pub fn cmd_status(store: &Store) {
    let root = resolve_root_dir();
    let _ = discovery::discover_projects(store, &root);

    let mut projects = store.list_active_projects().unwrap();
    for p in &mut projects { enrich_with_roadmap(p); }
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
        // Show DOD rollup if DOD.md exists
        if let Some(ref path) = p.path {
            if let Some(dod_file) = dod::load_dod(std::path::Path::new(path)) {
                let (complete, total) = dod::rollup(&dod_file);
                if total > 0 {
                    let dod_bar: String = dod_file.criteria.iter().map(|c| {
                        if c.automated.is_done() && c.human.is_done() { '✓' }
                        else if c.automated.is_done() { '·' }
                        else { '✗' }
                    }).collect();
                    println!("       DOD: {}/{} [{}]", complete, total, dod_bar);
                }
            }
            // Cut-losses warning
            if let Ok(Some(rec)) = store.get_research(p.id) {
                if rec.consecutive_flags >= 2 {
                    println!("       ⚠  Research recommends re-evaluating this project.");
                }
            }
        }
    }

    let suggestions = collect_naming_suggestions(&projects);
    if !suggestions.is_empty() {
        println!("\nNaming suggestions:");
        for (id, name, ideas) in suggestions {
            println!("  [{}] {} -> {}", id, name, ideas.join(", "));
        }
    }
}

fn resolve_root_dir() -> PathBuf {
    if let Ok(val) = std::env::var("PM_ROOT") {
        return PathBuf::from(val);
    }
    PathBuf::from("/home/markw/projects")
}

fn cmd_done(store: &Store, id: i64) {
    let count = store.touch_project(id).unwrap();
    if count == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Marked progress on project {}. Keep shipping!", id);
}

fn cmd_unscore(store: &Store, id: i64) {
    let count = store.move_to_inbox(id).unwrap();
    if count == 0 {
        println!("Project {} not found", id);
        return;
    }
    println!("Project {} moved to inbox", id);
}

fn cmd_add(store: &Store, name: &str) {
    let id = store.add_project(name).unwrap();
    println!("Added project '{}' to inbox (id: {})", name, id);
    println!(
        "Link and create a roadmap: pm link {} <path> && pm roadmap {}",
        id, id
    );
}

fn cmd_roadmap(store: &Store, id: i64, add_component: Option<Vec<String>>, force: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' is not linked to a codebase.", project.name);
            println!("Link first: pm link {} <path>", id);
            return;
        }
    };
    let project_path = Path::new(&path);
    let docs_dir = project_path.join("docs");
    let yaml_path = docs_dir.join("roadmap.yaml");

    if let Some(parts) = add_component {
        if !yaml_path.exists() {
            println!("No roadmap.yaml found. Run 'pm roadmap {}' first.", id);
            return;
        }
        let name = &parts[0];
        let comp_path = &parts[1];
        let existing = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        let component_entry = format!(
            "  - id: {}\n    label: {}\n    path: {}\n",
            name.to_lowercase().replace(' ', "-"), name, comp_path
        );
        let updated = if existing.contains("components:") {
            existing.replacen("phases:", &format!("{}\nphases:", component_entry), 1)
        } else {
            existing.replacen("phases:", &format!("components:\n{}\nphases:", component_entry), 1)
        };
        std::fs::write(&yaml_path, updated).expect("write roadmap.yaml");
        println!("Added component '{}' at '{}' to roadmap.", name, comp_path);
        return;
    }

    if yaml_path.exists() && !force {
        println!("docs/roadmap.yaml already exists for '{}'.", project.name);
        println!("Edit it directly, use --add-component to register a new repo, or --force to rebuild from plan files.");
        return;
    }

    std::fs::create_dir_all(&docs_dir).expect("create docs/");

    // Preserve existing assessment block if present (from previous research)
    let existing_assessment = if yaml_path.exists() {
        std::fs::read_to_string(&yaml_path)
            .ok()
            .and_then(|content| extract_assessment_block(&content))
    } else {
        None
    };

    let tasks = scanner::list_tasks(project_path);
    let yaml = if tasks.is_empty() {
        println!("No plan files found in docs/plans/ — generating blank template.");
        roadmap::scaffold_template(&project.name)
    } else {
        // Show what we found
        let mut phase_order: Vec<String> = Vec::new();
        for t in &tasks {
            if !phase_order.contains(&t.plan_file) {
                phase_order.push(t.plan_file.clone());
            }
        }
        println!("Building roadmap from {} plan file(s):\n", phase_order.len());
        for plan in &phase_order {
            let total = tasks.iter().filter(|t| &t.plan_file == plan).count();
            let done = tasks.iter().filter(|t| &t.plan_file == plan && t.source != crate::domain::TaskSource::Pending).count();
            let (_, label) = phase_label_from_filename(plan);
            println!("  {} — {}/{} done", label, done, total);
        }
        println!();

        // Use existing assessment or prompt for scores
        let assessment_block = if let Some(ref existing) = existing_assessment {
            println!("Using existing assessment (run 'pm research {}' to update).\n", id);
            existing.clone()
        } else {
            println!("Assessment scores (Enter to use default; run 'pm research {}' to research properly):", id);
            let impact       = prompt_score("  Impact       (1-10, how many people need this?)  ").unwrap_or(7);
            let monetization = prompt_score("  Monetization (1-10, how well can it be monetised?)").unwrap_or(7);
            let cloneability = prompt_score("  Cloneability (1-10, how hard to copy the value?) ").unwrap_or(6);
            default_assessment_block(impact, monetization, cloneability)
        };

        build_roadmap_yaml(&project.name, &tasks, &phase_order, &assessment_block)
    };

    std::fs::write(&yaml_path, yaml).expect("write roadmap.yaml");
    store.update_state(id, crate::domain::ProjectState::Active).ok();
    println!("\nCreated docs/roadmap.yaml for '{}'.", project.name);
    println!("Review weights in the file, then run 'pm show {}' to see live readiness.", id);
}

fn phase_label_from_filename(filename: &str) -> (String, String) {
    let stem = filename.trim_end_matches(".md");
    // Strip YYYY-MM-DD- date prefix if present
    let stripped = if stem.len() > 10
        && stem.as_bytes().get(4) == Some(&b'-')
        && stem.as_bytes().get(7) == Some(&b'-')
        && stem.as_bytes().get(10) == Some(&b'-')
    {
        &stem[11..]
    } else {
        stem
    };
    let id = stripped.replace('_', "-").to_lowercase();
    let label = id
        .split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    (id, label)
}

fn prompt_score(question: &str) -> Option<u8> {
    use std::io::Write;
    print!("{}: ", question);
    std::io::stdout().flush().ok();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u8>().ok().filter(|&n| (1..=10).contains(&n))
}

fn extract_assessment_block(yaml_content: &str) -> Option<String> {
    let start = yaml_content.find("assessment:")?;
    let end = yaml_content.find("\nphases:")?;
    if end > start {
        Some(yaml_content[start..end].trim_end().to_string())
    } else {
        None
    }
}

fn default_assessment_block(impact: u8, monetization: u8, cloneability: u8) -> String {
    let today = Local::now().date_naive();
    format!(
        "assessment:\n  impact: {impact}\n  monetization: {monetization}\n  cloneability: {cloneability}\n  researched_at: \"{today}\"\n  reasoning: |\n    Impact {impact}: (run 'pm research <id>' to fill in)\n    Monetization {monetization}: (run 'pm research <id>' to fill in)\n    Cloneability {cloneability}: (run 'pm research <id>' to fill in)\n  signals:\n    - \"(add market evidence here)\"",
        impact = impact,
        monetization = monetization,
        cloneability = cloneability,
        today = today,
    )
}

fn build_roadmap_yaml(
    project_name: &str,
    tasks: &[crate::domain::TaskStatus],
    phase_order: &[String],
    assessment_block: &str,
) -> String {
    let n = phase_order.len();
    let total_tasks: usize = tasks.len();

    let mut yaml = format!(
        "project: {name}\n{assessment}\n\nphases:\n",
        name = project_name,
        assessment = assessment_block,
    );

    for (i, plan_file) in phase_order.iter().enumerate() {
        let phase_tasks: Vec<_> = tasks.iter().filter(|t| &t.plan_file == plan_file).collect();
        let (phase_id, phase_label) = phase_label_from_filename(plan_file);

        // Weight proportional to task count; last phase absorbs rounding remainder
        let weight = if n == 1 {
            1.0f64
        } else if i == n - 1 {
            let so_far: f64 = phase_order[..i].iter().map(|pf| {
                let cnt = tasks.iter().filter(|t| &t.plan_file == pf).count();
                (cnt as f64 / total_tasks as f64 * 100.0).round() / 100.0
            }).sum();
            (1.0 - so_far).max(0.01)
        } else {
            let cnt = phase_tasks.len();
            (cnt as f64 / total_tasks as f64 * 100.0).round() / 100.0
        };

        yaml.push_str(&format!(
            "  - id: {id}\n    label: {label}\n    weight: {weight:.2}\n    tasks:\n",
            id = phase_id,
            label = phase_label,
            weight = weight,
        ));

        for task in &phase_tasks {
            let task_id = format!("{}-{}", phase_id, task.task_number);
            let done = task.source != crate::domain::TaskSource::Pending;
            let label = task.description.replace('"', "'");
            yaml.push_str(&format!(
                "      - id: {task_id}\n        label: \"{label}\"\n        done: {done}\n",
                task_id = task_id,
                label = label,
                done = done,
            ));
        }
        yaml.push('\n');
    }

    yaml
}

fn cmd_research(store: &Store, id: Option<i64>, show_diff: bool, show_full: bool, refresh: bool, scheduled: bool) {
    let freq = research::load_frequency();

    if scheduled {
        let projects = store.list_active_projects().unwrap();
        let due: Vec<_> = projects.iter()
            .filter(|p| p.path.is_some())
            .filter(|p| {
                let rec = store.get_research(p.id).unwrap();
                let researched_at = rec.as_ref().and_then(|r| r.researched_at.as_deref());
                research::is_research_due(researched_at, &freq)
            })
            .collect();

        if due.is_empty() {
            println!("No projects due for research.");
            return;
        }
        println!("Running scheduled research for {} project(s)...\n", due.len());
        for p in due {
            run_research_for_project(store, p.id, false);
        }
        return;
    }

    let id = match id {
        Some(i) => i,
        None => {
            println!("Specify a project ID: pm research <id>");
            println!("Or run: pm research --scheduled");
            return;
        }
    };

    run_research_for_project(store, id, refresh || show_diff || show_full);

    if show_diff {
        print_research_diff(store, id);
        return;
    }
    if show_full {
        print_research_full(store, id);
        return;
    }
}

fn run_research_for_project(store: &Store, id: i64, force: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };

    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' has no linked path.", project.name);
            return;
        }
    };

    let freq = research::load_frequency();
    let rec = store.get_research(id).unwrap();
    let researched_at = rec.as_ref().and_then(|r| r.researched_at.as_deref());

    if !force && !research::is_research_due(researched_at, &freq) {
        println!("'{}' was researched recently. Use --refresh to force.", project.name);
        print_research_diff(store, id);
        return;
    }

    // Extract USP for prompt
    let usp = dod::extract_usp_from_charter(std::path::Path::new(&path))
        .unwrap_or_else(|| project.name.clone());

    println!("Researching '{}' via Claude (Codex fallback enabled)...", project.name);

    match research::run_research_claude(&project.name, &usp) {
        Err(e) => println!("Error: {}", e),
        Ok(current_summary) => {
            // Compute diff if we have previous data
            let diff_output = rec.as_ref()
                .filter(|r| !r.summary.is_empty())
                .and_then(|prev| {
                    let prev_date = prev.researched_at.as_deref().unwrap_or("(unknown date)");
                    research::run_diff_claude(&project.name, &usp, &prev.summary, &current_summary, prev_date).ok()
                });

            // Save to DB (rotates previous automatically)
            store.save_research(id, &current_summary).unwrap();

            println!("\n'{}' research complete.", project.name);

            if let Some(diff) = diff_output {
                println!("\n{}", diff);
            } else {
                println!("\n{}", current_summary);
            }

            // Check cut-losses warning
            let updated_rec = store.get_research(id).unwrap();
            if let Some(r) = updated_rec {
                if r.consecutive_flags >= 2 {
                    println!("\n⚠  Research has flagged '{}' for re-evaluation {} times in a row.", project.name, r.consecutive_flags);
                    println!("   Run 'pm research {} --full' to review.", id);
                }
            }
        }
    }
}

fn print_research_diff(store: &Store, id: i64) {
    match store.get_research(id).unwrap() {
        None => println!("No research data. Run: pm research {}", id),
        Some(r) => {
            println!("{}", r.summary);
        }
    }
}

fn print_research_full(store: &Store, id: i64) {
    match store.get_research(id).unwrap() {
        None => println!("No research data. Run: pm research {}", id),
        Some(r) => println!("{}", r.summary),
    }
}

fn check_research_due_notice(store: &Store) {
    let freq = research::load_frequency();
    if matches!(freq, research::ResearchFrequency::Never) {
        return;
    }
    let projects = store.list_active_projects().unwrap_or_default();
    let due: Vec<_> = projects.iter()
        .filter(|p| p.path.is_some())
        .filter(|p| {
            let rec = store.get_research(p.id).unwrap_or(None);
            let researched_at = rec.as_ref().and_then(|r| r.researched_at.as_deref());
            research::is_research_due(researched_at, &freq)
        })
        .map(|p| p.name.as_str().to_string())
        .collect();

    if !due.is_empty() {
        println!("ℹ  Research due for: {}", due.join(", "));
        println!("   Run: pm research --scheduled\n");
    }
}

fn cmd_throne(store: &Store) {
    let mut projects = store.list_active_projects().unwrap();
    for p in &mut projects { enrich_with_roadmap(p); }
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
    let root = resolve_root_dir();
    let untracked = discovery::list_nonrepo_folders(&root);
    if projects.is_empty() && untracked.is_empty() {
        println!("Inbox empty. Add ideas with: pm add \"idea\"");
        return;
    }

    println!("INBOX ({} items):\n", projects.len());
    for p in &projects {
        println!("  [{}] {}", p.id, p.name);
        if let Ok(Some(note)) = store.get_inbox_note(p.id) {
            println!("       {}", truncate(&note, 60));
        }
    }
    let possible = store.list_possible_duplicates(0.80).unwrap();
    if !possible.is_empty() {
        println!("\nPossible duplicates:");
        for p in &possible {
            let score = p.possible_duplicate_score.unwrap_or(0.0);
            println!("  [{}] {}  ({:.2})", p.id, p.name, score);
        }
    }

    let suggestions = collect_naming_suggestions(&projects);
    if !suggestions.is_empty() {
        println!("\nNaming suggestions:");
        for (id, name, ideas) in suggestions {
            println!("  [{}] {} -> {}", id, name, ideas.join(", "));
        }
    }

    if !untracked.is_empty() {
        println!("\nUntracked folders:");
        for name in untracked {
            println!("  {}", name);
        }
    }

    println!("\nLink and create a roadmap to activate: pm link <id> <path> && pm roadmap <id>");
}

fn collect_naming_suggestions(
    projects: &[crate::domain::Project],
) -> Vec<(i64, String, Vec<String>)> {
    let mut out = Vec::new();
    for p in projects {
        let Some(path) = &p.path else {
            continue;
        };
        let repo_path = Path::new(path);
        if !repo_path.is_dir() {
            continue;
        }
        let readme = read_readme_text(repo_path);
        let plans = read_plans_text(repo_path);
        let ideas = naming::suggest_names(&p.name, &readme, &plans);
        if !ideas.is_empty() {
            out.push((p.id, p.name.clone(), ideas));
        }
    }
    out
}

fn read_readme_text(repo_path: &Path) -> String {
    if let Ok(entries) = std::fs::read_dir(repo_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_lowercase();
            if file_name.starts_with("readme") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    return content;
                }
            }
        }
    }
    String::new()
}

fn read_plans_text(repo_path: &Path) -> String {
    let plans_dir = repo_path.join("docs").join("plans");
    if !plans_dir.is_dir() {
        return String::new();
    }
    let mut combined = String::new();
    if let Ok(entries) = std::fs::read_dir(plans_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    combined.push_str(&content);
                    combined.push('\n');
                }
            }
        }
    }
    combined
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        let mut out: String = chars.into_iter().collect();
        let pad = max.saturating_sub(out.chars().count());
        out.extend(std::iter::repeat(' ').take(pad));
        out
    } else {
        let slice: String = chars.into_iter().take(max - 3).collect();
        format!("{}...", slice)
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

    let standards_config = standards::StandardsConfig::load().ok();

    println!("Scanning {} linked projects...\n", projects.len());

    for p in &projects {
        if let Some(ref path) = p.path {
            print!("  {} ", p.name);

            let result = scanner::scan_project(path);

            // If roadmap.yaml exists, readiness is computed live — skip DB update.
            let has_roadmap = roadmap::load_roadmap(Path::new(path)).is_some();
            let mut readiness = if has_roadmap {
                p.readiness
            } else {
                let scan_readiness = if result.total_tasks > 0 {
                    ((result.completed_tasks as f32 / result.total_tasks as f32) * 100.0) as u8
                } else {
                    0
                };
                let r = if result.completed_tasks > 0 || p.readiness == 0 {
                    scan_readiness
                } else {
                    p.readiness
                };
                r
            };
            if !has_roadmap {
                if let Some(cfg) = &standards_config {
                    if let Ok(report) = standards::evaluate_repo(Path::new(path), cfg) {
                        let boosted = readiness as i32 + report.readiness_boost as i32;
                        readiness = boosted.min(100) as u8;
                    }
                }
            }

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
    let mut project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => {
            println!("Project {} not found", id);
            return;
        }
    };
    enrich_with_roadmap(&mut project);

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
    if let Some(c) = project.cloneability {
        println!("  Cloneability: {}/10", c);
    }
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
        let project_path = Path::new(path);
        println!("Linked to: {}", path);

        // Show roadmap details if present
        if let Some(rm) = roadmap::load_roadmap(project_path) {
            if let Some(bad_sum) = roadmap::validate_weights(&rm) {
                println!("⚠  Phase weights sum to {:.2} (expected 1.0)", bad_sum);
            }
            match &rm.assessment {
                None => println!("  No assessment — run 'pm research {}' to add scores.", id),
                Some(a) if roadmap::is_assessment_stale(&a.researched_at) => {
                    println!("⚠  Assessment is stale. Run 'pm research {}' to refresh.", id);
                }
                _ => {}
            }
            println!("\nRoadmap phases:");
            for phase in &rm.phases {
                if phase.tasks.is_empty() { continue; }
                let done = phase.tasks.iter().filter(|t| t.done).count();
                let total = phase.tasks.len();
                let pct = (done as f64 / total as f64 * 100.0).round() as usize;
                let bar = progress_bar(pct, 10);
                let comp = phase.component.as_deref().unwrap_or("—");
                println!("  {} ({})  {} {}%  [{}/{}]", phase.label, comp, bar, pct, done, total);
            }
            println!();
        } else {
            println!("  No roadmap.yaml — run 'pm roadmap {}' to enable algorithmic scoring.", id);
        }

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

        // Show DOD status
        match dod::load_dod(project_path) {
            None => {
                println!("DOD: not initialised (pm dod {})", id);
            }
            Some(ref dod_file) => {
                let (complete, total) = dod::rollup(dod_file);
                println!("DOD: {}/{} criteria complete", complete, total);
                for c in &dod_file.criteria {
                    let auto_sym = if c.automated.is_done() { "✓" } else { "–" };
                    let human_sym = if c.human.is_done() { "✓" } else { "–" };
                    println!("  [{}] auto:{} human:{}  {}", c.id, auto_sym, human_sym, c.description);
                }
                println!();
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
    println!("Revive with: pm roadmap {} (if linked)", id);
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

    let inbox_note = store.get_inbox_note(id).ok().flatten();
    match charter::generate_charter(Path::new(&path), &project.name, force, inbox_note.as_deref()) {
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
        Some(p) => normalize_plan_file(&p),
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

fn cmd_signoff(store: &Store, id: i64, all: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' has no linked path.", project.name);
            return;
        }
    };
    let project_path = Path::new(&path);

    let mut dod_file = match dod::load_dod(project_path) {
        Some(d) => d,
        None => {
            println!("No DOD.md found. Run: pm dod {}", id);
            return;
        }
    };

    let today = chrono::Local::now().date_naive();

    // Criteria eligible for sign-off: automated pass (unless --all)
    let eligible: Vec<usize> = dod_file.criteria.iter().enumerate()
        .filter(|(_, c)| {
            if c.human.is_done() { return false; } // already signed off
            if !all && !c.automated.is_done() { return false; } // not yet auto-verified
            true
        })
        .map(|(i, _)| i)
        .collect();

    if eligible.is_empty() {
        println!("Nothing to sign off.");
        if !all {
            println!("Run 'pm verify {}' first, or use --all to sign off unverified criteria.", id);
        }
        return;
    }

    println!("{} — Human Sign-off\n", project.name);
    println!("You are reviewing these criteria as an external user/payer.\n");

    let mut changed = 0;

    for idx in eligible {
        let c = &dod_file.criteria[idx];
        println!("─────────────────────────────────────────────────");
        println!("[{}] {}", c.id, c.description);
        println!("  Automated: {}", c.automated.label());

        // Show automated rationale if available
        let auto_rationale = match &c.automated {
            crate::dod::CriterionStatus::Pass { rationale, .. }
            | crate::dod::CriterionStatus::Fail { rationale, .. }
            | crate::dod::CriterionStatus::Inconclusive { rationale, .. } => rationale.as_deref(),
            _ => None,
        };

        println!("\nScenario:");
        for line in c.scenario.lines() {
            println!("  {}", line);
        }
        println!();
        println!("Sign off as external user/payer? [y/n/skip/?] ");

        loop {
            use std::io::Write;
            print!("> ");
            std::io::stdout().flush().ok();
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                break;
            }
            match input.trim() {
                "y" => {
                    print!("Note (Enter to skip): ");
                    std::io::stdout().flush().ok();
                    let mut note = String::new();
                    let _ = std::io::stdin().read_line(&mut note);
                    let rationale = if note.trim().is_empty() { None } else { Some(note.trim().to_string()) };
                    dod_file.criteria[idx].human = crate::dod::CriterionStatus::Pass { date: today, rationale };
                    println!("✓ Signed off.\n");
                    changed += 1;
                    break;
                }
                "n" => {
                    print!("Why? (required): ");
                    std::io::stdout().flush().ok();
                    let mut reason = String::new();
                    let _ = std::io::stdin().read_line(&mut reason);
                    let rationale = if reason.trim().is_empty() { Some("Failed sign-off.".to_string()) } else { Some(reason.trim().to_string()) };
                    dod_file.criteria[idx].human = crate::dod::CriterionStatus::Fail { date: today, rationale };
                    println!("✗ Marked fail.\n");
                    changed += 1;
                    break;
                }
                "skip" => {
                    println!("Skipped.\n");
                    break;
                }
                "?" => {
                    println!("Automated rationale: {}", auto_rationale.unwrap_or("(none)"));
                }
                _ => {
                    println!("Enter y, n, skip, or ? for rationale.");
                }
            }
        }
    }

    if changed > 0 {
        if let Err(e) = dod::save_dod(project_path, &dod_file) {
            println!("Error saving DOD.md: {}", e);
        } else {
            let (complete, total) = dod::rollup(&dod_file);
            println!("Saved. DOD: {}/{} criteria fully complete.", complete, total);
        }
    } else {
        println!("No changes made.");
    }
}

fn cmd_verify(store: &Store, id: i64, all: bool, criterion_filter: Option<String>) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' has no linked path. Use: pm link {} <path>", project.name, id);
            return;
        }
    };
    let project_path = Path::new(&path);

    let mut dod_file = match dod::load_dod(project_path) {
        Some(d) => d,
        None => {
            println!("No DOD.md found. Run: pm dod {}", id);
            return;
        }
    };

    // Build file tree for context
    let file_tree = build_file_tree(project_path, 50);

    let mut verified_count = 0;
    let criteria_to_run: Vec<usize> = dod_file.criteria.iter().enumerate()
        .filter(|(_, c)| {
            // Filter by criterion ID if specified
            if let Some(ref filter) = criterion_filter {
                if &c.id != filter { return false; }
            }
            // Skip already-passing unless --all
            if !all && c.automated.is_done() { return false; }
            true
        })
        .map(|(i, _)| i)
        .collect();

    if criteria_to_run.is_empty() {
        println!("No criteria to verify. All passing? Use --all to re-run.");
        return;
    }

    println!("Verifying {} criterion/criteria for '{}' via Claude...\n", criteria_to_run.len(), project.name);

    for idx in criteria_to_run {
        let c = &dod_file.criteria[idx];
        println!("[{}] {}...", c.id, truncate(&c.description, 50));

        // Load evidence file if hint exists
        let evidence_content = c.evidence.as_ref().and_then(|hint| {
            // Strip backticks from evidence hint
            let clean = hint.trim_matches('`');
            let ev_path = project_path.join(clean);
            std::fs::read_to_string(ev_path).ok()
        });

        let result = research::run_verify_claude(
            &project.name,
            &dod_file.usp,
            &c.id,
            &c.description,
            &c.scenario,
            c.evidence.as_deref(),
            evidence_content.as_deref(),
            &file_tree,
        );

        let today = chrono::Local::now().date_naive();

        match result {
            Err(e) => {
                println!("  Error: {}", e);
                println!("  Skipping — check Claude/Codex CLI availability.\n");
            }
            Ok(output) => {
                let (verdict, rationale) = research::parse_verdict(&output);
                let new_status = match verdict.as_str() {
                    "pass" => crate::dod::CriterionStatus::Pass { date: today, rationale: rationale.clone() },
                    "fail" => crate::dod::CriterionStatus::Fail { date: today, rationale: rationale.clone() },
                    _ => crate::dod::CriterionStatus::Inconclusive { date: today, rationale: rationale.clone() },
                };
                println!("  {} — {}", verdict.to_uppercase(), rationale.as_deref().unwrap_or("(no rationale)"));
                dod_file.criteria[idx].automated = new_status;
                verified_count += 1;
            }
        }
        println!();
    }

    // Write updated DOD.md
    if verified_count > 0 {
        if let Err(e) = dod::save_dod(project_path, &dod_file) {
            println!("Error saving DOD.md: {}", e);
        } else {
            println!("Updated docs/DOD.md with {} result(s).", verified_count);
            let (complete, total) = dod::rollup(&dod_file);
            println!("DOD: {}/{} criteria fully complete.", complete, total);
        }
    }
}

fn build_file_tree(path: &Path, limit: usize) -> String {
    let mut lines = Vec::new();
    collect_tree(path, path, 0, &mut lines, limit);
    lines.join("\n")
}

fn collect_tree(root: &Path, path: &Path, depth: usize, lines: &mut Vec<String>, limit: usize) {
    if lines.len() >= limit { return; }
    if depth > 3 { return; }

    let indent = "  ".repeat(depth);
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    // Skip hidden dirs and target/
    if name.starts_with('.') || name == "target" || name == "node_modules" {
        return;
    }

    if path.is_dir() && depth > 0 {
        lines.push(format!("{}{}/", indent, name));
    }

    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut sorted: Vec<_> = entries.flatten().collect();
            sorted.sort_by_key(|e| e.file_name());
            for entry in sorted {
                collect_tree(root, &entry.path(), depth + 1, lines, limit);
            }
        }
    } else if depth > 0 {
        lines.push(format!("{}{}", indent, name));
    }
}

fn cmd_dod(store: &Store, id: i64, show: bool, force: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let path = match project.path {
        Some(ref p) => p.clone(),
        None => {
            println!("Project '{}' has no linked path. Use: pm link {} <path>", project.name, id);
            return;
        }
    };
    let project_path = Path::new(&path);

    if show {
        match dod::load_dod(project_path) {
            None => {
                println!("No DOD.md found for '{}'. Run: pm dod {}", project.name, id);
            }
            Some(d) => {
                let (complete, total) = dod::rollup(&d);
                println!("{} — Definition of Done ({}/{} complete)\n", project.name, complete, total);
                println!("USP: {}\n", d.usp);
                for c in &d.criteria {
                    let auto_sym = if c.automated.is_done() { "✓" } else { "✗" };
                    let human_sym = if c.human.is_done() { "✓" } else { "✗" };
                    let done = c.automated.is_done() && c.human.is_done();
                    let marker = if done { "✓" } else { "–" };
                    println!("[{}] [{}] {} — auto:{} human:{}  {}",
                        c.id, marker, c.description, auto_sym, human_sym,
                        if done { "" } else { &format!("({})", c.automated.label()) }
                    );
                }
                println!("\nVerify: pm verify {}    Sign off: pm signoff {}", id, id);
            }
        }
        return;
    }

    // Init path
    let usp = dod::extract_usp_from_charter(project_path);
    match dod::generate_dod(project_path, &project.name, usp, force) {
        Ok(dod::DodAction::Created) => {
            println!("Created docs/DOD.md for '{}'.", project.name);
            println!("Fill in the criteria, then run: pm verify {}", id);
        }
        Ok(dod::DodAction::AlreadyExists) => {
            println!("docs/DOD.md already exists. Use --show to view, --force to overwrite.");
        }
        Ok(dod::DodAction::Overwritten) => {
            println!("Overwritten docs/DOD.md for '{}'.", project.name);
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn normalize_plan_file(input: &str) -> String {
    std::path::Path::new(input)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(input)
        .to_string()
}

fn cmd_pivot(store: &Store, id: i64, count: usize, _refresh: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };

    // Get USP from charter or DOD
    let usp = project.path.as_ref()
        .and_then(|p| dod::extract_usp_from_charter(std::path::Path::new(p)))
        .or_else(|| {
            project.path.as_ref()
                .and_then(|p| dod::load_dod(std::path::Path::new(p)))
                .map(|d| d.usp)
        })
        .unwrap_or_else(|| project.name.clone());

    // Check for research data
    let rec = store.get_research(id).unwrap();
    let research_summary: Option<String> = if let Some(ref r) = rec {
        Some(r.summary.clone())
    } else {
        println!("No research found for '{}'.", project.name);
        println!("Run 'pm research {}' for better pivot suggestions.", id);
        print!("Continue without research data? [y/n] > ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if input.trim() != "y" {
            return;
        }
        None
    };

    // Build profile from active projects (excluding the pivot target)
    let active = store.list_active_projects().unwrap();
    let fallback: Vec<(String, Option<String>)> = active.iter()
        .filter(|p| p.id != id)
        .map(|p| {
            let usp = p.path.as_ref()
                .and_then(|path| dod::extract_usp_from_charter(std::path::Path::new(path)));
            (p.name.clone(), usp)
        })
        .collect();
    let profile = research::load_profile(Some(&fallback));

    println!("Generating {} pivot ideas for '{}' via Claude...\n", count, project.name);

    let output = match research::run_pivot_claude(&project.name, &usp, research_summary.as_deref(), &profile, count) {
        Err(e) => {
            println!("Error: {}", e);
            println!("Check that the claude CLI is installed: claude --version");
            return;
        }
        Ok(o) => o,
    };

    let ideas = research::parse_pivot_ideas(&output);

    if ideas.is_empty() {
        println!("No structured ideas returned. Raw output:\n{}", output);
        return;
    }

    println!("Pivot ideas for '{}' ({} generated):\n", project.name, ideas.len());

    let mut added = 0;
    for (i, idea) in ideas.iter().enumerate() {
        println!("────────────────────────────────────────────────────");
        println!("[{}/{}] {}", i + 1, ideas.len(), idea.name);
        println!("      {}", idea.usp);
        if !idea.gap.is_empty() {
            println!("\n      Gap: {}", idea.gap);
        }
        if !idea.fit.is_empty() {
            println!("      Fit: {}", idea.fit);
        }
        println!();

        use std::io::Write;
        print!("Add to inbox? [y/n/skip] > ");
        std::io::stdout().flush().ok();
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            break;
        }
        match input.trim() {
            "y" => {
                let new_id = store.add_project(&idea.name).unwrap();
                store.set_inbox_note(new_id, &idea.usp).unwrap();
                println!("Added '{}' to inbox (id: {}).\n", idea.name, new_id);
                added += 1;
            }
            "n" => { println!("Discarded.\n"); }
            _ => { println!("Skipped.\n"); }
        }
    }

    if added > 0 {
        println!("Done. Run 'pm inbox' to review what you added.");
    } else {
        println!("Done. Nothing added to inbox.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_cmd_research_no_id_prints_help() {
        let store = Store::open_in_memory().unwrap();
        // Should not panic
        cmd_research(&store, None, false, false, false, false);
    }

    #[test]
    fn test_cmd_research_project_not_found() {
        let store = Store::open_in_memory().unwrap();
        cmd_research(&store, Some(9999), false, false, false, false);
    }

    #[test]
    fn test_cmd_research_scheduled_no_projects() {
        let store = Store::open_in_memory().unwrap();
        cmd_research(&store, None, false, false, false, true);
    }

    #[test]
    fn test_cmd_signoff_no_dod_file() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let id = store.add_project("signoff-test").unwrap();
        store.link_project(id, tmp.path().to_string_lossy().as_ref()).unwrap();
        // Should print message, not panic
        cmd_signoff(&store, id, false);
    }

    #[test]
    fn test_cmd_verify_no_linked_path() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("no-path").unwrap();
        // Should not panic
        cmd_verify(&store, id, false, None);
    }

    #[test]
    fn test_cmd_verify_no_dod_file() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let id = store.add_project("no-dod").unwrap();
        store.link_project(id, tmp.path().to_string_lossy().as_ref()).unwrap();
        // Should print message, not panic
        cmd_verify(&store, id, false, None);
    }

    #[test]
    fn test_cmd_dod_init_creates_file() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let id = store.add_project("dod-test").unwrap();
        store.link_project(id, tmp.path().to_string_lossy().as_ref()).unwrap();

        cmd_dod(&store, id, false, false);

        assert!(tmp.path().join("docs").join("DOD.md").exists());
    }

    #[test]
    fn test_cmd_dod_show_missing_prints_message() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let id = store.add_project("dod-show").unwrap();
        store.link_project(id, tmp.path().to_string_lossy().as_ref()).unwrap();

        // Should not panic
        cmd_dod(&store, id, true, false);
    }

    #[test]
    fn test_normalize_plan_file_accepts_path() {
        let input = "docs/plans/2026-02-09-plan.md";
        let output = normalize_plan_file(input);
        assert_eq!(output, "2026-02-09-plan.md");
    }

    #[test]
    fn test_truncate_handles_unicode() {
        let s = "naïve café";
        let result = std::panic::catch_unwind(|| truncate(s, 6));
        assert!(result.is_ok());
    }

    fn setup_project(store: &Store, name: &str) -> i64 {
        let id = store.add_project(name).unwrap();
        store.update_state(id, ProjectState::Active).unwrap();
        id
    }

    #[test]
    fn test_update_assessment_stores_scores() {
        let store = Store::open_in_memory().unwrap();
        let id = store.add_project("assess-me").unwrap();
        store.update_assessment(id, 8, 7, Some(6), None).unwrap();

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.impact, 8);
        assert_eq!(project.monetization, 7);
        assert_eq!(project.cloneability, Some(6));
    }

    #[test]
    fn test_cmd_mark_writes_progress_file() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let plans_dir = tmp.path().join("docs").join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();
        std::fs::write(plans_dir.join("plan.md"), "### Task 1: Do thing\n").unwrap();

        let id = store.add_project("mark-me").unwrap();
        store.update_state(id, ProjectState::Active).unwrap();
        store
            .link_project(id, tmp.path().to_string_lossy().as_ref())
            .unwrap();

        cmd_mark(&store, id, 1, Some("plan.md".to_string()));

        let progress = std::fs::read_to_string(tmp.path().join(".pm-progress")).unwrap();
        assert!(progress.contains("plan.md:1"));
    }

    #[test]
    fn test_cmd_scan_updates_readiness() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let standards = tmp.path().join("standards.yml");
        std::fs::write(&standards, "requirements: []\nnice_to_haves: []\n").unwrap();
        unsafe {
            std::env::set_var("PM_STANDARDS_CONFIG", &standards);
        }

        // init git repo
        Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        std::fs::write(tmp.path().join("README.md"), "init").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(tmp.path())
            .status()
            .unwrap();

        let plans_dir = tmp.path().join("docs").join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();
        std::fs::write(
            plans_dir.join("plan.md"),
            "### Task 1: Add widget pipeline\n### Task 2: Add reporting view\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join(".pm-progress"), "plan.md:1\n").unwrap();

        let id = setup_project(&store, "scan-me");
        store
            .link_project(id, tmp.path().to_string_lossy().as_ref())
            .unwrap();

        cmd_scan(&store);

        let project = store.get_project(id).unwrap().unwrap();
        assert_eq!(project.readiness, 50);
    }

    #[test]
    fn test_data_dir_override_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("PM_DATA_DIR", tmp.path());
        }
        let dir = resolve_data_dir();
        assert_eq!(dir, tmp.path());
        unsafe {
            std::env::remove_var("PM_DATA_DIR");
        }
    }

    #[test]
    fn test_cmd_status_and_next() {
        let store = Store::open_in_memory().unwrap();
        cmd_status(&store);

        let id = setup_project(&store, "alpha");
        store.update_scores(id, 7, 6, 40).unwrap();
        cmd_status(&store);
        cmd_next(&store);
    }

    #[test]
    fn test_cmd_inbox_throne_and_why() {
        let store = Store::open_in_memory().unwrap();
        let _id1 = store.add_project("idea-one").unwrap();
        let _id2 = store.add_project("idea-two").unwrap();
        cmd_inbox(&store);

        let id = setup_project(&store, "alpha");
        store.update_scores(id, 8, 7, 60).unwrap();
        cmd_throne(&store);
        cmd_why(&store);
    }

    #[test]
    fn test_cmd_link_show_and_charter() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let id = store.add_project("linked").unwrap();

        cmd_link(&store, id, tmp.path().to_string_lossy().as_ref());
        cmd_show(&store, id);
        cmd_charter(&store, id, false);
        cmd_charter(&store, id, false);
    }

    #[test]
    fn test_cmd_tasks_and_mark() {
        let store = Store::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let plans_dir = tmp.path().join("docs").join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();
        std::fs::write(plans_dir.join("plan.md"), "### Task 1: Do thing\n").unwrap();

        let id = store.add_project("tasks").unwrap();
        store
            .link_project(id, tmp.path().to_string_lossy().as_ref())
            .unwrap();

        cmd_tasks(&store, id);
        cmd_mark(&store, id, 1, Some("plan.md".to_string()));
    }

    #[test]
    fn test_cmd_trash_restore_rename_park() {
        let store = Store::open_in_memory().unwrap();
        let id = setup_project(&store, "old-name");
        store.soft_delete(id).unwrap();

        cmd_trash(&store);
        cmd_restore(&store, id);
        cmd_rename(&store, id, "new-name");
        cmd_park(&store, id, "pause");
    }

    #[test]
    fn test_cmd_pivot_project_not_found() {
        let store = Store::open_in_memory().unwrap();
        // Should not panic
        cmd_pivot(&store, 9999, 3, false);
    }
}
