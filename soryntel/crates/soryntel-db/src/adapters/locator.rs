//! File-system [`DatabaseLocatorPort`] adapter.

use soryntel_core::DatabaseLocatorPort;
use std::path::PathBuf;

/// Default `MainDB` file name (HSC 6).
pub const MAIN_DB_FILE: &str = "Maindb6.HSC";
/// Default `OwnDB` file name (HSC 6).
pub const OWN_DB_FILE: &str = "OWNDB6.HSC";

/// Looks for `Maindb6.HSC` / `OWNDB6.HSC` in an ordered list of directories and
/// returns the first existing file.
#[derive(Debug, Clone, Default)]
pub struct DatabaseDirLocator {
    dirs: Vec<PathBuf>,
}

impl DatabaseDirLocator {
    /// Searches `dirs` in order.
    pub fn new(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            dirs: dirs.into_iter().collect(),
        }
    }

    /// Searches `<directory of the executable>/Databases`.
    pub fn beside_executable() -> Self {
        let dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("Databases")));
        Self::new(dir)
    }

    /// Directories searched, in order.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    fn find(&self, file: &str) -> Option<PathBuf> {
        self.dirs.iter().map(|d| d.join(file)).find(|p| p.is_file())
    }
}

impl DatabaseLocatorPort for DatabaseDirLocator {
    fn default_main_db(&self) -> Option<PathBuf> {
        self.find(MAIN_DB_FILE)
    }

    fn default_own_db(&self) -> Option<PathBuf> {
        self.find(OWN_DB_FILE)
    }
}
