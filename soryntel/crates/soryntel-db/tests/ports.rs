//! Puertos de salida del catálogo ejercitados solo a través de `&dyn`/`&mut dyn` o
//! genéricos (nunca con los métodos concretos de `ThermoDatabase`).
//!
//! Los `.HSC` / `.REF` se generan en un directorio temporal a partir de datos en memoria.

use soryntel_core::{
    CatalogError, ChemicalSpecies, DatabaseLocatorPort, DbSource, ElementSearchMode,
    HscFileToolsPort, KeywordSearchField, MainDbPrecedence, MergeConflictDecision, OwnDbEditorPort,
    PhaseFilterFlags, ReferenceLibraryPort, SpeciesBrowserPort, SpeciesDatabaseAdminPort,
    SpeciesDataset, SpeciesMetadata, SpeciesRepository,
};
use soryntel_db::adapters::{DatabaseDirLocator, HscFileTools, MAIN_DB_FILE, OWN_DB_FILE};
use soryntel_db::hsc_binary::serialize_datasets_to_hsc;
use soryntel_db::ThermoDatabase;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FORMULA: &str = "Al2O3";
const MAIN_ONLY: &str = "ZrO2";
const NAME_MAIN: &str = "Al2O3 desde MainDB";
const NAME_OWN: &str = "Al2O3 desde OwnDB";

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("soryntel-db-ports-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn variant(base: &ChemicalSpecies, name: &str, formula: &str, h_shift_j: f64) -> ChemicalSpecies {
    let mut sp = base.clone();
    sp.formula = formula.into();
    sp.name = name.into();
    sp.intervals[0].h_or_delta_h_tr += h_shift_j;
    sp
}

fn metadata(formula: &str) -> SpeciesMetadata {
    SpeciesMetadata {
        structural_formula: formula.into(),
        reliability_class: 1,
        ..Default::default()
    }
}

fn dataset(sp: ChemicalSpecies, source: DbSource) -> SpeciesDataset {
    SpeciesDataset {
        source,
        metadata: metadata(&sp.formula),
        species: sp,
        dataset_index: 0,
        raw_phase_code: "s".into(),
    }
}

fn write_hsc(path: &Path, species: Vec<ChemicalSpecies>, source: DbSource) {
    let datasets: Vec<SpeciesDataset> = species.into_iter().map(|s| dataset(s, source)).collect();
    std::fs::write(path, serialize_datasets_to_hsc(&datasets)).unwrap();
}

/// Registro `.REF` de 293 bytes: clave (36) + cita (256) + bandera (1).
fn ref_record(key: &str, citation: &str, flag: u8) -> Vec<u8> {
    let mut rec = vec![b' '; 293];
    rec[..key.len()].copy_from_slice(key.as_bytes());
    rec[36..36 + citation.len()].copy_from_slice(citation.as_bytes());
    rec[292] = flag;
    rec
}

/// `MAINDB6.HSC` (Al2O3 + ZrO2) y `OWNDB6.HSC` (Al2O3) en `tmp`.
fn fixture(tmp: &TempDir) -> ThermoDatabase {
    let db = ThermoDatabase::with_open_defaults();
    let open = db.get_species(FORMULA).expect("Al2O3 en OpenDefault");
    let main = variant(&open, NAME_MAIN, FORMULA, -10_000.0);
    let main_only = variant(&open, "Zirconia de prueba", MAIN_ONLY, 0.0);
    let own = variant(&open, NAME_OWN, FORMULA, -20_000.0);
    write_hsc(
        &tmp.path("MAINDB6.HSC"),
        vec![main, main_only],
        DbSource::Main,
    );
    write_hsc(&tmp.path("OWNDB6.HSC"), vec![own], DbSource::Own);
    db
}

fn name_of(repo: &dyn SpeciesRepository, formula: &str) -> String {
    repo.get_species(formula).unwrap().name
}

// --- SpeciesDatabaseAdminPort -------------------------------------------------

fn load_layers(
    admin: &mut dyn SpeciesDatabaseAdminPort,
    dir: &TempDir,
    precedence: MainDbPrecedence,
) {
    let (unique, records) = admin
        .load_main_db(&dir.path("MAINDB6.HSC"), precedence)
        .unwrap();
    assert_eq!(unique, 2);
    assert!(records >= 2);
}

