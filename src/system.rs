use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const REPOSITORY_DIRECTORY: &str = "/etc/yum.repos.d";
const DNF5_SYSTEM_STATE_DIRECTORY: &str = "/usr/lib/sysimage/libdnf5";
const DNF5_SYSTEM_STATE_FILES: &[&str] = &[
    "environments.toml",
    "groups.toml",
    "modules.toml",
    "nevras.toml",
    "packages.toml",
    "system.toml",
];
const DNF5_SYSTEM_STATE_MODE: u32 = 0o644;
const MAX_COMMAND_DETAILS_CHARS: usize = 1200;
const INSTALLED_COPR_QUERY_FORMAT: &str = "%{name}\t%{evr}\t%{from_repo}\\n";
const PACKAGE_UPDATES_QUERY_FORMAT: &str = "%{name}\t%{evr}\t%{repoid}\\n";
const INSTALLED_ORIGINS_QUERY_FORMAT: &str = "%{name}|%{from_repo}\\n";

/// Read the major Fedora release from a parsed /etc/os-release file.
pub fn parse_fedora_release(os_release: &str) -> Option<u32> {
    let mut is_fedora = false;
    let mut version_id = None;

    for line in os_release.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "ID" => is_fedora = value.eq_ignore_ascii_case("fedora"),
            "VERSION_ID" => version_id = Some(value.to_string()),
            _ => {}
        }
    }

    if !is_fedora {
        return None;
    }

    version_id?.split('.').next()?.parse().ok()
}

pub fn current_fedora_release() -> Option<u32> {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|contents| parse_fedora_release(&contents))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoprRepository {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub file: PathBuf,
}

pub fn scan_copr_repositories() -> Result<Vec<CoprRepository>, String> {
    let directory = Path::new(REPOSITORY_DIRECTORY);
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Could not read {}: {error}", directory.display()))?;

    let mut repositories = Vec::new();
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("repo") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        repositories.extend(parse_copr_repositories(&contents, &path));
    }

    let mut overrides = HashMap::new();
    let override_directory = Path::new("/etc/dnf/repos.override.d");
    if let Ok(entries) = fs::read_dir(override_directory) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("repo") {
                continue;
            }
            if let Ok(contents) = fs::read_to_string(&path) {
                overrides.extend(parse_enabled_overrides(&contents));
            }
        }
    }

    for repository in &mut repositories {
        if let Some(enabled) = overrides.get(&repository.id) {
            repository.enabled = *enabled;
        }
    }

    repositories.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(repositories)
}

fn parse_enabled_value(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn parse_enabled_overrides(contents: &str) -> HashMap<String, bool> {
    let mut overrides = HashMap::new();
    let mut current_id: Option<String> = None;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let id = &line[1..line.len() - 1];
            current_id = if id.starts_with("copr:") {
                Some(id.to_string())
            } else {
                None
            };
            continue;
        }

        if let (Some(id), Some((key, value))) = (current_id.as_ref(), line.split_once('=')) {
            if key.trim().eq_ignore_ascii_case("enabled") {
                overrides.insert(id.clone(), parse_enabled_value(value));
            }
        }
    }

    overrides
}

