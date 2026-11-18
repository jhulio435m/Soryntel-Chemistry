//! Tests de reproducción del área B que necesitan el catálogo integrado
//! (OpenDefault) o los casos de uso. Hoy FALLAN; se ignoran para mantener CI
//! en verde (ver `docs/bugs/area-b.md`).

use soryntel_application::testing::InMemorySpeciesCatalog;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::aqua::{calculate_aqua, AquaCaseFile, AquaMethod, AquaSpeciesInput, OwnPitzerDb};
use soryntel_thermo::aqua_pitzer_data::builtin_main_pitzer_db;
use soryntel_thermo::excel_addins::{eval_formula_string, AddInUnits, CellEvalValue};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Consecuencia observable de B07: NaCl 1 mol/kg a 25 °C. Referencia:
/// Robinson & Stokes (1959) γ± = 0,657, φ = 0,936; Pitzer & Mayorga (1973)
/// con β0 = 0,0765, β1 = 0,2664, Cφ = 0,00127 da γ± = 0,656.
#[test]
#[ignore = "BUG-B07: Aqua da γ±(NaCl, 1 m, 25 °C) = 0,685 en lugar de 0,657 por A_phi bajo"]
fn b07_nacl_mean_activity_coefficient_one_molal() {
    let db = ThermoDatabase::with_open_defaults();
    let mut case = AquaCaseFile::preset_nacl_hcl();
    case.method = AquaMethod::Pitzer;
    case.species = vec![
        AquaSpeciesInput { formula: "H2O".into(), temp: 25.0, amount_kmol: 55.50844 },
        AquaSpeciesInput { formula: "Na(+a)".into(), temp: 25.0, amount_kmol: 1.0 },
        AquaSpeciesInput { formula: "Cl(-a)".into(), temp: 25.0, amount_kmol: 1.0 },
    ];
    let r = calculate_aqua(
        &case,
        &db,
        &builtin_main_pitzer_db(),
        &OwnPitzerDb::default(),
        &Default::default(),
    );
    let g_pm = (r.species[1].ac_molal * r.species[2].ac_molal).sqrt();
    assert!((g_pm - 0.657).abs() < 0.005, "γ± = {g_pm}, esperado 0,657");
    assert!(
        (r.summary.osmotic_coeff - 0.936).abs() < 0.003,
        "φ = {}, esperado 0,936",
        r.summary.osmotic_coeff
    );
}

/// `=Species(2, 1e20)` llega por el bridge de Add-ins: `pos as usize` satura a
/// `usize::MAX` y `pos + 8` desborda (pánico en debug).
#[test]
#[ignore = "BUG-B13: =Species(db, pos) desborda usize con pos enorme (pánico en debug)"]
fn b13_species_addin_does_not_overflow_on_huge_position() {
    let catalog = InMemorySpeciesCatalog::new();
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut units = AddInUnits::default();
        eval_formula_string(&catalog, &mut units, "=Species(2,1e20)", |_, _| String::new())
    }));
    match r {
        Ok(CellEvalValue::Error(_)) => {}
        Ok(other) => panic!("esperado un error de celda, obtenido {other:?}"),
        Err(_) => panic!("eval_formula_string entró en pánico"),
    }
}
