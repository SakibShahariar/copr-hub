use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const CACHE_CAPACITY: usize = 128;

type ReleaseCache = Mutex<HashMap<String, (Instant, Option<UpstreamRelease>)>>;

static RELEASE_CACHE: OnceLock<ReleaseCache> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamRelease {
    pub version: String,
    pub name: String,
    pub published_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    html_url: Option<String>,
    draft: Option<bool>,
    prerelease: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GitLabRelease {
    tag_name: String,
    name: Option<String>,
    released_at: Option<String>,
    _links: Option<GitLabLinks>,
}

#[derive(Debug, Deserialize)]
struct GitLabLinks {
    #[serde(rename = "self")]
    self_url: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("Copr-Hub/", env!("CARGO_PKG_VERSION")))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| format!("Could not prepare upstream release lookup: {error}"))
}

fn repository_parts(homepage: &str) -> Option<(String, String, bool)> {
    let url = reqwest::Url::parse(homepage).ok()?;
    if !matches!(url.scheme(), "https" | "http") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let is_gitlab = host == "gitlab.com";
    if host != "github.com" && !is_gitlab {
        return None;
    }
    let segments: Vec<_> = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .take_while(|segment| *segment != "-")
        .map(str::to_string)
        .collect();
    let (namespace, project) = if is_gitlab {
        if segments.len() < 2 {
            return None;
        }
        let project = segments.last()?.trim_end_matches(".git").trim();
        let namespace = segments[..segments.len() - 1].join("/");
        (namespace, project.to_string())
    } else {
        if segments.len() < 2 {
            return None;
        }
        (
            segments[0].trim().to_string(),
            segments[1].trim_end_matches(".git").trim().to_string(),
        )
    };
    if namespace.is_empty()
        || project.is_empty()
        || namespace.split('/').any(|part| matches!(part, "." | ".."))
        || matches!(project.as_str(), "." | "..")
    {
        return None;
    }
    Some((namespace, project, is_gitlab))
}

pub fn latest_release(homepage: &str) -> Result<Option<UpstreamRelease>, String> {
    let Some((owner, repo, is_gitlab)) = repository_parts(homepage) else {
        return Ok(None);
    };
    let key = format!(
        "{}:{owner}/{repo}",
        if is_gitlab { "gitlab" } else { "github" }
    );
    let cache = RELEASE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock() {
        if let Some((stored_at, release)) = cache.get(&key) {
            if stored_at.elapsed() < CACHE_TTL {
                return Ok(release.clone());
            }
        }
    }

    let result = fetch_release(&owner, &repo, is_gitlab);
    if let Ok(release) = &result {
        if let Ok(mut cache) = cache.lock() {
            if cache.len() >= CACHE_CAPACITY && !cache.contains_key(&key) {
                if let Some(oldest_key) = cache
                    .iter()
                    .min_by_key(|(_, (stored_at, _))| *stored_at)
                    .map(|(key, _)| key.clone())
                {
                    cache.remove(&oldest_key);
                }
            }
            cache.insert(key, (Instant::now(), release.clone()));
        }
    }
    result
}