fn parse_copr_repositories(contents: &str, path: &Path) -> Vec<CoprRepository> {
    let mut repositories = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_name = String::new();
    let mut current_enabled = true;

    let flush = |repositories: &mut Vec<CoprRepository>,
                 id: &mut Option<String>,
                 name: &mut String,
                 enabled: &mut bool| {
        if let Some(id) = id.take() {
            repositories.push(CoprRepository {
                id,
                name: if name.trim().is_empty() {
                    "COPR repository".to_string()
                } else {
                    name.trim().to_string()
                },
                enabled: *enabled,
                file: path.to_path_buf(),
            });
        }
        name.clear();
        *enabled = true;
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            flush(
                &mut repositories,
                &mut current_id,
                &mut current_name,
                &mut current_enabled,
            );
            let id = &line[1..line.len() - 1];
            if id.starts_with("copr:") {
                current_id = Some(id.to_string());
            }
            continue;
        }

        if current_id.is_none() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                key if key.trim().eq_ignore_ascii_case("name") => {
                    current_name = value.trim().to_string()
                }
                key if key.trim().eq_ignore_ascii_case("enabled") => {
                    current_enabled = parse_enabled_value(value);
                }
                _ => {}
            }
        }
    }

    flush(
        &mut repositories,
        &mut current_id,
        &mut current_name,
        &mut current_enabled,
    );
    repositories
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledCoprPackage {
    pub name: String,
    pub version: String,
    pub repository: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailablePackageUpdate {
    pub name: String,
    pub version: String,
    pub repository: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageDependencyImpact {
    pub requirements: Vec<String>,
    pub required_by: Vec<String>,
}

/// Whether the DNF5 package-state file has the ownership and permissions
/// expected by the Fedora DNF5 package, and is a safe repair candidate.
fn dnf5_state_file_is_repair_candidate(
    file_uid: u32,
    file_mode: u32,
    file_is_regular: bool,
    parent_uid: u32,
    parent_mode: u32,
    parent_is_directory: bool,
) -> bool {
    file_uid == 0
        && file_is_regular
        && parent_uid == 0
        && parent_is_directory
        && parent_mode & 0o022 == 0
        && file_mode & 0o7777 != DNF5_SYSTEM_STATE_MODE
}

fn dnf5_state_metadata() -> Result<(Vec<(PathBuf, fs::Metadata)>, fs::Metadata), String> {
    let directory = Path::new(DNF5_SYSTEM_STATE_DIRECTORY);
    let directory_metadata = fs::symlink_metadata(directory)
        .map_err(|error| format!("Could not inspect the DNF5 state directory: {error}"))?;
    let mut files = Vec::new();

    for name in DNF5_SYSTEM_STATE_FILES {
        let path = directory.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => files.push((path, metadata)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not inspect DNF5 state file {}: {error}",
                    path.display()
                ));
            }
        }
    }

    if files.is_empty() {
        return Err("No known DNF5 system-state TOML files were found.".to_string());
    }

    Ok((files, directory_metadata))
}

fn dnf5_state_files_are_safe(
    files: &[(PathBuf, fs::Metadata)],
    directory: &fs::Metadata,
) -> bool {
    directory.uid() == 0
        && directory.file_type().is_dir()
        && directory.mode() & 0o022 == 0
        && files
            .iter()
            .all(|(_, file)| file.uid() == 0 && file.file_type().is_file())
}

/// Detect restrictive permissions that prevent an ordinary user from reading
/// any of DNF5's known system package-state TOML files.
pub fn dnf5_system_state_permissions_need_repair() -> bool {
    let Ok((files, directory)) = dnf5_state_metadata() else {
        return false;
    };

    dnf5_state_files_are_safe(&files, &directory)
        && files.iter().any(|(_, file)| {
            dnf5_state_file_is_repair_candidate(
                file.uid(),
                file.mode(),
                file.file_type().is_file(),
                directory.uid(),
                directory.mode(),
                directory.file_type().is_dir(),
            )
        })
}

fn dnf5_state_error_is_permission_related(error: &str) -> bool {
    error.contains(DNF5_SYSTEM_STATE_DIRECTORY)
        && error.contains("error opening file")
        && DNF5_SYSTEM_STATE_FILES.iter().any(|name| {
            error.contains(&format!("{DNF5_SYSTEM_STATE_DIRECTORY}/{name}"))
        })
}

/// Whether a DNF failure is specifically caused by restrictive permissions on
/// a known state file and the complete state-file set is safe to repair.
pub fn dnf5_system_state_permission_repair_available(error: &str) -> bool {
    dnf5_state_error_is_permission_related(error) && dnf5_system_state_permissions_need_repair()
}

