//! Tests de reproducción del área B (descubrimiento de bugs).
//!
//! Cada test describe el comportamiento esperado y hoy FALLA. Se marcan con
//! `#[ignore = "BUG-Bxx: ..."]` para que CI siga en verde. Detalle en
//! `docs/bugs/area-b.md`. Para ejecutarlos: `cargo test -p soryntel-thermo
//! --test bugs_area_b -- --ignored`.

use soryntel_core::{
    BalanceEnergyUnit, CAL_TO_JOULE, EnergyUnit, HeatFlowUnit, ResultFormat, TemperatureUnit,
    R_JOULE,
};
use soryntel_thermo::balance_engine::{
    evaluate_workbook, preset_h2o, set_cell_value, solve_adiabatic_temperature, BalanceArea,
    BalanceWorkbook, DiagramConfig, SpeciesRow, StreamGroup,
};
use soryntel_thermo::equilibrium::{
    solve_single_point, EquilibriumAmountUnit, EquilibriumParams, EquilibriumPhase,
    EquilibriumSpeciesInput, EquilibriumSystem, SolutionModel,
};
use soryntel_thermo::{
    ChemicalSpecies, KelleyCoefficients, Phase, SpeciesRepository, SpeciesThermoInterval,
};
use std::collections::BTreeMap;

struct Repo(BTreeMap<String, ChemicalSpecies>);

impl SpeciesRepository for Repo {
    fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
        self.0.get(formula).cloned()
    }
    fn get_reference_element(&self, el: &str) -> Option<ChemicalSpecies> {
        let f = match el {
            "O" => "O2(g)",
            "H" => "H2(g)",
            other => other,
        };
        self.0.get(f).cloned()
    }
}

/// Especie de un solo intervalo con coeficientes en cal (convención Kelley/HSC).
fn species_cal(
    formula: &str,
    phase: Phase,
    h_kcal: f64,
    s_cal: f64,
    abcd: [f64; 4],
    mw: f64,
) -> ChemicalSpecies {
    ChemicalSpecies {
        formula: formula.into(),
        name: formula.into(),
        phase,
        molecular_weight: mw,
        density_g_cm3: 0.0,
        charge: 0,
        intervals: vec![SpeciesThermoInterval {
            t_min: 200.0,
            t_max: 3000.0,
            phase,
            kelley: KelleyCoefficients {
                a: abcd[0] * CAL_TO_JOULE,
                b: abcd[1] * CAL_TO_JOULE,
                c: abcd[2] * CAL_TO_JOULE,
                d: abcd[3] * CAL_TO_JOULE,
            },
            h_or_delta_h_tr: h_kcal * 1000.0 * CAL_TO_JOULE,
            s_or_delta_s_tr: s_cal * CAL_TO_JOULE,
            reference: "test".into(),
        }],
    }
}

/// O2(g) con los mismos datos que OpenDefault (JANAF 98).
fn o2() -> ChemicalSpecies {
    species_cal("O2(g)", Phase::Gas, 0.0, 49.003, [7.16, 1.0, -0.4, 0.0], 31.999)
}

fn eq_input(f: &str, mol: f64) -> EquilibriumSpeciesInput {
    EquilibriumSpeciesInput {
        formula: f.into(),
        feed_temp_c: 25.0,
        activity_coeff_expr: "1.0".into(),
        fixed_activity: None,
        discharge_coeff: 0.0,
        amount_unit: EquilibriumAmountUnit::Mol,
        initial_amount: mol,
        add_x_amount: 0.0,
        add_z_amount: 0.0,
        remove_percent: 0.0,
        excluded: false,
        species_data: None,
        stoichiometry: BTreeMap::new(),
    }
}

fn gas_phase(species: Vec<EquilibriumSpeciesInput>) -> EquilibriumPhase {
    EquilibriumPhase {
        name: "GAS".into(),
        model: SolutionModel::IdealMixture,
        is_gas_phase: true,
        species,
        electrode_role: String::new(),
        capacitance_farads: 0.0,
    }
}

// ---------------------------------------------------------------------------
// B01 — GEM: potencial químico de las especies de mezcla desplazado en -RT
// ---------------------------------------------------------------------------

