//! Casos de uso agrupados: addins.
use super::super::*;
pub struct AddinsUseCases;
impl AddinsUseCases {
    pub fn addin_catalog() -> &'static [AddInFunctionMeta] {
        ADDIN_FUNCTION_CATALOG
    }

    pub fn eval_addin_call(
        catalog: &dyn SpeciesCatalogPort,
        units: &mut AddInUnits,
        fn_name: &str,
        args: &[ArgValue],
    ) -> CellEvalValue {
        eval_addin_function(catalog, units, fn_name, args)
    }

    pub fn eval_addin_formula<F>(
        catalog: &dyn SpeciesCatalogPort,
        units: &mut AddInUnits,
        formula: &str,
        lookup: F,
    ) -> CellEvalValue
    where
        F: FnMut(usize, usize) -> String,
    {
        eval_formula_string(catalog, units, formula, lookup)
    }

    pub fn eval_addin_streameq(
        catalog: &dyn SpeciesCatalogPort,
        units: &AddInUnits,
        species: &[String],
        inputs: &[String],
        t_val: f64,
        p_val: f64,
    ) -> CellEvalValue {
        eval_streameq(catalog, units, species, inputs, t_val, p_val)
    }

    pub fn lookup_addin_mineral(query: &str) -> Option<MineralRecord> {
        lookup_mineral(query)
    }

    pub fn migrate_addin_formula(raw: &str) -> (String, Vec<String>) {
        migrate_legacy_formula(raw)
    }

    pub fn addin_webservice_formula(port: u16, fn_name: &str, args: &[&str]) -> String {
        build_webservice_formula(port, fn_name, args)
    }

    pub fn addin_streameq_sheet(
        catalog: &dyn SpeciesCatalogPort,
        units: &AddInUnits,
        species: &[String],
        inputs: &[String],
        t_val: f64,
        p_val: f64,
    ) -> ExportSheet {
        build_streameq_export_sheet(catalog, units, species, inputs, t_val, p_val)
    }

    pub fn addin_minerals_sheet() -> ExportSheet {
        build_minerals_export_sheet()
    }

    pub fn addin_sample_sheet(catalog: &dyn SpeciesCatalogPort, units: &AddInUnits) -> ExportSheet {
        build_addinsample_export_sheet(catalog, units)
    }

    pub fn addin_streameq_balance(
        species: &[String],
        inputs: &[String],
        outputs: &[CellEvalValue],
    ) -> Vec<(String, f64, f64)> {
        streameq_element_balance(species, inputs, outputs)
    }

    pub fn export_workbook_xlsx(sheets: &[ExportSheet]) -> Vec<u8> {
        build_xlsx(sheets)
    }

    pub fn export_workbook_ods(sheets: &[ExportSheet]) -> Vec<u8> {
        build_ods(sheets)
    }

    pub fn export_libreoffice_oxt(port: u16) -> Vec<u8> {
        build_libreoffice_oxt(port)
    }

    pub fn export_onlyoffice_plugin(port: u16) -> Vec<u8> {
        build_onlyoffice_plugin_zip(port)
    }

    pub fn export_excel_wps_vba(port: u16) -> String {
        generate_excel_wps_vba(port)
    }
}
