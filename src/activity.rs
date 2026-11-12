use crate::storage;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 100;
const MAX_HISTORY_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMPORT_BYTES: usize = 1024 * 1024;
const MAX_IMPORTED_ENTRIES: usize = 1_000;

static ACTIVITY_LOCK: Mutex<()> = Mutex::new(());

fn with_activity_lock<T>(operation: impl FnOnce() -> T) -> T {
    let _guard = ACTIVITY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}

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
    with_activity_lock(load_unlocked)
}

fn load_unlocked() -> Result<Vec<ActivityEntry>, String> {
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
            if entries
                .iter()
                .any(|entry| !activity_entry_has_valid_fields(entry))
            {
                return Err(
                    "The activity history contains an invalid or oversized entry.".to_string(),
                );
            }
            Ok(entries)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not open activity history: {error}")),
    }
}

fn matches_filter(entry: &ActivityEntry, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || entry.action.to_lowercase().contains(&query)
        || entry.target.to_lowercase().contains(&query)
        || entry.summary.to_lowercase().contains(&query)
}

/// Classify activity actions for the Activity page's category filter.
pub fn category_for_action(action: &str) -> &'static str {
    match action {
        "Project search" | "Saved projects" | "Project health check" | "Project health change" => {
            "Projects"
        }
        "Package operation" => "Packages",
        "Repository configuration" => "Repositories",
        _ => "Other",
    }
}

pub fn matches_time_range(timestamp: u64, range: &str, now: u64) -> bool {
    let seconds = match range {
        "Past 24 hours" => 24 * 60 * 60,
        "Past 7 days" => 7 * 24 * 60 * 60,
        "Past 30 days" => 30 * 24 * 60 * 60,
        _ => return true,
    };
    timestamp <= now && now.saturating_sub(timestamp) <= seconds
}