fn fetch_release(
    owner: &str,
    repo: &str,
    is_gitlab: bool,
) -> Result<Option<UpstreamRelease>, String> {
    let client = client()?;
    if is_gitlab {
        let project_path = format!("{owner}/{repo}");
        let mut endpoint = reqwest::Url::parse("https://gitlab.com/api/v4/projects/")
            .map_err(|error| error.to_string())?;
        endpoint
            .path_segments_mut()
            .map_err(|_| "Could not prepare GitLab release URL".to_string())?
            .push(&project_path)
            .push("releases")
            .push("permalink")
            .push("latest");
        let response = client
            .get(endpoint)
            .send()
            .map_err(|error| format!("GitLab release lookup failed: {error}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let release: GitLabRelease = response
            .error_for_status()
            .map_err(|error| format!("GitLab release lookup failed: {error}"))?
            .json()
            .map_err(|error| format!("Could not read GitLab release metadata: {error}"))?;
        let url = release._links.and_then(|links| links.self_url);
        return Ok(Some(UpstreamRelease {
            version: release.tag_name.clone(),
            name: release
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| release.tag_name.clone()),
            published_at: release.released_at,
            url,
        }));
    }

    let endpoint = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let response = client
        .get(endpoint)
        .send()
        .map_err(|error| format!("GitHub release lookup failed: {error}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: GitHubRelease = response
        .error_for_status()
        .map_err(|error| format!("GitHub release lookup failed: {error}"))?
        .json()
        .map_err(|error| format!("Could not read GitHub release metadata: {error}"))?;
    if release.draft.unwrap_or(false) || release.prerelease.unwrap_or(false) {
        return Ok(None);
    }
    Ok(Some(UpstreamRelease {
        version: release.tag_name.clone(),
        name: release
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| release.tag_name.clone()),
        published_at: release.published_at,
        url: release.html_url,
    }))
}


/// Compare a numeric upstream tag with a package version only when the repository
/// slug matches the package name. Unsupported version schemes return None.
pub fn compare_package_version(
    homepage: &str,
    release_version: &str,
    package_name: &str,
    package_version: &str,
) -> Option<std::cmp::Ordering> {
    let (_, repository, _) = repository_parts(homepage)?;
    let normalize_name = |name: &str| {
        name.chars()
            .filter(|character| *character != '-' && *character != '_')
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    if normalize_name(&repository) != normalize_name(package_name) {
        return None;
    }

    let upstream = parse_numeric_version(release_version, false)?;
    let packaged = parse_numeric_version(package_version, true)?;
    let width = upstream.len().max(packaged.len());
    for index in 0..width {
        let upstream_part = upstream.get(index).copied().unwrap_or(0);
        let packaged_part = packaged.get(index).copied().unwrap_or(0);
        match upstream_part.cmp(&packaged_part) {
            std::cmp::Ordering::Equal => {}
            ordering => return Some(ordering),
        }
    }
    Some(std::cmp::Ordering::Equal)
}

fn parse_numeric_version(value: &str, is_package_version: bool) -> Option<Vec<u64>> {
    let value = value.trim();
    if value.contains(':') {
        // RPM epochs change ordering and cannot safely be ignored.
        return None;
    }
    let value = if is_package_version {
        value.split_once('-').map_or(value, |(version, _)| version)
    } else {
        value
    };
    let value = value
        .strip_prefix('v')
        .or_else(|| value.strip_prefix('V'))
        .unwrap_or(value);
    if value.is_empty() {
        return None;
    }
    value
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            part.parse::<u64>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::repository_parts;

    #[test]
    fn recognizes_github_and_gitlab_project_homepages() {
        assert_eq!(
            repository_parts("https://github.com/owner/project"),
            Some(("owner".into(), "project".into(), false))
        );
        assert_eq!(
            repository_parts("https://gitlab.com/group/project.git"),
            Some(("group".into(), "project".into(), true))
        );
        assert_eq!(
            repository_parts("https://gitlab.com/group/subgroup/project"),
            Some(("group/subgroup".into(), "project".into(), true))
        );
        assert_eq!(
            repository_parts("https://gitlab.com/group/subgroup/project/-/tree/main"),
            Some(("group/subgroup".into(), "project".into(), true))
        );
    }

    #[test]
    fn compares_versions_only_for_matching_packages_and_supported_syntax() {
        use super::compare_package_version;
        use std::cmp::Ordering;

        let homepage = "https://github.com/demo/tool";
        assert_eq!(
            compare_package_version(homepage, "v1.3.0", "tool", "1.2.9-2.fc44"),
            Some(Ordering::Greater)
        );
        assert_eq!(
            compare_package_version(homepage, "1.3.0", "tool", "1.3-1.fc44"),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_package_version(homepage, "1.2.0", "tool", "1.4.0-1.fc44"),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_package_version(homepage, "2.0.0", "different-package", "1.0.0-1"),
            None
        );
        assert_eq!(
            compare_package_version(homepage, "release-2.0", "tool", "1.0.0-1"),
            None
        );
        assert_eq!(
            compare_package_version(homepage, "2.0.0", "tool", "1:1.0.0-1"),
            None
        );
    }

    #[test]
    fn rejects_unverified_or_ambiguous_homepages() {
        assert_eq!(repository_parts("https://example.com/owner/project"), None);
        assert_eq!(repository_parts("file:///etc/passwd"), None);
        assert_eq!(repository_parts("https://github.com/owner"), None);
        assert_eq!(
            repository_parts("https://github.com/owner/project/-/tree/main"),
            Some(("owner".into(), "project".into(), false))
        );
    }
}
