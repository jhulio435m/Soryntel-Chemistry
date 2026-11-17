//! Ports (hexagonal architecture) for accessing species catalogs.
//!
//! Adapters such as the HSC database implement these traits; the domain and
//! application layers depend only on them.

use thiserror::Error;

use crate::species::{ChemicalSpecies, SpeciesMetadata};

/// Read-only lookup of chemical species by formula.
pub trait SpeciesRepository {
    /// Returns the species whose formula matches `formula`, if present.
    fn get_species(&self, formula: &str) -> Option<ChemicalSpecies>;

    /// Returns the reference-state species for the element `element_symbol`
    /// (e.g. `O2(g)` for `"O"`), if present.
    fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies>;
}

/// Full species catalog: lookup, search, enumeration and mutation.
///
/// # Future fallible operations
///
/// All current signatures are infallible and are kept as-is for compatibility.
/// The following methods are candidates to migrate to `Result<_, CatalogError>`
/// (likely via new `try_*` methods first) once adapters need to report failures:
///
/// - [`insert_or_update_species`](Self::insert_or_update_species): storage
///   write errors, read-only catalogs, invalid species data.
/// - [`search_species`](Self::search_species) and
///   [`search_species_by_elements`](Self::search_species_by_elements): backend
///   I/O or query errors, which are currently indistinguishable from "no match".
/// - [`get_species_metadata`](Self::get_species_metadata) and
///   [`get_species_at`](Self::get_species_at): to distinguish "unsupported by
///   this catalog" and "index out of range" from "not found".
/// - [`SpeciesRepository::get_species`]: backend I/O errors versus a missing
///   species.
pub trait SpeciesCatalogPort: SpeciesRepository {
    /// Free-text search over formulas/names, returning at most `limit` results.
    fn search_species(&self, query: &str, limit: usize) -> Vec<ChemicalSpecies>;

    /// Returns species composed only of `elements`, filtered by the enabled
    /// phase categories, returning at most `limit` results.
    #[allow(clippy::too_many_arguments)]
    fn search_species_by_elements(
        &self,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies>;

    /// Inserts `sp`, replacing any existing species with the same formula.
    fn insert_or_update_species(&mut self, sp: ChemicalSpecies);

    /// Number of species in the catalog.
    fn species_count(&self) -> usize;

    /// Descriptive metadata for `formula`. Defaults to `None` for catalogs
    /// without metadata.
    fn get_species_metadata(&self, _formula: &str) -> Option<SpeciesMetadata> {
        None
    }

    /// Species at position `index` in catalog order. Defaults to `None` for
    /// catalogs that do not support indexed access.
    fn get_species_at(&self, _index: usize) -> Option<ChemicalSpecies> {
        None
    }
}

/// Errors for future fallible catalog operations.
///
/// Not yet returned by any method of [`SpeciesRepository`] or
/// [`SpeciesCatalogPort`]; see the latter's documentation for the planned
/// migration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogError {
    /// No species with the given formula exists in the catalog.
    #[error("species not found in catalog: {0}")]
    NotFound(String),

    /// Indexed access beyond the catalog size.
    #[error("catalog index {index} out of range (len {len})")]
    IndexOutOfRange {
        /// Requested index.
        index: usize,
        /// Current catalog length.
        len: usize,
    },

    /// The species data is invalid and was rejected.
    #[error("invalid species data: {0}")]
    InvalidSpecies(String),

    /// The catalog does not support the requested operation (e.g. writes on a
    /// read-only source).
    #[error("operation not supported by this catalog: {0}")]
    Unsupported(String),

    /// The underlying storage backend failed.
    #[error("catalog backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_error_messages() {
        assert_eq!(
            CatalogError::NotFound("H2O".into()).to_string(),
            "species not found in catalog: H2O"
        );
        assert_eq!(
            CatalogError::IndexOutOfRange { index: 5, len: 3 }.to_string(),
            "catalog index 5 out of range (len 3)"
        );
    }
}
