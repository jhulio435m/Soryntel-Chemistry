//! Raíz de composición: crea los adaptadores y los servicios de aplicación.

use soryntel_core::{SpeciesDatabaseAdminPort, SpeciesWorkspacePort};
use soryntel_application::use_cases::{
    AddinsUseCases, AquaUseCases, BalanceUseCases, CatalogUseCases, EquilibriumUseCases,
    HeatLossUseCases, MiscUseCases, ReactionUseCases, SimulationUseCases, WaterUseCases,
};
use soryntel_core::DatabaseLocatorPort;
use soryntel_db::adapters::{DatabaseDirLocator, OWN_DB_FILE};
use soryntel_db::ThermoDatabase;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Aviso del arranque sin SoryntelDB.
pub const SORYNTEL_DB_MISSING: &str = "SoryntelDB no encontrada; catálogo limitado";
const SORYNTEL_DB_HINT: &str = "Coloque soryntel.sdb en Databases/ junto al ejecutable o en \
     Soryntel/Databases/ de la carpeta de documentos, o defina SORYNTEL_SDB.";

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
    load_warnings: Vec<String>,
}

#[allow(dead_code)]
impl AppServices {
    pub fn load(custom_main_db: Option<&str>) -> Self {
        // SoryntelDB (ADR 0007): `SORYNTEL_SDB` > `<exe>/Databases/soryntel.sdb` > carpeta de
        // datos del usuario. Precedencia de MainDB (ADR 0003): `--hsc-db` > `SORYNTEL_MAIN_DB` >
        // `<exe>/Databases/Maindb6.HSC` > carpeta de datos del usuario. Una ruta configurada
        // explícitamente que no carga no recurre a las rutas por defecto: el error se muestra
        // en la interfaz para que el usuario corrija la configuración en lugar de trabajar
        // sin saberlo con otra base.
        let locator = DatabaseDirLocator::standard();
        let main_db_path = custom_main_db
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("SORYNTEL_MAIN_DB").map(PathBuf::from))
            .or_else(|| locator.default_main_db());
        let own_db_path = std::env::var_os("SORYNTEL_OWN_DB")
            .map(PathBuf::from)
            .or_else(|| locator.default_own_db());
        let soryntel_db_path = locator.default_soryntel_db();
        Self::load_from_paths(
            soryntel_db_path.as_deref(),
            main_db_path.as_deref(),
            own_db_path.as_deref(),
        )
    }

    /// Carga el catálogo con las rutas ya resueltas (OwnDB > MainDB > SoryntelDB >
    /// OpenDefault). Los fallos no detienen el arranque: quedan en [`Self::load_warnings`]
    /// para mostrarlos en la interfaz (además de stderr). MainDB es opcional; sin SoryntelDB
    /// se avisa de que el catálogo está limitado.
    pub fn load_from_paths(
        soryntel_db: Option<&Path>,
        main_db: Option<&Path>,
        own_db: Option<&Path>,
    ) -> Self {
        let mut catalog = ThermoDatabase::with_open_defaults();
        let mut warnings = Vec::new();
        match soryntel_db {
            Some(path) => {
                if let Err(message) = CatalogUseCases::open_soryntel_db(&mut catalog, path) {
                    warnings.push(format!(
                        "No se pudo cargar SoryntelDB {}: {message}. Catálogo limitado.",
                        path.display()
                    ));
                }
            }
            None => warnings.push(format!("{SORYNTEL_DB_MISSING}. {SORYNTEL_DB_HINT}")),
        }
        if let Some(path) = main_db {
            if let Err(message) = CatalogUseCases::open_main_db(&mut catalog, path) {
                warnings.push(format!("No se pudo cargar MainDB {}: {message}", path.display()));
            }
        }
        if let Some(path) = own_db {
            if let Err(message) = CatalogUseCases::open_own_db(&mut catalog, path) {
                warnings.push(format!("No se pudo cargar OwnDB {}: {message}", path.display()));
            }
        }
        for warning in &warnings {
            eprintln!("[soryntel-db] Warning: {warning}");
        }
        let mut services = Self::from_catalog(catalog);
        services.load_warnings = warnings;
        services
    }

    /// Avisos de carga de las bases (SoryntelDB/MainDB/OwnDB) para mostrar en la interfaz.
    pub fn load_warnings(&self) -> &[String] {
        &self.load_warnings
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
            load_warnings: Vec::new(),
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

/// Directorio `Databases/` junto al ejecutable, si se puede determinar.
fn executable_db_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("Databases")))
}

/// Ruta de OwnDB que usa el escritorio cuando no hay ninguna cargada, con la misma búsqueda
/// que el arranque: `SORYNTEL_OWN_DB`, un `OWNDB6.HSC` existente junto al ejecutable o en la
/// carpeta de datos del usuario y, si no hay ninguno, `<exe>/Databases/OWNDB6.HSC` para
/// crearlo. Nunca devuelve el directorio temporal, que el arranque no consulta.
pub fn default_own_db_path() -> Option<PathBuf> {
    std::env::var_os("SORYNTEL_OWN_DB")
        .map(PathBuf::from)
        .or_else(|| DatabaseDirLocator::standard().default_own_db())
        .or_else(|| executable_db_dir().map(|dir| dir.join(OWN_DB_FILE)))
}

