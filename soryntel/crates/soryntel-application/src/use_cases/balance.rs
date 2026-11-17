//! Casos de uso agrupados: balance.
use super::super::*;
pub struct BalanceUseCases;
impl BalanceUseCases {
    pub fn balance_preset_cuconv1() -> BalanceWorkbook {
        domain_balance_preset_cuconv1()
    }

    pub fn balance_preset_butane() -> BalanceWorkbook {
        domain_balance_preset_butane()
    }

    pub fn balance_preset_cusmelt() -> BalanceWorkbook {
        domain_balance_preset_cusmelt()
    }

    pub fn balance_preset_h2o() -> BalanceWorkbook {
        domain_balance_preset_h2o()
    }

    pub fn balance_preset_fsf_process() -> BalanceWorkbook {
        domain_balance_preset_fsf_process()
    }

    pub fn balance_preset_ch4_cooling_water() -> BalanceWorkbook {
        soryntel_thermo::balance_engine::preset_ch4_cooling_water()
    }

    pub fn evaluate_balance_workbook(
        catalog: &dyn SpeciesCatalogPort,
        wb: &mut BalanceWorkbook,
    ) -> Result<(), SoryntelError> {
        domain_evaluate_balance_workbook(wb, catalog)
    }

    pub fn solve_balance_adiabatic_temperature(
        catalog: &dyn SpeciesCatalogPort,
        wb: &BalanceWorkbook,
        area_idx: usize,
    ) -> Result<f64, SoryntelError> {
        domain_solve_adiabatic_temp(wb, area_idx, catalog)
    }

    pub fn iterate_balance_target_rows(
        catalog: &dyn SpeciesCatalogPort,
        wb: &mut BalanceWorkbook,
        row_indices: &[usize],
    ) -> Result<(), SoryntelError> {
        domain_iterate_balance_targets(wb, row_indices, catalog)
    }

    pub fn run_balance_diagram_sweep(
        catalog: &dyn SpeciesCatalogPort,
        wb: &BalanceWorkbook,
    ) -> Result<BalanceDiagramResult, SoryntelError> {
        domain_run_balance_diagram_sweep(wb, catalog)
    }

    pub fn compute_balance_stream_compositions(
        area: &BalanceArea,
        is_out: bool,
    ) -> Vec<StreamComposition> {
        domain_compute_stream_compositions(area, is_out)
    }

    pub fn set_balance_species_kg(sp: &mut SpeciesRow, new_kg: f64) {
        domain_set_species_kg(sp, new_kg);
    }

    pub fn set_balance_species_nm3(
        catalog: &dyn SpeciesCatalogPort,
        sp: &mut SpeciesRow,
        new_nm3: f64,
    ) {
        domain_set_species_nm3(sp, new_nm3, catalog);
    }

    pub fn scale_balance_stream_kmol(stream: &mut StreamGroup, new_total_kmol: f64) {
        domain_scale_stream_kmol(stream, new_total_kmol);
    }

    pub fn scale_balance_stream_kg(stream: &mut StreamGroup, new_total_kg: f64) {
        domain_scale_stream_kg(stream, new_total_kg);
    }

    pub fn scale_balance_stream_nm3(stream: &mut StreamGroup, new_total_nm3: f64) {
        domain_scale_stream_nm3(stream, new_total_nm3);
    }

    pub fn convert_balance_temperature_unit(
        wb: &mut BalanceWorkbook,
        new_unit: soryntel_core::TemperatureUnit,
    ) {
        domain_convert_wb_temp_unit(wb, new_unit);
    }

    pub fn convert_balance_energy_unit(
        wb: &mut BalanceWorkbook,
        new_unit: soryntel_core::BalanceEnergyUnit,
    ) {
        domain_convert_wb_energy_unit(wb, new_unit);
    }

    pub fn build_balance_linked_paste_stream(
        wb: &BalanceWorkbook,
        src_area_idx: usize,
        src_is_out: bool,
        src_stream_idx: usize,
    ) -> Option<StreamGroup> {
        domain_build_linked_paste_stream(wb, src_area_idx, src_is_out, src_stream_idx)
    }

    pub fn export_balance_workbook_xlsx(wb: &BalanceWorkbook) -> Vec<u8> {
        let sheets = domain_build_balance_export_sheets(wb);
        build_xlsx(&sheets)
    }

    pub fn serialize_bal8_workbook(wb: &BalanceWorkbook) -> Result<String, SoryntelError> {
        domain_serialize_bal8_workbook(wb)
    }

    pub fn deserialize_bal8_workbook(raw: &str) -> Result<BalanceWorkbook, SoryntelError> {
        domain_deserialize_bal8_workbook(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_preset_has_expected_identity() {
        let workbook = BalanceUseCases::balance_preset_cuconv1();
        assert!(!workbook.title.is_empty());
        assert!(!workbook.areas.is_empty());
    }

    #[test]
    fn bal8_roundtrip_preserves_preset() {
        let workbook = BalanceUseCases::balance_preset_cuconv1();
        let raw = BalanceUseCases::serialize_bal8_workbook(&workbook).unwrap();
        let restored = BalanceUseCases::deserialize_bal8_workbook(&raw).unwrap();
        assert_eq!(restored.title, workbook.title);
        assert_eq!(restored.areas.len(), workbook.areas.len());
    }
}