/// N2O4(g) = 2 NO2(g) con G°(T) = 0 para ambas especies: ΔG° = 0, así que
/// Kp = x(NO2)²·P / x(N2O4) debe valer exactamente 1 a cualquier presión.
#[test]
#[ignore = "BUG-B01: el GEM converge a Kp = e^-Δn·K (fila de fase con +ȳ extra en el RHS)"]
fn b01_gem_gas_only_equilibrium_constant_matches_delta_g() {
    let zero = |f: &str| species_cal(f, Phase::Gas, 0.0, 0.0, [0.0; 4], 46.0);
    let repo = Repo(BTreeMap::from([
        ("N2O4(g)".to_string(), zero("N2O4(g)")),
        ("NO2(g)".to_string(), zero("NO2(g)")),
    ]));
    let sys = EquilibriumSystem {
        title: "N2O4".into(),
        phases: vec![gas_phase(vec![eq_input("N2O4(g)", 1.0), eq_input("NO2(g)", 0.0)])],
    };
    let params = EquilibriumParams {
        num_indep_vars: 0,
        ..Default::default()
    };
    for p_bar in [0.1, 1.0, 10.0] {
        let pt = solve_single_point(&sys, &params, &repo, 500.0, p_bar, 0, 0.0, None, None, 0.0)
            .unwrap();
        assert!(pt.converged);
        let n_a = pt.species[0].equil_amount_mol;
        let n_b = pt.species[1].equil_amount_mol;
        let tot = n_a + n_b;
        let kp = (n_b / tot).powi(2) / (n_a / tot) * p_bar;
        // Hoy: Kp = e^-1 = 0.3679 para todas las presiones.
        assert!((kp - 1.0).abs() < 1e-4, "P={p_bar} bar: Kp={kp}, esperado 1");
    }
}

// ---------------------------------------------------------------------------
// B02 — Exergía física de gases: corrección de presión contada dos veces
// ---------------------------------------------------------------------------

/// Gas ideal a T0 = 298,15 K: Ex_phys = R·T0·ln(P/P0) con P0 = 1,01325 bar
/// (Szargut, Morris & Steward 1988). Debe ser 0 en el estado muerto.
#[test]
#[ignore = "BUG-B02: evaluate_balance_point_si suma dos veces -R·ln(P) en Ex_phys de gases"]
fn b02_physical_exergy_of_ideal_gas_counts_pressure_once() {
    let repo = Repo(BTreeMap::from([("O2(g)".to_string(), o2())]));
    let sp = o2();
    for p_bar in [1.01325, 5.0, 20.0] {
        let pt = soryntel_thermo::balance::evaluate_balance_point_si(
            &repo, &sp, 298.15, p_bar, false, false,
        )
        .unwrap();
        let expected = R_JOULE * 298.15 * (p_bar / 1.01325_f64).ln() / 1000.0;
        // Hoy: 5 bar -> 7,947 MJ/kmol (esperado 3,957); 1,01325 bar -> 0,0326 (esperado 0).
        assert!(
            (pt.ex_phys_mj_kmol - expected).abs() < 1e-3,
            "P={p_bar}: Ex_phys={} MJ/kmol, esperado {expected}",
            pt.ex_phys_mj_kmol
        );
    }
}

// ---------------------------------------------------------------------------
// B03 — Exergía química: se ignora en silencio el elemento de referencia ausente
// ---------------------------------------------------------------------------

