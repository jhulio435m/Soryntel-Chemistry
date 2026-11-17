//! Casos de uso agrupados: equilibrium.
use super::super::*;
pub struct EquilibriumUseCases;
impl EquilibriumUseCases {
    /// Analiza un término de especie (fórmula con coeficiente opcional) con el mismo parser
    /// estequiométrico del solver; lo usan, p. ej., los chips de elementos de la UI.
    pub fn parse_species_term(
        formula: &str,
    ) -> Result<crate::models::reaction::ParsedTerm, SoryntelError> {
        soryntel_solver::parse_term(formula)
    }

    pub fn collect_cp_warnings<R: crate::ports::SpeciesRepository + ?Sized>(
        catalog: &R,
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

    pub fn solve_equilibrium_sweep<R: crate::ports::SpeciesRepository + ?Sized>(
        catalog: &R,
        sys: &EquilibriumSystem,
        params: &EquilibriumParams,
    ) -> Result<EquilibriumSweepResult, SoryntelError> {
        solve_equilibrium_sweep(sys, params, catalog)
    }
}