#[test]
fn admin_precedencia_explicita_al_cargar_main() {
    let tmp = TempDir::new("admin-precedence");

    let mut db = fixture(&tmp);
    load_layers(&mut db, &tmp, MainDbPrecedence::OverrideOpenDefault);
    assert_eq!(name_of(&db, FORMULA), NAME_MAIN);

    let mut legacy = fixture(&tmp);
    load_layers(&mut legacy, &tmp, MainDbPrecedence::PreserveOpenDefault);
    assert_ne!(name_of(&legacy, FORMULA), NAME_MAIN);
    // MainDB sigue aportando especies nuevas con cualquier política.
    assert_eq!(name_of(&legacy, MAIN_ONLY), "Zirconia de prueba");
}

#[test]
fn admin_carga_guarda_y_resume() {
    let tmp = TempDir::new("admin-io");
    let mut db = fixture(&tmp);
    let admin: &mut dyn SpeciesDatabaseAdminPort = &mut db;

    load_layers(admin, &tmp, MainDbPrecedence::OverrideOpenDefault);
    let (own_unique, _) = admin.load_own_db(&tmp.path("OWNDB6.HSC")).unwrap();
    assert_eq!(own_unique, 1);

    let status = admin.status();
    assert_eq!(status.main_unique_species, 2);
    assert_eq!(status.own_unique_species, 1);
    assert_eq!(
        status.main_path.as_deref(),
        Some(tmp.path("MAINDB6.HSC").as_path())
    );

    assert_eq!(admin.save_own_db(&tmp.path("OWN_COPY.HSC")).unwrap(), 1);
    assert_eq!(admin.status().own_path, Some(tmp.path("OWN_COPY.HSC")));
    assert_eq!(
        admin.save_main_db_copy(&tmp.path("MAIN_COPY.HSC")).unwrap(),
        2
    );
    assert!(admin.refresh_catalogue(DbSource::Own).unwrap().exists());
    admin.sort_layer(DbSource::Main);

    admin.create_empty_own_db(&tmp.path("EMPTY.HSC")).unwrap();
    assert_eq!(admin.status().own_unique_species, 0);
    assert_eq!(name_of(&db, FORMULA), NAME_MAIN);
}

#[test]
fn admin_errores_de_io_son_backend() {
    let tmp = TempDir::new("admin-err");
    let mut db = ThermoDatabase::with_open_defaults();
    let admin: &mut dyn SpeciesDatabaseAdminPort = &mut db;
    let missing = tmp.path("NO_EXISTE.HSC");
    assert!(matches!(
        admin.load_main_db(&missing, MainDbPrecedence::OverrideOpenDefault),
        Err(CatalogError::Backend(_))
    ));
    assert!(matches!(
        admin.load_own_db(&missing),
        Err(CatalogError::Backend(_))
    ));
    // Built-in no tiene archivo asociado.
    assert!(matches!(
        admin.refresh_catalogue(DbSource::OpenDefault),
        Err(CatalogError::Backend(_))
    ));
}

// --- SpeciesBrowserPort -------------------------------------------------------

/// Recorre todo el catálogo por páginas, sin `search("", N)`.
fn collect_paged<B: SpeciesBrowserPort + ?Sized>(
    browser: &B,
    source: Option<DbSource>,
    page: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut offset = 0;
    loop {
        let chunk = browser.species_page(source, offset, page);
        if chunk.is_empty() {
            break;
        }
        offset += chunk.len();
        out.extend(chunk.into_iter().map(|e| e.formula));
    }
    out
}

