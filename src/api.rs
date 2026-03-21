use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::Local;
use serde::Serialize;
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex};

use crate::dod;
use crate::domain::{ProjectState, TaskSource};
use crate::roadmap;
use crate::scanner;
use crate::store::Store;

pub type AppState = Arc<Mutex<Store>>;

#[derive(Serialize)]
pub struct ApiProject {
    pub id: i64,
    pub name: String,
    pub state: String,
    pub impact: u8,
    pub monetization: u8,
    pub readiness: u8,
    pub uniqueness: Option<u8>,
    pub cloneability: Option<u8>,
    pub defensibility: u8,
    pub priority_score: i32,
    pub days_stale: i64,
    pub last_activity: String,
    pub created_at: String,
    pub soft_deadline: Option<String>,
    pub path: Option<String>,
    pub project_type: String,
    pub next_milestone: Option<String>,
    pub milestone_target: Option<String>,
}

#[derive(Serialize)]
pub struct ApiProjectDetail {
    #[serde(flatten)]
    pub project: ApiProject,
    pub inbox_note: Option<String>,
    pub roadmap: Option<ApiRoadmap>,
    pub dod: Option<ApiDod>,
    pub research: Option<ApiResearch>,
    pub tasks: Vec<ApiTask>,
}

#[derive(Serialize)]
pub struct ApiRoadmap {
    pub project: String,
    pub assessment: Option<ApiAssessment>,
    pub phases: Vec<ApiPhase>,
    pub readiness: u8,
    pub weight_valid: bool,
}

#[derive(Serialize)]
pub struct ApiAssessment {
    pub impact: u8,
    pub monetization: u8,
    pub cloneability: Option<u8>,
    pub uniqueness: Option<u8>,
    pub researched_at: String,
    pub reasoning: Option<String>,
    pub signals: Option<Vec<String>>,
    pub stale: bool,
}

#[derive(Serialize)]
pub struct ApiPhase {
    pub id: String,
    pub label: String,
    pub weight: f64,
    pub component: Option<String>,
    pub tasks: Vec<ApiRoadmapTask>,
    pub progress: f64,
}

#[derive(Serialize)]
pub struct ApiRoadmapTask {
    pub id: String,
    pub label: String,
    pub done: bool,
}

