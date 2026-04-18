use crate::api;
use crate::discovery;
use crate::domain::{ProjectState, ProjectType};
use crate::store::Store;
use chrono::Local;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pm")]
#[command(about = "Pre-seed project prioritiser")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ranked list of active projects with action recommendations
    Status {
        #[arg(long, value_enum, default_value_t = StatusSort::Score)]
        sort: StatusSort,
        #[arg(long, help = "Include unscored projects (score = 0)")]
        all: bool,
    },
    /// Add a new project
    Add {
        name: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long, value_enum, default_value_t = CliProjectType::Oss)]
        r#type: CliProjectType,
    },
    /// Show project detail
    Show {
        id: i64,
    },
    /// Soft-delete projects (recoverable for 30 days)
    Remove {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<i64>,
    },
    /// Rename a project
    Rename {
        id: i64,
        name: String,
    },
    /// Archive projects (move to archived state)
    Archive {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<i64>,
    },
    /// Reactivate an archived project
    Activate {
        id: i64,
    },
    /// Set project archetype
    Type {
        id: i64,
        #[arg(value_enum)]
        project_type: CliProjectType,
    },
    /// Set axis value or lifecycle stage
    Score {
        id: i64,
        #[arg(long, value_parser = parse_axis_kv)]
        axis: Vec<(String, u8)>,
        #[arg(long)]
        stage: Option<u8>,
    },
    /// Start web dashboard
    Web {
        #[arg(long, default_value = "3141")]
        port: u16,
    },
    /// Scan linked projects and auto-score
    Scan {
        /// Fetch remote signals (GitHub, arxiv, Steam, analytics)
        #[arg(long)]
        fetch: bool,
    },
    /// Record a pivot event (resets stage to 1)
    Pivot {
        id: i64,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Run one-shot schema migration from legacy model
    Migrate,
    /// Calibrate action thresholds from event history
    Calibrate,
    /// Show highest-priority project
    Next,
    /// Set or clear the next-task note on a project
    Task {
        id: i64,
        /// Manual next-task text (omit with --clear to remove)
        text: Option<String>,
        #[arg(long, conflicts_with = "text")]
        clear: bool,
    },
    /// List deleted projects
    Trash,
    /// Restore a deleted project
    Restore {
        id: i64,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliProjectType {
    Oss,
    Research,
    Game,
    Webapp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StatusSort {
    Score,
    Stale,
    Name,
    Stage,
}

fn parse_axis_kv(s: &str) -> Result<(String, u8), String> {
    let (name, val) = s.split_once('=')
        .ok_or_else(|| "Expected name=value (e.g. velocity=8)".to_string())?;
    let v: u8 = val.parse().map_err(|_| "Value must be 0-10".to_string())?;
    if v > 10 {
        return Err("Value must be 0-10".to_string());
    }
    Ok((name.to_string(), v))
}

pub fn run() {
    let cli = Cli::parse();
    let store = open_store();

    match cli.command {
        Commands::Status { sort, all } => cmd_status(&store, sort, all),
        Commands::Add { name, path, r#type } => cmd_add(&store, &name, path, r#type),
        Commands::Show { id } => cmd_show(&store, id),
        Commands::Remove { ids } => cmd_remove(&store, &ids),
        Commands::Rename { id, name } => cmd_rename(&store, id, &name),
        Commands::Archive { ids } => cmd_archive(&store, &ids),
        Commands::Activate { id } => cmd_activate(&store, id),
        Commands::Type { id, project_type } => cmd_type(&store, id, project_type),
        Commands::Score { id, axis, stage } => cmd_score(&store, id, axis, stage),

        Commands::Web { port } => cmd_web(store, port),
        Commands::Scan { fetch } => cmd_scan(&store, fetch),
        Commands::Pivot { id, reason } => cmd_pivot(&store, id, reason),
        Commands::Calibrate => cmd_calibrate(&store),
        Commands::Migrate => cmd_migrate(&store),
        Commands::Next => cmd_next(&store),
        Commands::Task { id, text, clear } => cmd_task(&store, id, text, clear),
        Commands::Trash => cmd_trash(&store),
        Commands::Restore { id } => cmd_restore(&store, id),
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

fn stage_label(stage: u8) -> &'static str {
    match stage {
        0 => "idea",
        1 => "spike",
        2 => "prototype",
        3 => "validated",
        4 => "shipped",
        5 => "traction+",
        _ => "?",
    }
}

fn action_icon(action: &str) -> &'static str {
    match action {
        "KILL" => "\x1b[31m\u{2717} KILL\x1b[0m",
        "PIVOT" => "\x1b[33m\u{21BB} PIVOT\x1b[0m",
        "PUSH" => "\x1b[32m\u{25B2} PUSH\x1b[0m",
        "GROOM" => "\x1b[36m\u{270E} GROOM\x1b[0m",
        "INTEGRATE" => "\x1b[35m\u{2295} INTEGRATE\x1b[0m",
        "SUSTAIN" => "\x1b[34m\u{25C9} SUSTAIN\x1b[0m",
        "REPURPOSE" => "\x1b[33m\u{21BA} REPURPOSE\x1b[0m",
        _ => "\x1b[90m\u{25CB} OBSERVE\x1b[0m",
    }
}

fn archetype_icon(type_str: &str) -> &'static str {
    match type_str {
        "oss" => "\u{2699}",
        "research" => "\u{2234}",
        "game" => "\u{265E}",
        "webapp" => "\u{25C8}",
        "study" => "\u{270E}",
        _ => " ",
    }
}

fn cmd_status(store: &Store, sort: StatusSort, show_all: bool) {
    let projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects. Add one with: pm add \"project name\"");
        return;
    }

    let today = Local::now().date_naive();
    let thresholds = store.load_thresholds().unwrap_or_default();
    let all_scored: Vec<_> = projects.iter().map(|p| {
        let action = p.action_with_thresholds(&thresholds, None);
        let score = p.priority_score(today);
        (p, score, action)
    }).collect();

    let hidden = all_scored.iter().filter(|(_, s, _)| *s == 0).count();
    let mut scored: Vec<_> = if show_all {
        all_scored
    } else {
        all_scored.into_iter().filter(|(_, s, _)| *s > 0).collect()
    };

    match sort {
        StatusSort::Score => scored.sort_by(|a, b| {
            b.0.stage.cmp(&a.0.stage)
                .then(b.1.cmp(&a.1))
        }),
        StatusSort::Stale => scored.sort_by(|a, b| {
            let da = (today - a.0.last_activity).num_days();
            let db = (today - b.0.last_activity).num_days();
            b.0.stage.cmp(&a.0.stage).then(db.cmp(&da))
        }),
        StatusSort::Name => scored.sort_by(|a, b| {
            b.0.stage.cmp(&a.0.stage).then(a.0.name.cmp(&b.0.name))
        }),
        StatusSort::Stage => scored.sort_by(|a, b| {
            b.0.stage.cmp(&a.0.stage)
                .then(b.1.cmp(&a.1))
        }),
    }

    let dim = "\x1b[90m";
    let reset = "\x1b[0m";
    let bold = "\x1b[1m";

    println!("{bold}{:>4}  {:<13} {:>5}  {:<25} {:<10} {:>4} {:>4} {:>4} {:>4} {:>5}  {:<40}{reset}",
        "ID", "Action", "Score", "Project", "Type",
        "Vel", "Fit", "Dst", "Lev", "Stale", "Next");
    println!("{dim}{}{reset}", "\u{2500}".repeat(126));

    let mut last_stage: Option<u8> = None;
    for (p, score, action) in &scored {
        if last_stage != Some(p.stage) {
            if last_stage.is_some() {
                println!();
            }
            let count = scored.iter().filter(|(proj, _, _)| proj.stage == p.stage).count();
            println!("{dim}  \u{2504}\u{2504} {} ({}) {}{reset}",
                stage_label(p.stage), count, "\u{2504}".repeat(68));
            last_stage = Some(p.stage);
        }

        let act = action_icon(action.label());
        let days_stale = (today - p.last_activity).num_days();
        let icon = archetype_icon(p.project_type.as_str());
        let type_col = format!("{} {}", icon, p.project_type.display());

        let fmt_ax = |v: Option<u8>| -> String {
            match v {
                Some(n) => format!("{:>4}", n),
                None => format!("  {dim}\u{00B7}{reset}"),
            }
        };

        let next = crate::next_task::resolve(p, action);
        let next_col = match &next {
            Some(n) => truncate(&n.text, 40),
            None => String::new(),
        };

        println!("{:>4}  {:<22} {:>5}  {:<25} {:<10} {} {} {} {} {:>4}d  {}",
            p.id,
            act,
            score,
            truncate(&p.name, 24),
            type_col,
            fmt_ax(p.velocity),
            fmt_ax(p.fit_signal),
            fmt_ax(p.distinctness),
            fmt_ax(p.leverage),
            days_stale,
            next_col,
        );
    }

    if !show_all && hidden > 0 {
        println!("\n{dim}{} unscored projects hidden (use --all to show){reset}", hidden);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max-3]) }
}

