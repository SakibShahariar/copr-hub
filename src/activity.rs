use crate::storage;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 100;
const MAX_HISTORY_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMPORT_BYTES: usize = 1024 * 1024;
const MAX_IMPORTED_ENTRIES: usize = 1_000;

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

fn activity_entry_has_valid_fields(entry: &ActivityEntry) -> bool {
    !entry.action.trim().is_empty()
        && !entry.target.trim().is_empty()
        && entry.action.len() <= 512
        && entry.target.len() <= 4096
        && entry.summary.len() <= 8192
}

pub fn load() -> Result<Vec<ActivityEntry>, String> {
    let path = history_path();
    match fs::metadata(&path) {
        Ok(metadata) if metadata.len() > MAX_HISTORY_FILE_BYTES => {
            return Err("The activity history exceeds the 2 MiB safety limit.".to_string());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not inspect activity history: {error}")),
    }

    match fs::read_to_string(&path) {
        Ok(contents) => {
            let entries: Vec<ActivityEntry> = serde_json::from_str(&contents)
                .map_err(|error| format!("Could not read activity history: {error}"))?;
            if entries.len() > MAX_ENTRIES {
                return Err(format!(
                    "The activity history contains more than {MAX_ENTRIES} entries."
                ));
            }
            if entries.iter().any(|entry| !activity_entry_has_valid_fields(entry)) {
                return Err("The activity history contains an invalid or oversized entry.".to_string());
            }
            Ok(entries)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not open activity history: {error}")),
    }
}

/// Export the local activity history to a portable JSON file.
pub fn export_to(path: &std::path::Path) -> Result<usize, String> {
    let entries = load()?;
    let contents = serde_json::to_vec_pretty(&entries)
        .map_err(|error| format!("Could not encode activity history: {error}"))?;
    storage::atomic_write(path, &contents)
        .map_err(|error| format!("Could not export activity history: {error}"))?;
    Ok(entries.len())
}

fn parse_import(contents: &str) -> Result<Vec<ActivityEntry>, String> {
    if contents.len() > MAX_IMPORT_BYTES {
        return Err("The activity backup is larger than the 1 MiB import limit.".to_string());
    }
    let entries: Vec<ActivityEntry> = serde_json::from_str(contents)
        .map_err(|error| format!("This is not a valid Copr Hub activity backup: {error}"))?;
    if entries.len() > MAX_IMPORTED_ENTRIES {
        return Err(format!(
            "The activity backup contains more than {MAX_IMPORTED_ENTRIES} entries."
        ));
    }
    if entries.iter().any(|entry| !activity_entry_has_valid_fields(entry)) {
        return Err("The backup contains an invalid or oversized activity entry.".to_string());
    }
    Ok(entries)
}

fn same_entry(left: &ActivityEntry, right: &ActivityEntry) -> bool {
    left.timestamp == right.timestamp
        && left.action == right.action
        && left.target == right.target
        && left.summary == right.summary
}

fn merge_entries(
    current: &mut Vec<ActivityEntry>,
    imported: impl IntoIterator<Item = ActivityEntry>,
) -> usize {
    let mut added_entries = Vec::new();
    for entry in imported {
        if !current.iter().any(|existing| same_entry(existing, &entry)) {
            current.push(entry.clone());
            added_entries.push(entry);
        }
    }
    current.sort_by_key(|entry| Reverse(entry.timestamp));
    current.truncate(MAX_ENTRIES);
    added_entries
        .iter()
        .filter(|added| current.iter().any(|entry| same_entry(entry, added)))
        .count()
}

/// Import an activity backup, preserving current entries and avoiding duplicates.
pub fn import_from(path: &std::path::Path) -> Result<usize, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect the selected activity backup: {error}"))?;
    if metadata.len() > MAX_IMPORT_BYTES as u64 {
        return Err("The activity backup is larger than the 1 MiB import limit.".to_string());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read the selected activity backup: {error}"))?;
    let imported = parse_import(&contents)?;
    let mut entries = load()?;
    let added = merge_entries(&mut entries, imported);
    let path = history_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Could not determine the activity history directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create activity history directory: {error}"))?;
    let contents = serde_json::to_vec_pretty(&entries)
        .map_err(|error| format!("Could not encode merged activity history: {error}"))?;
    storage::atomic_write(&path, &contents)
        .map_err(|error| format!("Could not save imported activity history: {error}"))?;
    Ok(added)
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
    fn import_parser_rejects_invalid_and_oversized_entries() {
        assert!(parse_import("{bad")
            .unwrap_err()
            .contains("valid Copr Hub activity backup"));
        let invalid = r#"[{"timestamp":1,"action":" ","target":"demo","summary":"ok"}]"#;
        assert!(parse_import(invalid)
            .unwrap_err()
            .contains("invalid or oversized"));
        let oversized = format!(
            r#"[{{"timestamp":1,"action":"search","target":"demo","summary":"{}"}}]"#,
            "x".repeat(8193)
        );
        assert!(parse_import(&oversized).is_err());

        let too_large = " ".repeat(MAX_IMPORT_BYTES + 1);
        assert!(parse_import(&too_large)
            .unwrap_err()
            .contains("1 MiB import limit"));

        let entry = ActivityEntry {
            timestamp: 1,
            action: "search".into(),
            target: "demo".into(),
            summary: String::new(),
        };
        let too_many = serde_json::to_string(&vec![entry; MAX_IMPORTED_ENTRIES + 1]).unwrap();
        assert!(parse_import(&too_many)
            .unwrap_err()
            .contains("more than 1000 entries"));
    }

    #[test]
    fn importing_merges_newest_entries_without_duplicates_and_caps_history() {
        let entry = |timestamp, target: &str| ActivityEntry {
            timestamp,
            action: "Project search".into(),
            target: target.into(),
            summary: "1 result".into(),
        };
        let mut current = vec![entry(10, "existing")];
        let added = merge_entries(
            &mut current,
            vec![
                entry(10, "existing"),
                entry(30, "newer"),
                entry(20, "middle"),
            ],
        );
        assert_eq!(added, 2);
        assert_eq!(
            current
                .iter()
                .map(|item| item.timestamp)
                .collect::<Vec<_>>(),
            vec![30, 20, 10]
        );

        let many = (0..150).map(|timestamp| entry(timestamp, &format!("target-{timestamp}")));
        let mut capped = Vec::new();
        merge_entries(&mut capped, many);
        assert_eq!(capped.len(), MAX_ENTRIES);
        assert_eq!(capped[0].timestamp, 149);
        assert_eq!(capped[MAX_ENTRIES - 1].timestamp, 50);
    }

    #[test]
    fn activity_entry_validation_rejects_blank_and_oversized_fields() {
        let mut entry = ActivityEntry {
            timestamp: 1,
            action: "search".to_string(),
            target: "demo".to_string(),
            summary: "ok".to_string(),
        };
        assert!(activity_entry_has_valid_fields(&entry));

        entry.action = "  ".to_string();
        assert!(!activity_entry_has_valid_fields(&entry));
        entry.action = "search".to_string();
        entry.target = "x".repeat(4097);
        assert!(!activity_entry_has_valid_fields(&entry));
        entry.target = "demo".to_string();
        entry.summary = "x".repeat(8193);
        assert!(!activity_entry_has_valid_fields(&entry));
    }

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
