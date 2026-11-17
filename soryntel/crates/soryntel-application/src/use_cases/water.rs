//! Casos de uso agrupados: water.
use super::super::*;
pub struct WaterUseCases;
impl WaterUseCases {
    pub fn evaluate_water_species(
        species: &ChemicalSpecies,
        temperature_k: f64,
    ) -> Result<(soryntel_core::ThermoPointSI, String), SoryntelError> {
        soryntel_thermo::evaluate_species_si(species, temperature_k, false, false)
    }

    pub fn sat_temp_ls_k(value: f64) -> f64 {
        soryntel_thermo::water::sat_temp_ls_k(value)
    }

    pub fn sat_temp_gl_k(value: f64) -> f64 {
        soryntel_thermo::water::sat_temp_gl_k(value)
    }

    pub fn sat_pressure_gs_bar(value: f64) -> f64 {
        soryntel_thermo::water::sat_pressure_gs_bar(value)
    }

    pub fn h2o_saturation_pressure_bar(value: f64) -> f64 {
        soryntel_thermo::balance::h2o_saturation_pressure_bar(value)
    }

    pub fn default_water_request() -> WaterCalcRequest {
        domain_default_water_request()
    }

    pub fn calculate_water(
        catalog: &dyn SpeciesCatalogPort,
        req: &WaterCalcRequest,
    ) -> WaterCalcResult {
        domain_calculate_water_points(req, Some(catalog))
    }

    /// Checked entry point for interactive and external requests.
    pub fn try_calculate_water(
        catalog: &dyn SpeciesCatalogPort,
        req: &WaterCalcRequest,
    ) -> Result<WaterCalcResult, SoryntelError> {
        soryntel_thermo::water::validate_water_request(req)?;
        Ok(domain_calculate_water_points(req, Some(catalog)))
    }

    pub fn water_phase_curves() -> WaterDiagramCurves {
        domain_water_diagram_curves()
    }
}