/// Export activity narrowed by the Activity page's text, category, and time filters.
pub fn export_filtered_to(
    path: &std::path::Path,
    query: &str,
    category: &str,
    range: &str,
) -> Result<usize, String> {
    with_activity_lock(|| {
        let entries = load_unlocked()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|entry| matches_filter(entry, query))
            .filter(|entry| category == "All" || category_for_action(&entry.action) == category)
            .filter(|entry| matches_time_range(entry.timestamp, range, now))
            .collect();
        let contents = serde_json::to_vec_pretty(&filtered)
            .map_err(|error| format!("Could not encode activity history: {error}"))?;
        storage::atomic_write(path, &contents)
            .map_err(|error| format!("Could not export activity history: {error}"))?;
        Ok(filtered.len())
    })
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
    if entries
        .iter()
        .any(|entry| !activity_entry_has_valid_fields(entry))
    {
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
) -> ActivityImportSummary {
    let mut incoming = 0;
    let mut duplicates = 0;
    let mut added_entries = Vec::new();
    for entry in imported {
        incoming += 1;
        if current.iter().any(|existing| same_entry(existing, &entry))
            || added_entries.iter().any(|added| same_entry(added, &entry))
        {
            duplicates += 1;
        } else {
            current.push(entry.clone());
            added_entries.push(entry);
        }
    }
    current.sort_by_key(|entry| Reverse(entry.timestamp));
    current.truncate(MAX_ENTRIES);
    let added = added_entries
        .iter()
        .filter(|added| current.iter().any(|entry| same_entry(entry, added)))
        .count();
    ActivityImportSummary {
        incoming,
        added,
        dropped: added_entries.len() - added,
        duplicates,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityImportSummary {
    /// Number of entries parsed from the backup.
    pub incoming: usize,
    /// New entries retained in the recent-history file.
    pub added: usize,
    /// New entries omitted because they fell beyond the history cap.
    pub dropped: usize,
    /// Entries already present or duplicated within the backup.
    pub duplicates: usize,
}

/// Import an activity backup, preserving current entries and avoiding duplicates.
pub fn import_from(path: &std::path::Path) -> Result<ActivityImportSummary, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect the selected activity backup: {error}"))?;
    if metadata.len() > MAX_IMPORT_BYTES as u64 {
        return Err("The activity backup is larger than the 1 MiB import limit.".to_string());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read the selected activity backup: {error}"))?;
    let imported = parse_import(&contents)?;
    with_activity_lock(|| {
        let mut entries = load_unlocked()?;
        let summary = merge_entries(&mut entries, imported);
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
        Ok(summary)
    })
}

pub fn clear() -> Result<(), String> {
    with_activity_lock(|| {
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
    })
}

pub fn record(action: &str, target: &str, summary: &str) -> Result<(), String> {
    with_activity_lock(|| {
        let entry = ActivityEntry {
            timestamp: 0,
            action: action.to_string(),
            target: target.to_string(),
            summary: summary.to_string(),
        };
        if !activity_entry_has_valid_fields(&entry) {
            return Err("Activity entry contains invalid or oversized fields.".to_string());
        }

        let path = history_path();
        let parent = path
            .parent()
            .ok_or_else(|| "Could not determine the activity history directory.".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create activity history directory: {error}"))?;

        let mut entries = load_unlocked()?;
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

        // Activity history is machine-read; compact JSON reduces repeated write volume.
        let contents = serde_json::to_vec(&entries)
            .map_err(|error| format!("Could not encode activity history: {error}"))?;
        storage::atomic_write(&path, &contents)
            .map_err(|error| format!("Could not save activity history: {error}"))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_serializes_concurrent_updates() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Barrier,
        };
        use std::time::Duration;

        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let active = active.clone();
                let maximum = maximum.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    with_activity_lock(|| {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        maximum.fetch_max(current, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(5));
                        active.fetch_sub(1, Ordering::SeqCst);
                    });
                })
            })
            .collect();

        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(maximum.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn activity_filter_matches_action_target_and_summary_case_insensitively() {
        let entry = ActivityEntry {
            timestamp: 1,
            action: "Package Install".into(),
            target: "Fedora COPR".into(),
            summary: "Installed Café Tools".into(),
        };

        assert!(matches_filter(&entry, "PACKAGE"));
        assert!(matches_filter(&entry, "copr"));
        assert!(matches_filter(&entry, "café"));
        assert!(!matches_filter(&entry, "unrelated"));
        assert!(matches_filter(&entry, "   "));
    }

    #[test]
    fn activity_actions_map_to_stable_categories() {
        assert_eq!(category_for_action("Project search"), "Projects");
        assert_eq!(category_for_action("Project health change"), "Projects");
        assert_eq!(category_for_action("Package operation"), "Packages");
        assert_eq!(
            category_for_action("Repository configuration"),
            "Repositories"
        );
        assert_eq!(category_for_action("Imported legacy action"), "Other");
    }

    #[test]
    fn activity_time_filter_handles_ranges_and_future_timestamps() {
        let now = 1_800_000_000;
        assert!(matches_time_range(now, "Past 24 hours", now));
        assert!(matches_time_range(now - 86_400, "Past 24 hours", now));
        assert!(!matches_time_range(now - 86_401, "Past 24 hours", now));
        assert!(matches_time_range(now - 30 * 86_400, "Past 30 days", now));
        assert!(!matches_time_range(now + 1, "Past 30 days", now));
        assert!(matches_time_range(0, "Any time", now));
    }

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
        let summary = merge_entries(
            &mut current,
            vec![
                entry(10, "existing"),
                entry(30, "newer"),
                entry(20, "middle"),
            ],
        );
        assert_eq!(summary.incoming, 3);
        assert_eq!(summary.added, 2);
        assert_eq!(summary.dropped, 0);
        assert_eq!(summary.duplicates, 1);
        assert_eq!(
            current
                .iter()
                .map(|item| item.timestamp)
                .collect::<Vec<_>>(),
            vec![30, 20, 10]
        );

        let many = (0..150).map(|timestamp| entry(timestamp, &format!("target-{timestamp}")));
        let mut capped = Vec::new();
        let capped_summary = merge_entries(&mut capped, many);
        assert_eq!(capped_summary.incoming, 150);
        assert_eq!(capped_summary.added, MAX_ENTRIES);
        assert_eq!(capped_summary.dropped, 50);
        assert_eq!(capped_summary.duplicates, 0);
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
