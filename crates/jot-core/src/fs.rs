//! Filesystem utilities for atomic operations.

use std::fs;
use std::io;
use std::path::Path;

/// Atomically rename a file, with fallback for platforms where rename fails if target exists.
///
/// On some platforms (notably Windows), `fs::rename` fails if the destination already exists.
/// This function handles that case by removing the destination first and retrying.
///
/// If the rename ultimately fails, the temp file is cleaned up.
///
/// # Errors
///
/// Returns an error if the rename fails even after the fallback attempt.
pub fn rename_with_fallback(temp_path: &Path, destination: &Path) -> io::Result<()> {
    if let Err(initial_err) = fs::rename(temp_path, destination) {
        // Best-effort replace on platforms where rename fails if target exists.
        let _ = fs::remove_file(destination);
        fs::rename(temp_path, destination).map_err(|retry_err| {
            // Clean up the temp file on failure
            let _ = fs::remove_file(temp_path);
            io::Error::new(
                retry_err.kind(),
                format!(
                    "Atomic rename failed (initial: {}, retry: {})",
                    initial_err, retry_err
                ),
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_rename_new_file() {
        let dir = tempdir().unwrap();
        let temp = dir.path().join("temp.txt");
        let dest = dir.path().join("dest.txt");

        File::create(&temp).unwrap().write_all(b"test").unwrap();

        rename_with_fallback(&temp, &dest).unwrap();

        assert!(!temp.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "test");
    }

    #[test]
    fn test_rename_overwrites_existing() {
        let dir = tempdir().unwrap();
        let temp = dir.path().join("temp.txt");
        let dest = dir.path().join("dest.txt");

        File::create(&dest).unwrap().write_all(b"old").unwrap();
        File::create(&temp).unwrap().write_all(b"new").unwrap();

        rename_with_fallback(&temp, &dest).unwrap();

        assert!(!temp.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new");
    }
}
