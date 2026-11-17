//! Precedencia de capas al resolver una especie presente en varias fuentes.
//!
//! Contrato documentado en `ThermoDatabase` (HSC Chemistry 6, cap. 28.17):
//! **`OwnDB` > `MainDB` > `OpenDefault`**.
//!
//! Los `.HSC` se generan en un directorio temporal a partir de datos construidos en memoria
//! (nunca se leen bases de datos del usuario).

use soryntel_core::{ChemicalSpecies, SpeciesCatalogPort, SpeciesMetadata, SpeciesRepository};
use soryntel_db::hsc_binary::serialize_datasets_to_hsc;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::species::{DbSource, SpeciesDataset};
use std::path::PathBuf;

const FORMULA: &str = "Al2O3";
const NAME_OPEN: &str = "Aluminum oxide (corundum)";
const NAME_MAIN: &str = "Al2O3 desde MainDB";
const NAME_OWN: &str = "Al2O3 desde OwnDB";
/// Especie que solo existe en MainDB (comprueba que MainDB aporta especies nuevas).
const MAIN_ONLY: &str = "ZrO2";

/// Directorio temporal único por test (sin dependencias externas).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "soryntel-db-precedence-{}-{tag}",
            std::process::id()
        ));
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

/// Copia de la especie `OpenDefault` con otro nombre y otra entalpía de formación,
/// para poder distinguir de qué capa procede el registro resuelto.
fn variant(base: &ChemicalSpecies, name: &str, formula: &str, h_shift_j: f64) -> ChemicalSpecies {
    let mut sp = base.clone();
    sp.formula = formula.into();
    sp.name = name.into();
    sp.intervals[0].h_or_delta_h_tr += h_shift_j;
    sp
}

fn dataset(sp: ChemicalSpecies, source: DbSource) -> SpeciesDataset {
    SpeciesDataset {
        source,
        metadata: SpeciesMetadata {
            structural_formula: sp.formula.clone(),
            reliability_class: 1,
            ..Default::default()
        },
        species: sp,
        dataset_index: 0,
        raw_phase_code: "s".into(),
    }
}

fn write_hsc(path: &PathBuf, species: Vec<ChemicalSpecies>, source: DbSource) {
    let datasets: Vec<SpeciesDataset> = species.into_iter().map(|s| dataset(s, source)).collect();
    std::fs::write(path, serialize_datasets_to_hsc(&datasets)).unwrap();
}

fn h0(db: &ThermoDatabase, formula: &str) -> f64 {
    db.get_species(formula).unwrap().intervals[0].h_or_delta_h_tr
}

/// Escenario: `Al2O3` presente en las tres capas, `ZrO2` solo en MainDB.
fn fixture(tmp: &TempDir) -> (ThermoDatabase, f64, f64, f64) {
    let db = ThermoDatabase::with_open_defaults();
    let open = db.get_species(FORMULA).expect("Al2O3 en OpenDefault");
    let h_open = open.intervals[0].h_or_delta_h_tr;

    // Desplazamientos grandes frente a la precisión f32 del formato .HSC.
    let main = variant(&open, NAME_MAIN, FORMULA, -10_000.0);
    let main_only = variant(&open, "Zirconia de prueba", MAIN_ONLY, 0.0);
    let own = variant(&open, NAME_OWN, FORMULA, -20_000.0);
    let h_main = main.intervals[0].h_or_delta_h_tr;
    let h_own = own.intervals[0].h_or_delta_h_tr;

    write_hsc(&tmp.path("MAINDB6.HSC"), vec![main, main_only], DbSource::Main);
    write_hsc(&tmp.path("OWNDB6.HSC"), vec![own], DbSource::Own);
    (db, h_open, h_main, h_own)
}

#[test]
fn open_default_es_la_capa_base() {
    let db = ThermoDatabase::with_open_defaults();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::OpenDefault));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OPEN);
}

