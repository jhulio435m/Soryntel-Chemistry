//! Casos de uso agrupados: catalog.
use super::super::*;
pub struct CatalogUseCases;
impl CatalogUseCases {
    pub fn search_species(
        catalog: &dyn SpeciesCatalogPort,
        query: &str,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species(query, limit)
    }

    pub fn search_species_by_elements(
        catalog: &dyn SpeciesCatalogPort,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species_by_elements(
            elements,
            include_gas,
            include_liquid,
            include_solid,
            include_aqua_ions,
            include_aqua_neutrals,
            limit,
        )
    }

    pub fn species_count(catalog: &dyn SpeciesCatalogPort) -> usize {
        catalog.species_count()
    }

    pub fn species_compute_mw(
        formula: &str,
    ) -> Result<(f64, std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        compute_molecular_weight(formula)
    }

    pub fn species_parse_elements(
        formula: &str,
    ) -> Result<(std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        parse_hsc_formula_elements(formula)
    }

    pub fn species_fit_cp(
        points: &[CpDataPoint],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<CpFitResult, SoryntelError> {
        fit_cp_kelley(points, mask, locked_si)
    }

    pub fn species_convert_cp_function(
        expr: &str,
        unit: &str,
        t1_k: f64,
        t2_k: f64,
        params: &[f64; 6],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(Vec<CpDataPoint>, CpFitResult), SoryntelError> {
        convert_cp_function(expr, unit, t1_k, t2_k, params, mask, locked_si)
    }

    pub fn species_convert_gibbs(
        input: &GibbsCoeffsInput,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        convert_gibbs_function(input)
    }

    pub fn species_gibbs_delta_to_normal(
        catalog: &dyn SpeciesCatalogPort,
        target_formula: &str,
        delta_out: &GibbsConvertOutput,
        t_ref_k: f64,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        gibbs_delta_to_normal(catalog, target_formula, delta_out, t_ref_k)
    }

    pub fn species_extrapolate_liquid_298(
        solid_intervals: &[soryntel_thermo::SpeciesThermoInterval],
        t_melt_k: f64,
        delta_h_fus_j: f64,
        liquid_kelley: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(f64, f64), SoryntelError> {
        extrapolate_liquid_298(solid_intervals, t_melt_k, delta_h_fus_j, liquid_kelley)
    }

    pub fn species_analyze_aqueous(species: &ChemicalSpecies) -> Option<AqueousScaleSummary> {
        analyze_aqueous_scales(species)
    }

    pub fn species_color_spec(code: i16) -> &'static HscColorSpec {
        color_spec(code)
    }

    pub fn species_awdb_weight(symbol: &str) -> Option<f64> {
        awdb_atomic_weight(symbol)
    }

    pub fn species_stoichiometry_matches(
        candidate: &std::collections::BTreeMap<String, f64>,
        target: &std::collections::BTreeMap<String, f64>,
        fix_ratio: bool,
    ) -> bool {
        stoichiometry_matches(candidate, target, fix_ratio)
    }
}