/// Sin el elemento de referencia Al no se puede calcular ΔfG(Al2O3). La
/// función debe señalarlo (convención propia: 0.0 = "no disponible") en lugar
/// de devolver G(Al2O3) - 1,5·G(O2) + Σν·ex_e como si fuera ΔfG.
#[test]
#[ignore = "BUG-B03: species_chemical_exergy_kj_mol omite en silencio la referencia del elemento ausente"]
fn b03_chemical_exergy_does_not_silently_skip_missing_reference_element() {
    let al2o3 = ChemicalSpecies {
        formula: "Al2O3".into(),
        name: "corundum".into(),
        phase: Phase::Solid,
        molecular_weight: 101.961,
        density_g_cm3: 3.965,
        charge: 0,
        intervals: vec![SpeciesThermoInterval {
            t_min: 100.0,
            t_max: 800.0,
            phase: Phase::Solid,
            kelley: KelleyCoefficients {
                a: 2.337 * CAL_TO_JOULE,
                b: 70.441 * CAL_TO_JOULE,
                c: -0.594 * CAL_TO_JOULE,
                d: -47.365 * CAL_TO_JOULE,
            },
            h_or_delta_h_tr: -400.5 * 1000.0 * CAL_TO_JOULE,
            s_or_delta_s_tr: 12.177 * CAL_TO_JOULE,
            reference: "OpenDefault".into(),
        }],
    };
    // Catálogo con O2(g) pero sin Al (igual que OpenDefault).
    let repo = Repo(BTreeMap::from([("O2(g)".to_string(), o2())]));
    let ex = soryntel_thermo::balance::species_chemical_exergy_kj_mol(&repo, &al2o3);
    // Szargut (1988): ex_ch(Al2O3, corindón) = 200,4 kJ/mol. Hoy devuelve 183,6.
    assert!(
        ex == 0.0 || (ex - 200.4).abs() < 1.0,
        "ex_ch(Al2O3) = {ex} kJ/mol: valor finito incorrecto sin la referencia de Al"
    );
}

// ---------------------------------------------------------------------------
// B04 — .rea8: K = ∞ se guarda como null y el workbook ya no se puede abrir
// ---------------------------------------------------------------------------

#[test]
fn b04_rea8_with_infinite_k_roundtrips() {
    // 4Al + 3O2(g) = 2Al2O3 a 298,15 K: log K ≈ 554 (JANAF), K desborda a +inf.
    let al = species_cal("Al", Phase::Solid, 0.0, 6.77, [4.94, 2.96, 0.0, 0.0], 26.98);
    let al2o3 = species_cal("Al2O3", Phase::Solid, -400.5, 12.177, [24.0, 0.0, 0.0, 0.0], 101.96);
    let repo = Repo(BTreeMap::from([
        ("Al".to_string(), al),
        ("Al2O3".to_string(), al2o3),
        ("O2(g)".to_string(), o2()),
    ]));
    let req = soryntel_thermo::CalculationRequest {
        input: "4Al + 3O2(g) = 2Al2O3".into(),
        t_from: 25.0,
        t_to: 25.0,
        t_step: 1.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
    };
    let res = soryntel_thermo::calculate(&req, &repo).unwrap();
    assert!(res.reaction_rows[0].k_eq.is_infinite());
    let wb = soryntel_thermo::ReactionWorkbookFile {
        version: "1".into(),
        formula_input: req.input.clone(),
        smallest_coeff: 1.0,
        t_from: 25.0,
        t_to: 25.0,
        t_step: 1.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
        y_axis: soryntel_thermo::ReactionYAxis::LogK,
        collect_data: true,
        plot_all_data: false,
        log_y_scale: false,
        decimals: 3,
        sheets: vec![soryntel_thermo::ReactionCollectedSheet {
            name: "S1".into(),
            response: res,
            visible_in_plot: true,
            amount_scale: 1.0,
        }],
        active_sheet_idx: 0,
    };
    let json = soryntel_thermo::serialize_rea8_workbook(&wb).unwrap();
    // Hoy: "invalid type: null, expected f64".
    soryntel_thermo::deserialize_rea8_workbook(&json).expect("el .rea8 guardado debe poder abrirse");
}

