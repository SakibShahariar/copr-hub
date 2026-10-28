use serde::Deserialize;
use std::time::Duration;

const COPR_API_BASE: &str = "https://copr.fedorainfracloud.org/api_3";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SEARCH_LIMIT: &str = "25";

#[derive(Debug, Clone, Deserialize)]
pub struct CoprProject {
    pub name: String,
    #[serde(default)]
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

pub fn search_projects(pattern: &str) -> Result<Vec<CoprProject>, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Ok(Vec::new());
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("Copr-Hub/", env!("CARGO_PKG_VERSION")))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| format!("Could not prepare the network client: {error}"))?;

    let response = client
        .get(format!("{COPR_API_BASE}/project/list/"))
        .query(&[("pattern", pattern), ("limit", SEARCH_LIMIT)])
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                "The COPR request timed out. Check your connection and try again.".to_string()
            } else {
                format!("Could not reach the COPR service: {error}")
            }
        })?
        .error_for_status()
        .map_err(|error| format!("The COPR service returned an error: {error}"))?;

    let result: ProjectListResponse = response
        .json()
        .map_err(|error| format!("Could not read the COPR response: {error}"))?;

    Ok(result.projects)
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
    fn missing_optional_metadata_is_supported() {
        let project: CoprProject = serde_json::from_str(r#"{"name":"demo"}"#).unwrap();

        assert_eq!(project.name, "demo");
        assert!(project.description.is_none());
        assert!(project.homepage.is_none());
    }
}
