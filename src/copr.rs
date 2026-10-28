use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

const COPR_API_BASE: &str = "https://copr.fedorainfracloud.org/api_3";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SEARCH_LIMIT: &str = "25";

#[derive(Debug, Clone, Deserialize)]
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
        .map_err(|error| format!("The COPR service returned an error: {error}"))?;

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
        .map_err(|error| format!("The COPR service returned an error: {error}"))?;

    response
        .json()
        .map_err(|error| format!("Could not read the project build information: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
