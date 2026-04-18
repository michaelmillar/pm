use crate::domain::{Project, ProjectAction};
use crate::scanner;
use std::path::Path;

pub struct NextTask {
    pub text: String,
    pub source: &'static str,
}

pub fn resolve(project: &Project, action: &ProjectAction) -> Option<NextTask> {
    if let Some(t) = &project.next_task {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            return Some(NextTask { text: trimmed.to_string(), source: "manual" });
        }
    }
    if let Some(path) = &project.path {
        let p = Path::new(path);
        if p.exists() {
            if let Some(item) = scanner::extract_next_task(p) {
                return Some(NextTask { text: item, source: "file" });
            }
        }
    }
    auto_derive(project, action).map(|t| NextTask { text: t, source: "auto" })
}

fn auto_derive(project: &Project, action: &ProjectAction) -> Option<String> {
    let missing_axes = missing_axes(project);
    if !missing_axes.is_empty() {
        return Some(format!("Score {}", missing_axes.join(", ")));
    }
    Some(match action {
        ProjectAction::Push => "Ship next milestone".to_string(),
        ProjectAction::Pivot => "Pivot direction or kill".to_string(),
        ProjectAction::Kill => "Archive or revive with new angle".to_string(),
        ProjectAction::Groom => format!("Groom: improve {}", weakest_axis(project)),
        ProjectAction::Integrate(target) => format!("Fold into {}", target),
        ProjectAction::Sustain => "Maintain release cadence".to_string(),
        ProjectAction::Repurpose => "Repurpose codebase or kill".to_string(),
        ProjectAction::Observe => stage_advice(project),
    })
}

fn missing_axes(project: &Project) -> Vec<&'static str> {
    let mut out = Vec::new();
    if project.fit_signal.is_none() { out.push("fit"); }
    if project.velocity.is_none() { out.push("velocity"); }
    if project.distinctness.is_none() { out.push("distinctness"); }
    if project.leverage.is_none() { out.push("leverage"); }
    out
}

fn weakest_axis(project: &Project) -> &'static str {
    let axes = [
        ("velocity", project.velocity),
        ("fit", project.fit_signal),
        ("distinctness", project.distinctness),
        ("leverage", project.leverage),
    ];
    axes.iter()
        .filter_map(|(n, v)| v.map(|val| (n, val)))
        .min_by_key(|(_, v)| *v)
        .map(|(n, _)| *n)
        .unwrap_or("any axis")
}

fn stage_advice(project: &Project) -> String {
    match project.stage {
        0 => "Decide direction or shelve".to_string(),
        1 => "Build prototype or kill".to_string(),
        2 => "Add tests + CI to validate".to_string(),
        3 => "Ship MVP".to_string(),
        _ => "Track usage signal".to_string(),
    }
}
