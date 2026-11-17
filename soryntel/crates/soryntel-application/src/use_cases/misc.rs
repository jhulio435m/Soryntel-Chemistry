//! Casos de uso agrupados: misc.
use super::super::*;
use soryntel_core::{SpeciesBrowserPort, ThermoPointSI};
use soryntel_thermo::estimate::EstimateParityStats;

/// Máximo de especies de la base que se evalúan para la nube de paridad de Estimate
/// (mismo tope que el antiguo `search("", 20_000)`).
pub const ESTIMATE_PARITY_DB_LIMIT: usize = 20_000;

pub struct MiscUseCases;
impl MiscUseCases {
    pub fn evaluate_estimate(catalog: &dyn SpeciesCatalogPort, row: &mut EstimateRow) {
        evaluate_estimate_row(row, catalog);
    }

    /// Filas de ejemplo de la Fig. 1 de HSC Estimate (sin evaluar).
    pub fn estimate_default_rows() -> Vec<EstimateRow> {
        soryntel_thermo::estimate::default_fig1_rows()
    }

    /// Filas clásicas de HSC Estimate (sin evaluar).
    pub fn estimate_classic_rows() -> Vec<EstimateRow> {
        soryntel_thermo::estimate::classic_hsc_estimate_rows()
    }

    /// Filas de óxidos de la Fig. 5 (sin evaluar).
    pub fn estimate_fig5_oxides_rows() -> Vec<EstimateRow> {
        soryntel_thermo::estimate::fig5_oxides_rows()
    }

    /// Analiza una fórmula de Estimate: `(elementos, carga, fase indicada)`.
    pub fn estimate_parse_formula(
        formula: &str,
    ) -> Result<(std::collections::BTreeMap<String, f64>, i32, Option<soryntel_thermo::Phase>), String>
    {
        soryntel_thermo::estimate::parse_estimate_formula(formula)
    }

    /// Curva Cp(T) estimada para `row` en `[t_min_k, t_max_k]` con `steps` puntos.
    pub fn estimate_cp_curve(
        catalog: &dyn SpeciesCatalogPort,
        row: &EstimateRow,
        t_min_k: f64,
        t_max_k: f64,
        steps: usize,
    ) -> Vec<(f64, f64)> {
        soryntel_thermo::estimate::generate_cp_curve(row, catalog, t_min_k, t_max_k, steps)
    }

    /// Ajuste de Kelley de puntos `(T, Cp)`.
    pub fn estimate_fit_kelley(points: &[(f64, f64)]) -> soryntel_thermo::KelleyCoefficients {
        soryntel_thermo::estimate::fit_kelley_cp(points)
    }

    /// Construye la especie estimada a partir de la fila y de sus rangos Kelley.
    pub fn estimate_build_species(
        row: &EstimateRow,
        ranges: Vec<(f64, f64, soryntel_thermo::KelleyCoefficients)>,
        reference: &str,
    ) -> ChemicalSpecies {
        soryntel_thermo::estimate::build_estimated_species_with_intervals(row, ranges, reference)
    }

    /// Estadísticos de paridad `(nombre, estimado, base)`.
    pub fn estimate_parity_stats(pairs: &[(&str, f64, f64)]) -> EstimateParityStats {
        soryntel_thermo::estimate::compute_parity_stats(pairs)
    }

    /// Punto termodinámico SI de `species` a `t_k` (sin Criss–Cobble, lado inferior en
    /// las transiciones), o `None` si está fuera de rango.
    pub fn estimate_species_point(species: &ChemicalSpecies, t_k: f64) -> Option<ThermoPointSI> {
        soryntel_thermo::evaluate_species_si(species, t_k, false, false)
            .ok()
            .map(|p| p.0)
    }

    /// Especies de la vista resuelta que alimentan la nube de paridad: orden alfabético por
    /// fórmula y como mucho [`ESTIMATE_PARITY_DB_LIMIT`], igual que el antiguo
    /// `search("", 20_000)`, pero recorriendo el catálogo con `iter_species`.
    pub fn estimate_reference_species(browser: &dyn SpeciesBrowserPort) -> Vec<ChemicalSpecies> {
        let mut list: Vec<ChemicalSpecies> =
            browser.iter_species(None).map(|e| e.species).collect();
        list.sort_by(|a, b| a.formula.cmp(&b.formula));
        list.truncate(ESTIMATE_PARITY_DB_LIMIT);
        list
    }

    pub fn calculate_ehph(
        catalog: &dyn SpeciesCatalogPort,
        req: &EhPhRequest,
    ) -> Result<EhPhResult, String> {
        calculate_ehph(req, Some(catalog))
    }

    pub fn calculate_diagram(
        catalog: &dyn SpeciesCatalogPort,
        req: &DiagramRequest,
    ) -> Result<DiagramResult, String> {
        calculate_diagram(req, catalog)
    }

    pub fn calculate_phase_stability(
        catalog: &dyn SpeciesCatalogPort,
        input: &PhaseStabilityInput,
    ) -> Result<PhaseStabilityDiagram, PhaseStabilityError> {
        calculate_phase_stability(catalog, input)
    }

