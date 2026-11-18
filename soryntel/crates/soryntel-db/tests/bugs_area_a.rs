//! Reproducciones de los hallazgos del área A (ver `docs/bugs/area-a.md`).
//!
//! Ejecutar con `cargo test -p soryntel-db --test bugs_area_a`.

use soryntel_core::{
    ChemicalSpecies, DbSource, KeywordSearchField, MergeConflictDecision, Phase,
    PhaseFilterFlags, SpeciesCatalogPort, SpeciesDataset, SpeciesMetadata, SpeciesRepository,
};
use soryntel_db::hsc_binary::{
    export_datasets_csv, group_records_to_datasets, import_datasets_csv,
    parse_hsc_binary_records, serialize_datasets_to_hsc, write_catalogue_file,
};
use soryntel_db::ThermoDatabase;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("soryntel-db-bugs-a-{}-{tag}", std::process::id()));
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

fn base_species() -> ChemicalSpecies {
    ThermoDatabase::with_open_defaults().get_species("Al2O3").unwrap()
}

/// Especie de un solo intervalo `[t_min, t_max]`.
fn one_interval(formula: &str, name: &str, t_min: f64, t_max: f64) -> ChemicalSpecies {
    let mut sp = base_species();
    sp.formula = formula.into();
    sp.name = name.into();
    sp.intervals.truncate(1);
    sp.intervals[0].t_min = t_min;
    sp.intervals[0].t_max = t_max;
    sp
}

fn meta() -> SpeciesMetadata {
    SpeciesMetadata { solubility: "-".into(), reliability_class: 1, ..Default::default() }
}

fn ds(sp: ChemicalSpecies, idx: usize) -> SpeciesDataset {
    SpeciesDataset {
        source: DbSource::Own,
        species: sp,
        metadata: meta(),
        dataset_index: idx,
        raw_phase_code: "s".into(),
    }
}

/// `X` con dos conjuntos de datos alternativos no contiguos (como `Mg(+2a)` set 0 / set 1).
fn two_alt_datasets(formula: &str) -> Vec<SpeciesDataset> {
    vec![
        ds(one_interval(formula, "set 0", 298.15, 1000.0), 0),
        ds(one_interval(formula, "set 1", 298.15, 800.0), 1),
    ]
}

fn roundtrip(datasets: &[SpeciesDataset]) -> Vec<SpeciesDataset> {
    let bytes = serialize_datasets_to_hsc(datasets);
    let (recs, _) = parse_hsc_binary_records(&bytes).unwrap();
    let (order, map) = group_records_to_datasets(recs, DbSource::Own);
    order.iter().filter_map(|f| map.get(f).cloned()).flatten().collect()
}

fn write(path: &Path, datasets: &[SpeciesDataset]) {
    std::fs::write(path, serialize_datasets_to_hsc(datasets)).unwrap();
}

// ---------------------------------------------------------------------------------------------
// A01: la fusión de HSC colapsa los conjuntos alternativos de una misma fórmula.
#[test]
fn a01_merge_conserva_datasets_alternativos_del_destino() {
    let tmp = TempDir::new("a01");
    let src = tmp.path("src.HSC");
    let dst = tmp.path("dst.HSC");
    write(&src, &[ds(one_interval("ZrO2", "otra", 298.15, 1000.0), 0)]);
    write(&dst, &two_alt_datasets("Al2O3"));
    ThermoDatabase::merge_hsc_files(&src, &dst, &BTreeMap::new(), false).unwrap();
    let mut db = ThermoDatabase::default();
    db.load_own_hsc(&dst).unwrap();
    assert_eq!(
        db.get_all_datasets_for(DbSource::Own, "Al2O3").len(),
        2,
        "el destino tenía dos datasets de Al2O3 y la fusión sin conflicto debe conservarlos"
    );
}

// A02: editar (upsert) una especie de OwnDB borra sus datasets alternativos.
#[test]
fn a02_upsert_own_conserva_datasets_alternativos() {
    let tmp = TempDir::new("a02");
    let own = tmp.path("own.HSC");
    write(&own, &two_alt_datasets("Al2O3"));
    let mut db = ThermoDatabase::with_open_defaults();
    db.load_own_hsc(&own).unwrap();
    assert_eq!(db.get_all_datasets_for(DbSource::Own, "Al2O3").len(), 2);
    let mut edited = db.get_species("Al2O3").unwrap();
    edited.name = "editado".into();
    db.insert_species(edited);
    assert_eq!(
        db.get_all_datasets_for(DbSource::Own, "Al2O3").len(),
        2,
        "la doc dice 'replaces primary OwnDB dataset'; el set 1 no debe perderse"
    );
}

