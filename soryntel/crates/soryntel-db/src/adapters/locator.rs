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

    /// Directorios por defecto de ambos clientes (ADR 0003), en orden:
    /// `<exe>/Databases` y la carpeta de datos del usuario ([`Self::user_data_dir`]).
    pub fn standard() -> Self {
        Self::new(
            Self::beside_executable()
                .dirs
                .into_iter()
                .chain(Self::user_data_dir()),
        )
    }

    /// Carpeta de datos del usuario: `<Documentos>/Soryntel/Databases`, donde `<Documentos>`
    /// es `~/Documents` o `~/Documentos` si existen y, si no, la carpeta personal (`HOME` o
    /// `USERPROFILE`). `None` si no se puede determinar una ruta absoluta.
    pub fn user_data_dir() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())?;
        let documents = ["Documents", "Documentos"]
            .iter()
            .map(|name| home.join(name))
            .find(|dir| dir.is_dir())
            .unwrap_or(home);
        Some(documents.join("Soryntel").join("Databases"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_dirs_are_absolute_and_start_beside_the_executable() {
        let locator = DatabaseDirLocator::standard();
        let dirs = locator.dirs();
        assert!(!dirs.is_empty());
        assert!(dirs.iter().all(|d| d.is_absolute()), "{dirs:?}");
        assert!(dirs[0].ends_with("Databases"));
        if let Some(user) = DatabaseDirLocator::user_data_dir() {
            assert!(user.ends_with("Soryntel/Databases"));
            assert_eq!(dirs.last(), Some(&user));
        }
    }

    #[test]
    fn finds_first_existing_file_in_order() {
        let base = std::env::temp_dir().join(format!("soryntel-locator-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join(MAIN_DB_FILE), b"x").unwrap();
        std::fs::write(a.join(OWN_DB_FILE), b"x").unwrap();
        let locator = DatabaseDirLocator::new([a.clone(), b.clone()]);
        assert_eq!(locator.default_main_db(), Some(b.join(MAIN_DB_FILE)));
        assert_eq!(locator.default_own_db(), Some(a.join(OWN_DB_FILE)));
        let _ = std::fs::remove_dir_all(&base);
    }
}
