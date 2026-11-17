//! Casos de uso agrupados: water.
use super::super::*;
pub struct WaterUseCases;
impl WaterUseCases {
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