// A03: importar CSV con dos datasets de la misma fórmula conserva solo el último.
#[test]
fn a03_import_csv_conserva_todos_los_datasets() {
    let csv = export_datasets_csv(&two_alt_datasets("Al2O3"));
    let mut db = ThermoDatabase::default();
    let n = db.import_csv_into_own(&csv).unwrap();
    assert_eq!(n, 2);
    assert_eq!(db.get_all_datasets_for(DbSource::Own, "Al2O3").len(), n);
}

// A04: un nombre con salto de línea se exporta entrecomillado pero el import rompe la fila.
#[test]
fn a04_csv_ida_y_vuelta_con_salto_de_linea_en_campo() {
    let mut sp = one_interval("Al2O3", "Corindón\nα-alúmina", 298.15, 1000.0);
    sp.intervals[0].reference = "Barin 93, \"JANAF\"".into();
    let csv = export_datasets_csv(&[ds(sp, 0)]);
    let back = import_datasets_csv(&csv, DbSource::Own).expect("el CSV exportado debe importarse");
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].species.name, "Corindón\nα-alúmina");
}

// A05: CSV con BOM UTF-8 (Excel) o con línea en blanco antes de la cabecera crea una especie "Formula".
#[test]
fn a05_csv_con_bom_o_linea_en_blanco_no_importa_la_cabecera() {
    let csv = export_datasets_csv(&[ds(one_interval("Al2O3", "x", 298.15, 1000.0), 0)]);
    for variant in [format!("\u{feff}{csv}"), format!("\n{csv}")] {
        let back = import_datasets_csv(&variant, DbSource::Own).unwrap();
        let formulas: Vec<&str> = back.iter().map(|d| d.species.formula.as_str()).collect();
        assert_eq!(formulas, vec!["Al2O3"], "importado: {formulas:?}");
    }
}

// A06: valores numéricos ilegibles se sustituyen en silencio por valores por defecto.
#[test]
fn a06_csv_con_numeros_invalidos_devuelve_error() {
    // T1/T2 y H ilegibles (coma decimal europea entrecomillada): hoy se importan 298.15..1000 K y H=0.
    let line = "Al2O3,,,,,101.96,0,0,\"298,15\",\"2327,0\",s,\"-400,5\",12.177,2.337,70.441,-0.594,-47.365";
    let r = import_datasets_csv(line, DbSource::Own);
    assert!(r.is_err(), "se importó con valores inventados: {:?}", r.map(|v| v[0].species.intervals.clone()));
}

// A07: separador ';' no se detecta si un campo entrecomillado contiene ','.
#[test]
fn a07_csv_punto_y_coma_con_coma_en_referencia() {
    let line = "Al2O3;;Aluminum oxide;;;101.96;0;0;298.15;800;s;-400.5;12.177;2.337;70.441;-0.594;-47.365;3.97;0;-;\"Barin 77, JANAF 98\";1";
    let back = import_datasets_csv(line, DbSource::Own).expect("fila ';' válida");
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].species.intervals[0].reference, "Barin 77, JANAF 98");
}

// A08: convert_legacy_to_hsc6 trata como binario cualquier CSV cuyo tamaño sea múltiplo de 256.
#[test]
fn a08_convertir_csv_de_tamano_multiplo_de_256() {
    let tmp = TempDir::new("a08");
    let mut csv = export_datasets_csv(&[ds(one_interval("Al2O3", "x", 298.15, 1000.0), 0)]);
    while csv.len() % 256 != 0 {
        csv.push('\n');
    }
    let src = tmp.path("legacy.csv");
    std::fs::write(&src, &csv).unwrap();
    let out = tmp.path("out.HSC");
    let n = ThermoDatabase::convert_legacy_to_hsc6(&src, &out);
    let mut db = ThermoDatabase::default();
    let formulas: Vec<String> = match n {
        Ok(_) => {
            db.load_own_hsc(&out).unwrap();
            db.get_all_datasets_for(DbSource::Own, "Al2O3").into_iter().map(|d| d.species.formula).collect()
        }
        Err(e) => panic!("error al convertir: {e}"),
    };
    assert_eq!(formulas, vec!["Al2O3".to_string()], "resultado: {:?}", db.status_summary());
}

