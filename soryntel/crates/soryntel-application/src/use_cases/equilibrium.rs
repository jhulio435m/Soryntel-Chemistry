//! Casos de uso agrupados: equilibrium.
use super::super::*;
pub struct EquilibriumUseCases;
impl EquilibriumUseCases {
    pub fn collect_cp_warnings(
        catalog: &dyn SpeciesCatalogPort,
        system: &EquilibriumSystem,
        params: &EquilibriumParams,
    ) -> Vec<crate::models::equilibrium::ExtrapolationWarning> {
        soryntel_thermo::equilibrium::collect_cp_warnings(system, params, catalog)
    }

    pub fn serialize_gem9_workbook(
        workbook: &crate::models::equilibrium::EquilibriumWorkbookFile,
    ) -> Result<String, String> {
        workbook.to_gem9_json()
    }

    pub fn deserialize_gem9_workbook(
        raw: &str,
    ) -> Result<crate::models::equilibrium::EquilibriumWorkbookFile, String> {
        crate::models::equilibrium::EquilibriumWorkbookFile::from_gem9_str(raw)
    }

    pub fn solve_equilibrium_sweep<R: SpeciesCatalogPort + ?Sized>(
        catalog: &R,
        sys: &EquilibriumSystem,
        params: &EquilibriumParams,
    ) -> Result<EquilibriumSweepResult, SoryntelError> {
        solve_equilibrium_sweep(sys, params, catalog)
    }
}
