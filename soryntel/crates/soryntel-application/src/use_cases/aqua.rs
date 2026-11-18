//! Casos de uso agrupados: aqua.
use super::super::*;
pub struct AquaUseCases;
impl AquaUseCases {
    pub fn preset_nacl_hcl() -> AquaCaseFile {
        AquaCaseFile::preset_nacl_hcl()
    }

    pub fn preset_millero_seawater() -> AquaCaseFile {
        AquaCaseFile::preset_millero_seawater()
    }

    pub fn preset_na_s_h2o_fig3() -> AquaCaseFile {
        AquaCaseFile::preset_na_s_h2o_fig3()
    }

    pub fn aqua_main_pitzer_db() -> PitzerDatabase {
        domain_aqua_main_pitzer_db()
    }

    pub fn aqua_calculate(
        catalog: &dyn SpeciesCatalogPort,
        req: &AquaCaseFile,
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
    ) -> AquaCalcResult {
        let mut result = domain_aqua_calculate(req, catalog, main_db, own_db, excluded_keys);
        if req.method == crate::models::aqua::AquaMethod::Davies {
            return result;
        }

        let report = domain_aqua_analyze_parameter_sets(req, main_db, own_db, excluded_keys);
        let temp_c = req.temp_unit.to_c(result.summary.temp_disp);
        let ionic_strength = result.summary.ionic_strength;
        let mut check = |label: String, range: &crate::models::aqua::ValidityRange| {
            let outside = |value, minimum: Option<f64>, maximum: Option<f64>| {
                minimum.is_some_and(|min| value < min - 1e-9)
                    || maximum.is_some_and(|max| value > max + 1e-9)
            };
            if outside(temp_c, range.min_t_c, range.max_t_c) {
                result.warnings.push(format!(
                    "Pitzer {label}: {temp_c:.2} °C fuera del rango de temperatura del parámetro."
                ));
            }
            if outside(ionic_strength, range.min_i, range.max_i) {
                result.warnings.push(format!(
                    "Pitzer {label}: I = {ionic_strength:.3} mol/kg fuera del rango de fuerza iónica del parámetro."
                ));
            }
        };
        for parameter in report.active_binaries {
            let entry = parameter.entry;
            check(format!("{}/{}", entry.cation, entry.anion), &entry.range);
        }
        for parameter in report.active_thetas {
            let entry = parameter.entry;
            check(format!("{}/{}", entry.ion_i, entry.ion_j), &entry.range);
        }
        for parameter in report.active_psis {
            let entry = parameter.entry;
            check(
                format!("{}/{}/{}", entry.ion1, entry.ion2, entry.ion3),
                &entry.range,
            );
        }
        for parameter in report.active_lambdas {
            let entry = parameter.entry;
            check(format!("{}/{}", entry.neutral, entry.ion), &entry.range);
        }
        result
    }

    pub fn aqua_analyze_params(
        req: &AquaCaseFile,
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
    ) -> ActiveParameterSetsReport {
        domain_aqua_analyze_parameter_sets(req, main_db, own_db, excluded_keys)
    }

    pub fn aqua_resolve_catalog(
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
        use_own_db: bool,
    ) -> MergedPitzerCatalog {
        domain_aqua_resolve_catalog(main_db, own_db, excluded_keys, use_own_db)
    }

    pub fn aqua_density(rows: &[DensityInputRow], t_c: f64) -> DensityCalcResult {
        domain_aqua_density(rows, t_c)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use soryntel_db::ThermoDatabase;

    #[test]
    fn pitzer_warns_past_the_binary_temperature_limit() {
        let catalog = ThermoDatabase::with_open_defaults();
        let mut request = AquaUseCases::preset_nacl_hcl();
        let main = AquaUseCases::aqua_main_pitzer_db();
        let own = OwnPitzerDb::default();
        let excluded = BTreeSet::new();
        for (temperature, expected) in [(300.0, false), (301.0, true)] {
            request.species[0].temp = temperature;
            let result = AquaUseCases::aqua_calculate(&catalog, &request, &main, &own, &excluded);
            assert_eq!(
                result.warnings.iter().any(|warning| warning.contains("Na(+a)/Cl(-a)") && warning.contains("temperatura")),
                expected,
                "{:?}",
                result.warnings,
            );
        }
    }
}
