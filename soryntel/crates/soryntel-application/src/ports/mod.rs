//! Puertos de salida de la aplicación.
pub use soryntel_core::{SpeciesCatalogPort, SpeciesRepository};

/// Almacenamiento abstracto para workbooks; `id` es opaco y no implica rutas locales.
pub trait ProjectStoragePort {
    fn read_rea8(&self, id: &str) -> Result<Option<String>, String>;
    fn write_rea8(&self, id: &str, contents: &str) -> Result<(), String>;
    fn read_bal8(&self, id: &str) -> Result<Option<String>, String>;
    fn write_bal8(&self, id: &str, contents: &str) -> Result<(), String>;
    fn read_htr8(&self, id: &str) -> Result<Option<String>, String>;
    fn write_htr8(&self, id: &str, contents: &str) -> Result<(), String>;
}