#[test]
fn b04_bal8_and_exe8_with_non_finite_values_roundtrip() {
    let mut wb = preset_h2o();
    wb.areas[0].in_streams[0].species[0].total_h = f64::INFINITY;
    wb.areas[0].in_streams[0].species[0].ex_phys = f64::NAN;
    wb.areas[0].extra_heat = f64::NEG_INFINITY;
    let json = soryntel_thermo::balance_engine::serialize_bal8_workbook(&wb).unwrap();
    let back = soryntel_thermo::balance_engine::deserialize_bal8_workbook(&json).unwrap();
    assert_eq!(back.areas[0].in_streams[0].species[0].total_h, f64::INFINITY);
    assert!(back.areas[0].in_streams[0].species[0].ex_phys.is_nan());
    assert_eq!(back.areas[0].extra_heat, f64::NEG_INFINITY);
    // Los archivos antiguos con `null` siguen abriéndose.
    let legacy = json.replace("\"inf\"", "null");
    assert!(soryntel_thermo::balance_engine::deserialize_bal8_workbook(&legacy).is_ok());

    let mut ex = soryntel_thermo::exergy::ExergyWorkbook::new_fig2_example();
    ex.streams[0].temp_k = f64::INFINITY;
    let json = ex.to_exe8_json().unwrap();
    let back = soryntel_thermo::exergy::ExergyWorkbook::from_exe8_str(&json).unwrap();
    assert_eq!(back.streams[0].temp_k, f64::INFINITY);
}

#[test]
fn b04_htr8_and_gem9_with_non_finite_inputs_roundtrip() {
    use soryntel_thermo::heat_loss::{model, presets};
    let mut wb = presets::preset_simple_case();
    wb.sheets[0].columns[0].thickness_m = f64::INFINITY;
    let json = model::serialize_htr8_workbook(&wb).unwrap();
    let back = model::deserialize_htr8_workbook(&json).unwrap();
    assert_eq!(back.sheets[0].columns[0].thickness_m, f64::INFINITY);

    let mut params = EquilibriumParams::default();
    params.temp_range.final_val = f64::INFINITY;
    let file = soryntel_thermo::equilibrium::EquilibriumWorkbookFile {
        system: EquilibriumSystem { title: "x".into(), phases: vec![] },
        params,
    };
    let json = file.to_gem9_json().unwrap();
    let back = soryntel_thermo::equilibrium::EquilibriumWorkbookFile::from_gem9_str(&json).unwrap();
    assert_eq!(back.params.temp_range.final_val, f64::INFINITY);
}

// ---------------------------------------------------------------------------
// B05 — Mass Balance: la copia de seguridad JSON no se puede generar
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BUG-B05: MassBalanceWorkbook no se serializa a JSON (claves BTreeMap<CellKey, _> no son cadenas)"]
fn b05_mass_balance_workbook_serializes_to_json() {
    let wb = soryntel_thermo::sim::mass_balance::preset_rougher_flotation_mb();
    // Hoy: Err("key must be a string"); el escritorio muestra "Serialize error".
    let json = serde_json::to_string_pretty(&wb).expect("la copia .HSCMas JSON debe generarse");
    let back: soryntel_thermo::sim::mass_balance::MassBalanceWorkbook =
        serde_json::from_str(&json).unwrap();
    assert_eq!(back.datasets.len(), wb.datasets.len());
}

// ---------------------------------------------------------------------------
// B06 — Heat Loss: densidad de flujo en BTU/(h·ft²) sin el factor de área
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BUG-B06: el flujo de calor en BTU/(h·ft²) se exporta como BTU/(h·m²) (falta 0,092903 m²/ft²)"]
fn b06_heat_flux_in_btu_per_h_ft2_uses_area_factor() {
    use soryntel_thermo::heat_loss::{model::build_heat_loss_export_sheets, presets, solve_workbook};
    use soryntel_thermo::spreadsheet_export::ExportCellValue;
    let mut wb = presets::preset_simple_case();
    wb.heat_flow_unit = HeatFlowUnit::BtuPerHour;
    let res = solve_workbook(&wb).unwrap();
    let flux_w_m2 = res.sheet_results[0].columns[0].heat_flux_w_m2;
    let sheets = build_heat_loss_export_sheets(&wb, Some(&res));
    let label = sheets[0]
        .cells
        .iter()
        .find(|c| c.col == 1 && matches!(&c.value, ExportCellValue::Text(t) if t.contains("ft")))
        .expect("fila de flujo en BTU/(h·ft²)");
    let row = label.row;
    let exported = sheets[0]
        .cells
        .iter()
        .find_map(|c| match (&c.value, c.row == row && c.col == 2) {
            (ExportCellValue::Number(v), true) => Some(*v),
            _ => None,
        })
        .unwrap();
    // 1 W/m² = 3600/1055,05585 BTU/h por m² × 0,09290304 m²/ft² = 0,316998 BTU/(h·ft²).
    let expected = flux_w_m2 * 3600.0 / 1055.055_852_62 * 0.092_903_04;
    assert!(
        (exported - expected).abs() <= 1e-3 * expected.abs().max(1.0),
        "exportado {exported}, esperado {expected} BTU/(h·ft²)"
    );
}

