use crate::{copr, storage, upstream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_STATE_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_FRESHNESS_THRESHOLD_DAYS: u32 = 90;
const MIN_FRESHNESS_THRESHOLD_DAYS: u32 = 7;
const MAX_FRESHNESS_THRESHOLD_DAYS: u32 = 3650;

fn default_freshness_threshold_days() -> u32 {
    DEFAULT_FRESHNESS_THRESHOLD_DAYS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_freshness_threshold_days")]
    pub freshness_threshold_days: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            freshness_threshold_days: DEFAULT_FRESHNESS_THRESHOLD_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedMonitoringState {
    #[serde(default)]
    projects: HashMap<String, ProjectState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectState {
    #[serde(default)]
    target_states: HashMap<String, String>,
    #[serde(default)]
    latest_build_state: Option<String>,
    #[serde(default)]
    upstream_version: Option<String>,
    #[serde(default)]
    stale: bool,
    #[serde(default = "default_freshness_threshold_days")]
    stale_after_days: u32,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn config_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("copr-hub")
        .join("monitoring.json")
}

fn state_path() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("copr-hub")
        .join("monitoring-state.json")
}

fn project_key(project: &copr::CoprProject) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project.owner.hash(&mut hasher);
    "/".hash(&mut hasher);
    project.name.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf, max_bytes: u64) -> Option<T> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > max_bytes {
        return None;
    }
    let contents = fs::read(path).ok()?;
    serde_json::from_slice(&contents).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Could not encode monitoring data: {error}"))?;
    if contents.len() as u64 > MAX_STATE_BYTES {
        return Err("Monitoring data exceeds the 2 MiB safety limit.".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine monitoring data directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create monitoring data directory: {error}"))?;
    storage::atomic_write(path, &contents)
        .map_err(|error| format!("Could not save monitoring data: {error}"))
}

pub fn load_config() -> MonitoringConfig {
    read_json(&config_path(), 64 * 1024).unwrap_or_default()
}

pub fn save_config(config: &MonitoringConfig) -> Result<(), String> {
    write_json(&config_path(), config)
}

fn load_state() -> PersistedMonitoringState {
    read_json(&state_path(), MAX_STATE_BYTES).unwrap_or_default()
}

fn save_state(state: &PersistedMonitoringState) -> Result<(), String> {
    write_json(&state_path(), state)
}

fn is_failed(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "failed" | "error")
}

fn current_state(monitor: &copr::ProjectMonitor, threshold_days: u32) -> ProjectState {
    let mut target_states = HashMap::new();
    for package in &monitor.packages {
        for (target, build) in &package.chroots {
            target_states.insert(
                format!("{}@{}", package.name, target),
                build
                    .state
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_ascii_lowercase(),
            );
        }
    }
    let latest = monitor.latest_build.as_ref();
    let latest_build_timestamp = latest.and_then(|build| build.ended_on.or(build.submitted_on));
    let threshold_days = threshold_days.clamp(MIN_FRESHNESS_THRESHOLD_DAYS, MAX_FRESHNESS_THRESHOLD_DAYS);
    let stale = latest_build_timestamp
        .and_then(|timestamp| i64::try_from(now_unix()).ok().map(|now| now - timestamp))
        .is_some_and(|age| age >= i64::from(threshold_days) * 24 * 60 * 60);
    ProjectState {
        target_states,
        latest_build_state: latest
            .and_then(|build| build.state.clone())
            .map(|state| state.to_ascii_lowercase()),
        upstream_version: None,
        stale,
        stale_after_days: threshold_days,
    }
}

fn compare_states(previous: Option<&ProjectState>, current: &ProjectState) -> Vec<String> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    let mut alerts = Vec::new();
    let mut newly_failed = Vec::new();
    for (target, current_state) in &current.target_states {
        if is_failed(current_state)
            && !previous
                .target_states
                .get(target)
                .is_some_and(|old| is_failed(old))
        {
            newly_failed.push(target.clone());
        }
    }
    newly_failed.sort();
    if !newly_failed.is_empty() {
        let shown = newly_failed
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if newly_failed.len() > 4 {
            format!(" and {} more", newly_failed.len() - 4)
        } else {
            String::new()
        };
        alerts.push(format!(
            "{} newly failed build target(s): {shown}{more}",
            newly_failed.len()
        ));
    }

    if !previous.stale && current.stale {
        alerts.push(
            format!("Project build activity has become stale (no build in at least {} days).", current.stale_after_days),
        );
    }

    if let (Some(old), Some(new)) = (
        previous.upstream_version.as_deref(),
        current.upstream_version.as_deref(),
    ) {
        if old != new {
            alerts.push(format!(
                "New upstream release detected: {new} (previously {old})."
            ));
        }
    }

    if newly_failed.is_empty()
        && current.latest_build_state.as_deref().is_some_and(is_failed)
        && !previous
            .latest_build_state
            .as_deref()
            .is_some_and(is_failed)
    {
        alerts.push("The latest project build has failed.".to_string());
    }
    alerts
}

