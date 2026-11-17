//! Casos de uso agrupados: misc.
use super::super::*;
pub struct MiscUseCases;
impl MiscUseCases {
    pub fn evaluate_estimate(catalog: &dyn SpeciesCatalogPort, row: &mut EstimateRow) {
        evaluate_estimate_row(row, catalog);
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