// ---------------------------------------------------------------------------
// B07 — Aqua (Pitzer): A_phi a 25 °C usa ε(1000 bar)
// ---------------------------------------------------------------------------

/// Bradley & Pitzer (1979): ε = ε1000 + C·ln((B+P)/(B+1000)). El código solo
/// usa ε1000, así que ε(25 °C) = 81,8 en lugar de 78,4 y A_phi baja un 6 %.
/// Valor de referencia (Pitzer 1991; el propio comentario del código): 0,3915.
#[test]
#[ignore = "BUG-B07: pitzer_a_phi(298,15) = 0,367; falta el término de presión de Bradley-Pitzer (ε = 81,8)"]
fn b07_pitzer_a_phi_at_25c() {
    let a = soryntel_thermo::aqua::pitzer_a_phi(298.15);
    assert!((a - 0.3915).abs() < 0.001, "A_phi(298,15 K) = {a}, esperado 0,3915");
}

// ---------------------------------------------------------------------------
// B08 — Balance: referencia de hoja desconocida escribe en IN1
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BUG-B08: set_cell_value con hoja inexistente escribe en el área 1 en lugar de fallar"]
fn b08_set_cell_value_rejects_unknown_sheet() {
    let mut wb = preset_h2o();
    let before = wb.areas[0].in_streams[0].species[0].amount_kmol;
    let r = set_cell_value(&mut wb, "FEED!D3", 5.0);
    let after = wb.areas[0].in_streams[0].species[0].amount_kmol;
    // Hoy: Ok(()) y IN1!D3 pasa de 0,06 a 5 kmol.
    assert!(r.is_err(), "una hoja inexistente debe dar InvalidCellReference");
    assert_eq!(before, after, "no debe modificar IN1");
}

// ---------------------------------------------------------------------------
// B09 / B10 — Balance: especies ausentes del catálogo
// ---------------------------------------------------------------------------

fn o2_balance(out_formulas: &[&str]) -> BalanceWorkbook {
    BalanceWorkbook {
        title: "t".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::MJ,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "A1".into(),
            in_streams: vec![StreamGroup::new(
                "IN",
                1000.0,
                vec![SpeciesRow::new("O2(g)", 1000.0, 1.0)],
            )],
            extra_heat: 0.0,
            out_streams: vec![StreamGroup::new(
                "OUT",
                25.0,
                out_formulas
                    .iter()
                    .map(|f| SpeciesRow::new(*f, 25.0, 1.0))
                    .collect(),
            )],
            heat_losses: 0.0,
        }],
        target_rows: vec![],
        diagram_config: DiagramConfig {
            title: String::new(),
            x_min: 0.0,
            x_max: 1.0,
            x_step: 1.0,
            x_cell_ref: String::new(),
            x_label: String::new(),
            y_series: vec![],
            use_target_iteration: false,
        },
        user_sheets: vec![],
        summaries: vec![],
        circular_iterations: 0,
        circular_converged: true,
    }
}

#[test]
#[ignore = "BUG-B09: la temperatura adiabática ignora en silencio especies de salida ausentes del catálogo"]
fn b09_adiabatic_temperature_fails_on_missing_output_species() {
    let repo = Repo(BTreeMap::from([("O2(g)".to_string(), o2())]));
    let wb = o2_balance(&["O2(g)", "Xx2(g)"]);
    // Hoy: Ok(≈1000 °C), calculado como si "Xx2(g)" (1 kmol) no existiera.
    let r = solve_adiabatic_temperature(&wb, 0, &repo);
    assert!(r.is_err(), "esperado SpeciesNotFound, obtenido {r:?}");
}

