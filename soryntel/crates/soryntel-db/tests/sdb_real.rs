//! Validación de SoryntelDB contra la MainDB real de HSC 6.
//!
//! Solo se ejecuta con `SORYNTEL_TEST_MAINDB=/ruta/a/Maindb6.HSC`; si no, se omite. El `.REF` y
//! el `.cat` se buscan junto a la MainDB. El `.sdb` se genera en un directorio temporal y se
//! borra al terminar: ningún dato fuente ni generado entra en el repositorio.

mod common;

use soryntel_core::{SpeciesBrowserPort, SpeciesRepository};
use soryntel_db::sdb::{self, HscSources};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::evaluate_species_si;
use soryntel_thermo::species::DbSource;
use std::path::{Path, PathBuf};

const NOTICE: &str = "Datos termoquímicos derivados de HSC Chemistry 6 bajo convenio con Soryntel.";
const CREATED: &str = "2026-01-01T00:00:00Z";
const TEMPERATURES: [f64; 3] = [298.15, 500.0, 1000.0];

fn companion(main: &Path, names: &[&str]) -> Option<PathBuf> {
    let dir = main.parent()?;
    let entries: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).collect();
    names.iter().find_map(|name| {
        entries
            .iter()
            .find(|p| p.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(name)))
            .cloned()
    })
}

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn soryntel_db_matches_direct_maindb_load() {
    let Some(main) = common::test_maindb("soryntel_db_matches_direct_maindb_load") else {
        return;
    };
    let reference = companion(&main, &["MainDB6.REF", "Maindb6.ref"]);
    let catalogue = companion(&main, &["Maindb6.cat"]);
    let tmp = TempDir(std::env::temp_dir().join(format!("soryntel-sdb-real-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&tmp.0);
    std::fs::create_dir_all(&tmp.0).unwrap();
    let sources = HscSources {
        main: &main,
        references: reference.as_deref(),
        catalogue: catalogue.as_deref(),
    };

    // Conversión determinista: dos conversiones dan el mismo archivo.
    let (a, b) = (tmp.0.join("a.sdb"), tmp.0.join("b.sdb"));
    for out in [&a, &b] {
        let doc = sdb::convert_hsc6(&sources, NOTICE, CREATED).unwrap();
        sdb::write_sdb(out, &doc).unwrap();
    }
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(bytes, std::fs::read(&b).unwrap(), "la conversión no es determinista");

    let mut direct = ThermoDatabase::default();
    direct.load_main_hsc(&main).unwrap();
    let started = std::time::Instant::now();
    let mut direct_timing = ThermoDatabase::default();
    direct_timing.load_main_hsc(&main).unwrap();
    let hsc_elapsed = started.elapsed();
    let started = std::time::Instant::now();
    let mut via_sdb = ThermoDatabase::default();
    let (species, intervals) = via_sdb.load_soryntel_db(&a).unwrap();
    eprintln!(
        "[sdb_real] carga: .sdb {:.0} ms, .HSC {:.0} ms",
        started.elapsed().as_secs_f64() * 1e3,
        hsc_elapsed.as_secs_f64() * 1e3
    );
    let status = direct.status_summary();
    assert_eq!(species, status.main_unique_species, "número de especies");
    let doc = sdb::read_sdb(&a).unwrap();
    assert_eq!(doc.species_count, species);

    let formulas: Vec<String> = direct.iter_species(Some(DbSource::Main)).map(|e| e.formula).collect();
    assert_eq!(formulas.len(), species);
    let (mut datasets, mut direct_intervals, mut points) = (0usize, 0usize, 0usize);
    for formula in &formulas {
        let x = direct.get_all_datasets_for(DbSource::Main, formula);
        let y = via_sdb.get_all_datasets_for(DbSource::Soryntel, formula);
        assert_eq!(x.len(), y.len(), "{formula}: número de juegos de datos");
        for (dx, dy) in x.iter().zip(&y) {
            assert_eq!(dx.dataset_index, dy.dataset_index, "{formula}");
            assert_eq!(dx.metadata, dy.metadata, "{formula}");
            assert_eq!(dx.raw_phase_code, dy.raw_phase_code, "{formula}");
            let (sx, sy) = (&dx.species, &dy.species);
            assert_eq!(sx.intervals.len(), sy.intervals.len(), "{formula}");
            direct_intervals += sx.intervals.len();
            let lo = sx.intervals.first().unwrap().t_min;
            let hi = sx.intervals.last().unwrap().t_max;
            for t in TEMPERATURES {
                let t = t.clamp(lo, hi);
                let px = evaluate_species_si(sx, t, false, false);
                let py = evaluate_species_si(sy, t, false, false);
                match (px, py) {
                    (Ok((px, _)), Ok((py, _))) => {
                        for (name, u, v) in [
                            ("H", px.h, py.h),
                            ("S", px.s, py.s),
                            ("Cp", px.cp, py.cp),
                            ("G", px.g, py.g),
                        ] {
                            assert_eq!(
                                u.to_bits(),
                                v.to_bits(),
                                "{formula} juego {} {name}({t} K): {u} != {v}",
                                dx.dataset_index
                            );
                        }
                        points += 1;
                    }
                    (Err(ex), Err(ey)) => assert_eq!(ex.to_string(), ey.to_string(), "{formula}"),
                    (rx, ry) => panic!("{formula} {t} K: {rx:?} vs {ry:?}"),
                }
            }
        }
        datasets += x.len();
    }
    assert_eq!(doc.dataset_count, datasets, "número de juegos de datos");
    assert_eq!(intervals, direct_intervals);
    eprintln!(
        "[sdb_real] {species} especies, {datasets} juegos, {intervals} intervalos, \
         {points} puntos idénticos, {} bytes",
        bytes.len()
    );

    // Valores de referencia NIST-JANAF (4.ª ed.) a 298,15 y 1000 K: H en kJ/mol (formación
    // a 298,15 K más H(T) − H(298,15)), S y Cp en J/(mol·K).
    let janaf = [
        ("H2O(g)", 298.15, -241.826, 188.835, 33.590),
        ("H2O(g)", 1000.0, -241.826 + 26.000, 232.738, 41.268),
        ("CO2(g)", 298.15, -393.522, 213.795, 37.135),
        ("CO2(g)", 1000.0, -393.522 + 33.397, 269.299, 54.308),
    ];
    for (formula, t, h_kj, s, cp) in janaf {
        let species = via_sdb
            .get_species(formula)
            .unwrap_or_else(|| panic!("{formula} no está en SoryntelDB"));
        let (p, _) = evaluate_species_si(&species, t, false, false).unwrap();
        eprintln!(
            "[sdb_real] {formula} {t} K: H {:.3} kJ/mol (JANAF {h_kj:.3}), S {:.3} ({s}), Cp {:.3} ({cp})",
            p.h / 1000.0,
            p.s,
            p.cp
        );
        assert!((p.h / 1000.0 - h_kj).abs() < 0.5, "{formula} H({t} K) = {}", p.h / 1000.0);
        assert!((p.s - s).abs() < 0.5, "{formula} S({t} K) = {}", p.s);
        assert!((p.cp - cp).abs() < 0.5, "{formula} Cp({t} K) = {}", p.cp);
    }
}
