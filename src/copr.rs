use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Debug, Deserialize)]
pub struct ProjectMonitor {
    #[serde(default)]
    pub packages: Vec<MonitoredPackage>,
    #[serde(skip)]
    pub latest_build: Option<ProjectBuildSummary>,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct MonitoredPackage {
    pub name: String,
    #[serde(default)]
    pub chroots: HashMap<String, ChrootBuild>,
}

#[derive(Debug, Deserialize)]
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

fn request_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "The COPR request timed out. Check your connection and try again.".to_string()
    } else if error.is_connect() {
        "Could not connect to COPR. Check your network connection.".to_string()
    } else {
        format!("The COPR service request failed: {error}")
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

    let response = client()?
        .get(format!("{COPR_API_BASE}/project/list/"))
        .query(&[("pattern", pattern), ("limit", SEARCH_LIMIT)])
        .send()
        .map_err(request_error)?
        .error_for_status()
        .map_err(|error| service_response_error(error, "search"))?;

    let result: ProjectListResponse = response
        .json()
        .map_err(|error| format!("Could not read the COPR response: {error}"))?;

    Ok(result.projects)
}

pub fn fetch_project_monitor(owner: &str, project: &str) -> Result<ProjectMonitor, String> {
    let response = client()?
        .get(format!("{COPR_API_BASE}/monitor"))
        .query(&[("ownername", owner), ("projectname", project)])
        .send()
        .map_err(request_error)?
        .error_for_status()
        .map_err(|error| service_response_error(error, "project"))?;

    let mut monitor: ProjectMonitor = response
        .json()
        .map_err(|error| format!("Could not read the project build information: {error}"))?;

    // The monitor endpoint has no timestamps, so use the newest project build for freshness.
    let latest_build = client()
        .and_then(|client| {
            client
                .get(format!("{}/build/list/", COPR_API_BASE))
                .query(&[
                    ("ownername", owner),
                    ("projectname", project),
                    ("limit", "1"),
                ])
                .send()
                .map_err(request_error)?
                .error_for_status()
                .map_err(|error| service_response_error(error, "recent build"))?
                .json::<BuildListResponse>()
                .map_err(|error| format!("Could not read recent build information: {error}"))
        })
        .ok()
        .and_then(|mut response| response.builds.drain(..).next());
    monitor.latest_build = latest_build;
    Ok(monitor)
}

pub fn build_freshness_message(build: Option<&ProjectBuildSummary>, now: i64) -> String {
    let Some(build) = build else {
        return "Recent build age is unavailable; COPR did not provide build timestamps.".to_string();
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
        let recent = ProjectBuildSummary { state: Some("succeeded".into()), ended_on: Some(now - 7 * 86_400), submitted_on: None };
        assert!(build_freshness_message(Some(&recent), now).contains("7 days ago"));
        assert!(!build_freshness_message(Some(&recent), now).contains("No recent"));
        let stale = ProjectBuildSummary { state: Some("failed".into()), ended_on: Some(now - 100 * 86_400), submitted_on: None };
        let message = build_freshness_message(Some(&stale), now);
        assert!(message.contains("No recent project build"));
        assert!(message.contains("failed"));
        let missing = ProjectBuildSummary { state: None, ended_on: None, submitted_on: None };
        assert!(build_freshness_message(Some(&missing), now).contains("age unavailable"));
        assert!(build_freshness_message(None, now).contains("unavailable"));
    }

    #[test]
    fn empty_search_does_not_make_a_request() {
        assert!(search_projects("   ").unwrap().is_empty());
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
}