/// Prepara el archivo OwnDB donde se persistirá una especie nueva y devuelve su ruta.
///
/// Debe llamarse **antes** de añadir la especie a la capa OwnDB: si el archivo destino
/// existe y no es el cargado, se abre (para no sobrescribir su contenido); si no existe, se
/// crea vacío con `create_empty_own_db`. `requested` vacío usa la OwnDB cargada o
/// [`default_own_db_path`].
pub fn prepare_own_db_target(
    admin: &mut dyn SpeciesDatabaseAdminPort,
    requested: &str,
) -> Result<PathBuf, String> {
    let loaded = CatalogUseCases::status(admin).own_path;
    let path = match requested.trim() {
        "" => loaded
            .clone()
            .or_else(default_own_db_path)
            .ok_or_else(|| "No hay ninguna ruta de OwnDB disponible.".to_string())?,
        text => PathBuf::from(text),
    };
    if loaded.as_deref() == Some(path.as_path()) && path.is_file() {
        return Ok(path);
    }
    if path.is_file() {
        CatalogUseCases::open_own_db(admin, &path)
            .map_err(|e| format!("No se pudo abrir OwnDB {}: {e}", path.display()))?;
    } else {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("No se pudo crear la carpeta de OwnDB {}: {e}", parent.display())
            })?;
        }
        CatalogUseCases::create_empty_own_db(admin, &path)
            .map_err(|e| format!("No se pudo crear OwnDB {}: {e}", path.display()))?;
    }
    Ok(path)
}

/// Persiste la capa OwnDB en `path` (archivo `.HSC` + `.CAT`).
pub fn persist_own_db(admin: &mut dyn SpeciesDatabaseAdminPort, path: &Path) -> Result<usize, String> {
    CatalogUseCases::save_own_db(admin, path)
        .map_err(|e| format!("No se pudo guardar OwnDB {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("soryntel-comp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn database_load_failures_are_kept_for_the_ui() {
        let services = AppServices::load_from_paths(
            Some(Path::new("/nonexistent/soryntel.sdb")),
            Some(Path::new("/nonexistent/ñ dir/Maindb6.HSC")),
            Some(Path::new("/nonexistent/OWNDB6.HSC")),
        );
        let warnings = services.load_warnings();
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings[0].contains("SoryntelDB") && warnings[0].contains("limitado"));
        assert!(warnings[1].contains("MainDB") && warnings[1].contains("ñ dir"));
        assert!(warnings[2].contains("OwnDB"));
        let none = AppServices::load_from_paths(None, None, None);
        assert_eq!(none.load_warnings().len(), 1, "sin SoryntelDB se avisa; MainDB es opcional");
        assert!(none.load_warnings()[0].starts_with(SORYNTEL_DB_MISSING));
    }

    #[test]
    fn soryntel_db_loads_at_startup_without_warnings() {
        use soryntel_core::DbSource;
        use soryntel_db::sdb;
        let dir = scratch("sdb");
        std::fs::create_dir_all(&dir).unwrap();
        let mut base = ThermoDatabase::with_open_defaults()
            .get_all_datasets_for(DbSource::OpenDefault, "O2(g)");
        base[0].species.intervals[0].h_or_delta_h_tr += 1_000.0;
        let file = sdb::build_sdb(
            vec![base],
            Vec::new(),
            sdb::SdbSource { name: "sintética".into(), files: Vec::new() },
            "aviso",
            "2026-01-01T00:00:00Z",
        );
        let path = dir.join("soryntel.sdb");
        sdb::write_sdb(&path, &file).unwrap();
        let services = AppServices::load_from_paths(Some(&path), None, None);
        assert!(services.load_warnings().is_empty(), "{:?}", services.load_warnings());
        let catalog = services.catalog();
        assert_eq!(catalog.resolved_source_of("O2(g)"), Some(DbSource::Soryntel));
        assert!(catalog.get_species("Al2O3").is_none(), "OpenDefault no se usa con SoryntelDB");
        drop(catalog);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_own_db_path_is_never_the_temp_dir() {
        if std::env::var_os("SORYNTEL_OWN_DB").is_none() {
            let path = default_own_db_path().expect("ruta por defecto");
            assert!(!path.starts_with(std::env::temp_dir()), "{}", path.display());
            assert!(path.ends_with("OWNDB6.HSC"));
        }
    }

    #[test]
    fn prepare_creates_missing_own_db_and_keeps_existing_contents() {
        use soryntel_core::{ChemicalSpecies, SpeciesCatalogPort, SpeciesRepository};
        let dir = scratch("own");
        let path = dir.join("sub dir ñ").join("OWNDB6.HSC");
        let mut db = ThermoDatabase::with_open_defaults();
        let target = prepare_own_db_target(&mut db, path.to_str().unwrap()).unwrap();
        assert_eq!(target, path);
        assert!(path.is_file(), "se crea el archivo vacío");
        let mut sp = db.get_species("Al2O3").unwrap();
        sp.formula = "XyzO2".into();
        db.insert_or_update_species(ChemicalSpecies { name: "Prueba".into(), ..sp });
        persist_own_db(&mut db, &target).unwrap();

        // Un proceso nuevo que guarda otra especie en el mismo archivo conserva la anterior.
        let mut other = ThermoDatabase::with_open_defaults();
        let target = prepare_own_db_target(&mut other, path.to_str().unwrap()).unwrap();
        assert!(other.get_species("XyzO2").is_some(), "se abrió el archivo existente");
        persist_own_db(&mut other, &target).unwrap();
        let mut reread = ThermoDatabase::with_open_defaults();
        CatalogUseCases::open_own_db(&mut reread, &path).unwrap();
        assert!(reread.get_species("XyzO2").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