/// Restore readable permissions on all known DNF5 state TOML files. The paths
/// are fixed, and every existing target must be a root-owned regular file in
/// the root-owned, non-writable DNF5 state directory before pkexec is invoked.
pub fn repair_dnf5_system_state_permissions() -> Result<(), String> {
    let (files, directory) = dnf5_state_metadata()?;
    if !dnf5_state_files_are_safe(&files, &directory) {
        return Err(
            "For safety, Copr Hub will not change DNF5 state files because their ownership or directory permissions are unexpected."
                .to_string(),
        );
    }

    if files
        .iter()
        .all(|(_, file)| file.mode() & 0o7777 == DNF5_SYSTEM_STATE_MODE)
    {
        return Ok(());
    }

    let mut command = Command::new("pkexec");
    command
        .arg("/usr/bin/chmod")
        .arg("0644")
        .arg("--");
    for (path, _) in &files {
        command.arg(path);
    }
    let output = command
        .output()
        .map_err(|error| format!("Could not start the privileged permission repair: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Could not repair DNF5 system-state permissions: {}",
            command_failure_details(
                &String::from_utf8_lossy(&output.stderr),
                &String::from_utf8_lossy(&output.stdout),
                "Administrator authorization was cancelled or the permission change failed.",
            )
        ));
    }

    let (repaired_files, repaired_directory) = dnf5_state_metadata()?;
    if !dnf5_state_files_are_safe(&repaired_files, &repaired_directory)
        || repaired_files
            .iter()
            .any(|(_, file)| file.mode() & 0o7777 != DNF5_SYSTEM_STATE_MODE)
    {
        return Err(
            "One or more DNF5 state files did not have the expected root-owned 0644 mode after repair."
                .to_string(),
        );
    }

    Ok(())
}

pub fn list_installed_copr_packages() -> Result<Vec<InstalledCoprPackage>, String> {
    let output = Command::new("dnf")
        .args([
            "repoquery",
            "--installed",
            "--queryformat",
            INSTALLED_COPR_QUERY_FORMAT,
        ])
        .output()
        .map_err(|error| format!("Could not start DNF to inspect installed packages: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "DNF could not query installed packages. Check that DNF5 is installed.".to_string()
        } else {
            format!("DNF could not query installed packages: {stderr}")
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut packages = parse_installed_copr_packages(&stdout);
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

/// Query enabled repositories for upgrade candidates. DNF handles RPM EVR
/// ordering and repository policy; callers should only display returned candidates.
pub fn list_package_updates() -> Result<Vec<AvailablePackageUpdate>, String> {
    let output = Command::new("dnf")
        .args([
            "repoquery",
            "--upgrades",
            "--queryformat",
            PACKAGE_UPDATES_QUERY_FORMAT,
        ])
        .output()
        .map_err(|error| format!("Could not start DNF to check package updates: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "DNF could not check available package updates. Check enabled repositories and connectivity."
                .to_string()
        } else {
            format!("DNF could not check available package updates: {stderr}")
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut updates = parse_package_updates(&stdout);
    updates.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.repository.cmp(&right.repository))
            .then_with(|| left.version.cmp(&right.version))
    });
    Ok(updates)
}

fn parse_package_updates(output: &str) -> Vec<AvailablePackageUpdate> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next()?.trim();
            let version = fields.next()?.trim();
            let repository = fields.next()?.trim();
            if !is_valid_package_name(name) || version.is_empty() || repository.is_empty() {
                return None;
            }
            Some(AvailablePackageUpdate {
                name: name.to_string(),
                version: version.to_string(),
                repository: repository.to_string(),
            })
        })
        .collect()
}

/// Inspect the local RPM database only. No repository refresh or network request
/// is made, so this view remains useful offline.
pub fn inspect_package_dependency_impact(
    package_name: &str,
) -> Result<PackageDependencyImpact, String> {
    if !is_valid_package_name(package_name) {
        return Err("The package name contains unsupported characters.".to_string());
    }

    let requirements = run_rpm_query(&["-q", "--requires", package_name])?;
    let reverse_output = run_rpm_query_allow_empty(&["-q", "--whatrequires", package_name])?;
    Ok(PackageDependencyImpact {
        requirements: bounded_lines(&requirements),
        required_by: bounded_lines(&reverse_output),
    })
}

