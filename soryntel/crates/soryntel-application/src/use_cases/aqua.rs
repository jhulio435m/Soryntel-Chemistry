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
        domain_aqua_calculate(req, catalog, main_db, own_db, excluded_keys)
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
