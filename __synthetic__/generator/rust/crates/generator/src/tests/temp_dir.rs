//! A folder for one test, removed when the test ends.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// A fresh folder under the system's temp folder, named after the test that
/// made it, deleted (with everything in it) when dropped.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// `<temp>/generator-tests-<name>`, emptied first in case an earlier run
    /// left it behind.
    pub(crate) fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("generator-tests-{name}"));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A folder that cannot be removed only leaves clutter in temp; a
        // panic here would hide the test's own result.
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("removing {}: {error}", self.path.display());
        }
    }
}
