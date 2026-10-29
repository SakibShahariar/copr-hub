use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_STALE_CACHE_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_CAPACITY: usize = 128;

type ReleaseCache = Mutex<HashMap<String, (Instant, Option<UpstreamRelease>)>>;

static RELEASE_CACHE: OnceLock<ReleaseCache> = OnceLock::new();
static CACHE_LOADED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
struct PersistedCacheEntry {
    cached_at_unix: u64,
    release: Option<UpstreamRelease>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedReleaseCache {
    entries: HashMap<String, PersistedCacheEntry>,
}

fn persistent_cache_path() -> PathBuf {
    gtk::glib::user_cache_dir()
        .join("copr-hub")
        .join("upstream-releases.json")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn persisted_entry_age(cached_at_unix: u64, now_unix: u64) -> Option<Duration> {
    let age = now_unix.checked_sub(cached_at_unix)?;
    (age <= MAX_STALE_CACHE_AGE.as_secs()).then(|| Duration::from_secs(age))
}

/// Load recent release metadata once so offline launches can still show useful
/// version/date information. The cache is advisory and can always be discarded.
fn load_persistent_cache() {
    let path = persistent_cache_path();
    let Ok(metadata) = fs::metadata(&path) else {
        return;
    };
    if metadata.len() > 2 * 1024 * 1024 {
        return;
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    let Ok(saved) = serde_json::from_str::<PersistedReleaseCache>(&contents) else {
        return;
    };
    let now = unix_now();
    let entries = saved
        .entries
        .into_iter()
        .filter_map(|(key, entry)| {
            let age = persisted_entry_age(entry.cached_at_unix, now)?;
            if key.len() > 1024 {
                return None;
            }
            Some((key, (Instant::now() - age, entry.release)))
        })
        .take(CACHE_CAPACITY)
        .collect::<HashMap<_, _>>();
    if let Ok(mut cache) = RELEASE_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        cache.extend(entries);
    }
}

fn save_persistent_cache() {
    let Some(cache) = RELEASE_CACHE.get() else {
        return;
    };
    let Ok(cache) = cache.lock() else {
        return;
    };
    let now = unix_now();
    let entries = cache
        .iter()
        .filter_map(|(key, (stored_at, release))| {
            let age = stored_at.elapsed().as_secs();
            (age <= MAX_STALE_CACHE_AGE.as_secs()).then(|| {
                (
                    key.clone(),
                    PersistedCacheEntry {
                        cached_at_unix: now.saturating_sub(age),
                        release: release.clone(),
                    },
                )
            })
        })
        .collect();
    let saved = PersistedReleaseCache { entries };
    let Ok(contents) = serde_json::to_vec(&saved) else {
        return;
    };
    let path = persistent_cache_path();
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_ok() {
        let _ = crate::storage::atomic_write(&path, &contents);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRelease {
    pub version: String,
    pub name: String,
    pub published_at: Option<String>,
    pub url: Option<String>,
    pub stale_cache: bool,
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

fn has_rate_limit_headers(headers: &reqwest::header::HeaderMap) -> bool {
    headers.contains_key(reqwest::header::RETRY_AFTER)
        || headers
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|remaining| remaining == "0")
}

fn is_rate_limited_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || (status == reqwest::StatusCode::FORBIDDEN && has_rate_limit_headers(headers))
}

fn is_retryable_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> bool {
    is_retryable_status(status) || is_rate_limited_response(status, headers)
}

fn upstream_status_error(
    provider: &str,
    status: reqwest::StatusCode,
    rate_limited: bool,
) -> String {
    if rate_limited {
        format!("{provider} API rate limit reached; wait a moment before checking releases again.")
    } else if status.is_server_error() {
        format!("{provider} is temporarily unavailable (HTTP {status}); try again later.")
    } else {
        format!("{provider} release lookup failed (HTTP {status}).")
    }
}

/// Retry one time for temporary upstream failures; release lookups are safe GET requests.
fn send_with_retry(
    request: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let retry_request = request.try_clone();
    match request.send() {
        Ok(response)
            if is_retryable_response(response.status(), response.headers())
                && retry_request.is_some() =>
        {
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
        }
        Ok(response) => Ok(response),
        Err(error) if (error.is_timeout() || error.is_connect()) && retry_request.is_some() => {
            thread::sleep(Duration::from_millis(350));
            retry_request
                .expect("retry request was checked above")
                .send()
        }
        Err(error) => Err(error),
    }
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
    CACHE_LOADED.get_or_init(load_persistent_cache);
    let cache = RELEASE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock() {
        if let Some((stored_at, release)) = cache.get(&key) {
            if stored_at.elapsed() < CACHE_TTL {
                return Ok(release.clone());
            }
        }
    }

    let stale_candidate = cache.lock().ok().and_then(|cache| {
        cache
            .get(&key)
            .and_then(|(stored_at, release)| stale_cache_fallback(*stored_at, release.clone()))
    });

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
        save_persistent_cache();
        return result;
    }

    if let Some(mut release) = stale_candidate {
        release.stale_cache = true;
        return Ok(Some(release));
    }
    result
}

fn stale_cache_fallback(
    stored_at: Instant,
    release: Option<UpstreamRelease>,
) -> Option<UpstreamRelease> {
    if stored_at.elapsed() > MAX_STALE_CACHE_AGE {
        return None;
    }
    let mut release = release?;
    release.stale_cache = true;
    Some(release)
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
        let response = send_with_retry(client.get(endpoint))
            .map_err(|error| format!("GitLab release lookup failed: {error}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(upstream_status_error(
                "GitLab",
                response.status(),
                is_rate_limited_response(response.status(), response.headers()),
            ));
        }
        let release: GitLabRelease = response
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
            stale_cache: false,
        }));
    }

    let endpoint = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let response = send_with_retry(client.get(endpoint))
        .map_err(|error| format!("GitHub release lookup failed: {error}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(upstream_status_error(
            "GitHub",
            response.status(),
            is_rate_limited_response(response.status(), response.headers()),
        ));
    }
    let release: GitHubRelease = response
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
        stale_cache: false,
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

