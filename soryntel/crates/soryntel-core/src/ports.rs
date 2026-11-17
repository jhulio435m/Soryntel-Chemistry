//! Ports (hexagonal architecture) for accessing species catalogs.
//!
//! Adapters such as the HSC database implement these traits; the domain and
//! application layers depend only on them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::catalog::{
    DatabaseStatusSummary, DbMergeConflict, DbSource, ElementSearchMode, KeywordSearchField,
    MainDbPrecedence, MergeConflictDecision, PhaseFilterFlags, ReferenceEntry,
    SpeciesCatalogEntry, SpeciesDataset,
};
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

// ---------------------------------------------------------------------------
// Output ports for database administration, browsing and editing.
//
// Conventions:
// - Mutating methods take `&mut self`; sharing across threads is the caller's
//   concern (e.g. `RwLock<Box<dyn Port + Send + Sync>>`), never the port's.
// - Operations with I/O return `Result<_, CatalogError>`; I/O and file-format
//   failures are reported as `CatalogError::Backend`.
// ---------------------------------------------------------------------------

/// Management of the `MainDB` / `OwnDB` layers and their `.HSC` / `.CAT` files.
pub trait SpeciesDatabaseAdminPort {
    /// Loads `MainDB` from `path` with the given precedence over built-in species.
    /// Returns `(unique species, binary records)`.
    fn load_main_db(
        &mut self,
        path: &Path,
        precedence: MainDbPrecedence,
    ) -> Result<(usize, usize), CatalogError>;

    /// Loads `OwnDB` from `path`, replacing in-memory `OwnDB` edits.
    /// Returns `(unique species, binary records)`.
    fn load_own_db(&mut self, path: &Path) -> Result<(usize, usize), CatalogError>;

    /// Writes `OwnDB` to `path` (plus `.CAT`) and makes it the active `OwnDB` file.
    /// Returns the number of datasets written.
    fn save_own_db(&mut self, path: &Path) -> Result<usize, CatalogError>;

    /// Creates an empty `OwnDB` file at `path` and clears the in-memory `OwnDB`.
    fn create_empty_own_db(&mut self, path: &Path) -> Result<(), CatalogError>;

    /// Writes a copy of `MainDB` to `path`. Returns the number of datasets written.
    fn save_main_db_copy(&self, path: &Path) -> Result<usize, CatalogError>;

    /// Rewrites the `.CAT` index next to the active file of `source`.
    fn refresh_catalogue(&self, source: DbSource) -> Result<PathBuf, CatalogError>;

    /// Sorts the entries of `source` in HSC order.
    fn sort_layer(&mut self, source: DbSource);

    /// Summary statistics and active file paths.
    fn status(&self) -> DatabaseStatusSummary;
}