#[test]
fn precedencia_own_sobre_main_sobre_open_default() {
    let tmp = TempDir::new("strict");
    let (mut db, h_open, h_main, h_own) = fixture(&tmp);
    let eps = 0.5; // J/mol; el .HSC guarda kcal en f32.

    // MainDB > OpenDefault (carga activa: `load_main_hsc` = overwrite=true).
    db.load_main_hsc(tmp.path("MAINDB6.HSC")).unwrap();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Main));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_MAIN);
    assert!((h0(&db, FORMULA) - h_main).abs() < eps);
    assert!((h0(&db, FORMULA) - h_open).abs() > 1000.0);
    // MainDB también aporta especies que no estaban en OpenDefault.
    assert_eq!(db.resolved_source_of(MAIN_ONLY), Some(DbSource::Main));
    // Las especies que solo están en OpenDefault siguen disponibles.
    assert_eq!(db.resolved_source_of("TiO2"), Some(DbSource::OpenDefault));

    // OwnDB > MainDB (carga desde archivo).
    db.load_own_hsc(tmp.path("OWNDB6.HSC")).unwrap();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Own));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OWN);
    assert!((h0(&db, FORMULA) - h_own).abs() < eps);
    assert_eq!(db.resolved_source_of(MAIN_ONLY), Some(DbSource::Main));

    // El puerto del catálogo ve el mismo registro resuelto (sin duplicados por capa).
    let hits: Vec<_> = SpeciesCatalogPort::search_species(&db, FORMULA, 50)
        .into_iter()
        .filter(|s| s.formula == FORMULA)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, NAME_OWN);

    // Borrar de OwnDB revierte de forma transparente a MainDB...
    assert!(db.delete_own_species(FORMULA));
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Main));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_MAIN);

    // ...y recargar MainDB después no desplaza a OwnDB.
    db.insert_species(variant(&db.get_species(FORMULA).unwrap(), NAME_OWN, FORMULA, -5_000.0));
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Own));
    db.load_main_hsc(tmp.path("MAINDB6.HSC")).unwrap();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Own));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OWN);
}

#[test]
fn edicion_en_memoria_va_a_own_db_y_gana_sobre_open_default() {
    let mut db = ThermoDatabase::with_open_defaults();
    let open = db.get_species(FORMULA).unwrap();
    db.insert_or_update_species(variant(&open, NAME_OWN, FORMULA, -1_000.0));
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Own));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OWN);

    assert!(db.delete_own_species(FORMULA));
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::OpenDefault));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OPEN);
}

/// DISCREPANCIA DOCUMENTADA (no corregida): con el cargador heredado
/// `load_external_hsc_readonly(path, overwrite = false)` el orden efectivo es
/// **`OwnDB` > `OpenDefault` > `MainDB`**: las especies integradas conservan prioridad sobre
/// MainDB "por paridad de regresión". Solo `overwrite = true` (`load_main_hsc`) aplica el
/// contrato estricto `OwnDB > MainDB > OpenDefault`.
#[test]
fn cargador_heredado_sin_overwrite_prioriza_open_default_sobre_main() {
    let tmp = TempDir::new("legacy");
    let (mut db, _h_open, _h_main, _h_own) = fixture(&tmp);

    db.load_external_hsc_readonly(tmp.path("MAINDB6.HSC"), false).unwrap();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::OpenDefault));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OPEN);
    // Las especies nuevas de MainDB sí se incorporan.
    assert_eq!(db.resolved_source_of(MAIN_ONLY), Some(DbSource::Main));
    // Los datasets de MainDB siguen indexados aunque no estén resueltos.
    assert_eq!(db.get_all_datasets_for(DbSource::Main, FORMULA).len(), 1);

    // OwnDB sigue ganando a ambas capas.
    db.load_own_hsc(tmp.path("OWNDB6.HSC")).unwrap();
    assert_eq!(db.resolved_source_of(FORMULA), Some(DbSource::Own));
    assert_eq!(db.get_species(FORMULA).unwrap().name, NAME_OWN);
}