fn date_to_epoch_days(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.get(..2)?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }

    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn release_is_older_than(
    published_at: Option<&str>,
    now_unix_seconds: u64,
    threshold_days: u64,
) -> Option<bool> {
    let date = published_at?.get(..10)?;
    let published_days = date_to_epoch_days(date)?;
    let current_days = (now_unix_seconds / 86_400) as i64;
    if published_days > current_days {
        return Some(false);
    }
    Some((current_days - published_days) as u64 > threshold_days)
}

/// Returns true when a stable release date is known to be more than one year old.
pub fn release_is_stale(published_at: Option<&str>) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    release_is_older_than(published_at, now, 365).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::repository_parts;

    #[test]
    fn retries_only_rate_limits_and_server_errors() {
        use super::is_retryable_status;
        use reqwest::StatusCode;

        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::FORBIDDEN));
        assert!(!is_retryable_status(StatusCode::OK));
    }

    #[test]
    fn persisted_cache_entries_reject_future_and_expired_timestamps() {
        use super::{persisted_entry_age, MAX_STALE_CACHE_AGE};
        use std::time::Duration;

        assert_eq!(
            persisted_entry_age(900, 1_000),
            Some(Duration::from_secs(100))
        );
        assert_eq!(
            persisted_entry_age(1_000, 900),
            None,
            "future timestamps should not be treated as fresh cache entries"
        );
        assert_eq!(
            persisted_entry_age(
                1_000,
                1_000 + MAX_STALE_CACHE_AGE.as_secs() + 1
            ),
            None,
            "entries older than the stale-cache window should be discarded"
        );
    }

    #[test]
    fn persisted_cache_payload_round_trips_release_metadata() {
        use super::{PersistedCacheEntry, PersistedReleaseCache, UpstreamRelease};
        use std::collections::HashMap;

        let release = UpstreamRelease {
            version: "v2.4.1".into(),
            name: "Stable release".into(),
            published_at: Some("2026-10-01T12:00:00Z".into()),
            url: Some("https://github.com/example/project/releases/tag/v2.4.1".into()),
            stale_cache: false,
        };
        let payload = PersistedReleaseCache {
            entries: HashMap::from([(
                "github:example/project".into(),
                PersistedCacheEntry {
                    cached_at_unix: 1_800_000_000,
                    release: Some(release),
                },
            )]),
        };

        let encoded = serde_json::to_vec(&payload).unwrap();
        let decoded: PersistedReleaseCache = serde_json::from_slice(&encoded).unwrap();
        let entry = decoded.entries.get("github:example/project").unwrap();
        assert_eq!(entry.cached_at_unix, 1_800_000_000);
        let release = entry.release.as_ref().unwrap();
        assert_eq!(release.version, "v2.4.1");
        assert_eq!(
            release.published_at.as_deref(),
            Some("2026-10-01T12:00:00Z")
        );
    }

    #[test]
    fn stale_cache_fallback_is_limited_and_marked() {
        use super::{stale_cache_fallback, UpstreamRelease, MAX_STALE_CACHE_AGE};
        use std::time::{Duration, Instant};

        let release = || UpstreamRelease {
            version: "v1.2.3".into(),
            name: "Release 1.2.3".into(),
            published_at: Some("2026-09-01T00:00:00Z".into()),
            url: Some("https://github.com/demo/tool/releases/tag/v1.2.3".into()),
            stale_cache: false,
        };

        let cached = stale_cache_fallback(Instant::now(), Some(release())).unwrap();
        assert!(cached.stale_cache);
        assert!(stale_cache_fallback(Instant::now(), None).is_none());

        let expired_at = Instant::now() - MAX_STALE_CACHE_AGE - Duration::from_secs(1);
        assert!(stale_cache_fallback(expired_at, Some(release())).is_none());
    }

    #[test]
    fn upstream_http_errors_explain_rate_limits_and_temporary_outages() {
        use super::upstream_status_error;
        use reqwest::StatusCode;

        assert!(upstream_status_error("GitHub", StatusCode::FORBIDDEN, true)
            .contains("rate limit reached"));
        assert!(
            upstream_status_error("GitLab", StatusCode::SERVICE_UNAVAILABLE, false)
                .contains("temporarily unavailable")
        );
        assert!(upstream_status_error("GitHub", StatusCode::FORBIDDEN, false).contains("HTTP 403"));
    }

    #[test]
    fn retries_github_forbidden_responses_only_when_rate_limited() {
        use super::is_retryable_response;
        use reqwest::header::{HeaderMap, HeaderValue};
        use reqwest::StatusCode;

        let mut headers = HeaderMap::new();
        assert!(!is_retryable_response(StatusCode::FORBIDDEN, &headers));

        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        assert!(is_retryable_response(StatusCode::FORBIDDEN, &headers));

        headers.clear();
        headers.insert(reqwest::header::RETRY_AFTER, HeaderValue::from_static("2"));
        assert!(is_retryable_response(StatusCode::FORBIDDEN, &headers));
        assert!(!is_retryable_response(StatusCode::NOT_FOUND, &headers));
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
    fn retries_a_transient_server_response_once() {
        use super::send_with_retry;
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for status in ["503 Service Unavailable", "200 OK"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 1024];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        });

        let response = send_with_retry(
            reqwest::blocking::Client::new().get(format!("http://{address}/release")),
        )
        .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        server.join().unwrap();
    }

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
    fn identifies_old_release_dates_without_guessing_invalid_dates() {
        use super::release_is_older_than;

        let now = 1_800_000_000;
        assert_eq!(
            release_is_older_than(Some("2026-12-31T00:00:00Z"), now, 365),
            Some(false)
        );
        assert_eq!(
            release_is_older_than(Some("2025-01-01T00:00:00Z"), now, 365),
            Some(true)
        );
        assert_eq!(release_is_older_than(None, now, 365), None);
        assert_eq!(release_is_older_than(Some("not-a-date"), now, 365), None);
        assert_eq!(
            release_is_older_than(Some("2025-02-30T00:00:00Z"), now, 365),
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
