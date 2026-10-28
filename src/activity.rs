use crate::storage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub timestamp: u64,
    pub action: String,
    pub target: String,
    pub summary: String,
}

fn history_path() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("copr-hub")
        .join("activity.json")
}

pub fn load() -> Result<Vec<ActivityEntry>, String> {
    let path = history_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("Could not read activity history: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not open activity history: {error}")),
    }
}

pub fn clear() -> Result<(), String> {
    let path = history_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the activity history directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create activity history directory: {error}"))?;

    let contents = serde_json::to_vec_pretty(&Vec::<ActivityEntry>::new())
        .map_err(|error| format!("Could not encode empty activity history: {error}"))?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not clear activity history: {error}"))?;
    Ok(())
}

pub fn record(action: &str, target: &str, summary: &str) -> Result<(), String> {
    let path = history_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the activity history directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create activity history directory: {error}"))?;

    let mut entries = load()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    entries.insert(
        0,
        ActivityEntry {
            timestamp,
            action: action.to_string(),
            target: target.to_string(),
            summary: summary.to_string(),
        },
    );
    entries.truncate(MAX_ENTRIES);

    let contents = serde_json::to_vec_pretty(&entries)
        .map_err(|error| format!("Could not encode activity history: {error}"))?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save activity history: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_entry_round_trips_through_json() {
        let entry = ActivityEntry {
            timestamp: 1_800_000_000,
            action: "Project search".to_string(),
            target: "demo".to_string(),
            summary: "3 results".to_string(),
        };
        let encoded = serde_json::to_string(&entry).unwrap();
        let decoded: ActivityEntry = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.action, "Project search");
        assert_eq!(decoded.target, "demo");
        assert_eq!(decoded.summary, "3 results");
    }
}
