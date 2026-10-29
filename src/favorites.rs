use crate::copr::CoprProject;
use crate::storage;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const MAX_BACKUP_BYTES: usize = 5 * 1024 * 1024;
const MAX_IMPORTED_PROJECTS: usize = 5_000;
const MAX_SAVED_PROJECTS: usize = 10_000;
const MAX_SAVED_FILE_BYTES: u64 = 10 * 1024 * 1024;

fn project_has_valid_fields(project: &CoprProject) -> bool {
    !project.name.trim().is_empty()
        && !project.owner.trim().is_empty()
        && project.name.len() <= 512
        && project.owner.len() <= 512
        && project
            .description
            .as_ref()
            .is_none_or(|value| value.len() <= 16_384)
        && project
            .homepage
            .as_ref()
            .is_none_or(|value| value.len() <= 4096)
}

fn validate_project_entries(projects: &[CoprProject]) -> Result<(), String> {
    if projects
        .iter()
        .any(|project| !project_has_valid_fields(project))
    {
        return Err(
            "The saved-project list contains an invalid or oversized project entry.".to_string(),
        );
    }
    Ok(())
}

fn ensure_saved_project_limit(count: usize) -> Result<(), String> {
    if count > MAX_SAVED_PROJECTS {
        return Err(format!(
            "The saved-project list exceeds the {MAX_SAVED_PROJECTS}-project limit."
        ));
    }
    Ok(())
}

fn ensure_saved_file_size(size: usize) -> Result<(), String> {
    if size > MAX_SAVED_FILE_BYTES as usize {
        return Err("The saved-project list would exceed the 10 MiB safety limit.".to_string());
    }
    Ok(())
}

fn favorites_path() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("copr-hub")
        .join("favorites.json")
}

pub fn load() -> Result<Vec<CoprProject>, String> {
    let path = favorites_path();
    match fs::metadata(&path) {
        Ok(metadata) if metadata.len() > MAX_SAVED_FILE_BYTES => {
            return Err("The saved-project list exceeds the 10 MiB safety limit.".to_string());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not inspect saved projects: {error}")),
    }
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let projects: Vec<CoprProject> = serde_json::from_str(&contents)
                .map_err(|error| format!("Could not read saved projects: {error}"))?;
            ensure_saved_project_limit(projects.len())?;
            validate_project_entries(&projects)?;
            Ok(projects)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not open saved projects: {error}")),
    }
}

pub fn is_saved(project: &CoprProject) -> Result<bool, String> {
    Ok(load()?
        .iter()
        .any(|saved| saved.owner == project.owner && saved.name == project.name))
}

fn parse_backup(contents: &str) -> Result<Vec<CoprProject>, String> {
    if contents.len() > MAX_BACKUP_BYTES {
        return Err("The saved-project backup is larger than the 5 MiB import limit.".to_string());
    }
    let imported: Vec<CoprProject> = serde_json::from_str(contents)
        .map_err(|error| format!("This is not a valid Copr Hub saved-project backup: {error}"))?;
    if imported.len() > MAX_IMPORTED_PROJECTS {
        return Err(format!(
            "The backup contains more than {MAX_IMPORTED_PROJECTS} saved projects."
        ));
    }
    if imported
        .iter()
        .any(|project| !project_has_valid_fields(project))
    {
        return Err("The backup contains an invalid or oversized project entry.".to_string());
    }
    Ok(imported)
}

fn merge_projects(
    projects: &mut Vec<CoprProject>,
    imported: impl IntoIterator<Item = CoprProject>,
) -> usize {
    let mut identities: HashSet<(String, String)> = projects
        .iter()
        .map(|project| (project.owner.clone(), project.name.clone()))
        .collect();
    let mut added = 0;
    for project in imported {
        let identity = (project.owner.clone(), project.name.clone());
        if identities.insert(identity) {
            projects.push(project);
            added += 1;
        }
    }
    added
}

fn toggle_in_list(projects: &mut Vec<CoprProject>, project: &CoprProject) -> bool {
    let already_saved = projects
        .iter()
        .any(|saved| saved.owner == project.owner && saved.name == project.name);

    if already_saved {
        projects.retain(|saved| saved.owner != project.owner || saved.name != project.name);
        false
    } else {
        projects.push(project.clone());
        true
    }
}