// A09: 'ÿ' (U+00FF) se codifica como 0xFF, que el decodificador toma como terminador.
#[test]
fn a09_ida_y_vuelta_con_y_dieresis() {
    let back = roundtrip(&[ds(one_interval("Al2O3", "Aÿb óxido", 298.15, 1000.0), 0)]);
    assert_eq!(back[0].species.name, "Aÿb óxido");
}

// A10: la solubilidad con caracteres no ASCII se pierde ("-") al guardar y releer.
#[test]
fn a10_ida_y_vuelta_de_solubilidad_no_ascii() {
    let mut d = ds(one_interval("Al2O3", "x", 298.15, 1000.0), 0);
    d.metadata.solubility = "0,1 g/L 20°C".into();
    let back = roundtrip(&[d]);
    assert_eq!(back[0].metadata.solubility, "0,1 g/L 20°C");
}

// A11: fórmulas > 24 bytes se truncan en silencio y dos especies distintas se fusionan.
#[test]
fn a11_formulas_largas_no_colisionan() {
    let f1 = "C12H22O11*Ca(NO3)2*4H2O(a)";
    let f2 = "C12H22O11*Ca(NO3)2*4H2O(g)";
    assert!(f1.len() > 24);
    let tmp = TempDir::new("a11");
    let path = tmp.path("own.HSC");
    let mut db = ThermoDatabase::default();
    db.create_empty_own_hsc(&path).unwrap();
    let before = std::fs::read(&path).unwrap();
    db.insert_species(one_interval(f1, "uno", 298.15, 1000.0));
    db.insert_species(one_interval(f2, "dos", 298.15, 1000.0));
    let error = db.save_own_hsc(&path).unwrap_err().to_string();
    assert!(error.contains(f1) && error.contains(f2), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

// A12: una especie sin intervalos desaparece al guardar OwnDB (y save informa que se guardó).
#[test]
fn a12_guardar_especie_sin_intervalos() {
    let tmp = TempDir::new("a12");
    let path = tmp.path("own.HSC");
    let mut db = ThermoDatabase::default();
    let mut sp = one_interval("ZrO2", "sin datos aún", 298.15, 1000.0);
    sp.intervals.clear();
    db.insert_species(sp);
    let error = db.save_own_hsc(&path).unwrap_err().to_string();
    assert!(error.contains("ZrO2") && error.contains("interval"), "{error}");
    assert!(!path.exists());
}

// A13: el .CAT cuenta 1 registro por dataset sin intervalos, pero el .HSC no escribe ninguno.
#[test]
fn a13_cat_offsets_con_dataset_vacio() {
    let tmp = TempDir::new("a13");
    let hsc = tmp.path("x.HSC");
    let mut empty = one_interval("Al2O3", "vacío", 298.15, 1000.0);
    empty.intervals.clear();
    let datasets = vec![ds(empty, 0), ds(one_interval("ZnO", "z", 298.15, 1000.0), 0)];
    let cat = write_catalogue_file(&hsc, &datasets).unwrap();
    let body = std::fs::read_to_string(cat).unwrap();
    // La cubeta "Zn" debe apuntar al registro 1 (el único escrito); hoy apunta al 2.
    let max_offset = body.split_whitespace().filter_map(|s| s.parse::<usize>().ok()).max().unwrap();
    let real_records = serialize_datasets_to_hsc(&datasets).len() / 256;
    assert!(max_offset <= real_records, "offset {max_offset} > {real_records} registros reales");
}

// A14: el parser acepta registros con T1/T2 = NaN.
#[test]
fn a14_registro_con_temperaturas_nan_se_descarta() {
    let mut bytes = serialize_datasets_to_hsc(&[ds(one_interval("Al2O3", "x", 298.15, 1000.0), 0)]);
    bytes[0xA4..0xA8].copy_from_slice(&f32::NAN.to_le_bytes());
    let (recs, _) = parse_hsc_binary_records(&bytes).unwrap();
    assert!(recs.is_empty(), "registro con T1=NaN aceptado: t1={} t2={}", recs[0].t1, recs[0].t2);
}

// A15: el orden de la vista resuelta (get_species_at, export CSV/JSON) depende del HashMap.
#[test]
fn a15_orden_de_open_defaults_es_determinista() {
    let order = |db: &ThermoDatabase| -> Vec<String> {
        (0..db.species_count()).map(|i| db.get_species_at(i).unwrap().formula).collect()
    };
    let first = order(&ThermoDatabase::with_open_defaults());
    for _ in 0..32 {
        assert_eq!(order(&ThermoDatabase::with_open_defaults()), first);
    }
}

fn ref_record(key: &str, citation: &str) -> Vec<u8> {
    let mut rec = vec![b' '; 293];
    rec[..key.len()].copy_from_slice(key.as_bytes());
    rec[36..36 + citation.len()].copy_from_slice(citation.as_bytes());
    rec[292] = 1;
    rec
}

// A16: la búsqueda parcial de citas elige una entrada al azar (iteración de HashMap).
#[test]
fn a16_lookup_citations_por_prefijo_es_determinista() {
    let tmp = TempDir::new("a16");
    let path = tmp.path("MainDB6.REF");
    let mut bytes = Vec::new();
    for (k, c) in [("Barin 77", "Barin 1977"), ("Barin 89", "Barin 1989"), ("Barin 93", "Barin 1993")] {
        bytes.extend(ref_record(k, c));
    }
    std::fs::write(&path, bytes).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..32 {
        let mut db = ThermoDatabase::default();
        db.load_ref_dictionary(&path).unwrap();
        let keys: Vec<String> = db.lookup_citations("Barin").into_iter().map(|e| e.key).collect();
        seen.insert(keys);
    }
    assert_eq!(seen.len(), 1, "resultados distintos para la misma consulta: {seen:?}");
}

// A17: cargar otro MainDB sin .REF mantiene las referencias del anterior.
#[test]
fn a17_recargar_main_sin_ref_limpia_referencias() {
    let tmp = TempDir::new("a17");
    let d1 = tmp.path("one");
    let d2 = tmp.path("two");
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::create_dir_all(&d2).unwrap();
    write(&d1.join("Maindb6.HSC"), &[ds(one_interval("Al2O3", "x", 298.15, 1000.0), 0)]);
    std::fs::write(d1.join("Maindb6.REF"), ref_record("Barin 77", "Barin 1977")).unwrap();
    write(&d2.join("Maindb6.HSC"), &[ds(one_interval("ZrO2", "z", 298.15, 1000.0), 0)]);
    let mut db = ThermoDatabase::default();
    db.load_main_hsc(d1.join("Maindb6.HSC")).unwrap();
    assert_eq!(db.status_summary().references_count, 1);
    db.load_main_hsc(d2.join("Maindb6.HSC")).unwrap();
    let s = db.status_summary();
    assert!(
        s.references_count == 0 && s.ref_path.is_none(),
        "siguen activas {} referencias de {:?}",
        s.references_count,
        s.ref_path
    );
}

// A18: la selección de dataset activo sobrevive a recargar otro archivo.
#[test]
fn a18_recargar_own_reinicia_dataset_activo() {
    let tmp = TempDir::new("a18");
    let p = tmp.path("own.HSC");
    write(&p, &two_alt_datasets("Al2O3"));
    let mut db = ThermoDatabase::default();
    db.load_own_hsc(&p).unwrap();
    assert!(db.select_dataset(DbSource::Own, "Al2O3", 1));
    db.load_own_hsc(&p).unwrap();
    assert_eq!(db.get_species("Al2O3").unwrap().name, "set 0", "un archivo recién cargado debe activar el set 0");
}

// A19: errores al escribir el .CAT se ignoran (`let _ =`) y save informa éxito.
#[test]
fn a19_error_al_escribir_cat_se_propaga() {
    let tmp = TempDir::new("a19");
    let hsc = tmp.path("own.HSC");
    std::fs::create_dir_all(tmp.path("own.CAT")).unwrap(); // el .CAT no se puede escribir
    let mut db = ThermoDatabase::default();
    db.insert_species(one_interval("ZrO2", "z", 298.15, 1000.0));
    let r = db.save_own_hsc(&hsc);
    assert!(r.is_err(), "save_own_hsc devolvió Ok aunque el .CAT no se escribió");
}

// A20: un .HSC truncado (longitud no múltiplo de 256) se acepta sin aviso.
#[test]
fn a20_hsc_truncado_se_rechaza_o_avisa() {
    let mut bytes = serialize_datasets_to_hsc(&two_alt_datasets("Al2O3"));
    bytes.truncate(bytes.len() - 100);
    assert!(parse_hsc_binary_records(&bytes).is_err(), "archivo truncado aceptado como válido");
}

// A21: la carga de un ion sin sufijo HSC pierde carga y fase en la ida y vuelta.
#[test]
fn a21_ida_y_vuelta_de_ion_sin_sufijo_hsc() {
    let mut sp = one_interval("NH4+", "amonio", 298.15, 373.15);
    sp.phase = Phase::AqueousIon;
    sp.intervals[0].phase = Phase::AqueousIon;
    sp.charge = 1;
    let mut d = ds(sp, 0);
    d.raw_phase_code = "a".into();
    let back = roundtrip(&[d]);
    assert_eq!((back[0].species.phase, back[0].species.charge), (Phase::AqueousIon, 1));
}

// A22: coeficientes finitos en f64 pero fuera de rango f32 se escriben como ±inf.
#[test]
fn a22_valores_fuera_de_rango_f32() {
    let mut sp = one_interval("Al2O3", "x", 298.15, 1000.0);
    sp.intervals[0].kelley.d = 1e40;
    let tmp = TempDir::new("a22");
    let path = tmp.path("own.HSC");
    let mut db = ThermoDatabase::default();
    db.insert_species(sp);
    let error = db.save_own_hsc(&path).unwrap_err().to_string();
    assert!(error.contains("numeric"), "{error}");
    assert!(!path.exists());
}

// A23: la búsqueda por nombre no ignora mayúsculas fuera de ASCII.
#[test]
fn a23_busqueda_insensible_a_mayusculas_unicode() {
    let mut db = ThermoDatabase::default();
    db.insert_species(one_interval("Al2O3", "Óxido de aluminio", 298.15, 1000.0));
    let hits = db.search_advanced(
        "óxido",
        KeywordSearchField::ChemicalName,
        None,
        &PhaseFilterFlags::default(),
        10,
    );
    assert_eq!(hits.len(), 1);
}

// A24: decisiones de conflicto: con dos datasets en origen, el segundo pisa al primero.
#[test]
fn a24_merge_conserva_datasets_alternativos_del_origen() {
    let tmp = TempDir::new("a24");
    let src = tmp.path("src.HSC");
    let dst = tmp.path("dst.HSC");
    write(&src, &two_alt_datasets("Al2O3"));
    let mut decisions = BTreeMap::new();
    decisions.insert("Al2O3".to_string(), MergeConflictDecision::TakeSource);
    let (n, _) = ThermoDatabase::merge_hsc_files(&src, &dst, &decisions, false).unwrap();
    assert_eq!(n, 2, "dos datasets de origen, destino vacío: deben quedar los dos");
}

#[test]
fn a25_escritura_atomica_limpia_temporal_si_falla_rename() {
    let tmp = TempDir::new("a25");
    let path = tmp.path("own.HSC");
    std::fs::create_dir(&path).unwrap();
    let marker = path.join("previous");
    std::fs::write(&marker, b"original").unwrap();
    let mut db = ThermoDatabase::default();
    assert!(db.create_empty_own_hsc(&path).is_err());
    assert_eq!(std::fs::read(marker).unwrap(), b"original");
    assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn a03_csv_conserva_limites_de_datasets_contiguos() {
    let input = vec![
        ds(one_interval("Al2O3", "primario", 298.15, 500.0), 0),
        ds(one_interval("Al2O3", "alternativo", 500.0, 900.0), 1),
    ];
    let mut db = ThermoDatabase::default();
    assert_eq!(db.import_csv_into_own(&export_datasets_csv(&input)).unwrap(), 2);
    let datasets = db.get_all_datasets_for(DbSource::Own, "Al2O3");
    assert_eq!(datasets.len(), 2);
    assert_eq!(datasets[1].species.name, "alternativo");
}

#[test]
fn a27_ref_ilegible_no_reemplaza_main_activo() {
    let temp = TempDir::new("a27");
    let previous = temp.path("previous.HSC");
    let next = temp.path("next.HSC");
    write(&previous, &two_alt_datasets("Al2O3"));
    write(&next, &two_alt_datasets("ZnO"));
    std::fs::create_dir(next.with_extension("REF")).unwrap();
    let mut db = ThermoDatabase::default();
    db.load_main_hsc(&previous).unwrap();
    let error = db.load_main_hsc(&next).unwrap_err().to_string();
    assert!(error.contains("next.REF"), "{error}");
    assert_eq!(db.status_summary().main_path.as_deref(), Some(previous.as_path()));
}

#[test]
fn a29_rechaza_centinela_aun_con_temperaturas_validas() {
    let bytes = serialize_datasets_to_hsc(&[ds(one_interval("0", "", 298.15, 1000.0), 0)]);
    let (records, total) = parse_hsc_binary_records(&bytes).unwrap();
    assert_eq!(total, 1);
    assert!(records.is_empty());
}

#[test]
fn a39_csv_conserva_precision_para_temperatura_y_entalpia() {
    let mut species = one_interval("Al2O3", "x", 298.123456, 1000.0);
    species.intervals[0].h_or_delta_h_tr = -1_624_325.987_654;
    let expected = species.intervals[0].clone();
    let csv = export_datasets_csv(&[ds(species, 0)]);
    let datasets = import_datasets_csv(&csv, DbSource::Own).unwrap();
    let interval = &datasets[0].species.intervals[0];
    assert_eq!(interval.t_min, expected.t_min);
    assert!((interval.h_or_delta_h_tr - expected.h_or_delta_h_tr).abs() < 1e-8);
}

#[test]
fn a26_cat_distingue_carbono_de_calcio() {
    let temp = TempDir::new("a26");
    let cat = write_catalogue_file(&temp.path("own.HSC"), &[
        ds(one_interval("CaO", "", 298.15, 1000.0), 0),
        ds(one_interval("CO", "", 298.15, 1000.0), 0),
    ]).unwrap();
    let data = std::fs::read_to_string(cat).unwrap();
    let offsets: Vec<_> = data.split_whitespace().collect();
    assert_eq!((&offsets[2], &offsets[3]), (&"2", &"1"));
}

#[test]
fn a28_hsc_rechaza_limites_de_dataset_ambiguos() {
    let first = ds(one_interval("Al2O3", "primario", 298.15, 500.0), 0);
    let second = ds(one_interval("Al2O3", "alternativo", 500.0, 900.0), 1);
    let datasets = [first, second];
    assert_eq!(roundtrip(&datasets).len(), 1, "el lector HSC clásico fusiona estos intervalos");
    let error = soryntel_db::hsc_binary::serialize_datasets_checked(&datasets)
        .unwrap_err().to_string();
    assert!(error.contains("dataset boundaries"), "{error}");
}

#[test]
fn a01_merge_solo_reemplaza_el_indice_en_conflicto() {
    let temp = TempDir::new("merge-index");
    let source = temp.path("source.HSC");
    let target = temp.path("target.HSC");
    write(&target, &two_alt_datasets("Al2O3"));
    write(&source, &[ds(one_interval("Al2O3", "reemplazo", 298.15, 450.0), 0)]);
    let (count, conflicts) =
        ThermoDatabase::merge_hsc_files(&source, &target, &BTreeMap::new(), false).unwrap();
    assert_eq!(count, 2);
    assert_eq!(conflicts.len(), 1);
    let mut db = ThermoDatabase::default();
    db.load_own_hsc(target).unwrap();
    assert_eq!(db.get_all_datasets_for(DbSource::Own, "Al2O3")[0].species.name, "reemplazo");
}

#[test]
fn a02_editar_alternativo_conserva_el_primario() {
    let temp = TempDir::new("edit-index");
    let path = temp.path("own.HSC");
    write(&path, &two_alt_datasets("Al2O3"));
    let mut db = ThermoDatabase::default();
    db.load_own_hsc(path).unwrap();
    let original = db.get_all_datasets_for(DbSource::Own, "Al2O3");
    assert!(db.select_dataset(DbSource::Own, "Al2O3", 1));
    let mut replacement = original[1].species.clone();
    replacement.name = "alternativo editado".into();
    db.insert_species(replacement);
    let edited = db.get_all_datasets_for(DbSource::Own, "Al2O3");
    assert_eq!(edited[0].species.name, original[0].species.name);
    assert_eq!(edited[1].species.name, "alternativo editado");
}
