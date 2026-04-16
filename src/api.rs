use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::Local;
use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::domain::ProjectState;
use crate::store::Store;

pub type AppState = Arc<Mutex<Store>>;

#[derive(Serialize)]
pub struct ApiProject {
    pub id: i64,
    pub name: String,
    pub state: String,
    pub archetype: String,
    pub stage: u8,
    pub stage_label: String,
    pub velocity: Option<u8>,
    pub fit_signal: Option<u8>,
    pub distinctness: Option<u8>,
    pub leverage: Option<u8>,
    pub score: i32,
    pub action: String,
    pub action_target: Option<String>,
    pub days_stale: i64,
    pub last_activity: String,
    pub created_at: String,
    pub soft_deadline: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct ApiProjectDetail {
    #[serde(flatten)]
    pub project: ApiProject,
    pub sunk_cost_days: Option<i32>,
    pub pivot_count: u32,
}

#[derive(Serialize)]
pub struct ApiNextRecommendation {
    pub project: Option<ApiProject>,
    pub reason: String,
}

fn stage_label(stage: u8) -> &'static str {
    match stage {
        0 => "idea",
        1 => "spike",
        2 => "prototype",
        3 => "validated",
        4 => "shipped",
        5 => "traction+",
        _ => "unknown",
    }
}

fn project_to_api(p: &crate::domain::Project, all_projects: &[crate::domain::Project]) -> ApiProject {
    let today = Local::now().date_naive();

    let nearest = find_nearest_neighbour(p, all_projects);
    let action = p.action_recommendation(nearest.as_deref());

    ApiProject {
        id: p.id,
        name: p.name.clone(),
        state: match p.state {
            ProjectState::Active => "active".to_string(),
            ProjectState::Archived => "archived".to_string(),
        },
        archetype: p.project_type.as_str().to_string(),
        stage: p.stage,
        stage_label: stage_label(p.stage).to_string(),
        velocity: p.velocity,
        fit_signal: p.fit_signal,
        distinctness: p.distinctness,
        leverage: p.leverage,
        score: p.priority_score(today),
        action: action.label().to_string(),
        action_target: action.target().map(|s| s.to_string()),
        days_stale: (today - p.last_activity).num_days(),
        last_activity: p.last_activity.to_string(),
        created_at: p.created_at.to_string(),
        soft_deadline: p.soft_deadline.map(|d| d.to_string()),
        path: p.path.clone(),
    }
}

fn find_nearest_neighbour(target: &crate::domain::Project, all: &[crate::domain::Project]) -> Option<String> {
    use crate::similarity::token_similarity;
    let mut best_score = 0.0f32;
    let mut best_name = None;
    for other in all {
        if other.id == target.id { continue; }
        let sim = token_similarity(&target.name, &other.name);
        if sim > best_score {
            best_score = sim;
            best_name = Some(other.name.clone());
        }
    }
    if best_score > 0.3 { best_name } else { None }
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/projects", get(list_projects))
        .route("/projects/{id}", get(get_project_detail))
        .route("/archived", get(list_archived))
        .route("/next", get(get_next))
        .with_state(state);

    Router::new().nest("/api", api)
}

async fn list_projects(
    State(store): State<AppState>,
) -> Result<Json<Vec<ApiProject>>, StatusCode> {
    let store = store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects = store
        .list_active_projects()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut api_projects: Vec<ApiProject> = projects
        .iter()
        .map(|p| project_to_api(p, &projects))
        .collect();
    api_projects.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(Json(api_projects))
}

async fn get_project_detail(
    State(store): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiProjectDetail>, StatusCode> {
    let store = store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let project = store
        .get_project(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let all = store
        .list_active_projects()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let api_project = project_to_api(&project, &all);
    Ok(Json(ApiProjectDetail {
        project: api_project,
        sunk_cost_days: project.sunk_cost_days,
        pivot_count: project.pivot_count,
    }))
}

async fn list_archived(
    State(store): State<AppState>,
) -> Result<Json<Vec<ApiProject>>, StatusCode> {
    let store = store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects = store
        .list_archived_projects()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let api_projects: Vec<ApiProject> = projects
        .iter()
        .map(|p| project_to_api(p, &projects))
        .collect();
    Ok(Json(api_projects))
}

async fn get_next(
    State(store): State<AppState>,
) -> Result<Json<ApiNextRecommendation>, StatusCode> {
    let store = store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects = store
        .list_active_projects()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let today = Local::now().date_naive();

    let best = projects
        .iter()
        .max_by_key(|p| p.priority_score(today));

    match best {
        Some(p) => {
            let action = p.action_recommendation(None);
            Ok(Json(ApiNextRecommendation {
                project: Some(project_to_api(p, &projects)),
                reason: format!("Highest score ({}), action: {}", p.priority_score(today), action.label()),
            }))
        }
        None => Ok(Json(ApiNextRecommendation {
            project: None,
            reason: "No active projects".to_string(),
        })),
    }
}