    pub fn initialize_species_converter(
        entries: &[ConverterInputEntry],
    ) -> Result<(Vec<CalculatedElementRow>, f64), SpeciesConverterError> {
        initialize_input(entries)
    }

    pub fn fix_species_balance(
        inputs: &[ConverterInputEntry],
        outputs: &mut Vec<ConverterOutputSpec>,
    ) -> Result<(), SpeciesConverterError> {
        fix_element_balance(inputs, outputs)
    }

    pub fn convert_species(
        catalog: &dyn SpeciesCatalogPort,
        req: &SpeciesConverterRequest,
    ) -> Result<SpeciesConverterResult, SpeciesConverterError> {
        solve_species_conversion(catalog, req)
    }

    pub fn parse_benson_smiles(smiles: &str) -> Result<MoleculeGraph, String> {
        MoleculeGraph::from_smiles(smiles)
    }

    pub fn estimate_benson_properties(
        catalog: &dyn SpeciesCatalogPort,
        mol: &MoleculeGraph,
        custom_groups: Option<&[(String, usize)]>,
        custom_params: Option<&BensonParamsInput>,
    ) -> Result<BensonEstimationResult, String> {
        estimate_benson(mol, custom_groups, custom_params, Some(catalog))
    }

    pub fn save_benson_species(
        catalog: &mut dyn SpeciesCatalogPort,
        est: &BensonEstimationResult,
        phase: soryntel_thermo::Phase,
        custom_formula: &str,
    ) -> ChemicalSpecies {
        let sp = build_benson_chemical_species(est, phase, custom_formula);
        catalog.insert_or_update_species(sp.clone());
        sp
    }

    pub fn default_elements_workbook() -> ElementWorkbook {
        ElementWorkbook::default_workbook()
    }

    pub fn compute_elements_diagram(
        wb: &ElementWorkbook,
        row_idx: usize,
    ) -> Result<PeriodicDiagramData, String> {
        wb.compute_diagram(row_idx)
    }

    /// Índice en la tabla periódica del elemento `symbol`.
    pub fn element_index(symbol: &str) -> Option<usize> {
        soryntel_thermo::elements::find_element_index_by_symbol(symbol)
    }

    /// CSV de la tabla periódica del workbook.
    pub fn export_elements_csv(wb: &ElementWorkbook) -> String {
        wb.to_csv()
    }

    pub fn lookup_element_species(
        catalog: &dyn SpeciesCatalogPort,
        symbol: &str,
    ) -> Vec<ChemicalSpecies> {
        domain_lookup_element_species(catalog, symbol)
    }

    pub fn default_units_workbook() -> UnitsWorkbook {
        UnitsWorkbook::default_hsc6()
    }

    pub fn convert_unit_value(
        wb: &UnitsWorkbook,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        unit2_idx: usize,
        prefix2: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Result<f64, String> {
        wb.convert_value(
            qty_idx,
            unit1_idx,
            prefix1,
            unit2_idx,
            prefix2,
            value,
            delta_temp_mode,
        )
    }

    pub fn convert_unit_all_equivalents(
        wb: &UnitsWorkbook,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Vec<(String, String, Result<f64, String>)> {
        wb.convert_all_equivalents(qty_idx, unit1_idx, prefix1, value, delta_temp_mode)
    }

    pub fn compute_air_humidity_point(
        temp_c: f64,
        rh_pct: f64,
        total_p_bar: f64,
    ) -> Result<AirHumidityPoint, String> {
        domain_air_humidity_point(temp_c, rh_pct, total_p_bar)
    }

    pub fn air_humidity_grid_descending() -> AirHumidityGrid {
        domain_air_grid_descending()
    }

    pub fn air_humidity_3d_surface() -> AirHumidityGrid {
        domain_air_3d_surface()
    }

    pub fn recalculate_material_auto_properties(
        db: &mut soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
    ) -> Result<(), SoryntelError> {
        db.recalculate_auto_properties(material_idx)
    }

    pub fn sample_material_cp_curve(
        db: &soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
        points_per_range: usize,
    ) -> Vec<(f64, f64)> {
        db.sample_cp_curve(material_idx, points_per_range)
    }

    pub fn resolve_material_composition(
        db: &soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
    ) -> Result<
        (
            f64,
            Vec<soryntel_thermo::heat_loss::ResolvedSpeciesWt>,
            Vec<soryntel_thermo::heat_loss::ResolvedElementWt>,
        ),
        SoryntelError,
    > {
        db.resolve_composition(material_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_db::ThermoDatabase;

    #[test]
    fn estimate_reference_species_matches_legacy_search_order() {
        let db = ThermoDatabase::with_open_defaults();
        let via_port = MiscUseCases::estimate_reference_species(&db);
        let legacy = db.search("", ESTIMATE_PARITY_DB_LIMIT);
        let a: Vec<&str> = via_port.iter().map(|s| s.formula.as_str()).collect();
        let b: Vec<&str> = legacy.iter().map(|s| s.formula.as_str()).collect();
        assert!(!a.is_empty());
        assert_eq!(a, b);
    }
}