/// Compare a successful observation with the last one. The first observation only
/// establishes a baseline; it never emits notifications for pre-existing failures.
pub fn record_observation(
    project: &copr::CoprProject,
    monitor: &copr::ProjectMonitor,
    release: Option<&upstream::UpstreamRelease>,
) -> Result<Vec<String>, String> {
    let mut state = load_state();
    let key = project_key(project);
    let previous = state.projects.get(&key).cloned();
    let config = load_config();
    let mut current = current_state(monitor, config.freshness_threshold_days);
    if let Some(release) = release.filter(|release| !release.stale_cache) {
        current.upstream_version = Some(release.version.clone());
    } else if let Some(previous) = previous.as_ref() {
        current.upstream_version = previous.upstream_version.clone();
    }

    if let Some(previous) = previous.as_ref() {
        let mut changes = Vec::new();
        if previous.target_states != current.target_states {
            let failed_before = previous.target_states.values().filter(|value| is_failed(value)).count();
            let failed_now = current.target_states.values().filter(|value| is_failed(value)).count();
            changes.push(format!("failed targets: {failed_before} → {failed_now}"));
        }
        if previous.latest_build_state != current.latest_build_state {
            changes.push(format!(
                "latest build: {} → {}",
                previous.latest_build_state.as_deref().unwrap_or("unknown"),
                current.latest_build_state.as_deref().unwrap_or("unknown")
            ));
        }
        if previous.stale != current.stale {
            changes.push(if current.stale {
                format!("build activity became stale after {} days", current.stale_after_days)
            } else {
                "build activity is no longer stale".to_string()
            });
        }
        if previous.upstream_version != current.upstream_version {
            changes.push(format!(
                "upstream release: {} → {}",
                previous.upstream_version.as_deref().unwrap_or("unknown"),
                current.upstream_version.as_deref().unwrap_or("unknown")
            ));
        }
        if !changes.is_empty() {
            let _ = crate::activity::record(
                "Project health change",
                &format!("{}/{}", project.owner, project.name),
                &changes.join("; "),
            );
        }
    }

    let alerts = compare_states(previous.as_ref(), &current);
    state.projects.insert(key, current);
    save_state(&state)?;
    Ok(alerts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_state_normalization_is_case_insensitive() {
        assert!(is_failed("FAILED"));
        assert!(is_failed("Error"));
        assert!(!is_failed("running"));
        assert!(!is_failed("unknown"));
    }

    #[test]
    fn config_defaults_to_opt_in_off() {
        assert!(!MonitoringConfig::default().enabled);
        assert_eq!(MonitoringConfig::default().freshness_threshold_days, 90);
    }

    fn state() -> ProjectState {
        ProjectState {
            target_states: HashMap::from([(
                "demo@fedora-44-x86_64".to_string(),
                "succeeded".to_string(),
            )]),
            latest_build_state: Some("succeeded".to_string()),
            upstream_version: Some("1.0.0".to_string()),
            stale: false,
            stale_after_days: DEFAULT_FRESHNESS_THRESHOLD_DAYS,
        }
    }

    #[test]
    fn first_observation_establishes_baseline_without_alerts() {
        assert!(compare_states(None, &state()).is_empty());
    }

    #[test]
    fn alerts_only_on_new_failed_target_and_not_repeated_failures() {
        let previous = state();
        let mut current = previous.clone();
        current
            .target_states
            .insert("demo@fedora-44-x86_64".to_string(), "failed".to_string());
        current.latest_build_state = Some("failed".to_string());
        let alerts = compare_states(Some(&previous), &current);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].contains("newly failed build target"));

        assert!(compare_states(Some(&current), &current).is_empty());
    }

    #[test]
    fn detects_upstream_release_and_stale_transition() {
        let previous = state();
        let mut current = previous.clone();
        current.upstream_version = Some("1.1.0".to_string());
        current.stale = true;
        let alerts = compare_states(Some(&previous), &current);
        assert_eq!(alerts.len(), 2);
        assert!(alerts
            .iter()
            .any(|alert| alert.contains("New upstream release")));
        assert!(alerts.iter().any(|alert| alert.contains("become stale")));
    }

    #[test]
    fn timestamp_helper_is_sane() {
        assert!(now_unix() > 1_600_000_000);
    }
}
