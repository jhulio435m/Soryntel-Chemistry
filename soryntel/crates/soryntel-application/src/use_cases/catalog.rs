//! Casos de uso agrupados: catalog.
use super::super::*;
use soryntel_core::catalog::{
    DatabaseStatusSummary, DbMergeConflict, DbSource, ElementSearchMode, KeywordSearchField,
    MainDbPrecedence, MergeConflictDecision, PhaseFilterFlags, ReferenceEntry,
    SpeciesCatalogEntry, SpeciesDataset,
};
use soryntel_core::{
    CatalogError, HscFileToolsPort, OwnDbEditorPort, ReferenceLibraryPort, SpeciesBrowserPort,
    SpeciesDatabaseAdminPort, SpeciesMetadata,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Texto de error de un puerto de catálogo, listo para mostrarlo en la UI.
///
/// Los adaptadores envuelven los fallos de E/S y de formato en [`CatalogError::Backend`];
/// aquí se devuelve el mensaje original del backend, sin el prefijo del envoltorio, para que
/// la UI muestre el mismo texto que cuando llamaba a `ThermoDatabase` directamente.
pub fn catalog_error_message(err: CatalogError) -> String {
    match err {
        CatalogError::Backend(msg) => msg,
        CatalogError::NotFound(formula) => SoryntelError::SpeciesNotFound(formula).to_string(),
        other => other.to_string(),
    }
}

pub struct CatalogUseCases;
impl CatalogUseCases {
    // ------------------------------------------------------------------
    // Administración de MainDB / OwnDB (SpeciesDatabaseAdminPort)
    // ------------------------------------------------------------------

    /// Rutas activas y contadores de MainDB / OwnDB / REF.
    pub fn status(admin: &dyn SpeciesDatabaseAdminPort) -> DatabaseStatusSummary {
        admin.status()
    }

    /// Abre MainDB con la precedencia de escritorio (`OwnDB > MainDB > OpenDefault`).
    /// Devuelve `(especies únicas, registros binarios)`.
    pub fn open_main_db(
        admin: &mut dyn SpeciesDatabaseAdminPort,
        path: impl AsRef<Path>,
    ) -> Result<(usize, usize), String> {
        admin
            .load_main_db(path.as_ref(), MainDbPrecedence::OverrideOpenDefault)
            .map_err(catalog_error_message)
    }

    /// Abre OwnDB y descarta las ediciones en memoria. Devuelve `(especies, ranuras)`.
    pub fn open_own_db(
        admin: &mut dyn SpeciesDatabaseAdminPort,
        path: impl AsRef<Path>,
    ) -> Result<(usize, usize), String> {
        admin.load_own_db(path.as_ref()).map_err(catalog_error_message)
    }

    /// Persiste OwnDB (`.HSC` + `.CAT`). Devuelve el número de datasets escritos.
    pub fn save_own_db(
        admin: &mut dyn SpeciesDatabaseAdminPort,
        path: impl AsRef<Path>,
    ) -> Result<usize, String> {
        admin.save_own_db(path.as_ref()).map_err(catalog_error_message)
    }

    /// Persiste OwnDB en `path` creando antes su carpeta. Con una ruta vacía no escribe nada
    /// y devuelve `Ok(None)`.
    pub fn save_own_db_creating_dirs(
        admin: &mut dyn SpeciesDatabaseAdminPort,
        path: &str,
    ) -> Result<Option<usize>, String> {
        let path = path.trim();
        if path.is_empty() {
            return Ok(None);
        }
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::save_own_db(admin, path).map(Some)
    }

    /// Crea un OwnDB vacío en disco y vacía la capa OwnDB en memoria.
    pub fn create_empty_own_db(
        admin: &mut dyn SpeciesDatabaseAdminPort,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        admin
            .create_empty_own_db(path.as_ref())
            .map_err(catalog_error_message)
    }

    /// Ordena una capa en orden HSC.
    pub fn sort_layer(admin: &mut dyn SpeciesDatabaseAdminPort, source: DbSource) {
        admin.sort_layer(source);
    }

    /// Regenera los índices `.CAT` de MainDB y OwnDB. Devuelve `(main_ok, own_ok)`.
    pub fn refresh_catalogues(admin: &dyn SpeciesDatabaseAdminPort) -> (bool, bool) {
        let main_ok = admin.refresh_catalogue(DbSource::Main).is_ok();
        let own_ok = admin.refresh_catalogue(DbSource::Own).is_ok();
        (main_ok, own_ok)
    }

    // ------------------------------------------------------------------
    // Navegador (SpeciesBrowserPort)
    // ------------------------------------------------------------------

    /// Interpreta el filtro de número de carbonos («2, 3-4» → `[2, 3, 4]`).
    pub fn parse_carbon_filter(text: &str) -> Vec<u32> {
        soryntel_thermo::species::parse_carbon_limits(text)
    }

    /// Búsqueda por palabra clave sobre `field`, restringida a `source`.
    pub fn browse_keyword(
        browser: &dyn SpeciesBrowserPort,
        query: &str,
        field: KeywordSearchField,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        browser.search_keyword(query, field, source, filters, limit)
    }

    /// Búsqueda por elementos (`Some` / `All`).
    pub fn browse_elements(
        browser: &dyn SpeciesBrowserPort,
        elements: &[String],
        mode: ElementSearchMode,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        browser.search_elements(elements, mode, source, filters, limit)
    }

    /// Búsqueda estequiométrica; falla si `pattern` no es una fórmula válida.
    pub fn browse_stoichiometry(
        browser: &dyn SpeciesBrowserPort,
        pattern: &str,
        fix_ratio: bool,
        source: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Result<Vec<SpeciesCatalogEntry>, String> {
        browser
            .search_stoichiometry(pattern, fix_ratio, source, filters, limit)
            .map_err(catalog_error_message)
    }

    /// Especies de todas las entradas de `source` (`None` = vista resuelta), en orden de
    /// catálogo y sin límite. Sustituye al antiguo `search("", 20_000)`.
    pub fn all_species(
        browser: &dyn SpeciesBrowserPort,
        source: Option<DbSource>,
    ) -> Vec<ChemicalSpecies> {
        browser.iter_species(source).map(|e| e.species).collect()
    }

    /// Datasets alternativos de `formula` en `source`.
    pub fn datasets_for(
        browser: &dyn SpeciesBrowserPort,
        source: DbSource,
        formula: &str,
    ) -> Vec<SpeciesDataset> {
        browser.datasets_for(source, formula)
    }

    /// Activa el dataset `index` de `formula` en `source`.
    pub fn select_dataset(
        browser: &mut dyn SpeciesBrowserPort,
        source: DbSource,
        formula: &str,
        index: usize,
    ) -> bool {
        browser.select_dataset(source, formula, index)
    }

    /// Especie resuelta y sus metadatos (los de por defecto si el catálogo no tiene).
    pub fn species_with_metadata(
        catalog: &dyn SpeciesCatalogPort,
        formula: &str,
    ) -> Option<(ChemicalSpecies, SpeciesMetadata)> {
        let sp = catalog.get_species(formula)?;
        let meta = catalog.get_species_metadata(formula).unwrap_or_default();
        Some((sp, meta))
    }

    // ------------------------------------------------------------------
    // Edición de OwnDB e intercambio CSV (OwnDbEditorPort)
    // ------------------------------------------------------------------

    /// Inserta o reemplaza el dataset primario de OwnDB para `species.formula`.
    pub fn save_own_species(
        editor: &mut dyn OwnDbEditorPort,
        species: ChemicalSpecies,
        metadata: SpeciesMetadata,
        raw_phase_code: String,
    ) {
        editor.upsert_own(species, metadata, raw_phase_code);
    }

    /// Borra `formula` de OwnDB. Devuelve si existía.
    pub fn delete_own_species(editor: &mut dyn OwnDbEditorPort, formula: &str) -> bool {
        editor.delete_own(formula)
    }

    /// CSV de `source` (`None` = vista resuelta). La escritura en disco queda en la UI
    /// (`storage::write_atomic`).
    pub fn export_csv(editor: &dyn OwnDbEditorPort, source: Option<DbSource>) -> String {
        editor.export_csv(source)
    }

    /// Importa un texto CSV en OwnDB. Devuelve los datasets importados.
    pub fn import_csv_into_own(
        editor: &mut dyn OwnDbEditorPort,
        csv: &str,
    ) -> Result<usize, String> {
        editor.import_csv_into_own(csv).map_err(catalog_error_message)
    }

    /// Lee el fichero `path` e importa su CSV en OwnDB.
    pub fn import_csv_file_into_own(
        editor: &mut dyn OwnDbEditorPort,
        path: impl AsRef<Path>,
    ) -> Result<usize, String> {
        let csv = std::fs::read_to_string(path.as_ref()).map_err(|e| format!("{e}"))?;
        Self::import_csv_into_own(editor, &csv)
    }

    // ------------------------------------------------------------------
    // Referencias bibliográficas (ReferenceLibraryPort)
    // ------------------------------------------------------------------

    /// Carga el diccionario `.REF`. Devuelve el número de entradas.
    pub fn load_references(
        refs: &mut dyn ReferenceLibraryPort,
        path: impl AsRef<Path>,
    ) -> Result<usize, String> {
        refs.load_references(path.as_ref())
            .map_err(catalog_error_message)
    }

    /// Citas del campo `Reference` de un intervalo.
    pub fn citations_for(refs: &dyn ReferenceLibraryPort, ref_field: &str) -> Vec<ReferenceEntry> {
        refs.citations_for(ref_field)
    }

    /// Busca en las claves y en el texto de las citas.
    pub fn search_references(
        refs: &dyn ReferenceLibraryPort,
        query: &str,
        limit: usize,
    ) -> Vec<ReferenceEntry> {
        refs.search_references(query, limit)
    }

    // ------------------------------------------------------------------
    // Herramientas de ficheros `.HSC` (HscFileToolsPort)
    // ------------------------------------------------------------------

    /// Fusiona `src` en `dst` (con `inspect_only` solo inspecciona duplicados).
    /// Devuelve `(datasets fusionados, conflictos)`.
    pub fn merge_hsc_files(
        tools: &dyn HscFileToolsPort,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
        decisions: &BTreeMap<String, MergeConflictDecision>,
        inspect_only: bool,
    ) -> Result<(usize, Vec<DbMergeConflict>), String> {
        tools
            .merge_hsc_files(src.as_ref(), dst.as_ref(), decisions, inspect_only)
            .map_err(catalog_error_message)
    }

    /// Convierte un `.HSC` o CSV de HSC 2–5 a HSC 6. Devuelve los datasets escritos.
    pub fn convert_legacy_to_hsc6(
        tools: &dyn HscFileToolsPort,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<usize, String> {
        tools
            .convert_legacy_to_hsc6(src.as_ref(), dst.as_ref())
            .map_err(catalog_error_message)
    }

    // ------------------------------------------------------------------
    // Consultas sobre SpeciesCatalogPort y utilidades de especies
    // ------------------------------------------------------------------

    pub fn search_species(
        catalog: &dyn SpeciesCatalogPort,
        query: &str,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species(query, limit)
    }

    pub fn search_species_by_elements(
        catalog: &dyn SpeciesCatalogPort,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species_by_elements(
            elements,
            include_gas,
            include_liquid,
            include_solid,
            include_aqua_ions,
            include_aqua_neutrals,
            limit,
        )
    }

    pub fn species_count(catalog: &dyn SpeciesCatalogPort) -> usize {
        catalog.species_count()
    }

    /// Primer juego de datos de `formula` en la capa `source` (OwnDB o MainDB), sin aplicar
    /// la precedencia OwnDB > MainDB > OpenDefault de `get_species`.
    pub fn species_in_layer(
        browser: &dyn soryntel_core::SpeciesBrowserPort,
        source: soryntel_core::catalog::DbSource,
        formula: &str,
    ) -> Option<ChemicalSpecies> {
        browser
            .datasets_for(source, formula)
            .into_iter()
            .next()
            .map(|d| d.species)
    }

    pub fn species_compute_mw(
        formula: &str,
    ) -> Result<(f64, std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        compute_molecular_weight(formula)
    }

    pub fn species_parse_elements(
        formula: &str,
    ) -> Result<(std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        parse_hsc_formula_elements(formula)
    }

    pub fn species_fit_cp(
        points: &[CpDataPoint],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<CpFitResult, SoryntelError> {
        fit_cp_kelley(points, mask, locked_si)
    }

    pub fn species_convert_cp_function(
        expr: &str,
        unit: &str,
        t1_k: f64,
        t2_k: f64,
        params: &[f64; 6],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(Vec<CpDataPoint>, CpFitResult), SoryntelError> {
        convert_cp_function(expr, unit, t1_k, t2_k, params, mask, locked_si)
    }

    pub fn species_convert_gibbs(
        input: &GibbsCoeffsInput,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        convert_gibbs_function(input)
    }

    pub fn species_gibbs_delta_to_normal(
        catalog: &dyn SpeciesCatalogPort,
        target_formula: &str,
        delta_out: &GibbsConvertOutput,
        t_ref_k: f64,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        gibbs_delta_to_normal(catalog, target_formula, delta_out, t_ref_k)
    }

    pub fn species_extrapolate_liquid_298(
        solid_intervals: &[soryntel_thermo::SpeciesThermoInterval],
        t_melt_k: f64,
        delta_h_fus_j: f64,
        liquid_kelley: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(f64, f64), SoryntelError> {
        extrapolate_liquid_298(solid_intervals, t_melt_k, delta_h_fus_j, liquid_kelley)
    }

    pub fn species_analyze_aqueous(species: &ChemicalSpecies) -> Option<AqueousScaleSummary> {
        analyze_aqueous_scales(species)
    }

    pub fn species_color_spec(code: i16) -> &'static HscColorSpec {
        color_spec(code)
    }

    pub fn species_awdb_weight(symbol: &str) -> Option<f64> {
        awdb_atomic_weight(symbol)
    }

    pub fn species_stoichiometry_matches(
        candidate: &std::collections::BTreeMap<String, f64>,
        target: &std::collections::BTreeMap<String, f64>,
        fix_ratio: bool,
    ) -> bool {
        stoichiometry_matches(candidate, target, fix_ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_core::SpeciesRepository;
    use soryntel_db::ThermoDatabase;

    #[test]
    fn catalog_error_message_unwraps_backend_text() {
        assert_eq!(
            catalog_error_message(CatalogError::Backend("Cannot open OwnDB x".into())),
            "Cannot open OwnDB x"
        );
        assert_eq!(
            catalog_error_message(CatalogError::NotFound("H2O".into())),
            SoryntelError::SpeciesNotFound("H2O".into()).to_string()
        );
    }

    #[test]
    fn own_species_save_export_import_roundtrip() {
        let mut db = ThermoDatabase::with_open_defaults();
        let mut sp = CatalogUseCases::search_species(&db, "", 1)
            .pop()
            .expect("open defaults are not empty");
        sp.formula = "H2OTestX".into();
        sp.name = "Test water".into();
        CatalogUseCases::save_own_species(&mut db, sp, SpeciesMetadata::default(), "g".into());
        let csv = CatalogUseCases::export_csv(&db, Some(DbSource::Own));
        assert!(csv.contains("H2OTestX"));

        assert!(CatalogUseCases::delete_own_species(&mut db, "H2OTestX"));
        assert!(db.get_species("H2OTestX").is_none());
        let n = CatalogUseCases::import_csv_into_own(&mut db, &csv).unwrap();
        assert!(n >= 1);
        assert_eq!(db.get_species("H2OTestX").unwrap().name, "Test water");
    }

    #[test]
    fn open_missing_own_db_reports_backend_message() {
        let mut db = ThermoDatabase::with_open_defaults();
        let err = CatalogUseCases::open_own_db(&mut db, "/nonexistent/soryntel/OWNDB6.HSC")
            .unwrap_err();
        assert!(err.contains("Cannot open OwnDB"), "{err}");
        assert!(!err.contains("catalog backend error"), "{err}");
        assert_eq!(
            CatalogUseCases::save_own_db_creating_dirs(&mut db, "   ").unwrap(),
            None
        );
    }
}