#[derive(Serialize)]
pub struct ApiDod {
    pub project_name: String,
    pub usp: String,
    pub criteria: Vec<ApiCriterion>,
    pub complete: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct ApiCriterion {
    pub id: String,
    pub description: String,
    pub evidence: Option<String>,
    pub scenario: String,
    pub automated: String,
    pub human: String,
}

#[derive(Serialize)]
pub struct ApiResearch {
    pub summary: String,
    pub previous: Option<String>,
    pub researched_at: Option<String>,
    pub consecutive_flags: i64,
}

#[derive(Serialize)]
pub struct ApiTask {
    pub plan_file: String,
    pub task_number: usize,
    pub description: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct ApiNextRecommendation {
    pub project: Option<ApiProject>,
    pub reason: String,
}

fn project_to_api(p: &crate::domain::Project) -> ApiProject {
    let today = Local::now().date_naive();
    let mut proj = p.clone();
    // Enrich with live roadmap/milestones data so readiness is never stale
    if let Some(ref path) = proj.path {
        let project_path = FsPath::new(path);
        if let Some(scores) = roadmap::load_scores(project_path) {
            proj.readiness = scores.readiness;
            if let Some(v) = scores.impact { proj.impact = v; }
            if let Some(v) = scores.monetization { proj.monetization = v; }
            proj.cloneability = scores.cloneability;
            proj.uniqueness = scores.uniqueness;
            proj.defensibility = scores.defensibility;
        } else if let Some(mf) = crate::milestones::load_milestones(project_path) {
            proj.readiness = mf.readiness();
        }
    }
    let milestone_target = proj.path.as_ref().and_then(|path| {
        let mf = crate::milestones::load_milestones(FsPath::new(path))?;
        mf.target_summary()
    });
    ApiProject {
        id: proj.id,
        name: proj.name.clone(),
        state: state_str(&proj.state),
        impact: proj.impact,
        monetization: proj.monetization,
        readiness: proj.readiness,
        uniqueness: proj.uniqueness,
        cloneability: proj.cloneability,
        defensibility: proj.effective_defensibility(),
        priority_score: proj.priority_score(today),
        days_stale: (today - proj.last_activity).num_days(),
        last_activity: proj.last_activity.to_string(),
        created_at: proj.created_at.to_string(),
        soft_deadline: proj.soft_deadline.map(|d| d.to_string()),
        project_type: proj.project_type.as_str().to_string(),
        path: proj.path.clone(),
        next_milestone: compute_next_milestone(p),
        milestone_target,
    }
}

fn state_str(state: &ProjectState) -> String {
    match state {
        ProjectState::Inbox => "inbox",
        ProjectState::Active => "active",
        ProjectState::Parked => "parked",
        ProjectState::Shipped => "shipped",
        ProjectState::Killed => "killed",
    }
    .to_string()
}

fn compute_next_milestone(p: &crate::domain::Project) -> Option<String> {
    let path = p.path.as_ref()?;
    let project_path = FsPath::new(path);

    if let Some(rm) = roadmap::load_roadmap(project_path) {
        for phase in &rm.phases {
            for task in &phase.tasks {
                if !task.done {
                    return Some(format!("{}: {}", phase.label, task.label));
                }
            }
        }
        return Some("Done".to_string());
    }

    let mut pending: Vec<_> = scanner::list_tasks(project_path)
        .into_iter()
        .filter(|t| t.source == TaskSource::Pending)
        .collect();
    pending.sort_by(|a, b| {
        a.plan_file
            .cmp(&b.plan_file)
            .then_with(|| a.task_number.cmp(&b.task_number))
    });
    pending.first().map(|t| {
        format!(
            "{}#{}: {}",
            t.plan_file.trim_end_matches(".md"),
            t.task_number,
            t.description
        )
    })
}

fn roadmap_to_api(rm: &roadmap::Roadmap) -> ApiRoadmap {
    let readiness = roadmap::compute_readiness(rm);
    let weight_valid = roadmap::validate_weights(rm).is_none();

    ApiRoadmap {
        project: rm.project.clone(),
        assessment: rm.assessment.as_ref().map(|a| {
            let stale = roadmap::is_assessment_stale(&a.researched_at);
            ApiAssessment {
                impact: a.impact,
                monetization: a.monetization,
                cloneability: a.cloneability,
                uniqueness: a.uniqueness,
                researched_at: a.researched_at.clone(),
                reasoning: a.reasoning.clone(),
                signals: a.signals.clone(),
                stale,
            }
        }),
        phases: rm
            .phases
            .iter()
            .map(|phase| {
                let total = phase.tasks.len();
                let done = phase.tasks.iter().filter(|t| t.done).count();
                let progress = if total > 0 {
                    done as f64 / total as f64
                } else {
                    0.0
                };
                ApiPhase {
                    id: phase.id.clone(),
                    label: phase.label.clone(),
                    weight: phase.weight,
                    component: phase.component.clone(),
                    tasks: phase
                        .tasks
                        .iter()
                        .map(|t| ApiRoadmapTask {
                            id: t.id.clone(),
                            label: t.label.clone(),
                            done: t.done,
                        })
                        .collect(),
                    progress,
                }
            })
            .collect(),
        readiness,
        weight_valid,
    }
}

fn dod_to_api(d: &dod::DodFile) -> ApiDod {
    let (complete, total) = dod::rollup(d);
    ApiDod {
        project_name: d.project_name.clone(),
        usp: d.usp.clone(),
        criteria: d
            .criteria
            .iter()
            .map(|c| ApiCriterion {
                id: c.id.clone(),
                description: c.description.clone(),
                evidence: c.evidence.clone(),
                scenario: c.scenario.clone(),
                automated: c.automated.label().to_string(),
                human: c.human.label().to_string(),
            })
            .collect(),
        complete,
        total,
    }
}

async fn list_projects(State(state): State<AppState>) -> Json<Vec<ApiProject>> {
    let store = state.lock().unwrap();
    let projects = store.list_active_projects().unwrap_or_default();
    Json(projects.iter().map(project_to_api).collect())
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiProjectDetail>, StatusCode> {
    let store = state.lock().unwrap();
    let project = store
        .get_project(id)
        .unwrap_or(None)
        .ok_or(StatusCode::NOT_FOUND)?;
    let inbox_note = store.get_inbox_note(id).unwrap_or(None);
    let research = store.get_research(id).unwrap_or(None);
    drop(store);

    let rm = project
        .path
        .as_ref()
        .and_then(|p| roadmap::load_roadmap(FsPath::new(p)));
    let dod_data = project
        .path
        .as_ref()
        .and_then(|p| dod::load_dod(FsPath::new(p)));
    let tasks: Vec<ApiTask> = project
        .path
        .as_ref()
        .map(|p| {
            scanner::list_tasks(FsPath::new(p))
                .into_iter()
                .map(|t| ApiTask {
                    plan_file: t.plan_file,
                    task_number: t.task_number,
                    description: t.description,
                    source: match t.source {
                        TaskSource::Manual => "manual",
                        TaskSource::Git => "git",
                        TaskSource::Pending => "pending",
                    }
                    .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(ApiProjectDetail {
        project: project_to_api(&project),
        inbox_note,
        roadmap: rm.as_ref().map(roadmap_to_api),
        dod: dod_data.as_ref().map(dod_to_api),
        research: research.map(|r| ApiResearch {
            summary: r.summary,
            previous: r.previous,
            researched_at: r.researched_at,
            consecutive_flags: r.consecutive_flags,
        }),
        tasks,
    }))
}

async fn list_inbox(State(state): State<AppState>) -> Json<Vec<ApiProject>> {
    let store = state.lock().unwrap();
    let projects = store.list_inbox_projects().unwrap_or_default();
    Json(projects.iter().map(project_to_api).collect())
}

async fn get_next(State(state): State<AppState>) -> Json<ApiNextRecommendation> {
    let store = state.lock().unwrap();
    let mut projects = store.list_active_projects().unwrap_or_default();
    drop(store);

    let today = Local::now().date_naive();
    projects.sort_by(|a, b| b.priority_score(today).cmp(&a.priority_score(today)));

    match projects.first() {
        Some(p) => Json(ApiNextRecommendation {
            project: Some(project_to_api(p)),
            reason: format!(
                "Highest priority score ({}), {}% ready, {} days since last activity",
                p.priority_score(today),
                p.readiness,
                (today - p.last_activity).num_days()
            ),
        }),
        None => Json(ApiNextRecommendation {
            project: None,
            reason: "No active projects found.".to_string(),
        }),
    }
}

async fn list_parked(State(state): State<AppState>) -> Json<Vec<ApiProject>> {
    let store = state.lock().unwrap();
    let all = store.list_projects_for_dedupe().unwrap_or_default();
    let parked: Vec<_> = all
        .iter()
        .filter(|p| p.state == ProjectState::Parked)
        .map(project_to_api)
        .collect();
    Json(parked)
}

async fn list_trash(State(state): State<AppState>) -> Json<Vec<ApiProject>> {
    let store = state.lock().unwrap();
    let projects = store.list_deleted_projects().unwrap_or_default();
    Json(projects.iter().map(project_to_api).collect())
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/projects", get(list_projects))
        .route("/api/projects/:id", get(get_project))
        .route("/api/inbox", get(list_inbox))
        .route("/api/next", get(get_next))
        .route("/api/parked", get(list_parked))
        .route("/api/trash", get(list_trash))
        .with_state(state)
}

pub async fn serve(state: AppState, port: u16) {
    use tower_http::services::{ServeDir, ServeFile};

    let web_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("web")
        .join("dist");

    let app = if web_dir.exists() {
        let fallback = ServeFile::new(web_dir.join("index.html"));
        let serve_dir = ServeDir::new(&web_dir).fallback(fallback);
        api_router(state).fallback_service(serve_dir)
    } else {
        eprintln!("No web/dist/ found, serving API only. Run: cd web && npm run build");
        api_router(state)
    };

    let addr = format!("0.0.0.0:{}", port);
    println!("pm dashboard at http://localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
