use crate::copr::CoprProject;
use crate::storage;
use std::fs;
use std::path::PathBuf;

fn favorites_path() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("copr-hub")
        .join("favorites.json")
}

pub fn load() -> Result<Vec<CoprProject>, String> {
    let path = favorites_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("Could not read saved projects: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not open saved projects: {error}")),
    }
}

pub fn is_saved(project: &CoprProject) -> Result<bool, String> {
    Ok(load()?
        .iter()
        .any(|saved| saved.owner == project.owner && saved.name == project.name))
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
    let saved = toggle_in_list(&mut projects, project);
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode saved projects: {error}"))?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save project list: {error}"))?;
    Ok(saved)
}

/// Export the locally saved projects as a portable JSON backup.
pub fn export_to(path: &std::path::Path) -> Result<usize, String> {
    let projects = load()?;
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode saved projects: {error}"))?;
    fs::write(path, contents).map_err(|error| format!("Could not export saved projects: {error}"))?;
    Ok(projects.len())
}

/// Import a JSON backup, merging by owner/project identity without duplicating entries.
pub fn import_from(path: &std::path::Path) -> Result<usize, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read the selected backup: {error}"))?;
    let imported: Vec<CoprProject> = serde_json::from_str(&contents)
        .map_err(|error| format!("This is not a valid Copr Hub saved-project backup: {error}"))?;
    if imported.iter().any(|project| project.name.trim().is_empty() || project.owner.trim().is_empty()) {
        return Err("The backup contains a project without a name or owner.".to_string());
    }

    let path = favorites_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the saved-projects directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create saved-projects directory: {error}"))?;

    let mut projects = load()?;
    let mut added = 0;
    for project in imported {
        if !projects.iter().any(|saved| saved.owner == project.owner && saved.name == project.name) {
            projects.push(project);
            added += 1;
        }
    }
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode merged saved projects: {error}"))?;
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
