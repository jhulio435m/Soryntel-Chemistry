//! Casos de uso agrupados: simulation.
use super::super::*;
pub struct SimulationUseCases;
impl SimulationUseCases {
    pub fn empty_project() -> SimProject {
        domain_sim_empty_project()
    }

    pub fn sim_presets() -> Vec<(&'static str, SimProject)> {
        vec![
            ("Empty Flowsheet", domain_sim_empty_project()),
            ("40.1_model (Simple 2-Unit)", domain_sim_preset_40_1()),
            (
                "Hydro_example (Leaching + Cooler)",
                domain_sim_preset_hydro(),
            ),
            (
                "Concentrator_Flotation (Scenario)",
                domain_sim_preset_concentrator(),
            ),
            ("Recycle_Loop (Tarjan SCC)", domain_sim_preset_recycle()),
            ("Zn_TSL_Fuming (Sim LCA 49.3)", domain_sim_preset_zn_tsl()),
            (
                "Iron_Process (Sim LCA 49.4)",
                domain_sim_preset_iron_process(),
            ),
        ]
    }

    pub fn sim_model_catalog() -> Vec<UnitModelCatalogEntry> {
        domain_sim_model_catalog()
    }

    pub fn sim_calc_order(project: &SimProject) -> CalcOrderInfo {
        domain_sim_calc_order(project)
    }

    pub fn sim_run_static(project: &mut SimProject, selected_unit_ids: Option<&[usize]>) {
        domain_sim_run_static(project, selected_unit_ids);
    }

    pub fn sim_clear_internal(project: &mut SimProject) {
        domain_sim_clear_internal(project);
    }

    pub fn sim_empty_tanks(project: &mut SimProject) {
        domain_sim_empty_tanks(project);
    }

    pub fn sim_step_dynamic(project: &mut SimProject, steps: usize) {
        domain_sim_step_dynamic(project, steps);
    }

    pub fn sim_run_controls(project: &mut SimProject, only_idx: Option<usize>) {
        domain_sim_run_controls(project, only_idx);
    }

    pub fn sim_control_sweep(
        project: &SimProject,
        ctrl_idx: usize,
        steps: usize,
    ) -> Vec<(usize, f64, f64)> {
        domain_sim_control_sweep(project, ctrl_idx, steps)
    }

    pub fn sim_execute_scenario(project: &mut SimProject, scenario_idx: usize, seed: u64) {
        domain_sim_execute_scenario(project, scenario_idx, seed);
    }

    pub fn sim_mc_curves(col: &ScenarioColumn, points: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        domain_sim_eval_mc_curves(col, points)
    }

    pub fn sim_generate_reports(project: &SimProject) -> SimReportWorkbook {
        domain_sim_generate_reports(project)
    }

    pub fn sim_report_csv(sheet: &SimReportSheet) -> String {
        domain_sim_report_csv(sheet)
    }

    pub fn sim_filter_log(
        msg: &str,
        severity: LogSeverity,
        level: LogFilterLevel,
        query: &str,
    ) -> bool {
        domain_sim_filter_log(msg, severity, level, query)
    }

    pub fn sim_export_lca_json(project: &SimProject) -> Result<String, String> {
        domain_sim_export_lca_json(project)
    }

    pub fn sim_export_svg(project: &SimProject) -> String {
        domain_sim_export_svg(project)
    }

    pub fn sim_rewrite_sim7(expr: &str, sheet_name: &str) -> (String, String) {
        domain_sim_rewrite_sim7(expr, sheet_name)
    }

    pub fn sim_metric_value(
        stream: &soryntel_thermo::sim::model::SimStream,
        metric: &str,
    ) -> (f64, &'static str) {
        domain_sim_metric_value(stream, metric)
    }

    pub fn sim_global_balances(
        project: &SimProject,
    ) -> soryntel_thermo::sim::model::ConvergenceRound {
        domain_sim_eval_balances(project)
    }

    pub fn sim_lca_gabi_flows() -> Vec<LcaCatalogFlow> {
        domain_sim_lca_gabi_flows()
    }

    pub fn sim_lca_openlca_flows() -> Vec<LcaCatalogFlow> {
        domain_sim_lca_openlca_flows()
    }

    pub fn sim_lca_methods() -> Vec<LciaMethodSpec> {
        domain_sim_lca_methods()
    }

    pub fn sim_lca_classic_sync(state: &mut ClassicLcaState, project: &SimProject) {
        state.sync_from_project(project);
    }

    pub fn sim_lca_classic_normalize(state: &mut ClassicLcaState) -> Result<(), String> {
        state.normalize().map(|_| ())
    }

    pub fn sim_lca_classic_ecospold_xml(state: &mut ClassicLcaState) -> Result<String, String> {
        state.export_ecospold_v1_xml()
    }

    pub fn sim_lca_classic_jsonld_zip(state: &mut ClassicLcaState) -> Result<Vec<u8>, String> {
        state.export_openlca_jsonld_zip()
    }

