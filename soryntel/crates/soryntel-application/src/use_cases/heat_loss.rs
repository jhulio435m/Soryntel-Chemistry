//! Casos de uso agrupados: heat_loss.
use super::super::*;
pub struct HeatLossUseCases;
impl HeatLossUseCases {
    pub fn estimate_reactor_natural_convection_loss(
        area_m2: f64,
        t_surface_c: f64,
        t_room_c: f64,
        unit: soryntel_core::BalanceEnergyUnit,
    ) -> f64 {
        domain_estimate_air_cooled_heat_loss(area_m2, t_surface_c, t_room_c, unit)
    }

    pub fn heat_flow_w_to_balance_energy_per_hour(
        q_watts: f64,
        unit: soryntel_core::BalanceEnergyUnit,
    ) -> f64 {
        soryntel_thermo::heat_loss::heat_flow_w_to_balance_energy_per_hour(q_watts, unit)
    }

    pub fn solve_heat_loss_sheet(sheet: &HeatLossSheet) -> Result<SheetResult, SoryntelError> {
        solve_sheet(sheet)
    }

    pub fn solve_heat_loss_workbook(
        wb: &HeatLossWorkbook,
    ) -> Result<WorkbookResult, SoryntelError> {
        solve_workbook(wb)
    }

    pub fn solve_heat_loss_target(
        wb: &HeatLossWorkbook,
        spec: &soryntel_thermo::heat_loss::TargetSpec,
    ) -> Result<soryntel_thermo::heat_loss::TargetResult, SoryntelError> {
        soryntel_thermo::heat_loss::solve_target(wb, spec)
    }

    pub fn generate_heat_loss_temperature_profile(
        wb: &HeatLossWorkbook,
    ) -> Result<
        (
            f64,
            Vec<soryntel_thermo::heat_loss::TemperatureProfileSeries>,
        ),
        SoryntelError,
    > {
        soryntel_thermo::heat_loss::generate_temperature_profile(wb)
    }

    pub fn generate_heat_loss_parameter_sweep(
        wb: &HeatLossWorkbook,
        sheet_index: usize,
        param: soryntel_thermo::heat_loss::SweepParameter,
        min_x: f64,
        max_x: f64,
        steps: usize,
    ) -> Result<Vec<soryntel_thermo::heat_loss::SweepPoint>, SoryntelError> {
        soryntel_thermo::heat_loss::generate_parameter_sweep(
            wb,
            sheet_index,
            param,
            min_x,
            max_x,
            steps,
        )
    }

    pub fn generate_heat_loss_optimum_insulation(
        wb: &HeatLossWorkbook,
        spec: &soryntel_thermo::heat_loss::OptimumInsulationSpec,
    ) -> Result<
        (
            Vec<soryntel_thermo::heat_loss::OptimumInsulationPoint>,
            soryntel_thermo::heat_loss::OptimumInsulationPoint,
        ),
        SoryntelError,
    > {
        soryntel_thermo::heat_loss::generate_optimum_insulation_diagram(wb, spec)
    }

    pub fn evaluate_heat_loss_convection(
        fluid: &soryntel_thermo::heat_loss::FluidPropertyTable,
        regime: soryntel_thermo::heat_loss::ConvectionRegime,
        geom: soryntel_thermo::heat_loss::ConvectionGeometry,
        t_surface_k: f64,
        t_ambient_k: f64,
        char_length_m: f64,
        enclosure_height_m: Option<f64>,
        speed_m_s: f64,
        outer_diameter_m: Option<f64>,
    ) -> Result<soryntel_thermo::heat_loss::convection::ConvectionDiagnostics, SoryntelError> {
        soryntel_thermo::heat_loss::convection::evaluate_convection(
            fluid,
            regime,
            geom,
            t_surface_k,
            t_ambient_k,
            char_length_m,
            enclosure_height_m,
            speed_m_s,
            outer_diameter_m,
        )
    }

    pub fn evaluate_heat_loss_hc_function(
        terms: &[soryntel_thermo::heat_loss::ConvectionFunctionTerm],
        speed_m_s: f64,
    ) -> f64 {
        soryntel_thermo::heat_loss::convection::evaluate_hc_function(terms, speed_m_s)
    }

    pub fn calculate_heat_loss_gas_radiation(
        input: &soryntel_thermo::heat_loss::GasRadiationInput,
    ) -> Result<soryntel_thermo::heat_loss::GasRadiationResult, SoryntelError> {
        soryntel_thermo::heat_loss::calculate_gas_radiation(input)
    }

    pub fn calculate_heat_loss_gas_particle_radiation(
        gas_input: &soryntel_thermo::heat_loss::GasRadiationInput,
        particle_input: &soryntel_thermo::heat_loss::ParticleCloudInput,
    ) -> Result<
        (
            soryntel_thermo::heat_loss::GasRadiationResult,
            f64,
            f64,
            f64,
        ),
        SoryntelError,
    > {
        soryntel_thermo::heat_loss::calculate_gas_particle_radiation(gas_input, particle_input)
    }

    pub fn evaluate_heat_loss_particle_suspension_detailed(
        spec: &soryntel_thermo::heat_loss::ParticleSuspensionSpec,
    ) -> Result<soryntel_thermo::heat_loss::ParticleSuspensionDetailedResult, SoryntelError> {
        soryntel_thermo::heat_loss::evaluate_particle_suspension_detailed(spec)
    }

    pub fn generate_heat_loss_iterative_gas_radiation_table(
        pressure_bar: f64,
        optical_depth_m: f64,
        comp: &soryntel_thermo::heat_loss::GasComposition,
    ) -> Vec<(f64, f64)> {
        soryntel_thermo::heat_loss::generate_iterative_gas_radiation_table(
            pressure_bar,
            optical_depth_m,
            comp,
        )
    }

    pub fn apply_heat_loss_geometry_transition(
        wb: &mut HeatLossWorkbook,
        sheet_idx: usize,
        new_geom: soryntel_thermo::heat_loss::Geometry,
    ) {
        soryntel_thermo::heat_loss::apply_geometry_transition(wb, sheet_idx, new_geom);
    }

    pub fn serialize_htr8_workbook(wb: &HeatLossWorkbook) -> Result<String, String> {
        soryntel_thermo::heat_loss::serialize_htr8_workbook(wb)
    }

    pub fn deserialize_htr8_workbook(raw: &str) -> Result<HeatLossWorkbook, String> {
        soryntel_thermo::heat_loss::deserialize_htr8_workbook(raw)
    }

    pub fn export_heat_loss_workbook_xlsx(
        wb: &HeatLossWorkbook,
        res: Option<&WorkbookResult>,
    ) -> Vec<u8> {
        let sheets = soryntel_thermo::heat_loss::build_heat_loss_export_sheets(wb, res);
        build_xlsx(&sheets)
    }
}
