//! Port implementations for [`ThermoDatabase`].

use super::to_catalog_error;
use crate::ThermoDatabase;
use soryntel_core::{
    CatalogError, ChemicalSpecies, DatabaseStatusSummary, DbMergeConflict, DbSource,
    ElementSearchMode, HscFileToolsPort, KeywordSearchField, MainDbPrecedence,
    MergeConflictDecision, OwnDbEditorPort, PhaseFilterFlags, ReferenceEntry, ReferenceLibraryPort,
    SpeciesBrowserPort, SpeciesCatalogEntry, SpeciesDatabaseAdminPort, SpeciesDataset,
    SpeciesMetadata,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

impl SpeciesDatabaseAdminPort for ThermoDatabase {
    fn load_main_db(
        &mut self,
        path: &Path,
        precedence: MainDbPrecedence,
    ) -> Result<(usize, usize), CatalogError> {
        match precedence {
            MainDbPrecedence::OverrideOpenDefault => {
                self.load_main_hsc(path).map_err(to_catalog_error)
            }
            MainDbPrecedence::PreserveOpenDefault => {
                let unique = self
                    .load_external_hsc_readonly(path, false)
                    .map_err(to_catalog_error)?;
                Ok((unique, self.main_binary_records))
            }
        }
    }

    fn load_own_db(&mut self, path: &Path) -> Result<(usize, usize), CatalogError> {
        self.load_own_hsc(path).map_err(to_catalog_error)
    }

    fn save_own_db(&mut self, path: &Path) -> Result<usize, CatalogError> {
        self.save_own_hsc(path).map_err(to_catalog_error)
    }

    fn create_empty_own_db(&mut self, path: &Path) -> Result<(), CatalogError> {
        self.create_empty_own_hsc(path).map_err(to_catalog_error)
    }

    fn save_main_db_copy(&self, path: &Path) -> Result<usize, CatalogError> {
        self.save_main_hsc_copy(path).map_err(to_catalog_error)
    }

    fn refresh_catalogue(&self, source: DbSource) -> Result<PathBuf, CatalogError> {
        ThermoDatabase::refresh_catalogue(self, source).map_err(to_catalog_error)
    }

    fn sort_layer(&mut self, source: DbSource) {
        self.sort_database(source);
    }

    fn status(&self) -> DatabaseStatusSummary {
        self.status_summary()
    }
}

impl SpeciesBrowserPort for ThermoDatabase {
    fn search_keyword(
        &self,
        query: &str,
        field: KeywordSearchField,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        self.search_advanced(query, field, source, filters, limit)
    }

    fn search_elements(
        &self,
        elements: &[String],
        mode: ElementSearchMode,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        self.search_elements_filtered(elements, mode, source, filters, limit)
    }

    fn search_stoichiometry(
        &self,
        pattern: &str,
        fix_ratio: bool,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Result<Vec<SpeciesCatalogEntry>, CatalogError> {
        ThermoDatabase::search_stoichiometry(self, pattern, fix_ratio, source, filters, limit)
            .map_err(to_catalog_error)
    }

    fn entry_count(&self, source: Option<DbSource>) -> usize {
        self.iter_entries(source).count()
    }

    fn iter_species(
        &self,
        source: Option<DbSource>,
    ) -> Box<dyn Iterator<Item = SpeciesCatalogEntry> + '_> {
        Box::new(self.iter_entries(source))
    }

    fn species_page(
        &self,
        source: Option<DbSource>,
        offset: usize,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        self.iter_entries(source).skip(offset).take(limit).collect()
    }

    fn datasets_for(&self, source: DbSource, formula: &str) -> Vec<SpeciesDataset> {
        self.get_all_datasets_for(source, formula)
    }

    fn select_dataset(&mut self, source: DbSource, formula: &str, index: usize) -> bool {
        ThermoDatabase::select_dataset(self, source, formula, index)
    }

    fn resolved_source_of(&self, formula: &str) -> Option<DbSource> {
        ThermoDatabase::resolved_source_of(self, formula)
    }
}

impl OwnDbEditorPort for ThermoDatabase {
    fn upsert_own(
        &mut self,
        species: ChemicalSpecies,
        metadata: SpeciesMetadata,
        raw_phase_code: String,
    ) {
        self.upsert_own_dataset(species, metadata, raw_phase_code);
    }

    fn delete_own(&mut self, formula: &str) -> bool {
        self.delete_own_species(formula)
    }

    fn import_csv_into_own(&mut self, csv: &str) -> Result<usize, CatalogError> {
        ThermoDatabase::import_csv_into_own(self, csv).map_err(to_catalog_error)
    }

    fn export_csv(&self, source: Option<DbSource>) -> String {
        ThermoDatabase::export_csv(self, source)
    }

    fn export_json(&self, source: Option<DbSource>) -> Result<String, CatalogError> {
        ThermoDatabase::export_json(self, source).map_err(to_catalog_error)
    }
}

impl ReferenceLibraryPort for ThermoDatabase {
    fn load_references(&mut self, path: &Path) -> Result<usize, CatalogError> {
        self.load_ref_dictionary(path).map_err(to_catalog_error)
    }

    fn citations_for(&self, ref_field: &str) -> Vec<ReferenceEntry> {
        self.lookup_citations(ref_field)
    }

    fn search_references(&self, query: &str, limit: usize) -> Vec<ReferenceEntry> {
        ThermoDatabase::search_references(self, query, limit)
    }
}

/// Stateless [`HscFileToolsPort`] adapter (no database instance required).
#[derive(Debug, Clone, Copy, Default)]
pub struct HscFileTools;

impl HscFileToolsPort for HscFileTools {
    fn merge_hsc_files(
        &self,
        src: &Path,
        dst: &Path,
        decisions: &BTreeMap<String, MergeConflictDecision>,
        inspect_only: bool,
    ) -> Result<(usize, Vec<DbMergeConflict>), CatalogError> {
        ThermoDatabase::merge_hsc_files(src, dst, decisions, inspect_only).map_err(to_catalog_error)
    }

    fn convert_legacy_to_hsc6(&self, src: &Path, dst: &Path) -> Result<usize, CatalogError> {
        ThermoDatabase::convert_legacy_to_hsc6(src, dst).map_err(to_catalog_error)
    }
}

/// `ThermoDatabase` also exposes the stateless file tools, delegating to [`HscFileTools`].
impl HscFileToolsPort for ThermoDatabase {
    fn merge_hsc_files(
        &self,
        src: &Path,
        dst: &Path,
        decisions: &BTreeMap<String, MergeConflictDecision>,
        inspect_only: bool,
    ) -> Result<(usize, Vec<DbMergeConflict>), CatalogError> {
        HscFileTools.merge_hsc_files(src, dst, decisions, inspect_only)
    }

    fn convert_legacy_to_hsc6(&self, src: &Path, dst: &Path) -> Result<usize, CatalogError> {
        HscFileTools.convert_legacy_to_hsc6(src, dst)
    }
}