fn cmd_add(store: &Store, name: &str, path: Option<String>, ptype: CliProjectType) {
    let id = store.add_project(name).unwrap();
    let pt = cli_type_to_domain(ptype);
    store.update_project_type(id, &pt).unwrap();
    if let Some(p) = path {
        store.link_project(id, &p).unwrap();
    }
    println!("Added project {} (id={}, type={})", name, id, pt.as_str());
}

fn cmd_show(store: &Store, id: i64) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let today = Local::now().date_naive();
    let thresholds = store.load_thresholds().unwrap_or_default();
    let action = project.action_with_thresholds(&thresholds, None);
    let days_stale = (today - project.last_activity).num_days();

    println!("{} (id={})", project.name, project.id);
    println!("  Type:    {}", project.project_type.as_str());
    println!("  State:   {}", if project.state == ProjectState::Active { "active" } else { "archived" });
    println!("  Stage:   {} ({})", project.stage, stage_label(project.stage));
    println!("  Score:   {}", project.priority_score(today));
    println!("  Action:  {}", action.label());
    if let Some(target) = action.target() {
        println!("  Target:  {}", target);
    }
    println!("  Axes:    velocity={} fit={} distinct={} leverage={}",
        fmt_axis(project.velocity), fmt_axis(project.fit_signal),
        fmt_axis(project.distinctness), fmt_axis(project.leverage));
    println!("  Stale:   {}d", days_stale);
    println!("  Pivots:  {}", project.pivot_count);
    if let Some(sunk) = project.sunk_cost_days {
        println!("  Sunk:    {}d", sunk);
    }
    if let Some(ref p) = project.path {
        println!("  Path:    {}", p);
    }
}

