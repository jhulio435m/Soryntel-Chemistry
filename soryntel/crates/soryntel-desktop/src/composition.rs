//! Raíz de composición: crea los adaptadores y los servicios de aplicación.

use soryntel_core::SpeciesWorkspacePort;
use soryntel_application::use_cases::{
    AddinsUseCases, AquaUseCases, BalanceUseCases, CatalogUseCases, EquilibriumUseCases,
    HeatLossUseCases, MiscUseCases, ReactionUseCases, SimulationUseCases, WaterUseCases,
};
use soryntel_db::ThermoDatabase;
use std::ops::{Deref, DerefMut};
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

    /// Préstamo de lectura del catálogo visto como puerto. El guard mantiene el lock de
    /// lectura del `RwLock`: no debe vivir más allá de la sentencia o bloque que lo usa ni
    /// solaparse con [`Self::catalog_mut`] en el mismo hilo (eso bloquearía el hilo).
    pub fn catalog(&self) -> CatalogRead<'_> {
        CatalogRead(self.catalog.read().unwrap_or_else(|e| e.into_inner()))
    }

    /// Préstamo de escritura del catálogo visto como puerto. Mismas reglas que
    /// [`Self::catalog`]: suéltese antes de tomar cualquier otro guard.
    pub fn catalog_mut(&self) -> CatalogWrite<'_> {
        CatalogWrite(self.catalog.write().unwrap_or_else(|e| e.into_inner()))
    }

    /// Ejecuta `f` con el catálogo en lectura; el lock se suelta al volver.
    pub fn with_catalog<T>(&self, f: impl FnOnce(&dyn SpeciesWorkspacePort) -> T) -> T {
        f(&*self.catalog())
    }

    /// Ejecuta `f` con el catálogo en escritura; el lock se suelta al volver.
    pub fn with_catalog_mut<T>(&self, f: impl FnOnce(&mut dyn SpeciesWorkspacePort) -> T) -> T {
        f(&mut *self.catalog_mut())
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
}

/// Guard de lectura del catálogo compartido que solo expone el puerto.
pub struct CatalogRead<'a>(RwLockReadGuard<'a, ThermoDatabase>);

impl<'a> Deref for CatalogRead<'a> {
    type Target = dyn SpeciesWorkspacePort + 'a;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Guard de escritura del catálogo compartido que solo expone el puerto.
pub struct CatalogWrite<'a>(RwLockWriteGuard<'a, ThermoDatabase>);

impl<'a> Deref for CatalogWrite<'a> {
    type Target = dyn SpeciesWorkspacePort + 'a;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for CatalogWrite<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
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