/// Species browser queries (per layer, with filters and alternative datasets).
pub trait SpeciesBrowserPort {
    /// Keyword search on `field`, restricted to `source` (`None` = resolved view).
    fn search_keyword(
        &self,
        query: &str,
        field: KeywordSearchField,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry>;

    /// Element search (`Some` = subset of `elements`, `All` = contains all).
    fn search_elements(
        &self,
        elements: &[String],
        mode: ElementSearchMode,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry>;

    /// Stoichiometric search; fails if `pattern` is not a valid formula.
    fn search_stoichiometry(
        &self,
        pattern: &str,
        fix_ratio: bool,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Result<Vec<SpeciesCatalogEntry>, CatalogError>;

    /// Number of entries listed for `source` (`None` = resolved view).
    fn entry_count(&self, source: Option<DbSource>) -> usize;

    /// Enumerates every entry of `source` in catalog order, without filters or limits.
    fn iter_species(
        &self,
        source: Option<DbSource>,
    ) -> Box<dyn Iterator<Item = SpeciesCatalogEntry> + '_>;

    /// Page of at most `limit` entries of `source`, starting at `offset`.
    fn species_page(
        &self,
        source: Option<DbSource>,
        offset: usize,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        self.iter_species(source).skip(offset).take(limit).collect()
    }

    /// All datasets stored for `formula` in `source`.
    fn datasets_for(&self, source: DbSource, formula: &str) -> Vec<SpeciesDataset>;

    /// Selects dataset `index` of `formula` in `source` as active. Returns `false`
    /// if the index is invalid.
    fn select_dataset(&mut self, source: DbSource, formula: &str, index: usize) -> bool;

    /// Layer currently resolved for `formula`.
    fn resolved_source_of(&self, formula: &str) -> Option<DbSource>;
}

/// Editing of the `OwnDB` layer and CSV/JSON exchange.
pub trait OwnDbEditorPort {
    /// Inserts or replaces the primary `OwnDB` dataset for `species.formula`.
    fn upsert_own(&mut self, species: ChemicalSpecies, metadata: SpeciesMetadata, raw_phase_code: String);

    /// Removes `formula` from `OwnDB`. Returns whether it existed.
    fn delete_own(&mut self, formula: &str) -> bool;

    /// Imports CSV rows into `OwnDB`. Returns the number of datasets imported.
    fn import_csv_into_own(&mut self, csv: &str) -> Result<usize, CatalogError>;

    /// Exports `source` (`None` = resolved view) as CSV.
    fn export_csv(&self, source: Option<DbSource>) -> String;

    /// Exports `source` (`None` = resolved view) as pretty-printed JSON.
    fn export_json(&self, source: Option<DbSource>) -> Result<String, CatalogError>;
}

/// Bibliographic dictionary (`MainDB6.REF`).
pub trait ReferenceLibraryPort {
    /// Loads the dictionary from `path`, replacing the current one. Returns the entry count.
    fn load_references(&mut self, path: &Path) -> Result<usize, CatalogError>;

    /// Citations referenced by a comma/semicolon-separated `Reference` field.
    fn citations_for(&self, ref_field: &str) -> Vec<ReferenceEntry>;

    /// Searches keys and citation text, returning at most `limit` entries.
    fn search_references(&self, query: &str, limit: usize) -> Vec<ReferenceEntry>;
}

/// Stateless `.HSC` file tools.
pub trait HscFileToolsPort {
    /// Merges `src` into `dst`. Duplicates are resolved by `decisions` (keyed by
    /// formula); with `inspect_only` nothing is written. Returns
    /// `(merged datasets, conflicts)`.
    fn merge_hsc_files(
        &self,
        src: &Path,
        dst: &Path,
        decisions: &BTreeMap<String, MergeConflictDecision>,
        inspect_only: bool,
    ) -> Result<(usize, Vec<DbMergeConflict>), CatalogError>;

    /// Converts a legacy HSC 2–5 `.HSC` or CSV file to HSC 6 at `dst`.
    /// Returns the number of datasets written.
    fn convert_legacy_to_hsc6(&self, src: &Path, dst: &Path) -> Result<usize, CatalogError>;
}

/// Full species workspace: everything the desktop catalog windows need.
///
/// Aggregates the catalog, administration, browser, OwnDB editor, reference
/// library and `.HSC` file-tool ports so a single `&dyn SpeciesWorkspacePort`
/// can be handed to UI modules. Any type implementing all of them gets it via
/// the blanket impl. Trait upcasting lets it coerce to any of its supertraits.
pub trait SpeciesWorkspacePort:
    SpeciesCatalogPort
    + SpeciesDatabaseAdminPort
    + SpeciesBrowserPort
    + OwnDbEditorPort
    + ReferenceLibraryPort
    + HscFileToolsPort
{
}

impl<T> SpeciesWorkspacePort for T where
    T: SpeciesCatalogPort
        + SpeciesDatabaseAdminPort
        + SpeciesBrowserPort
        + OwnDbEditorPort
        + ReferenceLibraryPort
        + HscFileToolsPort
        + ?Sized
{
}

/// Location of the default database files.
pub trait DatabaseLocatorPort {
    /// Default `MainDB` file, if one is available.
    fn default_main_db(&self) -> Option<PathBuf>;

    /// Default `OwnDB` file, if one is available.
    fn default_own_db(&self) -> Option<PathBuf>;
}

/// Errors for future fallible catalog operations.
///
/// Returned by the administration, browser, editor, reference and file-tool
/// ports. Not yet returned by [`SpeciesRepository`] or [`SpeciesCatalogPort`];
/// see the latter's documentation for the planned migration.
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
