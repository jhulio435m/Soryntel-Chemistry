//! Casos de uso agrupados: reaction.
use super::super::*;
pub struct ReactionUseCases;
impl ReactionUseCases {
    pub fn balance_reaction_equation(
        raw: &str,
        smallest_coeff: f64,
    ) -> Result<(String, soryntel_solver::ParsedReaction), SoryntelError> {
        soryntel_solver::balance_reaction(raw, smallest_coeff)
    }

    pub fn calculate_reaction(
        catalog: &dyn SpeciesCatalogPort,
        req: &CalculationRequest,
    ) -> Result<CalculationResponse, SoryntelError> {
        calculate(req, catalog)
    }

    pub fn are_reaction_sheets_plot_compatible(
        primary: &CalculationResponse,
        candidate: &CalculationResponse,
    ) -> bool {
        domain_reaction_plot_compatible(primary, candidate)
    }

    pub fn extract_reaction_plot_points(
        res: &CalculationResponse,
        y_axis: ReactionYAxis,
        log_y_scale: bool,
    ) -> Vec<[f64; 2]> {
        domain_extract_reaction_plot_points(res, y_axis, log_y_scale)
    }

    pub fn format_reaction_sheet_tsv(sheet: &ReactionCollectedSheet, decimals: usize) -> String {
        domain_format_reaction_sheet_tsv(sheet, decimals)
    }

    pub fn export_reaction_workbook_xlsx(sheets: &[ReactionCollectedSheet]) -> Vec<u8> {
        let export_sheets = domain_build_reaction_export_sheets(sheets);
        build_xlsx(&export_sheets)
    }

    pub fn render_reaction_chart_svg(
        title: &str,
        x_label: &str,
        y_label: &str,
        series: &[(String, Vec<[f64; 2]>, &str)],
        show_grid: bool,
        show_curve_labels: bool,
    ) -> String {
        domain_render_reaction_chart_svg(
            title,
            x_label,
            y_label,
            series,
            show_grid,
            show_curve_labels,
        )
    }

    pub fn serialize_rea8_workbook(wb: &ReactionWorkbookFile) -> Result<String, SoryntelError> {
        domain_serialize_rea8_workbook(wb)
    }

    pub fn deserialize_rea8_workbook(raw: &str) -> Result<ReactionWorkbookFile, SoryntelError> {
        domain_deserialize_rea8_workbook(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};

    #[test]
    fn balances_equation_through_reaction_use_case() {
        let (equation, _) =
            ReactionUseCases::balance_reaction_equation("H2 + O2 -> H2O", 1.0).unwrap();
        assert!(equation.contains("2H2"));
        assert!(equation.contains("O2"));
        assert!(equation.contains("2H2O"));
    }

    #[test]
    fn rea8_roundtrip_preserves_workbook_fields() {
        let workbook = ReactionWorkbookFile {
            version: "1".into(),
            formula_input: "H2 + O2 -> H2O".into(),
            smallest_coeff: 1.0,
            t_from: 0.0,
            t_to: 1000.0,
            t_step: 100.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Normal,
            show_transitions: true,
            criss_cobble: false,
            y_axis: ReactionYAxis::Cp,
            collect_data: false,
            plot_all_data: false,
            log_y_scale: false,
            decimals: 3,
            sheets: vec![],
            active_sheet_idx: 0,
        };
        let raw = ReactionUseCases::serialize_rea8_workbook(&workbook).unwrap();
        let restored = ReactionUseCases::deserialize_rea8_workbook(&raw).unwrap();
        assert_eq!(restored.formula_input, workbook.formula_input);
        assert_eq!(restored.t_to, workbook.t_to);
    }
}