    pub fn sim_lca_classic_xlsx(state: &mut ClassicLcaState) -> Result<Vec<u8>, String> {
        state.export_xlsx()
    }

    pub fn sim_lca_beta_sync(state: &mut BetaLcaState, project: &SimProject) {
        state.sync_from_project(project);
    }

    pub fn sim_lca_beta_calculate(state: &mut BetaLcaState) -> Result<(), String> {
        state.calculate_results()
    }

    pub fn sim_lca_beta_xlsx(state: &BetaLcaState) -> Vec<u8> {
        state.export_to_xlsx()
    }

    pub fn sim_lca_preset_classic_zn() -> (SimProject, ClassicLcaState) {
        let p = domain_sim_preset_zn_tsl();
        let l = domain_sim_lca_preset_classic_zn(&p);
        (p, l)
    }

    pub fn sim_lca_preset_beta_iron() -> (SimProject, BetaLcaState) {
        let p = domain_sim_preset_iron_process();
        let l = domain_sim_lca_preset_beta_iron(&p);
        (p, l)
    }

    pub fn mb_preset_rougher() -> MassBalanceWorkbook {
        domain_mb_preset_rougher()
    }

    pub fn mb_preset_grinding() -> MassBalanceWorkbook {
        domain_mb_preset_grinding()
    }

    pub fn mb_from_sim_project(project: &SimProject) -> MassBalanceWorkbook {
        MassBalanceWorkbook::from_sim_project(project)
    }

    pub fn mb_save_to_sim_project(wb: &MassBalanceWorkbook, project: &mut SimProject) -> usize {
        wb.save_to_sim_project(project)
    }

    pub fn mb_check_errors(wb: &MassBalanceWorkbook, dataset_idx: usize) -> Vec<ValidationIssue> {
        wb.check_errors(dataset_idx)
    }

    pub fn mb_calculate(
        wb: &mut MassBalanceWorkbook,
        dataset_idx: usize,
    ) -> Result<MbSolution, String> {
        wb.calculate_dataset(dataset_idx)
    }

    pub fn mb_calculate_batch(wb: &mut MassBalanceWorkbook) -> Vec<Result<MbSolution, String>> {
        wb.calculate_all_batch()
    }

    pub fn mb_import_hsc7(
        wb: &mut MassBalanceWorkbook,
        flowsheet_csv: Option<&str>,
        data_csv: &str,
        horizontal: bool,
    ) -> Result<usize, String> {
        wb.import_hsc7_text(flowsheet_csv, data_csv, horizontal)
    }

    pub fn mb_export_xlsx(wb: &MassBalanceWorkbook) -> Result<Vec<u8>, String> {
        wb.export_results_xlsx()
    }

    /// Incertidumbre de una celda medida:
    /// `(sd_meas, rsd_pct, sd_sampling_scaled, sd_eff_for_solver)`.
    #[allow(clippy::too_many_arguments)]
    pub fn mb_evaluate_cell_uncertainty(
        value: Option<f64>,
        var: &crate::models::mass_balance::MbVariable,
        stream: &crate::models::mass_balance::MbStream,
        pre_status: crate::models::mass_balance::PreMeasStatus,
        override_sd: Option<f64>,
        null_sd_estimate: f64,
        low_grade_coeff: f64,
    ) -> (f64, f64, f64, f64) {
        soryntel_thermo::sim::mass_balance::evaluate_cell_uncertainty(
            value,
            var,
            stream,
            pre_status,
            override_sd,
            null_sd_estimate,
            low_grade_coeff,
        )
    }

    /// Densidad relativa por defecto de un mineral según su nombre.
    pub fn mb_default_mineral_sg(name: &str) -> f64 {
        soryntel_thermo::sim::mass_balance::default_mineral_sg(name)
    }

    /// Búsqueda en el catálogo de minerales (como mucho `limit` resultados).
    pub fn mb_search_minerals_catalog(
        query: &str,
        limit: usize,
    ) -> Vec<&'static crate::models::mass_balance::MineralRawRow> {
        soryntel_thermo::sim::mass_balance::search_minerals_catalog(query, limit)
    }

    /// Importa una hoja externa en el estado beta y devuelve el nombre final de la hoja.
    pub fn sim_lca_beta_import_external_sheet(
        state: &mut BetaLcaState,
        table_name: &str,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    ) -> String {
        state.import_external_sheet(table_name, headers, rows)
    }

    /// Carga flujos desde un archivo `.zip` JSON-LD de openLCA (entradas `STORE`).
    pub fn sim_lca_classic_load_jsonld_zip(
        state: &mut ClassicLcaState,
        source_label: impl Into<String>,
        zip_bytes: &[u8],
    ) -> Result<usize, String> {
        state.load_openlca_jsonld_zip_bytes(source_label, zip_bytes)
    }
}
