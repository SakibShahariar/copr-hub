use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPOSITORY_DIRECTORY: &str = "/etc/yum.repos.d";

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
    for entry in entries.flatten() {
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
        for entry in entries.flatten() {
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
            if key.trim() == "enabled" {
                overrides.insert(id.clone(), !matches!(value.trim(), "0" | "false" | "False"));
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
                "name" => current_name = value.trim().to_string(),
                "enabled" => {
                    current_enabled = !matches!(value.trim(), "0" | "false" | "False");
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

pub fn list_installed_copr_packages() -> Result<Vec<InstalledCoprPackage>, String> {
    let output = Command::new("dnf")
        .args([
            "repoquery",
            "--installed",
            "--queryformat",
            "%{name}\t%{evr}\t%{from_repo}",
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

fn parse_installed_copr_packages(output: &str) -> Vec<InstalledCoprPackage> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next()?.trim();
            let version = fields.next()?.trim();
            let repository = fields.next()?.trim();
            if name.is_empty() || repository.is_empty() {
                return None;
            }
            if !repository.to_ascii_lowercase().contains("copr:") {
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

pub fn set_copr_repository_enabled(repository_id: &str, enabled: bool) -> Result<(), String> {
    if !repository_id.starts_with("copr:")
        || !repository_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ":._-".contains(character))
    {
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

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err("DNF could not change the repository. Authentication may have been cancelled, or the DNF config-manager plugin may be unavailable.".to_string())
    } else {
        Err(format!("DNF could not change the repository: {stderr}"))
    }
}

pub fn set_package_installed(package_name: &str, install: bool) -> Result<(), String> {
    if !package_name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        || !package_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "+._-".contains(character))
    {
        return Err("The package name contains unsupported characters.".to_string());
    }

    let action = if install { "install" } else { "remove" };
    let output = Command::new("pkexec")
        .args(["dnf", action, "--assumeyes", package_name])
        .output()
        .map_err(|error| format!("Could not start the privileged DNF helper: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "Authentication may have been cancelled, or DNF could not complete the transaction."
            .to_string()
    };
    Err(format!("DNF could not {action} {package_name}: {details}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_package_names_with_option_or_shell_characters() {
        assert!(set_package_installed("--assumeyes", true).is_err());
        assert!(set_package_installed("demo;touch /tmp/bad", true).is_err());
    }

    #[test]
    fn parses_dnf5_repository_enabled_overrides() {
        let contents =
            "[copr:copr.fedorainfracloud.org:alice:demo]\\nenabled=0\\n[fedora]\\nenabled=1\\n";
        let overrides = parse_enabled_overrides(contents);
        assert_eq!(
            overrides.get("copr:copr.fedorainfracloud.org:alice:demo"),
            Some(&false)
        );
        assert!(!overrides.contains_key("fedora"));
    }

    #[test]
    fn filters_installed_packages_to_copr_sources() {
        let output = "demo\t1.2-1.fc44\tcopr:copr.fedorainfracloud.org:alice:demo\nbase\t2.0-1.fc44\tfedora\n";
        let packages = parse_installed_copr_packages(output);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "demo");
        assert_eq!(packages[0].version, "1.2-1.fc44");
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
}
