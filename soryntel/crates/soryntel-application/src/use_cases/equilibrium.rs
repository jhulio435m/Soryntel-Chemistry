//! Casos de uso agrupados: equilibrium.
use super::super::*;
pub struct EquilibriumUseCases;
impl EquilibriumUseCases {
    pub fn solve_equilibrium_sweep<R: crate::ports::SpeciesRepository>(
        catalog: &R,
        sys: &EquilibriumSystem,
        params: &EquilibriumParams,
    ) -> Result<EquilibriumSweepResult, SoryntelError> {
        solve_equilibrium_sweep(sys, params, catalog)
    }
}