#[test]
fn browser_enumera_y_pagina_el_catalogo() {
    let tmp = TempDir::new("browser-iter");
    let mut db = fixture(&tmp);
    SpeciesDatabaseAdminPort::load_main_db(
        &mut db,
        &tmp.path("MAINDB6.HSC"),
        MainDbPrecedence::OverrideOpenDefault,
    )
    .unwrap();
    let browser: &dyn SpeciesBrowserPort = &db;

    let all: Vec<String> = browser.iter_species(None).map(|e| e.formula).collect();
    assert_eq!(all.len(), browser.entry_count(None));
    assert_eq!(
        all.len(),
        soryntel_core::SpeciesCatalogPort::species_count(&db)
    );
    assert!(all.contains(&MAIN_ONLY.to_string()));
    assert_eq!(collect_paged(browser, None, 7), all);

    // Mismo resultado que una búsqueda sin texto, sin filtros y sin límite.
    let listed: Vec<String> = browser
        .search_keyword(
            "",
            KeywordSearchField::Any,
            None,
            &PhaseFilterFlags::default(),
            usize::MAX,
        )
        .into_iter()
        .map(|e| e.formula)
        .collect();
    assert_eq!(listed, all);

    let main: Vec<String> = browser
        .iter_species(Some(DbSource::Main))
        .map(|e| e.formula)
        .collect();
    assert_eq!(main, vec![FORMULA.to_string(), MAIN_ONLY.to_string()]);
    assert_eq!(browser.entry_count(Some(DbSource::Own)), 0);
    assert!(browser.species_page(None, all.len(), 10).is_empty());
}

#[test]
fn browser_busquedas_y_datasets() {
    let tmp = TempDir::new("browser-search");
    let mut db = fixture(&tmp);
    SpeciesDatabaseAdminPort::load_main_db(
        &mut db,
        &tmp.path("MAINDB6.HSC"),
        MainDbPrecedence::OverrideOpenDefault,
    )
    .unwrap();
    let browser: &mut dyn SpeciesBrowserPort = &mut db;
    let filters = PhaseFilterFlags::default();

    let hits = browser.search_keyword(
        "zirconia",
        KeywordSearchField::ChemicalName,
        None,
        &filters,
        10,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].formula, MAIN_ONLY);
    assert_eq!(hits[0].source, DbSource::Main);

    let elems = browser.search_elements(
        &["Zr".into(), "O".into()],
        ElementSearchMode::All,
        Some(DbSource::Main),
        &filters,
        10,
    );
    assert_eq!(elems.len(), 1);

    let stoich = browser
        .search_stoichiometry("AlO1.5", true, Some(DbSource::Main), &filters, 10)
        .unwrap();
    assert!(stoich.iter().any(|e| e.formula == FORMULA));
    assert!(matches!(
        browser.search_stoichiometry("((", false, None, &filters, 10),
        Err(CatalogError::Backend(_))
    ));

    assert_eq!(browser.datasets_for(DbSource::Main, FORMULA).len(), 1);
    assert!(browser.select_dataset(DbSource::Main, FORMULA, 0));
    assert!(!browser.select_dataset(DbSource::Main, FORMULA, 5));
    assert!(!browser.select_dataset(DbSource::OpenDefault, FORMULA, 0));
    assert_eq!(browser.resolved_source_of(FORMULA), Some(DbSource::Main));
    assert_eq!(browser.resolved_source_of("NoExiste"), None);
}

// --- OwnDbEditorPort ----------------------------------------------------------

fn edit_own<E: OwnDbEditorPort + ?Sized>(editor: &mut E, sp: ChemicalSpecies) {
    let f = sp.formula.clone();
    editor.upsert_own(sp, metadata(&f), "s".into());
}

#[test]
fn editor_upsert_borrado_y_csv() {
    let mut db = ThermoDatabase::with_open_defaults();
    let base = db.get_species(FORMULA).unwrap();
    edit_own(&mut db, variant(&base, NAME_OWN, FORMULA, -20_000.0));
    assert_eq!(name_of(&db, FORMULA), NAME_OWN);
    assert_eq!(
        SpeciesBrowserPort::resolved_source_of(&db, FORMULA),
        Some(DbSource::Own)
    );

    let editor: &mut dyn OwnDbEditorPort = &mut db;
    let csv = editor.export_csv(Some(DbSource::Own));
    assert!(csv.contains(FORMULA));
    let json = editor.export_json(Some(DbSource::Own)).unwrap();
    assert!(json.contains(NAME_OWN));
    assert!(json.contains("\"source\": \"Own\""));

    assert!(editor.delete_own(FORMULA));
    assert!(!editor.delete_own(FORMULA));

    let imported = editor.import_csv_into_own(&csv).unwrap();
    assert_eq!(imported, 1);
    assert_eq!(name_of(&db, FORMULA), NAME_OWN);
}