#[test]
#[ignore = "BUG-B10: evaluate_workbook conserva valores calculados obsoletos de una especie que ya no está en el catálogo"]
fn b10_evaluate_workbook_clears_stale_values_of_missing_species() {
    let repo = Repo(BTreeMap::from([("O2(g)".to_string(), o2())]));
    let mut wb = o2_balance(&["O2(g)"]);
    evaluate_workbook(&mut wb, &repo).unwrap();
    assert!(wb.summaries[0].in_totals.heat_content > 1.0);
    // El usuario cambia la fórmula de entrada por una que no existe.
    wb.areas[0].in_streams[0].species[0].formula = "Xx2(g)".into();
    evaluate_workbook(&mut wb, &repo).unwrap();
    let sp = &wb.areas[0].in_streams[0].species[0];
    assert!(sp.missing_in_db);
    // Hoy: heat_content, total_h y kg siguen con los valores de O2(g).
    assert_eq!(sp.heat_content, 0.0, "heat_content obsoleto = {}", sp.heat_content);
    assert_eq!(wb.summaries[0].in_totals.kg, 0.0, "kg obsoletos en el total");
}

// ---------------------------------------------------------------------------
// B15 — Heat Loss: calculation_grid sin límite superior (.htr8 o UI)
// ---------------------------------------------------------------------------

#[test]
fn b15_heat_loss_rejects_unbounded_calculation_grid() {
    use soryntel_thermo::heat_loss::{model::ColumnType, presets, solve_workbook};
    let mut wb = presets::preset_simple_case();
    let col = wb.sheets[0]
        .columns
        .iter_mut()
        .find(|c| c.column_type == ColumnType::SolidLayer)
        .expect("capa sólida en el preset");
    col.calculation_grid = usize::MAX / 8;
    let r = std::panic::catch_unwind(|| solve_workbook(&wb));
    match r {
        Ok(Err(_)) => {}
        Ok(Ok(_)) => panic!("se esperaba un error de validación"),
        Err(_) => panic!("solve_workbook entró en pánico (capacity overflow)"),
    }
}

// ---------------------------------------------------------------------------
// B16 — Diagrams: el bucle de temperaturas dependía de que `t += step` avanzara
// ---------------------------------------------------------------------------

fn diagram_req(t_min: f64, t_max: f64, step: f64) -> soryntel_thermo::diagrams::DiagramRequest {
    use soryntel_thermo::diagrams::*;
    DiagramRequest {
        species: vec![DiagramSpeciesInput {
            formula: "O2(g)".into(),
            coeff: 1.0,
            row_index: None,
            species_override: None,
        }],
        diagram_type: DiagramType::HTotal,
        t_min_k: t_min,
        t_max_k: t_max,
        step_k: step,
        energy_unit: DiagramEnergyUnit::KiloJoules,
        per_kg: false,
    }
}

#[test]
fn b16_diagram_rejects_non_advancing_or_huge_ranges() {
    let repo = Repo(BTreeMap::from([("O2(g)".to_string(), o2())]));
    // 1e16 + 1 == 1e16 en f64: antes era un bucle infinito.
    let r = soryntel_thermo::diagrams::calculate_diagram(&diagram_req(1e16, 1.000_000_000_000_000_4e16, 1.0), &repo);
    assert!(r.is_ok() || r.is_err());
    if let Ok(d) = r {
        assert!(d.series[0].points.len() <= 5);
    }
    assert!(soryntel_thermo::diagrams::calculate_diagram(&diagram_req(300.0, 1e12, 1.0), &repo).is_err());
    assert!(soryntel_thermo::diagrams::calculate_diagram(&diagram_req(300.0, f64::INFINITY, 1.0), &repo).is_err());
    let ok = soryntel_thermo::diagrams::calculate_diagram(&diagram_req(300.0, 500.0, 100.0), &repo).unwrap();
    let ts: Vec<f64> = ok.series[0].points.iter().map(|p| p.t_k).collect();
    assert_eq!(ts, vec![300.0, 400.0, 500.0]);
}