pub fn toggle(project: &CoprProject) -> Result<bool, String> {
    let path = favorites_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the saved-projects directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create saved-projects directory: {error}"))?;

    let mut projects = load()?;
    let already_saved = projects
        .iter()
        .any(|saved| saved.owner == project.owner && saved.name == project.name);
    if !already_saved && !project_has_valid_fields(project) {
        return Err(
            "The project contains invalid or oversized metadata and cannot be saved.".to_string(),
        );
    }
    let saved = toggle_in_list(&mut projects, project);
    ensure_saved_project_limit(projects.len())?;
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode saved projects: {error}"))?;
    ensure_saved_file_size(contents.len())?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save project list: {error}"))?;
    Ok(saved)
}

/// Export the locally saved projects as a portable JSON backup.
pub fn export_to(path: &std::path::Path) -> Result<usize, String> {
    let projects = load()?;
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode saved projects: {error}"))?;
    storage::atomic_write(path, &contents)
        .map_err(|error| format!("Could not export saved projects: {error}"))?;
    Ok(projects.len())
}

/// A non-mutating preview of a saved-project backup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreview {
    pub incoming: usize,
    pub already_saved: usize,
    pub new_projects: usize,
}

/// Validate an import file and report the merge impact without changing saved data.
pub fn preview_import(path: &std::path::Path) -> Result<ImportPreview, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect the selected backup: {error}"))?;
    if metadata.len() > MAX_BACKUP_BYTES as u64 {
        return Err("The saved-project backup is larger than the 5 MiB import limit.".to_string());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read the selected backup: {error}"))?;
    let imported = parse_backup(&contents)?;
    let existing = load()?;
    let existing_keys: HashSet<(String, String)> = existing
        .iter()
        .map(|project| (project.owner.clone(), project.name.clone()))
        .collect();
    let mut seen = HashSet::new();
    let mut already_saved = 0;
    let mut new_projects = 0;
    for project in &imported {
        let key = (project.owner.clone(), project.name.clone());
        if existing_keys.contains(&key) || !seen.insert(key) {
            already_saved += 1;
        } else {
            new_projects += 1;
        }
    }
    Ok(ImportPreview {
        incoming: imported.len(),
        already_saved,
        new_projects,
    })
}