fn fmt_axis(v: Option<u8>) -> String {
    v.map(|v| format!("{}/10", v)).unwrap_or_else(|| "-".to_string())
}

fn cmd_remove(store: &Store, ids: &[i64]) {
    for &id in ids {
        match store.soft_delete(id) {
            Ok(0) => println!("Project {} not found", id),
            Ok(_) => println!("Removed {} (restore with: pm restore {})", id, id),
            Err(e) => eprintln!("Failed to remove {}: {}", id, e),
        }
    }
}

fn cmd_rename(store: &Store, id: i64, name: &str) {
    match store.rename_project(id, name) {
        Ok(0) => println!("Project {} not found", id),
        Ok(_) => println!("Renamed to '{}'", name),
        Err(e) => eprintln!("Failed: {}", e),
    }
}

fn cmd_archive(store: &Store, ids: &[i64]) {
    for &id in ids {
        match store.update_state(id, ProjectState::Archived) {
            Ok(0) => println!("Project {} not found", id),
            Ok(_) => println!("Archived {}", id),
            Err(e) => eprintln!("Failed to archive {}: {}", id, e),
        }
    }
}

fn cmd_activate(store: &Store, id: i64) {
    match store.update_state(id, ProjectState::Active) {
        Ok(0) => println!("Project {} not found", id),
        Ok(_) => println!("Activated project {}", id),
        Err(e) => eprintln!("Failed: {}", e),
    }
}

fn cmd_type(store: &Store, id: i64, ptype: CliProjectType) {
    let pt = cli_type_to_domain(ptype);
    match store.update_project_type(id, &pt) {
        Ok(0) => println!("Project {} not found", id),
        Ok(_) => println!("Type set to '{}'", pt.as_str()),
        Err(e) => eprintln!("Failed: {}", e),
    }
}

fn cmd_score(store: &Store, id: i64, axes: Vec<(String, u8)>, stage: Option<u8>) {
    if axes.is_empty() && stage.is_none() {
        println!("Provide --axis name=value and/or --stage N");
        return;
    }
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };

    for (name, value) in &axes {
        match store.update_axis(id, name, Some(*value)) {
            Ok(_) => println!("{} set to {}/10", name, value),
            Err(e) => eprintln!("Invalid axis '{}': {}", name, e),
        }
    }

    if let Some(s) = stage {
        if s > 5 {
            println!("Stage must be 0-5");
            return;
        }
        let old_stage = project.stage;
        store.update_stage(id, s).unwrap();
        store.record_stage_event(id, old_stage, s, None).unwrap();
        println!("Stage set to {} ({})", s, stage_label(s));
    }
}