// --- ReferenceLibraryPort -----------------------------------------------------

#[test]
fn referencias_carga_citas_y_busqueda() {
    let tmp = TempDir::new("refs");
    let mut bytes = ref_record(
        "Barin 89",
        "I. Barin: Thermochemical Data of Pure Substances",
        1,
    );
    bytes.extend(ref_record("SGTE 94", "SGTE Pure Substance Database", 0));
    std::fs::write(tmp.path("MainDB6.REF"), &bytes).unwrap();

    let mut db = ThermoDatabase::with_open_defaults();
    let refs: &mut dyn ReferenceLibraryPort = &mut db;
    assert_eq!(refs.load_references(&tmp.path("MainDB6.REF")).unwrap(), 2);

    let cites = refs.citations_for("Barin 89, SGTE 94; Desconocida");
    assert_eq!(cites.len(), 2);
    assert_eq!(cites[0].key, "Barin 89");
    assert_eq!(cites[0].flag, 1);

    assert_eq!(refs.search_references("sgte", 10).len(), 1);
    assert_eq!(refs.search_references("", 1).len(), 1);
    assert!(matches!(
        refs.load_references(&tmp.path("NO.REF")),
        Err(CatalogError::Backend(_))
    ));
    assert_eq!(SpeciesDatabaseAdminPort::status(&db).references_count, 2);
}

// --- HscFileToolsPort ---------------------------------------------------------

fn merge_and_convert(tools: &dyn HscFileToolsPort, tmp: &TempDir) {
    let src = tmp.path("MAINDB6.HSC");
    let dst = tmp.path("OWNDB6.HSC");

    let (_, conflicts) = tools
        .merge_hsc_files(&src, &dst, &BTreeMap::new(), true)
        .unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].formula, FORMULA);

    let decisions = BTreeMap::from([(FORMULA.to_string(), MergeConflictDecision::TakeSource)]);
    let (merged, _) = tools
        .merge_hsc_files(&src, &dst, &decisions, false)
        .unwrap();
    assert_eq!(merged, 2);

    let converted = tools
        .convert_legacy_to_hsc6(&dst, &tmp.path("CONVERTED.HSC"))
        .unwrap();
    assert_eq!(converted, 2);
    assert!(matches!(
        tools.convert_legacy_to_hsc6(&tmp.path("NO.HSC"), &tmp.path("X.HSC")),
        Err(CatalogError::Backend(_))
    ));
}

#[test]
fn herramientas_hsc_sin_estado() {
    let tmp = TempDir::new("tools");
    let _ = fixture(&tmp);
    merge_and_convert(&HscFileTools, &tmp);
}

#[test]
fn herramientas_hsc_desde_thermo_database() {
    let tmp = TempDir::new("tools-db");
    let db = fixture(&tmp);
    merge_and_convert(&db, &tmp);

    let mut check = ThermoDatabase::with_open_defaults();
    let (unique, _) =
        SpeciesDatabaseAdminPort::load_own_db(&mut check, &tmp.path("OWNDB6.HSC")).unwrap();
    assert_eq!(unique, 2);
    assert_eq!(name_of(&check, FORMULA), NAME_MAIN);
}

// --- DatabaseLocatorPort ------------------------------------------------------

#[test]
fn localizador_busca_en_orden() {
    let first = TempDir::new("loc-a");
    let second = TempDir::new("loc-b");
    std::fs::write(second.path(MAIN_DB_FILE), b"x").unwrap();
    std::fs::write(second.path(OWN_DB_FILE), b"x").unwrap();
    std::fs::write(first.path(OWN_DB_FILE), b"x").unwrap();

    let locator = DatabaseDirLocator::new([first.0.clone(), second.0.clone()]);
    let port: &dyn DatabaseLocatorPort = &locator;
    assert_eq!(port.default_main_db(), Some(second.path(MAIN_DB_FILE)));
    assert_eq!(port.default_own_db(), Some(first.path(OWN_DB_FILE)));

    let empty: &dyn DatabaseLocatorPort = &DatabaseDirLocator::new([]);
    assert_eq!(empty.default_main_db(), None);
    assert_eq!(empty.default_own_db(), None);
}
