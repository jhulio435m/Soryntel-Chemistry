//! Adapters implementing the `soryntel-core` catalog output ports on top of
//! [`ThermoDatabase`](crate::ThermoDatabase) and the file system.
//!
//! Every method delegates to the existing `ThermoDatabase` API without changing
//! its behaviour; only `SoryntelError` is translated into `CatalogError`.

mod locator;
mod thermo_database;

pub use locator::{
    DatabaseDirLocator, MAIN_DB_FILE, OWN_DB_FILE, SORYNTEL_DB_FILE, SORYNTEL_SDB_ENV,
};
pub use thermo_database::HscFileTools;

use soryntel_core::{CatalogError, SoryntelError};

/// Translates adapter errors into the port error type.
///
/// I/O and file-format failures become [`CatalogError::Backend`].
pub(crate) fn to_catalog_error(err: SoryntelError) -> CatalogError {
    match err {
        SoryntelError::SpeciesNotFound(f) => CatalogError::NotFound(f),
        other => CatalogError::Backend(other.to_string()),
    }
}
