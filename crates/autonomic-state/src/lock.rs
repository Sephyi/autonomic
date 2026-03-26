//! Exclusive file lock for single-writer enforcement.
//!
//! The Autonomic daemon is the sole writer to ~/.autonomic/.
//! This lock prevents multiple daemon instances from corrupting state.

use fs4::fs_std::FileExt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::error::StateError;

/// Exclusive file lock on ~/.autonomic/.lock.
/// Held for the lifetime of the daemon process.
/// Dropped automatically when the struct goes out of scope.
pub struct StateLock {
    _file: std::fs::File,
}

impl StateLock {
    /// Acquire an exclusive lock on the state directory.
    /// Returns `StateError::AlreadyRunning` if another instance holds the lock.
    pub fn acquire(state_dir: &Path) -> Result<Self, StateError> {
        let lock_path = state_dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;

        file.try_lock_exclusive()
            .map_err(|_| StateError::AlreadyRunning)?;

        // Write PID for diagnostics
        writeln!(&file, "{}", std::process::id())?;

        Ok(StateLock { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_lock_succeeds() {
        let dir = TempDir::new().unwrap();
        let _lock = StateLock::acquire(dir.path()).expect("should acquire lock");
    }

    // Note: Same-process flock() conflict detection varies by OS.
    // On macOS, flock() may not conflict within the same process.
    // The real use case is cross-process, which works correctly everywhere.
    #[test]
    #[cfg(target_os = "linux")]
    fn second_lock_fails() {
        let dir = TempDir::new().unwrap();
        let _lock1 = StateLock::acquire(dir.path()).expect("first lock should succeed");
        let result = StateLock::acquire(dir.path());
        assert!(
            matches!(result, Err(StateError::AlreadyRunning)),
            "second lock should fail with AlreadyRunning"
        );
    }

    #[test]
    fn lock_released_on_drop() {
        let dir = TempDir::new().unwrap();
        {
            let _lock = StateLock::acquire(dir.path()).unwrap();
        }
        // Lock should be released now
        let _lock2 = StateLock::acquire(dir.path()).expect("should acquire after drop");
    }
}
