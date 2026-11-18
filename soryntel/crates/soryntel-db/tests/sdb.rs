//! SoryntelDB (`.sdb`): conversión, fidelidad, validación de cabecera y precedencia.
//!
//! Todos los archivos (`.HSC`, `.REF`, `.cat` y `.sdb`) se generan sintéticos en un directorio
//! temporal a partir de datos construidos en memoria; nunca se leen bases del usuario.

use soryntel_core::{
    CatalogError, ReferenceLibraryPort, SpeciesBrowserPort, SpeciesDatabaseAdminPort,
    SpeciesRepository,
};
use soryntel_db::hsc_binary::serialize_datasets_to_hsc;
use soryntel_db::sdb::{self, HscSources, SDB_FORMAT, SDB_VERSION};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::species::{DbSource, SpeciesDataset};
use std::path::PathBuf;

const CREATED: &str = "2026-01-01T00:00:00Z";
const NOTICE: &str = "Datos sintéticos de prueba.";

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("soryntel-db-sdb-{}-{tag}", std::process::id()));
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

/// Juegos de datos de OpenDefault más un juego alternativo de `Mg` (índice 1) con un valor
/// `-0.0` y referencias que se resuelven contra el `.REF` sintético.
fn synthetic_datasets() -> Vec<SpeciesDataset> {
    let open = ThermoDatabase::with_open_defaults();
    let mut out = Vec::new();
    for entry in open.iter_species(Some(DbSource::OpenDefault)) {
        let mut datasets = open.get_all_datasets_for(DbSource::OpenDefault, &entry.formula);
        for ds in &mut datasets {
            ds.metadata.cas_number = "7439-95-4".into();
            for interval in &mut ds.species.intervals {
                interval.reference = "Barin 89, SGTE".into();
            }
        }
        if entry.formula == "Mg" {
            let mut alt = datasets[0].clone();
            alt.species.intervals.truncate(1);
            alt.species.intervals[0].h_or_delta_h_tr = -0.0;
            alt.species.intervals[0].reference = "Knacke 91".into();
            datasets.push(alt);
        }
        out.extend(datasets);
    }
    out
}

fn ref_bytes(entries: &[(&str, &str, u8)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (key, citation, flag) in entries {
        let mut rec = vec![b' '; 293];
        rec[..key.len()].copy_from_slice(key.as_bytes());
        rec[36..36 + citation.len()].copy_from_slice(citation.as_bytes());
        rec[292] = *flag;
        out.extend(rec);
    }
    out
}

struct Fixture {
    dir: TempDir,
    main: PathBuf,
    reference: PathBuf,
    catalogue: PathBuf,
}

fn fixture(tag: &str) -> Fixture {
    let dir = TempDir::new(tag);
    let main = dir.path("Synth.HSC");
    let reference = dir.path("Synth.REF");
    let catalogue = dir.path("Synth.cat");
    std::fs::write(&main, serialize_datasets_to_hsc(&synthetic_datasets())).unwrap();
    std::fs::write(
        &reference,
        ref_bytes(&[
            ("Barin 89", "I. Barin, Thermochemical Data of Pure Substances", 1),
            ("SGTE 94", "SGTE Pure Substance Database", 0),
            ("Knacke 91", "O. Knacke et al.", 1),
        ]),
    )
    .unwrap();
    std::fs::write(&catalogue, b" 1\r\n").unwrap();
    Fixture { dir, main, reference, catalogue }
}

impl Fixture {
    fn sources(&self) -> HscSources<'_> {
        HscSources {
            main: &self.main,
            references: Some(&self.reference),
            catalogue: Some(&self.catalogue),
        }
    }

    fn convert(&self, file: &str) -> PathBuf {
        let out = self.dir.path(file);
        let doc = sdb::convert_hsc6(&self.sources(), NOTICE, CREATED).unwrap();
        sdb::write_sdb(&out, &doc).unwrap();
        out
    }
}

