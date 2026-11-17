use soryntel_core::{
    DatabaseLocatorPort, MainDbPrecedence, SpeciesCatalogPort, SpeciesDatabaseAdminPort,
};
use soryntel_db::{adapters::DatabaseDirLocator, ThermoDatabase};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Resuelve OwnDB > MainDB > OpenDefault, igual que el escritorio.
/// Antes OpenDefault tenía prioridad sobre MainDB en el CLI.
pub(crate) fn catalog(custom_main: Option<&str>) -> Arc<dyn SpeciesCatalogPort + Send + Sync> {
    let locator = DatabaseDirLocator::new(
        DatabaseDirLocator::beside_executable()
            .dirs()
            .iter()
            .cloned()
            .chain([PathBuf::from("/home/nando/Descargas/HSC6/Databases")]),
    );
    let main = std::env::var_os("SORYNTEL_MAIN_DB")
        .map(PathBuf::from)
        .or_else(|| custom_main.map(PathBuf::from))
        .or_else(|| locator.default_main_db());
    let own = std::env::var_os("SORYNTEL_OWN_DB")
        .map(PathBuf::from)
        .or_else(|| locator.default_own_db());
    Arc::new(load_catalog(main.as_deref(), own.as_deref()))
}

fn load_catalog(main: Option<&Path>, own: Option<&Path>) -> ThermoDatabase {
    let mut catalog = ThermoDatabase::with_open_defaults();
    let admin: &mut dyn SpeciesDatabaseAdminPort = &mut catalog;
    if let Some(path) = main {
        match admin.load_main_db(path, MainDbPrecedence::OverrideOpenDefault) {
            Ok((count, _)) => eprintln!(
                "[soryntel-db] Loaded {count} species from {} (read-only)",
                path.display()
            ),
            Err(e) => eprintln!("[soryntel-db] Warning: {e}"),
        }
    }
    if let Some(path) = own {
        match admin.load_own_db(path) {
            Ok((count, _)) => eprintln!(
                "[soryntel-db] Loaded {count} species from {} (read-only)",
                path.display()
            ),
            Err(e) => eprintln!("[soryntel-db] Warning: {e}"),
        }
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_core::{DbSource, SpeciesDataset, SpeciesRepository};
    use soryntel_db::hsc_binary::serialize_datasets_to_hsc;

    #[test]
    fn cli_precedence_and_http_response() {
        let dir =
            std::env::temp_dir().join(format!("soryntel-app-precedence-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let main = dir.join("Maindb6.HSC");
        let own = dir.join("OWNDB6.HSC");
        let mut old = ThermoDatabase::with_open_defaults();
        let base = old.get_species("Al2O3").unwrap();
        let h0 = base.intervals[0].h_or_delta_h_tr;
        for (path, source, shift) in [
            (&main, DbSource::Main, -10_000.0),
            (&own, DbSource::Own, -20_000.0),
        ] {
            let mut species = base.clone();
            species.intervals[0].h_or_delta_h_tr += shift;
            let dataset = SpeciesDataset {
                source,
                species,
                metadata: Default::default(),
                dataset_index: 0,
                raw_phase_code: "s".into(),
            };
            std::fs::write(path, serialize_datasets_to_hsc(&[dataset])).unwrap();
        }
        old.load_main_db(&main, MainDbPrecedence::PreserveOpenDefault)
            .unwrap();
        let new = load_catalog(Some(&main), None);
        let layered = load_catalog(Some(&main), Some(&own));
        for (catalog, shift) in [(&old, 0.0), (&new, -10_000.0), (&layered, -20_000.0)] {
            let h = catalog.get_species("Al2O3").unwrap().intervals[0].h_or_delta_h_tr;
            assert!((h - h0 - shift).abs() < 0.5);
        }

        // Compare the real handler with the same request and fixture under both policies.
        let request = crate::http::calculation_request("Al2O3");
        let (before_status, before_type, before) = crate::http::dispatch(&request, &old);
        let (after_status, after_type, after) = crate::http::dispatch(&request, &new);
        assert_eq!((before_status, before_type), ("200 OK", "application/json"));
        assert_eq!((before_status, before_type), (after_status, after_type));
        let before: serde_json::Value = serde_json::from_str(&before).unwrap();
        let after: serde_json::Value = serde_json::from_str(&after).unwrap();
        let mut normalized_after = after.clone();
        for field in ["cp", "g", "h", "s"] {
            normalized_after["substance_rows"][0][field] =
                before["substance_rows"][0][field].clone();
        }
        for field in ["mass_g", "molar_mass", "volume"] {
            normalized_after["species_data"][0][field] = before["species_data"][0][field].clone();
        }
        assert_eq!(before, normalized_after);
        let h = |response: &serde_json::Value| response["substance_rows"][0]["h"].as_f64().unwrap();
        assert!((h(&after) - h(&before) + 10.0).abs() < 0.001);
        println!("ANTES {before}\nDESPUÉS {after}");
    }
}
