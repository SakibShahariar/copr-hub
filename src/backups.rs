use crate::{favorites, storage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const BACKUP_INTERVAL_SECS: u64 = 24 * 60 * 60;
const RETAIN_BACKUPS: usize = 7;
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupScheduleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub last_backup_unix: u64,
}

fn config_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("copr-hub")
        .join("backup-schedule.json")
}

fn backup_dir() -> PathBuf {
    gtk::glib::user_data_dir().join("copr-hub").join("backups")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load_config() -> BackupScheduleConfig {
    let path = config_path();
    let Ok(metadata) = fs::metadata(&path) else {
        return BackupScheduleConfig::default();
    };
    if metadata.len() > MAX_CONFIG_BYTES {
        return BackupScheduleConfig::default();
    }
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &BackupScheduleConfig) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(config)
        .map_err(|e| format!("Could not encode backup schedule: {e}"))?;
    let path = config_path();
    let parent = path
        .parent()
        .ok_or("Could not determine backup configuration directory.")?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create backup configuration directory: {e}"))?;
    storage::atomic_write(&path, &bytes).map_err(|e| format!("Could not save backup schedule: {e}"))
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let mut config = load_config();
    config.enabled = enabled;
    save_config(&config)
}

/// Create a private local JSON backup when the opt-in schedule is due.
/// Runs only while the app is open; it does not install a system service or timer.
pub fn run_if_due() -> Result<Option<PathBuf>, String> {
    let mut config = load_config();
    if !config.enabled {
        return Ok(None);
    }
    let now = now_unix();
    if config.last_backup_unix != 0
        && now.saturating_sub(config.last_backup_unix) < BACKUP_INTERVAL_SECS
    {
        return Ok(None);
    }
    let directory = backup_dir();
    fs::create_dir_all(&directory)
        .map_err(|e| format!("Could not create scheduled backup directory: {e}"))?;
    let timestamp = now;
    let path = directory.join(format!("saved-projects-{timestamp}.json"));
    favorites::export_to(&path)?;
    config.last_backup_unix = now;
    save_config(&config)?;
    let mut backups: Vec<_> = fs::read_dir(&directory)
        .map_err(|e| format!("Could not list scheduled backups: {e}"))?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("saved-projects-")
                && entry.path().extension().is_some_and(|ext| ext == "json")
        })
        .collect();
    backups.sort_by_key(|entry| entry.file_name());
    while backups.len() > RETAIN_BACKUPS {
        let oldest = backups.remove(0);
        fs::remove_file(oldest.path())
            .map_err(|e| format!("Could not rotate old scheduled backup: {e}"))?;
    }
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduling_is_opt_in_and_interval_is_one_day() {
        assert!(!BackupScheduleConfig::default().enabled);
        assert_eq!(BACKUP_INTERVAL_SECS, 86_400);
        assert_eq!(RETAIN_BACKUPS, 7);
    }
}