fn cmd_web(store: Store, port: u16) {
    let state = std::sync::Arc::new(std::sync::Mutex::new(store));
    let rt = tokio::runtime::Runtime::new().expect("Failed to start async runtime");
    rt.block_on(async {
        let app = api::build_router(state.clone());

        let web_dir = std::env::current_dir().unwrap().join("web").join("dist");
        let app = if web_dir.exists() {
            app.fallback_service(tower_http::services::ServeDir::new(web_dir))
        } else {
            app
        };

        let addr = format!("0.0.0.0:{}", port);
        println!("Dashboard: http://localhost:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}

fn cmd_scan(store: &Store, fetch: bool) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let scan_root = home.join("projects");

    match discovery::discover_projects(store, &scan_root) {
        Ok(()) => {}
        Err(e) => { eprintln!("Discovery failed: {}", e); return; }
    }

    let projects = store.list_active_projects().unwrap();
    crate::autoscore::score_all(store, &projects, fetch);
    if fetch {
        println!("Scanned and scored {} projects (with remote signals)", projects.len());
    } else {
        println!("Scanned and scored {} projects (disk only, use --fetch for remote signals)", projects.len());
    }
}

fn cmd_pivot(store: &Store, id: i64, reason: Option<String>) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    let old_stage = project.stage;
    store.record_pivot_event(id, reason.as_deref()).unwrap();
    store.record_stage_event(id, old_stage, 1, Some("pivot")).unwrap();
    println!("Pivoted '{}'. Stage reset to 1 (spike). Pivot count: {}",
        project.name, project.pivot_count + 1);
}

fn cmd_task(store: &Store, id: i64, text: Option<String>, clear: bool) {
    let project = match store.get_project(id).unwrap() {
        Some(p) => p,
        None => { println!("Project {} not found", id); return; }
    };
    if clear {
        store.update_next_task(id, None).unwrap();
        println!("Cleared next task for '{}'", project.name);
        return;
    }
    match text {
        Some(t) => {
            store.update_next_task(id, Some(&t)).unwrap();
            println!("Set next task for '{}': {}", project.name, t);
        }
        None => {
            match &project.next_task {
                Some(t) => println!("{}: {}", project.name, t),
                None => println!("{}: (no manual task set)", project.name),
            }
        }
    }
}

fn cmd_calibrate(store: &Store) {
    let projects = store.list_active_projects().unwrap();
    let stage_events = store.list_all_stage_events().unwrap();
    let pivot_events = store.list_all_pivot_events().unwrap();

    let result = crate::scoring::calibrate::compute_thresholds(
        &projects, &stage_events, &pivot_events,
    );

    println!("Calibration ({} events analysed)", result.event_count);
    for adj in &result.adjustments {
        println!("  {}", adj);
    }

    if result.event_count >= 10 {
        store.save_thresholds(&result.thresholds).unwrap();
        println!("Thresholds saved. Will be used by pm status and pm scan.");
    }
}

fn cmd_migrate(store: &Store) {
    match store.migrate_scoring() {
        Ok(count) => println!("Migrated {} projects to new scoring model", count),
        Err(e) => eprintln!("Migration failed: {}", e),
    }
}

fn cmd_next(store: &Store) {
    let projects = store.list_active_projects().unwrap();
    if projects.is_empty() {
        println!("No active projects");
        return;
    }
    let today = Local::now().date_naive();
    let thresholds = store.load_thresholds().unwrap_or_default();
    let best = projects.iter().max_by_key(|p| p.priority_score(today)).unwrap();
    let action = best.action_with_thresholds(&thresholds, None);
    println!("{} (score={}, action={}, stage={})",
        best.name, best.priority_score(today), action.label(), stage_label(best.stage));
}

fn cmd_trash(store: &Store) {
    let projects = store.list_deleted_projects().unwrap();
    if projects.is_empty() {
        println!("No deleted projects");
        return;
    }
    for p in &projects {
        let deleted = p.deleted_at.map(|d| d.to_string()).unwrap_or_default();
        println!("  {} {} (deleted {})", p.id, p.name, deleted);
    }
}

fn cmd_restore(store: &Store, id: i64) {
    match store.restore(id) {
        Ok(0) => println!("Project {} not found", id),
        Ok(_) => println!("Restored project {}", id),
        Err(e) => eprintln!("Failed: {}", e),
    }
}

fn cli_type_to_domain(t: CliProjectType) -> ProjectType {
    match t {
        CliProjectType::Oss => ProjectType::Oss,
        CliProjectType::Research => ProjectType::Research,
        CliProjectType::Game => ProjectType::Game,
        CliProjectType::Webapp => ProjectType::Webapp,
    }
}