fn bits(ds: &SpeciesDataset) -> Vec<u64> {
    let sp = &ds.species;
    let m = &ds.metadata;
    let mut v = vec![
        sp.molecular_weight.to_bits(),
        sp.density_g_cm3.to_bits(),
        m.melting_point_k.to_bits(),
        m.boiling_point_k.to_bits(),
    ];
    for i in &sp.intervals {
        v.extend([
            i.t_min, i.t_max, i.h_or_delta_h_tr, i.s_or_delta_s_tr, i.kelley.a, i.kelley.b,
            i.kelley.c, i.kelley.d,
        ]
        .map(f64::to_bits));
    }
    v
}

#[test]
fn conversion_preserves_every_dataset_bit_for_bit() {
    let fx = fixture("fidelity");
    let sdb_path = fx.convert("soryntel.sdb");

    let mut direct = ThermoDatabase::default();
    direct.load_main_hsc(&fx.main).unwrap();
    let mut via_sdb = ThermoDatabase::default();
    let (species, intervals) = via_sdb.load_soryntel_db(&sdb_path).unwrap();

    let status = direct.status_summary();
    assert_eq!(species, status.main_unique_species);
    assert_eq!(intervals, status.main_binary_records);
    let formulas: Vec<String> = direct.iter_species(Some(DbSource::Main)).map(|e| e.formula).collect();
    let sdb_formulas: Vec<String> =
        via_sdb.iter_species(Some(DbSource::Soryntel)).map(|e| e.formula).collect();
    assert_eq!(formulas, sdb_formulas, "mismo orden de especies");

    let mut datasets = 0;
    for formula in &formulas {
        let a = direct.get_all_datasets_for(DbSource::Main, formula);
        let b = via_sdb.get_all_datasets_for(DbSource::Soryntel, formula);
        assert_eq!(a.len(), b.len(), "{formula}");
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(y.source, DbSource::Soryntel);
            assert_eq!(x.dataset_index, y.dataset_index, "{formula}");
            assert_eq!(x.raw_phase_code, y.raw_phase_code, "{formula}");
            assert_eq!(x.metadata, y.metadata, "{formula}");
            assert_eq!(x.species.name, y.species.name);
            assert_eq!(x.species.phase, y.species.phase);
            assert_eq!(x.species.charge, y.species.charge);
            assert_eq!(x.species.intervals.len(), y.species.intervals.len());
            for (i, j) in x.species.intervals.iter().zip(&y.species.intervals) {
                assert_eq!((i.phase, &i.reference), (j.phase, &j.reference));
            }
            assert_eq!(bits(x), bits(y), "{formula} dataset {}", x.dataset_index);
        }
        datasets += a.len();
    }
    let mg = via_sdb.get_all_datasets_for(DbSource::Soryntel, "Mg");
    assert_eq!(mg.len(), 2, "se conserva el juego alternativo");
    assert_eq!(mg[1].species.intervals[0].h_or_delta_h_tr.to_bits(), (-0.0f64).to_bits());

    let doc = sdb::read_sdb(&sdb_path).unwrap();
    assert_eq!(doc.dataset_count, datasets);
    assert_eq!(doc.format, SDB_FORMAT);
    assert_eq!(doc.version, SDB_VERSION);
    assert_eq!(doc.notice, NOTICE);
    assert_eq!(doc.created, CREATED);
    let roles: Vec<&str> = doc.source.files.iter().map(|f| f.role.as_str()).collect();
    assert_eq!(roles, ["main", "references", "catalogue"]);
    let main_bytes = std::fs::read(&fx.main).unwrap();
    assert_eq!(doc.source.files[0].name, "Synth.HSC", "solo el nombre, sin la ruta local");
    assert_eq!(doc.source.files[0].size, main_bytes.len() as u64);
    assert_eq!(doc.source.files[0].sha256, sdb::sha256_hex(&main_bytes));
    let mg_refs = &doc.species.iter().find(|s| s.formula == "Mg").unwrap().references;
    assert_eq!(mg_refs, &["Barin 89", "SGTE 94", "Knacke 91"], "clave exacta y prefijo truncado");
    assert_eq!(doc.references.len(), 3);
}

