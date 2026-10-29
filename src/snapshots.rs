use crate::{copr, favorites};
use crate::upstream::UpstreamRelease;
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SNAPSHOT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SNAPSHOT_AGE_SECS: u64 = 180 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub project: copr::CoprProject,
    pub saved_at_unix: u64,
    #[serde(default)]
    pub monitor: Option<copr::ProjectMonitor>,
    #[serde(default)]
    pub monitor_fetched_at_unix: Option<u64>,
    #[serde(default)]
    pub build_history: Option<Vec<copr::ProjectBuildEvent>>,
    #[serde(default)]
    pub history_fetched_at_unix: Option<u64>,
    #[serde(default)]
    pub upstream_release: Option<UpstreamRelease>,
    #[serde(default)]
    pub upstream_fetched_at_unix: Option<u64>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn snapshot_path(project: &copr::CoprProject) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project.owner.hash(&mut hasher);
    "/".hash(&mut hasher);
    project.name.hash(&mut hasher);
    gtk::glib::user_cache_dir()
        .join("copr-hub")
        .join("projects")
        .join(format!("{:016x}.json", hasher.finish()))
}

fn valid_project(project: &copr::CoprProject) -> bool {
    !project.owner.trim().is_empty()
        && !project.name.trim().is_empty()
        && project.owner.len() <= 512
        && project.name.len() <= 512
}

fn write_snapshot(snapshot: &ProjectSnapshot) -> Result<(), String> {
    if !valid_project(&snapshot.project) {
        return Err("Cannot cache a project with invalid identity fields.".to_string());
    }
    let contents = serde_json::to_vec(snapshot)
        .map_err(|error| format!("Could not encode project snapshot: {error}"))?;
    if contents.len() as u64 > MAX_SNAPSHOT_BYTES {
        return Err("The project snapshot exceeds the 4 MiB safety limit.".to_string());
    }
    let path = snapshot_path(&snapshot.project);
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the project snapshot directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create project snapshot directory: {error}"))?;
    crate::storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save project snapshot: {error}"))
}

pub fn load(project: &copr::CoprProject) -> Option<ProjectSnapshot> {
    if !valid_project(project) {
        return None;
    }
    let path = snapshot_path(project);
    let metadata = fs::metadata(&path).ok()?;
    if metadata.len() > MAX_SNAPSHOT_BYTES {
        return None;
    }
    let contents = fs::read(&path).ok()?;
    let snapshot: ProjectSnapshot = serde_json::from_slice(&contents).ok()?;
    let age = now_unix().checked_sub(snapshot.saved_at_unix)?;
    if age > MAX_SNAPSHOT_AGE_SECS
        || snapshot.project.owner != project.owner
        || snapshot.project.name != project.name
    {
        return None;
    }
    Some(snapshot)
}

fn load_or_create(project: &copr::CoprProject) -> Result<ProjectSnapshot, String> {
    Ok(load(project).unwrap_or_else(|| ProjectSnapshot {
        project: project.clone(),
        saved_at_unix: now_unix(),
        monitor: None,
        monitor_fetched_at_unix: None,
        build_history: None,
        history_fetched_at_unix: None,
        upstream_release: None,
        upstream_fetched_at_unix: None,
    }))
}

pub fn save_monitor(
    project: &copr::CoprProject,
    monitor: copr::ProjectMonitor,
) -> Result<(), String> {
    let mut snapshot = load_or_create(project)?;
    snapshot.monitor = Some(monitor);
    snapshot.monitor_fetched_at_unix = Some(now_unix());
    snapshot.saved_at_unix = now_unix();
    write_snapshot(&snapshot)
}

pub fn save_history(
    project: &copr::CoprProject,
    history: Vec<copr::ProjectBuildEvent>,
) -> Result<(), String> {
    let mut snapshot = load_or_create(project)?;
    snapshot.build_history = Some(history);
    snapshot.history_fetched_at_unix = Some(now_unix());
    snapshot.saved_at_unix = now_unix();
    write_snapshot(&snapshot)
}

pub fn save_release(
    project: &copr::CoprProject,
    release: Option<UpstreamRelease>,
) -> Result<(), String> {
    let mut snapshot = load_or_create(project)?;
    snapshot.upstream_release = release;
    snapshot.upstream_fetched_at_unix = Some(now_unix());
    snapshot.saved_at_unix = now_unix();
    write_snapshot(&snapshot)
}

pub fn cache_age_label(timestamp: Option<u64>) -> String {
    let Some(timestamp) = timestamp else {
        return "No cached copy available".to_string();
    };
    let Some(age) = now_unix().checked_sub(timestamp) else {
        return "Cached timestamp is in the future; data may be unreliable".to_string();
    };
    let hours = age / 3600;
    if hours < 1 {
        "Cached less than an hour ago".to_string()
    } else if hours < 48 {
        format!("Cached {hours} hour(s) ago")
    } else {
        format!("Cached {} day(s) ago", hours / 24)
    }
}

pub fn cache_is_stale(timestamp: Option<u64>) -> bool {
    timestamp
        .and_then(|value| now_unix().checked_sub(value))
        .is_none_or(|age| age > 6 * 60 * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> copr::CoprProject {
        copr::CoprProject {
            name: "sample".to_string(),
            owner: "alice".to_string(),
            description: None,
            homepage: None,
        }
    }

    #[test]
    fn cache_age_is_explicit_for_missing_and_old_timestamps() {
        assert_eq!(cache_age_label(None), "No cached copy available");
        let old = now_unix().saturating_sub(3 * 24 * 60 * 60);
        assert_eq!(cache_age_label(Some(old)), "Cached 3 day(s) ago");
        assert!(cache_is_stale(Some(old)));
        assert!(!cache_is_stale(Some(now_unix())));
    }

    #[test]
    fn snapshot_identity_is_validated() {
        assert!(valid_project(&project()));
        let mut invalid = project();
        invalid.owner = " ".to_string();
        assert!(!valid_project(&invalid));
    }
}
