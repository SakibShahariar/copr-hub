use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Replace a file atomically using a private temporary file.
///
/// User data files are created with owner-only permissions so that activity
/// history and saved projects are not readable by other local accounts.
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target path has no parent directory",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "target path has no file name")
    })?;

    let temporary_path;
    let mut temporary_file = loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&candidate)
        {
            Ok(file) => {
                temporary_path = candidate;
                break file;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let write_result = temporary_file
        .write_all(contents)
        .and_then(|()| temporary_file.sync_all());
    drop(temporary_file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "copr-hub-storage-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn atomically_replaces_existing_file_contents() {
        let directory = test_directory("replace");
        let path = directory.join("data.json");

        fs::write(&path, b"old").unwrap();
        atomic_write(&path, b"new contents").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new contents");
        assert_eq!(
            fs::read_dir(&directory).unwrap().count(),
            1,
            "temporary files should be removed after a successful write"
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn user_data_files_have_owner_only_permissions() {
        let directory = test_directory("permissions");
        let path = directory.join("private.json");

        atomic_write(&path, b"private data").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn replacement_restores_private_permissions() {
        let directory = test_directory("replacement-permissions");
        let path = directory.join("data.json");

        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        atomic_write(&path, b"new").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn concurrent_writes_use_independent_temporary_files() {
        let directory = test_directory("concurrent");
        let path = directory.join("data.json");

        let writers: Vec<_> = (0..8)
            .map(|index| {
                let path = path.clone();
                std::thread::spawn(move || {
                    atomic_write(&path, format!("writer-{index}").as_bytes()).unwrap();
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }

        let final_contents = fs::read_to_string(&path).unwrap();
        assert!(final_contents.starts_with("writer-"));
        assert_eq!(
            fs::read_dir(&directory).unwrap().count(),
            1,
            "concurrent writes should not leave temporary files behind"
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_paths_without_a_file_name() {
        assert_eq!(
            atomic_write(Path::new(""), b"data").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