#[test]
fn conversion_is_deterministic() {
    let fx = fixture("determinism");
    let a = std::fs::read(fx.convert("a.sdb")).unwrap();
    let b = std::fs::read(fx.convert("b.sdb")).unwrap();
    assert_eq!(a, b);
    assert!(a.ends_with(b"}\n"));
    let other = sdb::convert_hsc6(&fx.sources(), NOTICE, "2030-01-01T00:00:00Z").unwrap();
    let c = sdb::to_bytes(&other).unwrap();
    let differing: Vec<(usize, &[u8])> = a
        .split(|&b| b == b'\n')
        .zip(c.split(|&b| b == b'\n'))
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, (x, _))| (i, x))
        .collect();
    assert_eq!(differing.len(), 1, "solo cambia la línea de created");
    assert!(std::str::from_utf8(differing[0].1).unwrap().contains("\"created\""));
}

fn expect_error(bytes: &[u8]) -> CatalogError {
    sdb::parse_sdb(bytes).expect_err("debe rechazarse")
}

#[test]
fn header_and_body_are_validated() {
    let fx = fixture("validation");
    let good = std::fs::read(fx.convert("good.sdb")).unwrap();
    let text = String::from_utf8(good.clone()).unwrap();
    assert!(sdb::parse_sdb(&good).is_ok());

    let wrong_format = text.replacen("\"soryntel-db\"", "\"otra-cosa\"", 1);
    assert!(matches!(expect_error(wrong_format.as_bytes()), CatalogError::Backend(m) if m.contains("not a SoryntelDB")));

    let future = text.replacen("\"version\": 1,", "\"version\": 2,", 1);
    assert!(matches!(expect_error(future.as_bytes()), CatalogError::Unsupported(m) if m.contains("version 2")));

    // Una versión futura con otro esquema se identifica por la cabecera, no por serde.
    let future_schema = r#"{"format":"soryntel-db","version":7,"compounds":[]}"#;
    assert!(matches!(expect_error(future_schema.as_bytes()), CatalogError::Unsupported(_)));
    assert!(matches!(expect_error(br#"{"species":[]}"#), CatalogError::Backend(m) if m.contains("not a SoryntelDB")));
    assert!(matches!(expect_error(b"\x00\x01binario"), CatalogError::Backend(_)));

    let count = text.replacen("\"species_count\": ", "\"species_count\": 9", 1);
    assert!(matches!(expect_error(count.as_bytes()), CatalogError::InvalidSpecies(m) if m.contains("species_count")));

    let mut doc = sdb::parse_sdb(&good).unwrap();
    doc.species[1].formula = doc.species[0].formula.clone();
    let dup = serde_json::to_vec(&doc).unwrap();
    assert!(matches!(expect_error(&dup), CatalogError::InvalidSpecies(m) if m.contains("duplicate species")));

    let mut doc = sdb::parse_sdb(&good).unwrap();
    doc.species[0].datasets[0].intervals[0].t_max = 0.0;
    assert!(matches!(expect_error(&serde_json::to_vec(&doc).unwrap()), CatalogError::InvalidSpecies(_)));

    let mut doc = sdb::parse_sdb(&good).unwrap();
    doc.species[0].datasets[0].molecular_weight = f64::NAN;
    assert!(matches!(sdb::to_bytes(&doc), Err(CatalogError::InvalidSpecies(m)) if m.contains("molecular_weight")));

    let missing = fx.dir.path("no-existe.sdb");
    let mut db = ThermoDatabase::with_open_defaults();
    let err = SpeciesDatabaseAdminPort::load_soryntel_db(&mut db, &missing).unwrap_err();
    assert!(matches!(&err, CatalogError::Backend(m) if m.contains("no-existe.sdb")), "{err:?}");
    assert!(db.get_species("Al2O3").is_some(), "un fallo de carga no toca el catálogo");
}

#[test]
fn layer_precedence_own_main_soryntel_open_default() {
    let fx = fixture("precedence");
    let sdb_path = fx.convert("soryntel.sdb");
    let open = ThermoDatabase::with_open_defaults();
    let al2o3 = open.get_species("Al2O3").unwrap();
    let h_open = al2o3.intervals[0].h_or_delta_h_tr;

    let write_layer = |file: &str, formula: &str, shift: f64, source: DbSource| {
        let mut ds = open.get_all_datasets_for(DbSource::OpenDefault, "Al2O3").remove(0);
        ds.source = source;
        ds.species.formula = formula.into();
        ds.species.intervals[0].h_or_delta_h_tr += shift;
        let path = fx.dir.path(file);
        std::fs::write(&path, serialize_datasets_to_hsc(&[ds])).unwrap();
        path
    };
    // El .sdb sintético contiene Al2O3 con el valor de OpenDefault (redondeado a f32). MainDB
    // y OwnDB lo desplazan para distinguir la capa.
    let main = write_layer("Main.HSC", "Al2O3", -10_000.0, DbSource::Main);
    let own = write_layer("Own.HSC", "Al2O3", -20_000.0, DbSource::Own);

    let mut db = ThermoDatabase::with_open_defaults();
    assert_eq!(db.resolved_source_of("Al2O3"), Some(DbSource::OpenDefault));
    let admin: &mut dyn SpeciesDatabaseAdminPort = &mut db;
    let (count, _) = admin.load_soryntel_db(&sdb_path).unwrap();
    assert_eq!(count, open.len());
    assert_eq!(db.resolved_source_of("Al2O3"), Some(DbSource::Soryntel));
    assert!((db.get_species("Al2O3").unwrap().intervals[0].h_or_delta_h_tr - h_open).abs() < 1.0);
    assert!(
        db.iter_species(None).all(|e| e.source == DbSource::Soryntel),
        "con SoryntelDB no se usa OpenDefault"
    );
    let status = db.status();
    assert_eq!(status.soryntel_path.as_deref(), Some(sdb_path.as_path()));
    assert_eq!(status.soryntel_unique_species, count);
    assert_eq!(status.references_count, 3, "referencias del .sdb sin .REF cargado");
    assert_eq!(db.citations_for("Knacke 91").len(), 1);

    // MainDB por encima de SoryntelDB; sin `.REF` compañero se conservan las del .sdb.
    db.load_main_hsc(&main).unwrap();
    assert_eq!(db.resolved_source_of("Al2O3"), Some(DbSource::Main));
    assert_eq!(db.resolved_source_of("Mg"), Some(DbSource::Soryntel));
    assert!((db.get_species("Al2O3").unwrap().intervals[0].h_or_delta_h_tr - h_open + 10_000.0).abs() < 1.0);
    assert_eq!(db.status().references_count, 3);

    db.load_own_hsc(&own).unwrap();
    assert_eq!(db.resolved_source_of("Al2O3"), Some(DbSource::Own));
    assert!((db.get_species("Al2O3").unwrap().intervals[0].h_or_delta_h_tr - h_open + 20_000.0).abs() < 1.0);

    // Juego alternativo seleccionable en la capa SoryntelDB.
    assert!(db.select_dataset(DbSource::Soryntel, "Mg", 1));
    assert_eq!(db.get_species("Mg").unwrap().intervals.len(), 1);
    assert!(!db.select_dataset(DbSource::Soryntel, "Mg", 5));
    assert!(db.refresh_catalogue(DbSource::Soryntel).is_err(), "SoryntelDB no tiene .CAT");

    // Un .REF cargado explícitamente no se sustituye al recargar SoryntelDB.
    db.load_ref_dictionary(&fx.reference).unwrap();
    db.load_soryntel_db(&sdb_path).unwrap();
    assert_eq!(db.status().ref_path.as_deref(), Some(fx.reference.as_path()));
}

#[test]
fn soryntel_layer_order_follows_the_file_and_can_be_sorted() {
    let fx = fixture("order");
    let sdb_path = fx.convert("soryntel.sdb");
    let mut db = ThermoDatabase::default();
    db.load_soryntel_db(&sdb_path).unwrap();
    let doc = sdb::read_sdb(&sdb_path).unwrap();
    let listed: Vec<String> = db.iter_species(Some(DbSource::Soryntel)).map(|e| e.formula).collect();
    let in_file: Vec<String> = doc.species.iter().map(|s| s.formula.clone()).collect();
    assert_eq!(listed, in_file);
    let mg = db
        .iter_species(Some(DbSource::Soryntel))
        .find(|e| e.formula == "Mg")
        .unwrap();
    assert_eq!(mg.dataset_count, 2);
    db.sort_layer(DbSource::Soryntel);
    assert_eq!(db.entry_count(Some(DbSource::Soryntel)), in_file.len());
}
