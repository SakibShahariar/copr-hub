use serde::Deserialize;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

const STALE_BUILD_AFTER_DAYS: i64 = 90;

const COPR_API_BASE: &str = "https://copr.fedorainfracloud.org/api_3";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SEARCH_LIMIT: &str = "25";

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct CoprProject {
    #[serde(alias = "projectname")]
    pub name: String,
    #[serde(default, alias = "ownername")]
    pub owner: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectListResponse {
    #[serde(default)]
    projects: Vec<CoprProject>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ProjectMonitor {
    #[serde(default)]
    pub packages: Vec<MonitoredPackage>,
    #[serde(default)]
    pub latest_build: Option<ProjectBuildSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FedoraCompatibility {
    Compatible,
    Incompatible,
    Unknown,
}

pub fn fedora_compatibility(
    monitor: &ProjectMonitor,
    fedora_release: Option<u32>,
) -> FedoraCompatibility {
    let Some(fedora_release) = fedora_release else {
        return FedoraCompatibility::Unknown;
    };
    let target_prefix = format!("fedora-{fedora_release}-");
    let mut has_targets = false;

    for package in &monitor.packages {
        for target in package.chroots.keys() {
            has_targets = true;
            if target.starts_with(&target_prefix) {
                return FedoraCompatibility::Compatible;
            }
        }
    }

    if has_targets {
        FedoraCompatibility::Incompatible
    } else {
        FedoraCompatibility::Unknown
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ProjectBuildSummary {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub ended_on: Option<i64>,
    #[serde(default)]
    pub submitted_on: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BuildListResponse {
    #[serde(default)]
    builds: Vec<ProjectBuildSummary>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ProjectBuildEvent {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub submitted_on: Option<i64>,
    #[serde(default)]
    pub started_on: Option<i64>,
    #[serde(default)]
    pub ended_on: Option<i64>,
    #[serde(default)]
    pub chroots: Vec<String>,
    #[serde(default)]
    pub source_package: Option<BuildSourcePackage>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct BuildSourcePackage {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct MonitoredPackage {
    pub name: String,
    #[serde(default)]
    pub chroots: HashMap<String, ChrootBuild>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ChrootBuild {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub pkg_version: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("Copr-Hub/", env!("CARGO_PKG_VERSION")))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| format!("Could not prepare the network client: {error}"))
}

/// Whether a monitoring error is likely to affect every saved project.
pub fn should_stop_monitoring_scan(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("timed out")
        || error.contains("could not connect to copr")
        || error.contains("could not prepare the network client")
        || error.contains("too many requests")
        || error.contains("temporarily unavailable")
}

fn request_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "The COPR request timed out. Check your connection and try again.".to_string()
    } else if error.is_connect() {
        "Could not connect to COPR. Check your network connection.".to_string()
    } else {
        format!("The COPR service request failed: {error}")
    }
}

fn retry_delay(retry_after: Option<&str>) -> Duration {
    retry_after
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .map(|delay| delay.min(Duration::from_secs(3)))
        .filter(|delay| !delay.is_zero())
        .unwrap_or(Duration::from_millis(350))
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Retry one time for transient connection failures and temporary COPR outages.
/// GET requests are safe to retry; the short backoff avoids hammering an unhealthy service.
fn send_with_retry(
    request: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::Response, String> {
    let retry_request = request.try_clone();
    match request.send() {
        Ok(response) if is_retryable_status(response.status()) && retry_request.is_some() => {
            let delay = retry_delay(
                response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok()),
            );
            drop(response);
            thread::sleep(delay);
            retry_request
                .expect("retry request was checked above")
                .send()
                .map_err(request_error)
        }
        Ok(response) => Ok(response),
        Err(error) if (error.is_timeout() || error.is_connect()) && retry_request.is_some() => {
            thread::sleep(Duration::from_millis(350));
            retry_request
                .expect("retry request was checked above")
                .send()
                .map_err(request_error)
        }
        Err(error) => Err(request_error(error)),
    }
}

fn service_status_message(status: reqwest::StatusCode, operation: &str) -> String {
    match status {
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            "COPR is receiving too many requests. Wait a moment and try again.".to_string()
        }
        reqwest::StatusCode::NOT_FOUND => format!("COPR could not find the requested {operation}."),
        status if status.is_server_error() => {
            format!("COPR is temporarily unavailable (HTTP {status}). Try again later.")
        }
        status => format!("COPR rejected the {operation} request (HTTP {status})."),
    }
}

fn service_response_error(error: reqwest::Error, operation: &str) -> String {
    error
        .status()
        .map(|status| service_status_message(status, operation))
        .unwrap_or_else(|| format!("The COPR service request failed: {error}"))
}

pub fn search_projects(pattern: &str) -> Result<Vec<CoprProject>, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Ok(Vec::new());
    }

    // COPR's project/list endpoint only lists projects; full-text discovery
    // belongs to project/search and uses the "query" parameter.
    let response = client()?
        .get(format!("{COPR_API_BASE}/project/search"))
        .query(&[("query", pattern), ("limit", SEARCH_LIMIT)]);
    let response = send_with_retry(response)?
        .error_for_status()
        .map_err(|error| service_response_error(error, "search"))?;

    let result: ProjectListResponse = response
        .json()
        .map_err(|error| format!("Could not read the COPR response: {error}"))?;

    Ok(result.projects)
}

/// Fetch recent real project build records for a chronological build timeline.
pub fn fetch_project_build_history(
    owner: &str,
    project: &str,
) -> Result<Vec<ProjectBuildEvent>, String> {
    #[derive(Deserialize)]
    struct BuildHistoryResponse {
        #[serde(default)]
        builds: Vec<ProjectBuildEvent>,
    }

    let response = send_with_retry(
        client()?
            .get(format!("{}/build/list/", COPR_API_BASE))
            .query(&[
                ("ownername", owner),
                ("projectname", project),
                ("limit", "20"),
            ]),
    )?
    .error_for_status()
    .map_err(|error| service_response_error(error, "build history"))?;

    let result: BuildHistoryResponse = response
        .json()
        .map_err(|error| format!("Could not read project build history: {error}"))?;
    let mut builds = result.builds;
    builds.sort_by(|left, right| {
        right
            .submitted_on
            .or(right.started_on)
            .or(right.ended_on)
            .unwrap_or(0)
            .cmp(
                &left
                    .submitted_on
                    .or(left.started_on)
                    .or(left.ended_on)
                    .unwrap_or(0),
            )
    });
    Ok(builds)
}

fn fetch_project_monitor_data(owner: &str, project: &str) -> Result<ProjectMonitor, String> {
    let response = send_with_retry(
        client()?
            .get(format!("{COPR_API_BASE}/monitor"))
            .query(&[("ownername", owner), ("projectname", project)]),
    )?
    .error_for_status()
    .map_err(|error| service_response_error(error, "project"))?;

    response
        .json()
        .map_err(|error| format!("Could not read the project build information: {error}"))
}

fn fetch_latest_build(owner: &str, project: &str) -> Result<Option<ProjectBuildSummary>, String> {
    let response = send_with_retry(
        client()?
            .get(format!("{}/build/list/", COPR_API_BASE))
            .query(&[
                ("ownername", owner),
                ("projectname", project),
                ("limit", "1"),
            ]),
    )?
    .error_for_status()
    .map_err(|error| service_response_error(error, "recent build"))?
    .json::<BuildListResponse>()
    .map_err(|error| format!("Could not read recent build information: {error}"))?;
    Ok(response.builds.into_iter().next())
}

fn build_summary_from_event(build: &ProjectBuildEvent) -> ProjectBuildSummary {
    ProjectBuildSummary {
        state: build.state.clone(),
        ended_on: build.ended_on,
        submitted_on: build.submitted_on.or(build.started_on).or(build.ended_on),
    }
}

/// Fetch monitor and build history together so monitoring can reuse the history
/// response for the latest-build freshness summary.
pub fn fetch_project_monitor_with_history(
    owner: &str,
    project: &str,
) -> Result<(ProjectMonitor, Option<Vec<ProjectBuildEvent>>), String> {
    let mut monitor = fetch_project_monitor_data(owner, project)?;
    match fetch_project_build_history(owner, project) {
        Ok(history) => {
            monitor.latest_build = history.first().map(build_summary_from_event);
            Ok((monitor, Some(history)))
        }
        Err(_) => {
            // Preserve monitor results when history is unavailable. Fall back to
            // the small latest-build request used by standalone monitor lookups.
            monitor.latest_build = fetch_latest_build(owner, project).ok().flatten();
            Ok((monitor, None))
        }
    }
}

pub fn fetch_project_monitor(owner: &str, project: &str) -> Result<ProjectMonitor, String> {
    let mut monitor = fetch_project_monitor_data(owner, project)?;
    monitor.latest_build = fetch_latest_build(owner, project).ok().flatten();
    Ok(monitor)
}

/// Summarize reported build states for each Fedora chroot in stable target order.
pub fn build_target_health_summary(packages: &[MonitoredPackage]) -> String {
    use std::collections::BTreeMap;

    // Counts are succeeded, failed, in-progress, and other/unknown states.
    let mut targets: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for package in packages {
        for (target, build) in &package.chroots {
            let counts = targets.entry(target.clone()).or_default();
            match build
                .state
                .as_deref()
                .unwrap_or("unknown")
                .to_ascii_lowercase()
                .as_str()
            {
                "succeeded" | "success" => counts[0] += 1,
                "failed" | "error" => counts[1] += 1,
                "pending" | "waiting" | "starting" | "running" | "building" => counts[2] += 1,
                _ => counts[3] += 1,
            }
        }
    }

    if targets.is_empty() {
        return "No per-target build states were reported".to_string();
    }

    targets
        .into_iter()
        .take(5)
        .map(|(target, counts)| {
            let mut parts = Vec::new();
            if counts[0] > 0 {
                parts.push(format!("{} succeeded", counts[0]));
            }
            if counts[1] > 0 {
                parts.push(format!("{} failed", counts[1]));
            }
            if counts[2] > 0 {
                parts.push(format!("{} in progress", counts[2]));
            }
            if counts[3] > 0 {
                parts.push(format!("{} other/unknown", counts[3]));
            }
            format!("{target}: {}", parts.join(", "))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn build_freshness_message(build: Option<&ProjectBuildSummary>, now: i64) -> String {
    let Some(build) = build else {
        return "Recent build age is unavailable; COPR did not provide build timestamps."
            .to_string();
    };
    let timestamp = build.ended_on.or(build.submitted_on);
    let Some(timestamp) = timestamp.filter(|timestamp| *timestamp > 0) else {
        return format!(
            "Latest project build state: {} · build age unavailable.",
            build.state.as_deref().unwrap_or("unknown")
        );
    };
    let age_days = now.saturating_sub(timestamp).max(0) / 86_400;
    let age = if age_days == 0 {
        "less than a day ago".to_string()
    } else if age_days == 1 {
        "1 day ago".to_string()
    } else {
        format!("{age_days} days ago")
    };
    let state = build.state.as_deref().unwrap_or("unknown");
    if age_days >= STALE_BUILD_AFTER_DAYS {
        format!("No recent project build: latest reported build was {age} (state: {state}).")
    } else {
        format!("Latest project build: {age} (state: {state}).")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_build_summary_uses_history_timestamp_fallback_order() {
        let event = ProjectBuildEvent {
            id: Some(7),
            state: Some("succeeded".into()),
            submitted_on: None,
            started_on: Some(200),
            ended_on: Some(300),
            chroots: Vec::new(),
            source_package: None,
        };

        let summary = build_summary_from_event(&event);
        assert_eq!(summary.state.as_deref(), Some("succeeded"));
        assert_eq!(summary.ended_on, Some(300));
        assert_eq!(summary.submitted_on, Some(200));
    }

    #[test]
    fn monitoring_scan_stops_only_for_scan_wide_failures() {
        assert!(should_stop_monitoring_scan(
            "Could not connect to COPR. Check your network connection."
        ));
        assert!(should_stop_monitoring_scan(
            "The COPR request timed out. Check your connection and try again."
        ));
        assert!(should_stop_monitoring_scan(
            "COPR is receiving too many requests. Wait a moment and try again."
        ));
        assert!(should_stop_monitoring_scan(
            "COPR is temporarily unavailable (HTTP 503). Try again later."
        ));
        assert!(!should_stop_monitoring_scan(
            "COPR could not find the requested project."
        ));
        assert!(!should_stop_monitoring_scan(
            "COPR rejected the project request (HTTP 403)."
        ));
    }

    #[test]
    fn retryable_statuses_are_limited_to_rate_limits_and_server_errors() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(reqwest::StatusCode::FORBIDDEN));
        assert!(!is_retryable_status(reqwest::StatusCode::OK));
    }

    #[test]
    fn service_status_messages_cover_common_api_failures() {
        assert!(
            service_status_message(reqwest::StatusCode::TOO_MANY_REQUESTS, "search")
                .contains("too many requests")
        );
        assert!(
            service_status_message(reqwest::StatusCode::NOT_FOUND, "project")
                .contains("could not find")
        );
        assert!(
            service_status_message(reqwest::StatusCode::BAD_GATEWAY, "search")
                .contains("temporarily unavailable")
        );
        assert!(
            service_status_message(reqwest::StatusCode::FORBIDDEN, "search").contains("HTTP 403")
        );
    }

    #[test]
    fn build_freshness_distinguishes_recent_stale_and_missing_timestamps() {
        let now = 2_000_000_000_i64;
        let recent = ProjectBuildSummary {
            state: Some("succeeded".into()),
            ended_on: Some(now - 7 * 86_400),
            submitted_on: None,
        };
        assert!(build_freshness_message(Some(&recent), now).contains("7 days ago"));
        assert!(!build_freshness_message(Some(&recent), now).contains("No recent"));
        let stale = ProjectBuildSummary {
            state: Some("failed".into()),
            ended_on: Some(now - 100 * 86_400),
            submitted_on: None,
        };
        let message = build_freshness_message(Some(&stale), now);
        assert!(message.contains("No recent project build"));
        assert!(message.contains("failed"));
        let missing = ProjectBuildSummary {
            state: None,
            ended_on: None,
            submitted_on: None,
        };
        assert!(build_freshness_message(Some(&missing), now).contains("age unavailable"));
        assert!(build_freshness_message(None, now).contains("unavailable"));
    }

    #[test]
    fn build_target_health_groups_states_by_target_and_handles_unknowns() {
        let packages: Vec<MonitoredPackage> = serde_json::from_str(
            r#"[
                {"name":"one","chroots":{
                    "fedora-44-x86_64":{"state":"succeeded"},
                    "fedora-43-x86_64":{"state":"failed"}
                }},
                {"name":"two","chroots":{
                    "fedora-44-x86_64":{"state":"running"},
                    "fedora-43-x86_64":{"state":"unknown"}
                }}
            ]"#,
        )
        .unwrap();

        let summary = build_target_health_summary(&packages);
        assert!(summary.contains("fedora-43-x86_64: 1 failed, 1 other/unknown"));
        assert!(summary.contains("fedora-44-x86_64: 1 succeeded, 1 in progress"));
        assert_eq!(
            build_target_health_summary(&[]),
            "No per-target build states were reported"
        );
    }

    #[test]
    fn empty_search_does_not_make_a_request() {
        assert!(search_projects("   ").unwrap().is_empty());
    }

    #[test]
    fn project_search_response_deserializes_full_text_results() {
        let response: ProjectListResponse = serde_json::from_str(
            r#"{"projects":[
                {"projectname":"applications","ownername":"aahsnr-work",
                 "description":"Desktop applications including Zen Browser"}
            ]}"#,
        )
        .unwrap();

        assert_eq!(response.projects.len(), 1);
        assert_eq!(response.projects[0].name, "applications");
        assert_eq!(response.projects[0].owner, "aahsnr-work");
    }

    #[test]
    fn project_response_deserializes_optional_metadata() {
        let project: CoprProject = serde_json::from_str(
            r#"{"name":"demo","owner":"alice","description":"Example project","homepage":"https://example.com"}"#,
        )
        .unwrap();

        assert_eq!(project.name, "demo");
        assert_eq!(project.owner, "alice");
        assert_eq!(project.description.as_deref(), Some("Example project"));
        assert_eq!(project.homepage.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn project_response_accepts_api_v3_field_names() {
        let project: CoprProject =
            serde_json::from_str(r#"{"projectname":"demo","ownername":"alice"}"#).unwrap();

        assert_eq!(project.name, "demo");
        assert_eq!(project.owner, "alice");
    }

    #[test]
    fn missing_optional_metadata_is_supported() {
        let project: CoprProject = serde_json::from_str(r#"{"name":"demo"}"#).unwrap();

        assert!(project.description.is_none());
        assert!(project.homepage.is_none());
    }

    #[test]
    fn bounds_retry_after_delays_and_uses_a_safe_default() {
        use super::retry_delay;
        use std::time::Duration;

        assert_eq!(retry_delay(Some("2")), Duration::from_secs(2));
        assert_eq!(retry_delay(Some("30")), Duration::from_secs(3));
        assert_eq!(retry_delay(Some("invalid")), Duration::from_millis(350));
        assert_eq!(retry_delay(Some("0")), Duration::from_millis(350));
        assert_eq!(retry_delay(None), Duration::from_millis(350));
    }

    #[test]
    fn monitor_response_parses_package_build_statuses() {
        let monitor: ProjectMonitor = serde_json::from_str(
            r#"{"packages":[{"name":"demo","chroots":{"fedora-44-x86_64":{"state":"succeeded","pkg_version":"1.2-1"}}}]}"#,
        )
        .unwrap();

        assert_eq!(monitor.packages.len(), 1);
        assert_eq!(monitor.packages[0].name, "demo");
        let build = monitor.packages[0].chroots.get("fedora-44-x86_64").unwrap();
        assert_eq!(build.state.as_deref(), Some("succeeded"));
        assert_eq!(build.pkg_version.as_deref(), Some("1.2-1"));
    }

    #[test]
    fn detects_fedora_compatibility_without_guessing_missing_targets() {
        let monitor: ProjectMonitor = serde_json::from_str(
            r#"{"packages":[{"name":"demo","chroots":{"fedora-44-x86_64":{"state":"succeeded"},"fedora-43-aarch64":{"state":"failed"}}}]}"#,
        )
        .unwrap();
        assert_eq!(
            fedora_compatibility(&monitor, Some(44)),
            FedoraCompatibility::Compatible
        );
        assert_eq!(
            fedora_compatibility(&monitor, Some(45)),
            FedoraCompatibility::Incompatible
        );
        assert_eq!(
            fedora_compatibility(&monitor, None),
            FedoraCompatibility::Unknown
        );

        let empty = ProjectMonitor {
            packages: Vec::new(),
            latest_build: None,
        };
        assert_eq!(
            fedora_compatibility(&empty, Some(44)),
            FedoraCompatibility::Unknown
        );
    }
}
