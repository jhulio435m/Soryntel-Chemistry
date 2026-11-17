//! Raíz de composición: crea los adaptadores y los servicios de aplicación.

use soryntel_application::ports::SpeciesCatalogPort;
use soryntel_application::use_cases::{
    AddinsUseCases, AquaUseCases, BalanceUseCases, CatalogUseCases, EquilibriumUseCases,
    HeatLossUseCases, MiscUseCases, ReactionUseCases, SimulationUseCases, WaterUseCases,
};
use soryntel_db::ThermoDatabase;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[allow(dead_code)]
pub struct AppServices {
    catalog: Arc<RwLock<ThermoDatabase>>,
    reaction: ReactionUseCases,
    catalog_use_cases: CatalogUseCases,
    equilibrium: EquilibriumUseCases,
    balance: BalanceUseCases,
    heat_loss: HeatLossUseCases,
    aqua: AquaUseCases,
    water: WaterUseCases,
    simulation: SimulationUseCases,
    addins: AddinsUseCases,
    misc: MiscUseCases,
}

#[allow(dead_code)]
impl AppServices {
    pub fn load(custom_main_db: Option<&str>) -> Self {
        let mut catalog = ThermoDatabase::with_open_defaults();
        let executable_db_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("Databases")));

        let main_db_path = std::env::var_os("SORYNTEL_MAIN_DB")
            .map(PathBuf::from)
            .or_else(|| custom_main_db.map(PathBuf::from))
            .or_else(|| {
                default_database_path(
                    executable_db_dir.as_deref(),
                    "Maindb6.HSC",
                    "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC",
                )
            });
        if let Some(path) = main_db_path {
            let _ = catalog.load_main_hsc(path);
        }

        let own_db_path = std::env::var_os("SORYNTEL_OWN_DB")
            .map(PathBuf::from)
            .or_else(|| {
                default_database_path(
                    executable_db_dir.as_deref(),
                    "OWNDB6.HSC",
                    "/home/nando/Descargas/HSC6/Databases/OWNDB6.HSC",
                )
            });
        if let Some(path) = own_db_path {
            let _ = catalog.load_own_hsc(path);
        }

        Self::from_catalog(catalog)
    }

    pub fn from_catalog(catalog: ThermoDatabase) -> Self {
        Self {
            catalog: Arc::new(RwLock::new(catalog)),
            reaction: ReactionUseCases,
            catalog_use_cases: CatalogUseCases,
            equilibrium: EquilibriumUseCases,
            balance: BalanceUseCases,
            heat_loss: HeatLossUseCases,
            aqua: AquaUseCases,
            water: WaterUseCases,
            simulation: SimulationUseCases,
            addins: AddinsUseCases,
            misc: MiscUseCases,
        }
    }

    pub fn shared_catalog(&self) -> Arc<RwLock<ThermoDatabase>> {
        Arc::clone(&self.catalog)
    }

    pub fn with_catalog<T>(&self, f: impl FnOnce(&dyn SpeciesCatalogPort) -> T) -> T {
        f(&*self.legacy_db())
    }

    pub fn with_catalog_mut<T>(&self, f: impl FnOnce(&mut dyn SpeciesCatalogPort) -> T) -> T {
        f(&mut *self.legacy_db_mut())
    }

    pub fn reaction(&self) -> &ReactionUseCases {
        &self.reaction
    }

    pub fn catalog_use_cases(&self) -> &CatalogUseCases {
        &self.catalog_use_cases
    }

    pub fn equilibrium(&self) -> &EquilibriumUseCases {
        &self.equilibrium
    }

    pub fn balance(&self) -> &BalanceUseCases {
        &self.balance
    }

    pub fn heat_loss(&self) -> &HeatLossUseCases {
        &self.heat_loss
    }

    pub fn aqua(&self) -> &AquaUseCases {
        &self.aqua
    }

    pub fn water(&self) -> &WaterUseCases {
        &self.water
    }

    pub fn simulation(&self) -> &SimulationUseCases {
        &self.simulation
    }

    pub fn addins(&self) -> &AddinsUseCases {
        &self.addins
    }

    pub fn misc(&self) -> &MiscUseCases {
        &self.misc
    }

    // TODO(hex): retirar cuando los módulos UI usen puertos
    pub fn legacy_db(&self) -> RwLockReadGuard<'_, ThermoDatabase> {
        self.catalog.read().unwrap_or_else(|e| e.into_inner())
    }

    // TODO(hex): retirar cuando los módulos UI usen puertos
    pub fn legacy_db_mut(&self) -> RwLockWriteGuard<'_, ThermoDatabase> {
        self.catalog.write().unwrap_or_else(|e| e.into_inner())
    }
}

fn default_database_path(
    executable_db_dir: Option<&Path>,
    file_name: &str,
    legacy_path: &str,
) -> Option<PathBuf> {
    executable_db_dir
        .map(|dir| dir.join(file_name))
        .filter(|path| path.exists())
        .or_else(|| {
            Path::new(legacy_path)
                .exists()
                .then(|| PathBuf::from(legacy_path))
        })
}
