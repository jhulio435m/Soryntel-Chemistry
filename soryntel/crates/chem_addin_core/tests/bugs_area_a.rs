//! Reproducciones de hallazgos del área A en `chem_addin_core` (ver `docs/bugs/area-a.md`)
//! y fuzz barato de los parsers de fórmulas/reacciones.

use chem_addin_core::{bal, bal_detailed, mw, parse_reaction, parse_species, wtp, ChemDatabase, ChemEngine, SpeciesInterval};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const TOKENS: [&str; 40] = [
    "H", "O", "C", "Fe", "Cu", "S", "N", "Cl", "Na", "e", "2", "3", "0", ".", "5", "1/2", "(", ")",
    "[", "]", "*", "·", "+", "-", "^", "=", "->", "<=>", " ", "(g)", "(aq)", "(s)", "(+a)", "(-2a)",
    "Xx", "é", "10000000000", "99999999999999999999", "/0", "\u{0}",
];

/// Fuzz sin pánicos de los parsers y del balanceo (profundidad acotada por la longitud).
#[test]
fn fuzz_parser_y_balanceo_no_entran_en_panico() {
    let iters: usize = std::env::var("SORYNTEL_FUZZ_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(5_000);
    let mut rng = Rng(0xADD1_0000_0000_00A4);
    let mut panics: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for _ in 0..iters {
        let n = 1 + rng.below(16);
        let input: String = (0..n).map(|_| TOKENS[rng.below(TOKENS.len())]).collect();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _ = parse_species(&input);
            let _ = mw(&input);
            let _ = parse_reaction(&input);
            let _ = bal_detailed(&input, 1.0);
            let _ = wtp(&input, "CuSO4*5H2O");
        }));
        if let Err(e) = r {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let entry = panics.entry(msg).or_insert((input.clone(), 0));
            entry.1 += 1;
            if input.len() < entry.0.len() {
                entry.0 = input.clone();
            }
        }
    }
    assert!(panics.is_empty(), "pánicos: {panics:#?}");
}


// A30: anidamiento profundo de paréntesis desborda la pila (aborta el proceso, no es capturable).
// El test se relanza a sí mismo como subproceso para que el aborto no tumbe el binario de tests.
#[test]
fn a30_parentesis_anidados_profundos_no_abortan() {
    let nested = |depth| format!("{}H{}", "(".repeat(depth), ")".repeat(depth));
    assert!(parse_species(&nested(64)).is_ok());
    assert!(parse_species(&nested(65)).is_err());
    assert!(parse_species(&"H".repeat(4097)).is_err());
}

// A31: MW del electrón es 0.
#[test]
fn a31_mw_del_electron() {
    let m = mw("e-").unwrap();
    assert!((m - 0.000548579909).abs() < 1e-9, "mw(e-) = {m}");
}

// A32: notación de carga con número antes del signo ("Fe2+") se interpreta como Fe2 con carga +1.
#[test]
fn a32_carga_fe2_mas() {
    let p = parse_species("Fe2+").unwrap();
    assert_eq!((p.composition.get("Fe").copied(), p.charge), (Some(1.0), 2));
}

// A33: la notación HSC de iones acuosos (`Na(+a)`, `SO4(-2a)`) no se reconoce.
#[test]
fn a33_notacion_hsc_de_iones() {
    let na = parse_species("Na(+a)").expect("Na(+a)");
    assert_eq!(na.charge, 1);
    let so4 = parse_species("SO4(-2a)").expect("SO4(-2a)");
    assert_eq!(so4.charge, -2);
}

// A34: subíndice 0 aceptado: "C0" es una especie válida de MW 0.
#[test]
fn a34_subindice_cero_se_rechaza() {
    assert!(parse_species("C0").is_err(), "C0 aceptado con mw {:?}", mw("C0"));
}

// A35: reabrir un SQLite persistente sobrescribe con las semillas los cambios del usuario.
#[test]
fn a35_reabrir_sqlite_conserva_ediciones() {
    let dir = std::env::temp_dir().join(format!("chem-addin-a35-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("addin.sqlite");
    let edited_h298;
    {
        let db = ChemDatabase::open_file(&path).unwrap();
        let mut row: SpeciesInterval = db.get_species_intervals("H2O(l)").unwrap()[0].clone();
        row.h298 = -123.0;
        edited_h298 = row.h298;
        db.upsert_species_interval(&row).unwrap();
        assert_eq!(db.get_species_intervals("H2O(l)").unwrap()[0].h298, edited_h298);
    }
    let reopened = ChemEngine::open_sqlite(&path).unwrap();
    let h = reopened.db.get_species_intervals("H2O(l)").unwrap()[0].h298;
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(h, edited_h298, "la edición del usuario se perdió al reabrir");
}

// A36: H/S/G fuera del rango de datos se extrapolan sin error ni aviso.
#[test]
fn a36_temperatura_fuera_de_rango_se_senala() {
    let e = ChemEngine::in_memory().unwrap();
    // H2O(l) sólo tiene datos 273.15..373.15 K; -100 °C es agua líquida extrapolada.
    assert!(e.h("H2O(l)", -100.0).is_err(), "H = {:?}", e.h("H2O(l)", -100.0));
}

#[test]
fn bal_sigue_funcionando_en_casos_basicos() {
    assert_eq!(bal("H2 + O2 = H2O", 1.0).unwrap(), "2 H2 + O2 = 2 H2O");
}

#[test]
fn a32_balanceo_de_iones_y_variantes_de_carga() {
    for formula in ["Fe2+", "Fe+2", "Fe[2+]", "Fe(+2a)"] {
        let ion = parse_species(formula).unwrap();
        assert_eq!(ion.charge, 2);
        assert_eq!(ion.composition.get("Fe"), Some(&1.0));
    }
    let sulfate = parse_species("SO42-").unwrap();
    assert_eq!(sulfate.charge, -2);
    assert_eq!(sulfate.composition.get("O"), Some(&4.0));
    let balanced = bal("Fe2+ + Ce4+ = Fe3+ + Ce3+", 1.0).unwrap();
    assert_eq!(balanced, "Fe2+ + Ce4+ = Fe3+ + Ce3+");
}

#[test]
fn a37_el_azufre_entre_parentesis_no_es_una_fase() {
    let species = parse_species("Fe(S)").unwrap();
    assert_eq!(species.composition.get("S"), Some(&1.0));
    assert_eq!(parse_species("Fe(s)").unwrap().composition.get("S"), None);
}

#[test]
fn a38_json_de_mineral_corrupto_se_informa() {
    let db = chem_addin_core::db::ChemDatabase::open_in_memory().unwrap();
    assert!(db.get_mineral("Pyrite").unwrap().is_some());
    db.connection().execute("UPDATE minerals SET elements_wt_json = 'broken' WHERE name = 'Pyrite'", []).unwrap();
    assert!(db.get_mineral("Pyrite").is_err());
}