fn run_rpm_query(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("rpm")
        .args(arguments)
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("Could not start RPM to inspect dependencies: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "RPM could not inspect the installed package database.".to_string()
        } else {
            format!("RPM dependency query failed: {stderr}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_rpm_query_allow_empty(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("rpm")
        .args(arguments)
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("Could not start RPM to inspect dependencies: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let details = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    if details.contains("no package requires") || details.contains("no package provides") {
        return Ok(String::new());
    }
    let details = details.trim();
    Err(if details.is_empty() {
        "RPM could not inspect reverse dependencies.".to_string()
    } else {
        format!("RPM reverse-dependency query failed: {details}")
    })
}

fn bounded_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(100)
        .map(str::to_string)
        .collect()
}

fn is_valid_package_name(package_name: &str) -> bool {
    package_name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && package_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "+._-".contains(character))
}

fn parse_installed_copr_packages(output: &str) -> Vec<InstalledCoprPackage> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next()?.trim();
            let version = fields.next()?.trim();
            let repository = fields.next()?.trim();
            if name.is_empty() || version.is_empty() || repository.is_empty() {
                return None;
            }
            if !repository.to_ascii_lowercase().starts_with("copr:") {
                return None;
            }
            Some(InstalledCoprPackage {
                name: name.to_string(),
                version: version.to_string(),
                repository: repository.to_string(),
            })
        })
        .collect()
}

fn is_valid_copr_repository_id(repository_id: &str) -> bool {
    repository_id
        .strip_prefix("copr:")
        .is_some_and(|suffix| !suffix.is_empty())
        && repository_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ":._-".contains(character))
}

/// Cache installed RPM origins for the current app session so each repository row
/// does not launch a separate DNF query. Refreshing the repository page invalidates it.
type InstalledOrigins = Result<HashMap<String, Vec<String>>, String>;
static INSTALLED_ORIGINS: OnceLock<Mutex<Option<InstalledOrigins>>> = OnceLock::new();

pub fn invalidate_installed_package_origins() {
    if let Some(cache) = INSTALLED_ORIGINS.get() {
        if let Ok(mut cache) = cache.lock() {
            *cache = None;
        }
    }
}

/// Installed RPMs whose recorded source repository matches the supplied COPR ID.
/// DNF metadata can be unavailable offline; callers receive an explicit error.
pub fn installed_packages_from_repository(repository_id: &str) -> Result<Vec<String>, String> {
    if !is_safe_repository_id(repository_id) {
        return Err("Invalid repository ID.".to_string());
    }
    let cache = INSTALLED_ORIGINS.get_or_init(|| Mutex::new(None));
    let mut cached = cache
        .lock()
        .map_err(|_| "Installed package-origin cache is unavailable.".to_string())?;
    if cached.is_none() {
        *cached = Some(query_installed_package_origins());
    }
    match cached.as_ref().expect("cache initialized") {
        Ok(origins) => Ok(origins.get(repository_id).cloned().unwrap_or_default()),
        Err(error) => Err(error.clone()),
    }
}

fn query_installed_package_origins() -> InstalledOrigins {
    let output = Command::new("dnf")
        .args([
            "repoquery",
            "--installed",
            "--queryformat",
            INSTALLED_ORIGINS_QUERY_FORMAT,
        ])
        .output()
        .map_err(|error| {
            format!("Could not run DNF to inspect installed package origins: {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "DNF could not inspect installed package origins: {}",
            command_failure_details(
                &String::from_utf8_lossy(&output.stderr),
                "",
                "DNF returned no error details.",
            )
        ));
    }
    Ok(parse_installed_package_origins(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_installed_package_origins(output: &str) -> HashMap<String, Vec<String>> {
    let mut origins: HashMap<String, Vec<String>> = HashMap::new();
    for line in output.lines() {
        let Some((name, origin)) = line.split_once('|') else {
            continue;
        };
        let name = name.trim();
        let origin = origin.trim();
        if is_valid_package_name(name) && is_safe_repository_id(origin) {
            origins
                .entry(origin.to_string())
                .or_default()
                .push(name.to_string());
        }
    }
    for packages in origins.values_mut() {
        packages.sort();
        packages.dedup();
    }
    origins
}

fn is_safe_repository_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+' | b':')
        })
}