/// Import a JSON backup, merging by owner/project identity without duplicating entries.
pub fn import_from(path: &std::path::Path) -> Result<usize, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect the selected backup: {error}"))?;
    if metadata.len() > MAX_BACKUP_BYTES as u64 {
        return Err("The saved-project backup is larger than the 5 MiB import limit.".to_string());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read the selected backup: {error}"))?;
    let imported = parse_backup(&contents)?;

    let path = favorites_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the saved-projects directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create saved-projects directory: {error}"))?;

    let mut projects = load()?;
    let added = merge_projects(&mut projects, imported);
    ensure_saved_project_limit(projects.len()).map_err(|_| {
        format!(
            "Import would exceed the {MAX_SAVED_PROJECTS}-project saved-list limit; no changes were saved."
        )
    })?;
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode merged saved projects: {error}"))?;
    ensure_saved_file_size(contents.len()).map_err(|_| {
        "Import would exceed the 10 MiB saved-project limit; no changes were saved.".to_string()
    })?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save imported projects: {error}"))?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(owner: &str, name: &str) -> CoprProject {
        CoprProject {
            name: name.to_string(),
            owner: owner.to_string(),
            description: None,
            homepage: None,
        }
    }

    #[test]
    fn importing_merges_new_projects_without_duplicates() {
        let existing = project("alice", "demo");
        let mut projects = vec![existing.clone()];
        let added = merge_projects(
            &mut projects,
            vec![
                existing,
                project("bob", "demo"),
                project("bob", "demo"),
                project("carol", "tool"),
            ],
        );

        assert_eq!(added, 2);
        assert_eq!(projects.len(), 3);
        assert!(projects
            .iter()
            .any(|item| item.owner == "bob" && item.name == "demo"));
        assert!(projects
            .iter()
            .any(|item| item.owner == "carol" && item.name == "tool"));
    }

    #[test]
    fn backup_parser_rejects_invalid_json() {
        let error = parse_backup("{not json").unwrap_err();
        assert!(error.contains("valid Copr Hub saved-project backup"));
    }

    #[test]
    fn backup_parser_rejects_projects_without_identity() {
        let contents = r#"[{"owner":"alice","name":"  "},{"owner":"bob","name":"tool"}]"#;
        let error = parse_backup(contents).unwrap_err();
        assert!(error.contains("invalid or oversized project entry"));
    }

    #[test]
    fn backup_parser_rejects_oversized_files_and_project_lists() {
        let too_large = " ".repeat(MAX_BACKUP_BYTES + 1);
        assert!(parse_backup(&too_large)
            .unwrap_err()
            .contains("5 MiB import limit"));

        let projects = vec![project("owner", "project"); MAX_IMPORTED_PROJECTS + 1];
        let contents = serde_json::to_string(&projects).unwrap();
        assert!(parse_backup(&contents)
            .unwrap_err()
            .contains("more than 5000 saved projects"));
    }

    #[test]
    fn backup_parser_rejects_oversized_project_fields() {
        let mut oversized = project("owner", "project");
        oversized.homepage = Some("x".repeat(4097));
        let contents = serde_json::to_string(&vec![oversized]).unwrap();
        assert!(parse_backup(&contents)
            .unwrap_err()
            .contains("invalid or oversized project entry"));
    }

    #[test]
    fn saved_project_field_validation_rejects_blank_and_oversized_values() {
        assert!(!project_has_valid_fields(&project(" ", "demo")));
        assert!(!project_has_valid_fields(&project("alice", "")));
        assert!(!project_has_valid_fields(&project(
            "alice",
            &"x".repeat(513)
        )));

        let mut oversized_description = project("alice", "demo");
        oversized_description.description = Some("x".repeat(16_385));
        assert!(!project_has_valid_fields(&oversized_description));

        let mut oversized_homepage = project("alice", "demo");
        oversized_homepage.homepage = Some("x".repeat(4097));
        assert!(!project_has_valid_fields(&oversized_homepage));
    }

    #[test]
    fn saved_file_size_limit_accepts_boundary_and_rejects_overflow() {
        assert!(ensure_saved_file_size(MAX_SAVED_FILE_BYTES as usize).is_ok());
        assert!(ensure_saved_file_size(MAX_SAVED_FILE_BYTES as usize + 1)
            .unwrap_err()
            .contains("10 MiB safety limit"));
    }

    #[test]
    fn saved_project_limits_allow_regular_backups() {
        assert!(ensure_saved_project_limit(MAX_SAVED_PROJECTS).is_ok());
        assert!(ensure_saved_project_limit(MAX_SAVED_PROJECTS + 1).is_err());
    }

    #[test]
    fn backup_parser_accepts_an_empty_backup() {
        assert!(parse_backup("[]").unwrap().is_empty());
    }

    #[test]
    fn saved_project_identity_uses_owner_and_name() {
        let first = project("alice", "demo");
        let second = project("bob", "demo");
        assert_ne!(
            (first.owner.as_str(), first.name.as_str()),
            (second.owner.as_str(), second.name.as_str())
        );
    }

    #[test]
    fn toggling_saved_project_removes_all_duplicate_entries() {
        let target = project("alice", "demo");
        let other = project("bob", "demo");
        let mut projects = vec![target.clone(), other.clone(), target.clone()];

        assert!(!toggle_in_list(&mut projects, &target));
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].owner, "bob");
    }

    #[test]
    fn toggling_unsaved_project_adds_it_once() {
        let target = project("alice", "demo");
        let mut projects = Vec::new();

        assert!(toggle_in_list(&mut projects, &target));
        assert_eq!(projects.len(), 1);
        assert!(toggle_in_list(&mut projects, &project("bob", "demo")));
        assert_eq!(projects.len(), 2);
    }
}
