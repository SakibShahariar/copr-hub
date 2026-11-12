use crate::{copr, storage, upstream};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    // Length-prefix the owner to keep identities unambiguous and stable.
    format!("{}:{}:{}", project.owner.len(), project.owner, project.name)
}

fn legacy_project_key(project: &copr::CoprProject) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project.owner.hash(&mut hasher);
    "/".hash(&mut hasher);
    project.name.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn migrate_project_key(state: &mut PersistedMonitoringState, project: &copr::CoprProject) -> bool {
    let stable_key = project_key(project);
    if state.projects.contains_key(&stable_key) {
        return false;
    }
    if let Some(legacy_state) = state.projects.remove(&legacy_project_key(project)) {
        state.projects.insert(stable_key, legacy_state);
        return true;
    }
    false
}

fn prune_inactive_projects(
    state: &mut PersistedMonitoringState,
    active_projects: &[copr::CoprProject],
) -> bool {
    let mut migrated = false;
    for project in active_projects {
        migrated |= migrate_project_key(state, project);
    }
    let active_keys: HashSet<String> = active_projects.iter().map(project_key).collect();
    let previous_len = state.projects.len();
    state.projects.retain(|key, _| active_keys.contains(key));
    migrated || state.projects.len() != previous_len
}

/// Remove monitoring baselines for projects that are no longer saved.
pub fn prune_to_projects(projects: &[copr::CoprProject]) -> Result<bool, String> {
    let mut state = load_state();
    if !prune_inactive_projects(&mut state, projects) {
        return Ok(false);
    }
    save_state(&state)?;
    Ok(true)
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
    let threshold_days =
        threshold_days.clamp(MIN_FRESHNESS_THRESHOLD_DAYS, MAX_FRESHNESS_THRESHOLD_DAYS);
    let stale = is_stale(latest_build_timestamp, now_unix(), threshold_days);
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

fn is_stale(timestamp: Option<i64>, now: u64, threshold_days: u32) -> bool {
    let Some(timestamp) = timestamp else {
        return false;
    };
    let Ok(now) = i64::try_from(now) else {
        return false;
    };
    let threshold_secs =
        i64::from(threshold_days.clamp(MIN_FRESHNESS_THRESHOLD_DAYS, MAX_FRESHNESS_THRESHOLD_DAYS))
            * 24
            * 60
            * 60;
    now.saturating_sub(timestamp) >= threshold_secs
}

fn normalize_upstream_version(version: &str) -> &str {
    let version = version.trim();
    version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version)
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
        alerts.push(format!(
            "Project build activity has become stale (no build in at least {} days).",
            current.stale_after_days
        ));
    }

    if let (Some(old), Some(new)) = (
        previous.upstream_version.as_deref(),
        current.upstream_version.as_deref(),
    ) {
        if normalize_upstream_version(old) != normalize_upstream_version(new) {
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

fn update_observation(
    state: &mut PersistedMonitoringState,
    project: &copr::CoprProject,
    monitor: &copr::ProjectMonitor,
    release: Option<&upstream::UpstreamRelease>,
    config: &MonitoringConfig,
) -> (Vec<String>, Option<String>) {
    migrate_project_key(state, project);
    let key = project_key(project);
    let previous = state.projects.get(&key).cloned();
    let mut current = current_state(monitor, config.freshness_threshold_days);
    if let Some(release) = release.filter(|release| !release.stale_cache) {
        current.upstream_version = Some(release.version.clone());
    } else if let Some(previous) = previous.as_ref() {
        current.upstream_version = previous.upstream_version.clone();
    }

    let mut changes = Vec::new();
    if let Some(previous) = previous.as_ref() {
        if previous.target_states != current.target_states {
            let failed_before = previous
                .target_states
                .values()
                .filter(|value| is_failed(value))
                .count();
            let failed_now = current
                .target_states
                .values()
                .filter(|value| is_failed(value))
                .count();
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
                format!(
                    "build activity became stale after {} days",
                    current.stale_after_days
                )
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
    }

    let alerts = compare_states(previous.as_ref(), &current);
    state.projects.insert(key, current);
    let activity_summary = (!changes.is_empty()).then(|| changes.join("; "));
    (alerts, activity_summary)
}

fn record_activity_change(project: &copr::CoprProject, summary: &str) {
    let _ = crate::activity::record(
        "Project health change",
        &format!("{}/{}", project.owner, project.name),
        summary,
    );
}

/// One successfully fetched project observation for a monitoring cycle.
pub struct MonitoringObservation {
    pub project: copr::CoprProject,
    pub monitor: copr::ProjectMonitor,
    pub release: Option<upstream::UpstreamRelease>,
}

fn record_observations_with_state<F>(
    observations: &[MonitoringObservation],
    mut state: PersistedMonitoringState,
    config: &MonitoringConfig,
    save: F,
) -> Result<Vec<(String, String, Vec<String>)>, String>
where
    F: FnOnce(&PersistedMonitoringState) -> Result<(), String>,
{
    if observations.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::with_capacity(observations.len());
    let mut activity_changes = Vec::new();
    for observation in observations {
        let (alerts, activity_summary) = update_observation(
            &mut state,
            &observation.project,
            &observation.monitor,
            observation.release.as_ref(),
            config,
        );
        results.push((
            observation.project.owner.clone(),
            observation.project.name.clone(),
            alerts,
        ));
        if let Some(summary) = activity_summary {
            activity_changes.push((observation.project.clone(), summary));
        }
    }

    save(&state)?;
    for (project, summary) in activity_changes {
        record_activity_change(&project, &summary);
    }
    Ok(results)
}

/// Update all monitoring baselines from one cycle and write the state file once.
/// A failed state write returns an error and suppresses the cycle's alerts.
pub fn record_observations(
    observations: &[MonitoringObservation],
) -> Result<Vec<(String, String, Vec<String>)>, String> {
    if observations.is_empty() {
        return Ok(Vec::new());
    }
    record_observations_with_state(observations, load_state(), &load_config(), save_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pruning_removes_only_projects_that_are_no_longer_saved() {
        let active = copr::CoprProject {
            owner: "alice".into(),
            name: "active".into(),
            description: None,
            homepage: None,
        };
        let removed = copr::CoprProject {
            owner: "bob".into(),
            name: "removed".into(),
            description: None,
            homepage: None,
        };
        let mut state = PersistedMonitoringState {
            projects: HashMap::from([
                (project_key(&active), ProjectState::default()),
                (project_key(&removed), ProjectState::default()),
            ]),
        };

        assert!(prune_inactive_projects(
            &mut state,
            std::slice::from_ref(&active)
        ));
        assert_eq!(state.projects.len(), 1);
        assert!(state.projects.contains_key(&project_key(&active)));
        assert!(!state.projects.contains_key(&project_key(&removed)));
        assert!(!prune_inactive_projects(
            &mut state,
            std::slice::from_ref(&active)
        ));
    }

    #[test]
    fn legacy_project_baselines_migrate_to_stable_identity_keys() {
        let project = copr::CoprProject {
            owner: "alice".into(),
            name: "demo".into(),
            description: None,
            homepage: None,
        };
        let legacy_key = legacy_project_key(&project);
        let expected_state = ProjectState {
            latest_build_state: Some("failed".into()),
            ..ProjectState::default()
        };
        let mut state = PersistedMonitoringState {
            projects: HashMap::from([(legacy_key.clone(), expected_state.clone())]),
        };

        assert!(prune_inactive_projects(
            &mut state,
            std::slice::from_ref(&project)
        ));

        let stable_key = project_key(&project);
        assert_ne!(stable_key, legacy_key);
        assert_eq!(state.projects.len(), 1);
        assert_eq!(
            state.projects.get(&stable_key).unwrap().latest_build_state,
            expected_state.latest_build_state
        );
        assert!(!state.projects.contains_key(&legacy_key));
    }

    #[test]
    fn batched_observations_update_each_project_baseline() {
        let first = copr::CoprProject {
            owner: "alice".into(),
            name: "first".into(),
            description: None,
            homepage: None,
        };
        let second = copr::CoprProject {
            owner: "bob".into(),
            name: "second".into(),
            description: None,
            homepage: None,
        };
        let monitor = copr::ProjectMonitor {
            packages: Vec::new(),
            latest_build: None,
        };
        let config = MonitoringConfig::default();
        let mut state = PersistedMonitoringState::default();

        let (first_alerts, first_activity) =
            update_observation(&mut state, &first, &monitor, None, &config);
        let (second_alerts, second_activity) =
            update_observation(&mut state, &second, &monitor, None, &config);

        assert!(first_alerts.is_empty());
        assert!(second_alerts.is_empty());
        assert!(first_activity.is_none());
        assert!(second_activity.is_none());
        assert_eq!(state.projects.len(), 2);
        assert!(state.projects.contains_key(&project_key(&first)));
        assert!(state.projects.contains_key(&project_key(&second)));
    }

    #[test]
    fn empty_batch_does_not_load_or_write_monitoring_state() {
        let mut writes = 0;
        let result = record_observations_with_state(
            &[],
            PersistedMonitoringState::default(),
            &MonitoringConfig::default(),
            |_| {
                writes += 1;
                Ok(())
            },
        )
        .unwrap();

        assert!(result.is_empty());
        assert_eq!(writes, 0);
    }

    #[test]
    fn single_observation_uses_one_write_and_establishes_baseline() {
        let observation = MonitoringObservation {
            project: copr::CoprProject {
                owner: "alice".into(),
                name: "demo".into(),
                description: None,
                homepage: None,
            },
            monitor: copr::ProjectMonitor {
                packages: Vec::new(),
                latest_build: None,
            },
            release: None,
        };
        let mut writes = 0;
        let result = record_observations_with_state(
            &[observation],
            PersistedMonitoringState::default(),
            &MonitoringConfig::default(),
            |state| {
                writes += 1;
                assert_eq!(state.projects.len(), 1);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(writes, 1);
        assert_eq!(result.len(), 1);
        assert!(result[0].2.is_empty());
    }

    #[test]
    fn batched_recording_persists_once_and_propagates_save_errors() {
        let project = |owner: &str, name: &str| copr::CoprProject {
            owner: owner.into(),
            name: name.into(),
            description: None,
            homepage: None,
        };
        let monitor = copr::ProjectMonitor {
            packages: Vec::new(),
            latest_build: None,
        };
        let observations = vec![
            MonitoringObservation {
                project: project("alice", "first"),
                monitor: monitor.clone(),
                release: None,
            },
            MonitoringObservation {
                project: project("bob", "second"),
                monitor,
                release: None,
            },
        ];
        let mut writes = 0;
        let result = record_observations_with_state(
            &observations,
            PersistedMonitoringState::default(),
            &MonitoringConfig::default(),
            |state| {
                writes += 1;
                assert_eq!(state.projects.len(), 2);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(writes, 1);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|(_, _, alerts)| alerts.is_empty()));

        let error = record_observations_with_state(
            &observations,
            PersistedMonitoringState::default(),
            &MonitoringConfig::default(),
            |_| Err("disk full".to_string()),
        )
        .unwrap_err();
        assert_eq!(error, "disk full");
    }

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
    fn upstream_release_comparison_ignores_only_a_leading_v_prefix() {
        let previous = ProjectState {
            upstream_version: Some("v1.2.0".to_string()),
            ..state()
        };
        let mut current = previous.clone();
        current.upstream_version = Some("1.2.0".to_string());
        assert!(compare_states(Some(&previous), &current).is_empty());

        current.upstream_version = Some("1.2.0-1".to_string());
        let alerts = compare_states(Some(&previous), &current);
        assert!(alerts
            .iter()
            .any(|alert| alert.contains("New upstream release")));
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
    fn stale_detection_handles_threshold_and_future_timestamps() {
        let threshold = i64::from(DEFAULT_FRESHNESS_THRESHOLD_DAYS) * 24 * 60 * 60;
        let now = 1_800_000_000_u64;
        assert!(!is_stale(Some(now as i64 - threshold + 1), now, 90));
        assert!(is_stale(Some(now as i64 - threshold), now, 90));
        assert!(!is_stale(Some(now as i64 + 60), now, 90));
        assert!(!is_stale(None, now, 90));
        assert!(is_stale(Some(i64::MIN), now, 90));
    }

    #[test]
    fn timestamp_helper_is_sane() {
        assert!(now_unix() > 1_600_000_000);
    }
}