pub fn copr_project_spec_from_repository_id(repository_id: &str) -> Option<String> {
    let suffix = repository_id.strip_prefix("copr:copr.fedorainfracloud.org:")?;
    let (owner, project) = suffix.split_once(':')?;
    let valid_owner = !owner.is_empty()
        && owner
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "@._-".contains(character));
    let valid_project = !project.is_empty()
        && project
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ":._-".contains(character));
    (valid_owner && valid_project).then(|| format!("{owner}/{project}"))
}

/// Build a copyable DNF command for a supported standard COPR repository.
pub fn copr_repository_command(repository_id: &str, enable: bool) -> Option<String> {
    let project_spec = copr_project_spec_from_repository_id(repository_id)?;
    let action = if enable { "enable" } else { "disable" };
    Some(format!("dnf copr {action} {project_spec}"))
}

pub fn remove_copr_repository(repository_id: &str) -> Result<(), String> {
    let project_spec = copr_project_spec_from_repository_id(repository_id).ok_or_else(|| {
        "This repository ID is not a supported standard Fedora COPR project; it was left unchanged."
            .to_string()
    })?;

    let output = Command::new("pkexec")
        .args(["dnf", "--assumeyes", "copr", "remove", &project_spec])
        .output()
        .map_err(|error| format!("Could not start the privileged DNF helper: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let details = command_failure_details(
        &String::from_utf8_lossy(&output.stderr),
        &String::from_utf8_lossy(&output.stdout),
        "Authentication may have been cancelled, or the DNF COPR plugin may be unavailable.",
    );
    Err(format!("DNF could not remove the repository: {details}"))
}

pub fn set_copr_repository_enabled(repository_id: &str, enabled: bool) -> Result<(), String> {
    if !is_valid_copr_repository_id(repository_id) {
        return Err("The repository identifier is not a valid COPR repository ID.".to_string());
    }

    let setting = format!(
        "{repository_id}.enabled={}",
        if enabled { "1" } else { "0" }
    );
    let output = Command::new("pkexec")
        .args(["dnf", "config-manager", "setopt", &setting])
        .output()
        .map_err(|error| format!("Could not start the privileged DNF helper: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let details = command_failure_details(
        &String::from_utf8_lossy(&output.stderr),
        &String::from_utf8_lossy(&output.stdout),
        "Authentication may have been cancelled, or the DNF config-manager plugin may be unavailable.",
    );
    Err(format!("DNF could not change the repository: {details}"))
}

pub fn set_package_installed(package_name: &str, install: bool) -> Result<(), String> {
    if !is_valid_package_name(package_name) {
        return Err("The package name contains unsupported characters.".to_string());
    }

    let action = if install { "install" } else { "remove" };
    let mut command = Command::new("pkexec");
    command.args(["dnf", action, "--assumeyes"]);
    if !install {
        // Removing a package should use local metadata when offline instead of
        // attempting a repository refresh. DNF may still fail if its cache is
        // insufficient to resolve the transaction.
        command.arg("--cacheonly");
    }
    let output = command
        .arg(package_name)
        .output()
        .map_err(|error| format!("Could not start the privileged DNF helper: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let details = command_failure_details(
        &String::from_utf8_lossy(&output.stderr),
        &String::from_utf8_lossy(&output.stdout),
        "Authentication may have been cancelled, or DNF could not complete the transaction.",
    );
    Err(format!("DNF could not {action} {package_name}: {details}"))
}

fn command_failure_details(stderr: &str, stdout: &str, fallback: &str) -> String {
    let mut details = Vec::new();
    if !stderr.trim().is_empty() {
        details.push(format!("Error: {}", stderr.trim()));
    }
    if !stdout.trim().is_empty() {
        details.push(format!("Output: {}", stdout.trim()));
    }

    let details = if details.is_empty() {
        fallback.to_string()
    } else {
        details.join("\n")
    };

    let mut bounded: String = details.chars().take(MAX_COMMAND_DETAILS_CHARS).collect();
    if details.chars().count() > MAX_COMMAND_DETAILS_CHARS {
        bounded.push_str("… (output truncated)");
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnf5_state_permission_error_matches_any_known_state_file() {
        for name in DNF5_SYSTEM_STATE_FILES {
            let error = format!(
                "Loading system state TOML file failed: toml::parse: error opening file \"{DNF5_SYSTEM_STATE_DIRECTORY}/{name}\""
            );
            assert!(dnf5_state_error_is_permission_related(&error), "{name}");
        }
        assert!(!dnf5_state_error_is_permission_related(
            "toml::parse: error opening file \"/etc/dnf/other.toml\""
        ));
        assert!(!dnf5_state_error_is_permission_related(
            "packages.toml: invalid TOML syntax"
        ));
    }

    #[test]
    fn dnf5_state_permission_repair_requires_root_owned_regular_files_and_safe_directory() {
        assert!(dnf5_state_file_is_repair_candidate(
            0, 0o660, true, 0, 0o755, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            0, 0o644, true, 0, 0o755, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            1000, 0o660, true, 0, 0o755, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            0, 0o660, false, 0, 0o755, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            0, 0o660, true, 1000, 0o755, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            0, 0o660, true, 0, 0o775, true
        ));
        assert!(!dnf5_state_file_is_repair_candidate(
            0, 0o660, true, 0, 0o755, false
        ));
        assert_eq!(
            DNF5_SYSTEM_STATE_FILES,
            &[
                "environments.toml",
                "groups.toml",
                "modules.toml",
                "nevras.toml",
                "packages.toml",
                "system.toml",
            ]
        );
    }

    #[test]
    fn command_failure_details_include_both_streams_and_bound_output() {
        let details =
            command_failure_details(" permission denied ", "transaction failed", "fallback");
        assert!(details.contains("Error: permission denied"));
        assert!(details.contains("Output: transaction failed"));

        let long_output = "x".repeat(MAX_COMMAND_DETAILS_CHARS + 100);
        let details = command_failure_details(&long_output, "", "fallback");
        assert!(
            details.chars().count()
                <= MAX_COMMAND_DETAILS_CHARS + "… (output truncated)".chars().count()
        );
        assert!(details.ends_with("… (output truncated)"));
    }

    #[test]
    fn command_failure_details_use_fallback_when_streams_are_empty() {
        assert_eq!(
            command_failure_details("  ", "\n", "Authentication was cancelled."),
            "Authentication was cancelled."
        );
    }

    #[test]
    fn converts_only_standard_fedora_copr_ids_to_project_specs() {
        assert_eq!(
            copr_project_spec_from_repository_id("copr:copr.fedorainfracloud.org:alice:demo"),
            Some("alice/demo".to_string())
        );
        assert_eq!(
            copr_project_spec_from_repository_id(
                "copr:copr.fedorainfracloud.org:@team:demo:custom"
            ),
            Some("@team/demo:custom".to_string())
        );
        assert_eq!(
            copr_project_spec_from_repository_id("copr:other.example:alice:demo"),
            None
        );
        assert_eq!(
            copr_project_spec_from_repository_id("copr:copr.fedorainfracloud.org:alice:../unsafe"),
            None
        );
        assert_eq!(copr_project_spec_from_repository_id("fedora"), None);
    }

    #[test]
    fn validates_copr_repository_ids_before_invoking_privileged_commands() {
        assert!(is_valid_copr_repository_id(
            "copr:copr.fedorainfracloud.org:alice:demo"
        ));
        assert!(!is_valid_copr_repository_id("fedora"));
        assert!(!is_valid_copr_repository_id("copr:"));
        assert!(!is_valid_copr_repository_id("copr:demo;touch /tmp/bad"));
        assert!(!is_valid_copr_repository_id("copr:demo/../../bad"));
    }

    #[test]
    fn rejects_package_names_with_option_or_shell_characters() {
        assert!(set_package_installed("--assumeyes", true).is_err());
        assert!(set_package_installed("demo;touch /tmp/bad", true).is_err());
    }

    #[test]
    fn parses_dnf5_repository_enabled_overrides() {
        let contents = r#"
[copr:copr.fedorainfracloud.org:alice:demo]
enabled=0
[fedora]
enabled=1
"#;
        let overrides = parse_enabled_overrides(contents);
        assert_eq!(
            overrides.get("copr:copr.fedorainfracloud.org:alice:demo"),
            Some(&false)
        );
        assert!(!overrides.contains_key("fedora"));
    }

    #[test]
    fn parses_common_boolean_spellings_case_insensitively() {
        for value in ["0", "false", "FALSE", "False", "no", "NO", "off", "OFF"] {
            assert!(
                !parse_enabled_value(value),
                "{value} should disable a repository"
            );
        }
        for value in ["1", "true", "TRUE", "yes", "on", "unexpected"] {
            assert!(parse_enabled_value(value), "{value} should remain enabled");
        }
    }

    #[test]
    fn repository_option_names_are_case_insensitive() {
        let contents = "[copr:copr.fedorainfracloud.org:alice:demo]\nNAME=Demo\nENABLED=OFF\n";
        let repositories = parse_copr_repositories(contents, Path::new("/tmp/test.repo"));
        assert_eq!(repositories.len(), 1);
        assert_eq!(repositories[0].name, "Demo");
        assert!(!repositories[0].enabled);

        let overrides = parse_enabled_overrides(contents);
        assert_eq!(
            overrides.get("copr:copr.fedorainfracloud.org:alice:demo"),
            Some(&false)
        );
    }

    #[test]
    fn repository_files_and_overrides_share_boolean_parsing() {
        let contents = "[copr:copr.fedorainfracloud.org:alice:demo]\nenabled=FALSE\n";
        let repositories = parse_copr_repositories(contents, Path::new("/tmp/test.repo"));
        assert_eq!(repositories.len(), 1);
        assert!(!repositories[0].enabled);

        let overrides = parse_enabled_overrides(contents);
        assert_eq!(
            overrides.get("copr:copr.fedorainfracloud.org:alice:demo"),
            Some(&false)
        );
    }

    #[test]
    fn parses_update_candidates_and_ignores_malformed_rows() {
        let output = "demo\t2.0-1.fc44\tcopr:alice:demo\nbase\t3.0-1.fc44\tfedora\nmissing-version\t\tfedora\n--bad\t1.0\tfedora\n";
        let updates = parse_package_updates(output);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "demo");
        assert_eq!(updates[0].version, "2.0-1.fc44");
        assert_eq!(updates[1].name, "base");
        assert_eq!(updates[1].repository, "fedora");
    }

    #[test]
    fn dependency_output_is_trimmed_and_bounded() {
        let output = (0..105)
            .map(|index| format!(" capability-{index} "))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = bounded_lines(&output);
        assert_eq!(lines.len(), 100);
        assert_eq!(lines[0], "capability-0");
        assert_eq!(lines[99], "capability-99");
    }

    #[test]
    fn package_name_validation_rejects_options_and_shell_syntax() {
        assert!(is_valid_package_name("gtk4"));
        assert!(is_valid_package_name("python3.13"));
        assert!(!is_valid_package_name("--requires"));
        assert!(!is_valid_package_name("demo;touch"));
        assert!(!is_valid_package_name(""));
    }

    #[test]
    fn filters_installed_packages_to_copr_sources() {
        let output = "demo\t1.2-1.fc44\tcopr:copr.fedorainfracloud.org:alice:demo\nbase\t2.0-1.fc44\tfedora\nfalse-positive\t1.0-1\tnotcopr:example\n";
        let packages = parse_installed_copr_packages(output);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "demo");
        assert_eq!(packages[0].version, "1.2-1.fc44");
    }

    #[test]
    fn ignores_malformed_installed_package_rows() {
        let output = "missing-version\t\tcopr:copr.fedorainfracloud.org:alice:demo\nmissing-repo\t1.0-1\t\nmissing-columns\nvalid\t1.0-1\tcopr:copr.fedorainfracloud.org:alice:demo\n";
        let packages = parse_installed_copr_packages(output);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "valid");
    }

    #[test]
    fn dnf_query_formats_explicitly_terminate_each_result_row() {
        assert!(INSTALLED_COPR_QUERY_FORMAT.ends_with("\\n"));
        assert!(PACKAGE_UPDATES_QUERY_FORMAT.ends_with("\\n"));
        assert!(INSTALLED_ORIGINS_QUERY_FORMAT.ends_with("\\n"));
    }

    #[test]
    fn parses_dnf5_installed_origins_as_separate_rows() {
        let output = concat!(
            "gowall|copr:copr.fedorainfracloud.org:achno:gowall\n",
            "gowall-cli|copr:copr.fedorainfracloud.org:achno:gowall\n",
            "fedora-package|fedora\n",
            "gowall|copr:copr.fedorainfracloud.org:achno:gowall\n",
            "malformed-row\n",
            "--invalid|copr:copr.fedorainfracloud.org:achno:gowall\n",
        );

        let origins = parse_installed_package_origins(output);
        assert_eq!(
            origins.get("copr:copr.fedorainfracloud.org:achno:gowall"),
            Some(&vec!["gowall".to_string(), "gowall-cli".to_string()])
        );
        assert_eq!(
            origins.get("fedora"),
            Some(&vec!["fedora-package".to_string()])
        );
    }

    #[test]
    fn installed_origin_parser_returns_empty_map_for_empty_output() {
        assert!(parse_installed_package_origins("").is_empty());
    }

    #[test]
    fn builds_copyable_commands_only_for_standard_copr_repositories() {
        let id = "copr:copr.fedorainfracloud.org:alice:demo";
        assert_eq!(
            copr_repository_command(id, true).as_deref(),
            Some("dnf copr enable alice/demo")
        );
        assert_eq!(
            copr_repository_command(id, false).as_deref(),
            Some("dnf copr disable alice/demo")
        );
        assert_eq!(
            copr_repository_command("copr:example:alice:demo", true),
            None
        );
    }

    #[test]
    fn parses_enabled_and_disabled_copr_sections() {
        let contents = r#"
[copr:copr.fedorainfracloud.org:alice:editor]
name=Alice's Editor
enabled=1

[fedora]
name=Fedora
enabled=1

[copr:copr.fedorainfracloud.org:bob:theme]
name=Bob's Theme
enabled=0
"#;

        let repositories = parse_copr_repositories(contents, Path::new("/tmp/test.repo"));
        assert_eq!(repositories.len(), 2);
        assert_eq!(
            repositories[0].id,
            "copr:copr.fedorainfracloud.org:alice:editor"
        );
        assert!(repositories[0].enabled);
        assert_eq!(repositories[0].name, "Alice's Editor");
        assert!(!repositories[1].enabled);
        assert_eq!(repositories[1].name, "Bob's Theme");
    }

    #[test]
    fn defaults_repository_to_enabled_when_key_is_missing() {
        let contents = "[copr:copr.fedorainfracloud.org:alice:demo]\nname=Demo\n";
        let repositories = parse_copr_repositories(contents, Path::new("/tmp/test.repo"));
        assert_eq!(repositories.len(), 1);
        assert!(repositories[0].enabled);
    }

    #[test]
    fn ignores_non_copr_sections() {
        let contents = "[fedora]\nname=Fedora\n[copr:example:owner:project]\nname=Project\n";
        let repositories = parse_copr_repositories(contents, Path::new("/tmp/test.repo"));
        assert_eq!(repositories.len(), 1);
        assert_eq!(repositories[0].name, "Project");
    }

    #[test]
    fn detects_only_a_valid_fedora_release_from_os_release() {
        assert_eq!(
            parse_fedora_release("NAME=Fedora Linux\nID=fedora\nVERSION_ID=44\n"),
            Some(44)
        );
        assert_eq!(
            parse_fedora_release("ID=fedora\nVERSION_ID=44.2026\n"),
            Some(44)
        );
        assert_eq!(parse_fedora_release("ID=nobara\nVERSION_ID=44\n"), None);
        assert_eq!(
            parse_fedora_release("ID=fedora\nVERSION_ID=rawhide\n"),
            None
        );
    }
}
