//! Casos de uso agrupados: addins.
use super::super::*;
pub struct AddinsUseCases;
impl AddinsUseCases {
    /// Formato numérico de celda de los add-ins (`#NUM!` para valores no finitos).
    pub fn format_cell_number(v: f64, decimals: usize) -> String {
        soryntel_thermo::excel_addins::format_cell_number(v, decimals)
    }

    /// Etiqueta de unidad de temperatura para mostrar (p. ej. `°C`, `K`).
    pub fn format_temp_unit_display(u: &str) -> String {
        soryntel_thermo::excel_addins::format_temp_unit_display(u)
    }

    /// Analiza una referencia A1 (admite `$`) como `(fila, columna)` con base 0.
    pub fn parse_a1(token: &str) -> Option<(usize, usize)> {
        let (column, row) = token.trim().split_at(
            token.trim().find(|c: char| c.is_ascii_digit())?,
        );
        let column = column.strip_prefix('$').unwrap_or(column);
        let column = column.strip_suffix('$').unwrap_or(column);
        if column.is_empty() || !row.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let column = column.bytes().try_fold(0usize, |value, byte| {
            if !byte.is_ascii_alphabetic() {
                return None;
            }
            value.checked_mul(26)?.checked_add(usize::from(byte.to_ascii_uppercase() - b'A') + 1)
        })?;
        let row = row.parse::<usize>().ok()?;
        if row > 1_048_576 || column > 16_384 {
            return None;
        }
        Some((row.checked_sub(1)?, column.checked_sub(1)?))
    }

    /// Analiza A1 dentro de los límites de la hoja activa.
    pub fn parse_a1_in_sheet(token: &str, rows: usize, columns: usize) -> Option<(usize, usize)> {
        Self::parse_a1(token).filter(|&(row, column)| row < rows && column < columns)
    }

    /// Letra(s) de columna A1 para el índice de columna con base 0.
    pub fn col_to_a1(col: usize) -> String {
        soryntel_thermo::spreadsheet_export::col_to_a1(col)
    }

    fn is_range_slot(fn_upper: &str, arg_idx: usize) -> bool {
        if fn_upper.starts_with("STREAM") || fn_upper.starts_with("FLOW") || fn_upper == "DENSITYA"
        {
            arg_idx <= 1 && !matches!(fn_upper, "STREAMX" | "STREAMMASS" | "STREAMVOL")
        } else {
            matches!(fn_upper, "SUM" | "AVERAGE" | "MIN" | "MAX")
        }
    }

    pub fn parse_text_argument(fn_upper: &str, arg_idx: usize, raw: &str) -> ArgValue {
        let t = raw.trim();
        if t.contains('|') {
            let items = t.split('|').map(|s| s.trim().to_string()).collect();
            return ArgValue::Range(items);
        }
        if Self::is_range_slot(fn_upper, arg_idx) {
            let sep = if t.contains(';') {
                ';'
            } else if t.contains(',') {
                ','
            } else {
                '\0'
            };
            if sep != '\0' {
                let items = t.split(sep).map(|s| s.trim().to_string()).collect();
                return ArgValue::Range(items);
            }
            return ArgValue::Range(vec![t.to_string()]);
        }
        ArgValue::Scalar(t.to_string())
    }

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
