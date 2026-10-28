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

pub fn toggle(project: &CoprProject) -> Result<bool, String> {
    let path = favorites_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the saved-projects directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create saved-projects directory: {error}"))?;

    let mut projects = load()?;
    if let Some(index) = projects
        .iter()
        .position(|saved| saved.owner == project.owner && saved.name == project.name)
    {
        projects.remove(index);
    } else {
        projects.push(project.clone());
    }

    let saved = projects
        .iter()
        .any(|item| item.owner == project.owner && item.name == project.name);
    let contents = serde_json::to_vec_pretty(&projects)
        .map_err(|error| format!("Could not encode saved projects: {error}"))?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save project list: {error}"))?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_project_identity_uses_owner_and_name() {
        let first = CoprProject {
            name: "demo".to_string(),
            owner: "alice".to_string(),
            description: None,
            homepage: None,
        };
        let second = CoprProject {
            name: "demo".to_string(),
            owner: "bob".to_string(),
            description: None,
            homepage: None,
        };
        assert_ne!(
            (first.owner.as_str(), first.name.as_str()),
            (second.owner.as_str(), second.name.as_str())
        );
    }
}
