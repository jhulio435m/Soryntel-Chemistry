# Inventario de acoplamientos UI → infraestructura/dominio

Base: `cd62cbb` (fork-main). Alcance: `crates/soryntel-desktop/src/**` (incluye `sim_ui/` y
`addins_bridge.rs`) y `crates/soryntel-app/src/main.rs`.

Qué se inventaría:

1. Cada uso de `ThermoDatabase` / `soryntel_db::` (tipo, constructor o método).
2. Cada llamada directa a funciones de `soryntel_thermo::` / `soryntel_solver::`, o a métodos de
   modelos de `soryntel_thermo` que hacen cálculo o E/S, **sin pasar por `SoryntelApplication`**.

No se listan una a una las importaciones de DTO/enums de `soryntel_thermo` que solo sirven para
construir peticiones o pintar resultados (p. ej. `CalculationRequest`, `Phase`,
`EquilibriumPointResult`). Cada tabla las resume en una línea de «tipos importados». Las llamadas
que ya pasan por `SoryntelApplication::…` (con `db` como `&dyn SpeciesCatalogPort`) no son
acoplamientos y no aparecen.

Firmas sugeridas: los casos de uso reciben el puerto por constructor, por ejemplo
`ReactionUseCases::new(catalog: Arc<dyn SpeciesCatalogPort>)`. Los nombres de struct
(`CatalogUseCases`, `EquilibriumUseCases`, …) son una propuesta y siguen las capacidades
`reaction, catalog, equilibrium, balance, heat_loss, aqua, water, simulation, addins, misc`.
Si el diseño final usa otros nombres, basta con cambiarlos; la correspondencia es lo que importa.

Columna **Tipo**: `búsqueda`, `lectura` (get puntual), `escritura` (muta el catálogo),
`archivo` (carga/guardado), `merge`, `cálculo`, `export`, `info` (conteo/estado), `infra`
(construcción/wiring), `test` (solo `#[cfg(test)]`).

---

## 1. Resumen ejecutivo

### Hallazgos principales

1. **`species_ui.rs` concentra el acoplamiento fuerte**: 27 usos directos y 21 métodos distintos
   de `ThermoDatabase`. Solo 2 están en `SpeciesCatalogPort` (`get_species`,
   `get_species_metadata`). Gestiona MainDB/OwnDB/.REF, merge de `.HSC`, conversión legacy,
   CSV, `.CAT`, datasets alternativos y citas. Es el origen de casi todos los puertos nuevos.
2. **`main.rs` (desktop) es la raíz de composición de facto**: crea `ThermoDatabase`, carga
   MainDB/OwnDB desde **rutas absolutas codificadas** (`/home/nando/Descargas/HSC6/Databases/…`,
   líneas 153 y 167) y guarda el `ThermoDatabase` concreto en `SoryntelDesktopApp.db`
   (línea 456). Desde ahí se pasa `&ThermoDatabase` / `&mut ThermoDatabase` a los 20 módulos.
3. **Siete módulos calculan sin pasar por la aplicación**, aunque `SoryntelApplication` ya ofrece
   la función equivalente: `equilibrium_ui` (`solve_equilibrium_sweep`, `collect_cp_warnings`),
   `diagrams_ui` (`calculate_diagram`), `ehph_ui` (`calculate_ehph`,
   `delta_g_formation_h2o_kcal`), `phase_stability_ui` (`calculate_phase_stability`),
   `estimate_ui` (`evaluate_estimate_row`, `generate_cp_curve`, `fit_kelley_cp`,
   `build_estimated_species`), `exergy_ui` (`ExergyWorkbook::calculate(db)`, 16 sitios) y
   `benson_ui` (`decompose_molecule_to_benson`, `MoleculeGraph::from_smiles`). Migrarlos es
   mecánico: todas esas funciones ya aceptan `&dyn SpeciesRepository` o `R: SpeciesRepository`.
4. **`soryntel-app/src/main.rs` no usa la capa de aplicación**. Llama directamente a
   `soryntel_thermo::calculate` y a `soryntel_solver::balance_reaction`, y carga la base con
   `load_external_hsc_readonly(path, false)`. Con ese flag, **`OpenDefault` gana a `MainDB`**
   (ver §4). El escritorio usa `load_main_hsc`, que sí aplica `OwnDB > MainDB > OpenDefault`,
   así que la CLI/servidor web y el escritorio pueden resolver la misma fórmula con datos
   distintos.
5. **El bridge de add-ins trabaja sobre una copia fija de la base.** En
   `excel_addins_ui.rs:1046`, `BridgeHandle::start(Arc::new(db.clone()), …)` clona la base
   entera; lo que se edite en OwnDB después de arrancar el bridge no llega a Excel/Calc.
   `addins_bridge.rs` está tipado con `Arc<ThermoDatabase>` / `&ThermoDatabase`, aunque solo
   usa `len()` y delega el resto en `SoryntelApplication`.
6. **Hay escrituras sobre el catálogo desde UI de cálculo**. `estimate_ui` (401, 1715) y
   `benson_ui`, este último vía `SoryntelApplication::save_benson_species(&mut db)`, insertan
   especies en OwnDB. `SpeciesCatalogPort::insert_or_update_species` es `&mut self`. Si los
   casos de uso reciben `Arc<dyn SpeciesCatalogPort>`, **no pueden mutar**. Hace falta decidir
   entre `Arc<RwLock<…>>` en el adaptador, un puerto de escritura aparte con `&self` y
   mutabilidad interior, o dejar las mutaciones en un caso de uso que reciba `&mut`.
7. **Módulos que ya están bien**: `elements_ui`, `species_converter_ui` y `water_ui` ya reciben
   `&dyn SpeciesCatalogPort`. `aqua_ui`, `balance_ui` y `reaction_ui` solo usan `ThermoDatabase`
   como tipo de parámetro, salvo un `search`/`len`. `heat_loss_ui`, `units_ui`, `lca_ui`,
   `mass_balance_ui` y `sim_ui/*` no tocan `ThermoDatabase`, pero llaman métodos de dominio de
   `soryntel_thermo` sobre sus workbooks (presets, conversiones, cálculos de LCA o de balance de
   masa).
8. **Tipos de infraestructura en la firma pública**. `species_ui` usa
   `soryntel_db::hsc_binary::ReferenceEntry` y tipos de `soryntel_thermo::species`
   (`DbSource`, `SpeciesCatalogEntry`, `PhaseFilterFlags`, `KeywordSearchField`,
   `ElementSearchMode`, `DbMergeConflict`, `MergeConflictDecision`). Para que los puertos nuevos
   vivan en `soryntel-core`, esos tipos (más `DatabaseStatusSummary` y `SpeciesDataset`) tienen
   que bajar a `soryntel-core`, igual que se hizo con `SpeciesMetadata` en `cd62cbb`.

### Métodos de `ThermoDatabase` que usa la UI

| Método `ThermoDatabase` | ¿En `SpeciesCatalogPort`? | Dónde (archivo:línea, sin tests) | Puerto propuesto |
|---|---|---|---|
| `with_open_defaults()` | n/a (constructor) | desktop/main.rs:87, app/main.rs:32 | raíz de composición (adaptador) |
| `get_species` | sí (`SpeciesRepository`) | species_ui:774, diagrams:84, equilibrium:1022/1513, excel_addins:685/691 | `SpeciesCatalogPort` |
| `get_species_metadata` | sí | species_ui:775 | `SpeciesCatalogPort` |
| `search` | sí (`search_species`) | diagrams:164/809, ehph:1752, estimate:1099/1137, excel_addins:1480/3703, balance:2795, exergy:1238, app/main:137 | `SpeciesCatalogPort` |
| `search_by_elements` | sí (`search_species_by_elements`) | ehph:293/420, phase_stability:163/182, equilibrium:2734 | `SpeciesCatalogPort` |
| `len` | sí (`species_count`) | desktop/main:774/819/1082, reaction:682, excel_addins:2160/4144, addins_bridge:135/675, app/main:106/132 | `SpeciesCatalogPort` |
| `insert_species` | sí (`insert_or_update_species`) | estimate:401/1715 | `SpeciesCatalogPort` (requiere `&mut`, ver hallazgo 6) |
| `clone` | no | excel_addins:1046 | compartir `Arc<dyn …>` en vez de clonar |
| `status_summary` | **no** | species_ui:216/736 | `SpeciesDatabaseAdminPort` |
| `load_main_hsc` | **no** | desktop/main:155/159/161/164, species_ui:1893 | `SpeciesDatabaseAdminPort` |
| `load_own_hsc` | **no** | desktop/main:171/173/176, species_ui:1907 | `SpeciesDatabaseAdminPort` |
| `load_external_hsc_readonly` | **no** | app/main:49 | `SpeciesDatabaseAdminPort` (con `overwrite` explícito) |
| `save_own_hsc` | **no** | species_ui:605/1200/1916 | `SpeciesDatabaseAdminPort` |
| `create_empty_own_hsc` | **no** | species_ui:1922 | `SpeciesDatabaseAdminPort` |
| `refresh_catalogue` | **no** | species_ui:687/688 | `SpeciesDatabaseAdminPort` |
| `sort_database` | **no** | species_ui:681 | `SpeciesDatabaseAdminPort` |
| `search_advanced` | **no** | species_ui:368 | `SpeciesBrowserPort` |
| `search_elements_filtered` | **no** | species_ui:392 | `SpeciesBrowserPort` |
| `search_stoichiometry` | **no** | species_ui:401 | `SpeciesBrowserPort` |
| `get_all_datasets_for` | **no** | species_ui:1004 | `SpeciesBrowserPort` |
| `select_dataset` | **no** | species_ui:1019 | `SpeciesBrowserPort` (escritura de selección) |
| `upsert_own_dataset` | **no** | species_ui:547 | `OwnDbEditorPort` |
| `delete_own_species` | **no** | species_ui:633/1207 | `OwnDbEditorPort` |
| `import_csv_into_own` | **no** | species_ui:2003 | `OwnDbEditorPort` |
| `export_csv` | **no** | species_ui:612/1994 | `OwnDbEditorPort` (o `SpeciesExportPort`) |
| `load_ref_dictionary` | **no** | species_ui:1936 | `ReferenceLibraryPort` |
| `lookup_citations` | **no** | species_ui:1143 | `ReferenceLibraryPort` |
| `search_references` | **no** | species_ui:2053 | `ReferenceLibraryPort` |
| `merge_hsc_files` (asociada) | **no** | species_ui:1954/2021 | `HscFileToolsPort` |
| `convert_legacy_to_hsc6` (asociada) | **no** | species_ui:1983 | `HscFileToolsPort` |

`ThermoDatabase` tiene más métodos públicos que hoy no usa ninguna UI: `insert_species_with_metadata`,
`resolved_source_of`, `save_main_hsc_copy` y `export_json`. Conviene incluirlos en los puertos
nuevos cuando haya una UI que los necesite (`save_main_hsc_copy` y `export_json` encajan en
`SpeciesDatabaseAdminPort` y en el puerto de exportación).

---

## 2. Tablas por archivo

### 2.1 `soryntel-desktop/src/main.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::excel_addins::{AddInUnits, ArgValue}`;
`soryntel_thermo::sim::model::SimProject` (campos 496 y 498).

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 87 | `ThermoDatabase::with_open_defaults()` | infra | Crea el catálogo con las especies integradas | Raíz de composición: `let catalog: Arc<RwLock<ThermoDatabase>> = …; let app = AppServices::new(catalog.clone())` |
| 155, 159, 161, 164 | `db.load_main_hsc(p)` | archivo | Carga MainDB desde `--hsc-db`, `<exe>/Databases/Maindb6.HSC` o la ruta fija `/home/nando/.../Maindb6.HSC` | `CatalogUseCases::open_main_db(&self, path: &Path) -> Result<(usize, usize), SoryntelError>` + `DatabaseLocatorPort::default_main_db() -> Option<PathBuf>` |
| 171, 173, 176 | `db.load_own_hsc(p)` | archivo | Carga OwnDB desde `<exe>/Databases/OWNDB6.HSC` o la ruta fija `/home/nando/.../OWNDB6.HSC` | `CatalogUseCases::open_own_db(&self, path: &Path) -> Result<(usize, usize), SoryntelError>` + `DatabaseLocatorPort::default_own_db()` |
| 180–184 | `Arc::new(db)` → `addins_bridge::run_bridge_blocking` | infra | Modo `--addin-bridge`: entrega la base al servidor HTTP | `AddinsUseCases::new(catalog)` compartido con el bridge |
| 182, 192, 203, 222, 226 | `soryntel_thermo::excel_addins::{AddInUnits::default, ArgValue::Range/Scalar}` | infra | Construye unidades y argumentos para `--addin-eval` | DTO; podría reexportarse desde `soryntel_application::addins` |
| 456 | campo `db: ThermoDatabase` | infra | Estado global de la app con el tipo concreto | `catalog: Arc<dyn SpeciesCatalogPort>` (o un `AppServices` con los casos de uso) |
| 505–525 | `XxxModuleState::new(&db)` (×13) | infra | Inicializa cada módulo con `&ThermoDatabase` | Cada módulo recibe su caso de uso |
| 586–711 | `self.xxx_state.ui(ctx, &self.db / &mut self.db)` | infra | Paso por frame de la base a cada UI | Ídem |
| 774, 819, 1082 | `self.db.len()` | info | Contador de especies en cabecera, rejilla y tarjetas del lanzador | `CatalogUseCases::species_count(&self) -> usize` |
| 1612, 1638 | `ThermoDatabase::with_open_defaults()` | test | Tests del lanzador | adaptador en memoria |

### 2.2 `soryntel-desktop/src/species_ui.rs` (navegador y gestor de bases)

Tipos importados: `soryntel_db::hsc_binary::ReferenceEntry`, `ThermoDatabase`;
`soryntel_thermo::species::{DbSource, SpeciesCatalogEntry, PhaseFilterFlags, KeywordSearchField,
ElementSearchMode, DbMergeConflict, MergeConflictDecision, CpFitMask, CpDataPoint, …}`;
`soryntel_thermo::elements::PERIODIC_ELEMENTS`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 216 | `db.status_summary()` | info | Rutas y contadores de MainDB/OwnDB/REF al crear el módulo | `CatalogUseCases::status(&self) -> DatabaseStatusSummary` |
| 366 | `parse_carbon_limits(..)` (thermo) | cálculo | Interpreta el filtro de número de carbonos | `CatalogUseCases::parse_carbon_filter(text: &str) -> …` (puro) |
| 368 | `db.search_advanced(text, field, source, filters, limit)` | búsqueda | Búsqueda por palabra clave y campo, filtrada por capa y fase | `CatalogUseCases::browse_keyword(&self, q: &KeywordQuery) -> Vec<SpeciesCatalogEntry>` |
| 392 | `db.search_elements_filtered(elems, mode, source, filters, limit)` | búsqueda | Búsqueda por elementos (Some/All) | `CatalogUseCases::browse_elements(&self, q: &ElementQuery) -> Vec<SpeciesCatalogEntry>` |
| 401 | `db.search_stoichiometry(text, fix_ratio, source, filters, limit)` | búsqueda | Búsqueda estequiométrica | `CatalogUseCases::browse_stoichiometry(&self, q: &StoichQuery) -> Result<Vec<SpeciesCatalogEntry>, SoryntelError>` |
| 547 | `db.upsert_own_dataset(sp, meta, raw_ph)` | escritura | Guarda el registro del editor en OwnDB (con código de fase crudo) | `CatalogUseCases::save_own_species(&self, sp: ChemicalSpecies, meta: SpeciesMetadata, raw_phase: String)` |
| 605 | `db.save_own_hsc(path)` | archivo | Menú File → guardar OwnDB `.HSC` (+ `.CAT`) | `CatalogUseCases::save_own_db(&self, path: &Path) -> Result<usize, SoryntelError>` |
| 612 | `db.export_csv(source_filter)` + `fs::write` | export | Exporta la vista resuelta a CSV | `CatalogUseCases::export_csv(&self, source: Option<DbSource>) -> String` |
| 633 | `db.delete_own_species(f)` | escritura | Borra de OwnDB (vuelve a MainDB) | `CatalogUseCases::delete_own_species(&self, formula: &str) -> bool` |
| 681 | `db.sort_database(DbSource::Own)` | escritura | Ordena OwnDB alfabéticamente | `CatalogUseCases::sort_layer(&self, source: DbSource)` |
| 687, 688 | `db.refresh_catalogue(Main/Own)` | archivo | Regenera los índices `.CAT` | `CatalogUseCases::refresh_catalogue(&self, source: DbSource) -> Result<PathBuf, SoryntelError>` |
| 736 | `db.status_summary()` | info | Pie de estado | `CatalogUseCases::status` |
| 774 | `db.get_species(f)` | lectura | Carga la especie seleccionada en el editor | `CatalogUseCases::get_species(&self, f: &str) -> Option<ChemicalSpecies>` (puerto actual) |
| 775 | `db.get_species_metadata(f)` | lectura | Metadatos para el editor | `CatalogUseCases::get_metadata(&self, f: &str) -> Option<SpeciesMetadata>` (puerto actual) |
| 1004 | `db.get_all_datasets_for(source, formula)` | lectura | Lista los datasets alternativos de una fórmula | `CatalogUseCases::datasets_for(&self, source: DbSource, f: &str) -> Vec<SpeciesDataset>` |
| 1019 | `db.select_dataset(source, formula, idx)` | escritura | Activa un dataset alternativo y reconstruye la resolución | `CatalogUseCases::select_dataset(&self, source: DbSource, f: &str, idx: usize) -> bool` |
| 1027, 1250 | `color_spec(code)` (thermo) | cálculo | Paleta HSC | ya existe `SoryntelApplication::species_color_spec` → `CatalogUseCases::color_spec` |
| 1143 | `db.lookup_citations(ref_field)` | búsqueda | Resuelve claves bibliográficas contra el `.REF` | `CatalogUseCases::citations_for(&self, ref_field: &str) -> Vec<ReferenceEntry>` |
| 1200 | `db.save_own_hsc(p)` | archivo | Pestaña editor: «Persist OwnDB File (.HSC)» tras guardar el editor | `CatalogUseCases::save_own_db` |
| 1207 | `db.delete_own_species(f)` | escritura | Pestaña editor: borrar | `CatalogUseCases::delete_own_species` |
| 1893 | `db.load_main_hsc(path)` | archivo | DB Tools → abrir MainDB | `CatalogUseCases::open_main_db` |
| 1907 | `db.load_own_hsc(path)` | archivo | DB Tools → abrir OwnDB | `CatalogUseCases::open_own_db` |
| 1916 | `db.save_own_hsc(path)` | archivo | DB Tools → guardar OwnDB | `CatalogUseCases::save_own_db` |
| 1922 | `db.create_empty_own_hsc(path)` | archivo | Crea un OwnDB vacío (sentinela de 256 B) | `CatalogUseCases::create_empty_own_db(&self, path: &Path) -> Result<(), SoryntelError>` |
| 1936 | `db.load_ref_dictionary(path)` | archivo | Carga el diccionario `.REF` | `CatalogUseCases::load_references(&self, path: &Path) -> Result<usize, SoryntelError>` |
| 1954 | `ThermoDatabase::merge_hsc_files(src, dst, decisions, dry_run)` | merge | Inspecciona duplicados y fusiona dos `.HSC` en disco | `CatalogUseCases::merge_hsc_files(&self, src: &Path, dst: &Path, decisions: &[MergeConflictDecision], inspect_only: bool) -> Result<(usize, Vec<DbMergeConflict>), SoryntelError>` |
| 1983 | `ThermoDatabase::convert_legacy_to_hsc6(src, dst)` | archivo | Convierte un `.HSC` o CSV legacy a HSC 6 | `CatalogUseCases::convert_legacy(&self, src: &Path, dst: &Path) -> Result<usize, SoryntelError>` |
| 1994 | `db.export_csv(source_filter)` + `fs::write` | export | DB Tools → exportar CSV | `CatalogUseCases::export_csv` |
| 2003 | `fs::read_to_string` + `db.import_csv_into_own(&s)` | archivo / escritura | Importa CSV a OwnDB | `CatalogUseCases::import_csv_into_own(&self, csv: &str) -> Result<usize, SoryntelError>` |
| 2021 | `ThermoDatabase::merge_hsc_files(.., false)` | merge | Aplica las decisiones de merge | `CatalogUseCases::merge_hsc_files` |
| 2053 | `db.search_references(q, 150)` | búsqueda | Buscador de referencias bibliográficas | `CatalogUseCases::search_references(&self, q: &str, limit: usize) -> Vec<ReferenceEntry>` |
| 2075, 2092 | `with_open_defaults`, `get_species` | test | Test del editor → OwnDB | adaptador en memoria |

Las llamadas `SoryntelApplication::species_*` del archivo (480, 1083, 1177, 1422, 1594, 1717,
1759, 1877) ya están desacopladas y pasarían a `CatalogUseCases`.

### 2.3 `soryntel-desktop/src/equilibrium_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::equilibrium::{EquilibriumSystem,
EquilibriumParams, EquilibriumWorkbookFile, …}`, `Phase`, `SpeciesRepository`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 1022 | `db.get_species(formula)` | lectura | Rellena fase y datos de las especies del sistema | `EquilibriumUseCases::resolve_species(&self, system: &mut EquilibriumSystem)` |
| 1047 | `EquilibriumWorkbookFile::from_gem9_str(&s)` (+ `fs::read_to_string`) | archivo | Abre `.gem9` | `EquilibriumUseCases::deserialize_gem9(raw: &str) -> Result<EquilibriumWorkbookFile, String>` |
| 1069 | `wb.to_gem9_json()` (+ `fs::write`) | archivo | Guarda `.gem9` | `EquilibriumUseCases::serialize_gem9(wb: &EquilibriumWorkbookFile) -> Result<String, String>` |
| 1239, 1362, 1438 | `collect_cp_warnings(&system, &params, db)` | cálculo | Avisos de extrapolación de Cp | `EquilibriumUseCases::cp_warnings(&self, system: &EquilibriumSystem, params: &EquilibriumParams) -> Vec<ExtrapolationWarning>` |
| 1375 | `solve_equilibrium_sweep(&system, &params, db)` | cálculo | Resuelve el barrido de equilibrio (Gibbs) | ya existe `SoryntelApplication::solve_equilibrium_sweep` → `EquilibriumUseCases::solve_sweep(&self, system: &EquilibriumSystem, params: &EquilibriumParams) -> Result<EquilibriumSweepResult, SoryntelError>` |
| 1513 | `db.get_species(..).map(|d| d.phase)` | lectura | Ordena las especies en fases (gas/aq/líq/sól) | `EquilibriumUseCases::sort_phases(&self, system: &mut EquilibriumSystem)` |
| 2734 | `db.search_by_elements(elems, gas, liq, sol, ion, neu, 200)` | búsqueda | Selector de especies por elementos | `EquilibriumUseCases::find_species_by_elements(&self, q: &ElementFilter) -> Vec<ChemicalSpecies>` (puerto actual) |
| 3086 | `soryntel_solver::parse_term(formula)` | cálculo | Analiza la fórmula para la matriz de elementos | `EquilibriumUseCases::parse_formula(f: &str) -> Result<ParsedTerm, SoryntelError>` o `misc::parse_term` |
| 4470–4474, 4536–4541 | `with_open_defaults`, `load_main_hsc(hsc_path)`, `run_calculate(&db)` | test | Tests contra un `Maindb6.HSC` real | adaptador en memoria |

### 2.4 `soryntel-desktop/src/estimate_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::estimate::{EstimateRow, …}`,
`KelleyCoefficients`, `Phase`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 66, 156 | `default_fig1_rows()` | cálculo | Filas de ejemplo (Fig. 1) | `EstimateUseCases::default_rows() -> Vec<EstimateRow>` (misc) |
| 163 | `classic_hsc_estimate_rows()` | cálculo | Filas clásicas HSC | `EstimateUseCases::classic_rows()` |
| 68, 109, 176, 183, 234, 294, 456, 486, 1064, 1123, 1158 | `evaluate_estimate_row(row, db)` | cálculo | Estima Cp/H/S de una fila usando especies de referencia | ya existe `SoryntelApplication::evaluate_estimate` → `EstimateUseCases::evaluate(&self, row: &mut EstimateRow)` |
| 117 | `generate_cp_curve(row, db, tmin, tmax, steps)` | cálculo | Curva de Cp estimada | `EstimateUseCases::cp_curve(&self, row: &EstimateRow, t: (f64, f64), steps: usize) -> Vec<(f64, f64)>` |
| 118, 1703 | `fit_kelley_cp(&points)` | cálculo | Ajuste Kelley | `EstimateUseCases::fit_kelley(points: &[(f64, f64)]) -> KelleyCoefficients` |
| 389, 1708 | `build_estimated_species(row, t1, t2, kelley)` | cálculo | Construye `ChemicalSpecies` a partir de la estimación | parte de `EstimateUseCases::save_estimate_to_own_db` |
| 401, 1715 | `db.insert_species(sp)` | escritura | Guarda la especie estimada en OwnDB | `EstimateUseCases::save_estimate_to_own_db(&self, row: &EstimateRow, kelley: KelleyCoefficients) -> Result<String, SoryntelError>` (necesita escritura, ver hallazgo 6) |
| 1099 | `db.search(q, 150)` | búsqueda | Selector de especies de la base | `CatalogUseCases::search(&self, q: &str, limit: usize)` |
| 1137 | `db.search("", 20_000)` | búsqueda | Recorre **todo** el catálogo para la nube de paridad | `EstimateUseCases::parity_scatter(&self) -> Vec<(f64, f64)>`; el truco `search("")` debería ser `iter_species()` o un `get_species_at` paginado en el puerto |
| 1267 | `compute_parity_stats(pairs)` | cálculo | Estadísticos de paridad | `EstimateUseCases::parity_stats(pairs: &[(f64, f64)]) -> ParityStats` |

### 2.5 `soryntel-desktop/src/exergy_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::exergy::{ExergyWorkbook, ExergySide,
ExergyStreamKind}`, `Phase`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 90, 174, 184 | `ExergyWorkbook::new_default / new_fig2_example / new_sulphides_example` | infra | Presets | `ExergyUseCases::preset(kind) -> ExergyWorkbook` (misc) |
| 91, 166, 175, 185, 207, 243, 251, 260, 269, 337, 393, 404, 411, 1218, 1270 | `wb.calculate(db)` | cálculo | Recalcula exergía de todas las corrientes | `ExergyUseCases::calculate(&self, wb: &mut ExergyWorkbook)` |
| 204 | `ExergyWorkbook::from_exe8_str(&raw)` (+ `fs::read_to_string`) | archivo | Abre `.exe8` | `ExergyUseCases::deserialize_exe8(raw: &str) -> Result<ExergyWorkbook, String>` |
| 222 | `wb.to_exe8_json()` (+ `fs::write`) | archivo | Guarda `.exe8` | `ExergyUseCases::serialize_exe8(wb: &ExergyWorkbook) -> Result<String, String>` |
| 248, 257, 267, 334, 390, 402 | `wb.insert_stream / delete_stream / delete_species` | cálculo | Edita el modelo | métodos del modelo (aceptable) o `ExergyUseCases::edit_*` |
| 1070 | `wb.set_species_mass_kg(s, r, kg, db)` | cálculo | Fija la masa y recalcula la cantidad molar con el MW del catálogo | `ExergyUseCases::set_species_mass_kg(&self, wb: &mut ExergyWorkbook, s: usize, r: usize, kg: f64)` |
| 1203, 1267 | `wb.add_species_list_to_stream / add_species_to_stream` | cálculo | Añade especies a una corriente | `ExergyUseCases::add_species(...)` |
| 1238 | `db.search(q, 200)` | búsqueda | Selector de especies | `CatalogUseCases::search` |

### 2.6 `soryntel-desktop/src/diagrams_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::diagrams::{DiagramRequest, DiagramResult,
DiagramSpeciesInput, DiagramType, DiagramEnergyUnit}`, `ChemicalSpecies`, `SpeciesRepository`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 84 | `db.get_species(formula)` | lectura | Rango de T válido de la fila | `DiagramsUseCases::species_temp_range(&self, f: &str) -> Option<(f64, f64)>` |
| 85 | `species_db_temp_range(&sp)` | cálculo | Ídem | (dentro del anterior) |
| 164, 809 | `db.search(q, 40/50)` | búsqueda | Selector de especies | `CatalogUseCases::search` |
| 196 | `calculate_diagram(&req, db)` | cálculo | Diagrama G/H/S frente a T | ya existe `SoryntelApplication::calculate_diagram` → `DiagramsUseCases::calculate(&self, req: &DiagramRequest) -> Result<DiagramResult, SoryntelError>` (misc) |
| 238 | `balanced_coeff_for_element(f, el, target)` | cálculo | Normaliza coeficientes por elemento | `DiagramsUseCases::balanced_coeff(f: &str, el: &str, target: f64) -> Option<f64>` |

### 2.7 `soryntel-desktop/src/ehph_ui.rs`

Tipos importados: `ThermoDatabase as SpeciesDb`; `soryntel_thermo::ehph::{EhPhRequest, EhPhResult,
ElementSpec, PotentialScale}`, `ChemicalSpecies`, `Phase`. (Aviso: `soryntel_core::CAL_TO_JOULE`
importado y sin usar.)

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 176, 276 | `water_dielectric_constant(t)` | cálculo | Constante dieléctrica del agua | `EhPhUseCases::water_dielectric(t_k: f64) -> f64` (misc) |
| 277 | `delta_g_formation_h2o_kcal(db, tk, criss_cobble)` | cálculo | ΔG°f del H2O a T | `EhPhUseCases::delta_g_h2o_kcal(&self, t_k: f64, criss_cobble: bool) -> Option<f64>` |
| 293, 420 | `db.search_by_elements(..)` | búsqueda | Especies candidatas del sistema Eh-pH | `EhPhUseCases::candidate_species(&self, elements: &[String], …) -> Vec<ChemicalSpecies>` (puerto actual) |
| 374, 449 | `calculate_ehph(&req, Some(db))` | cálculo | Diagrama Eh-pH (simple y combinado) | ya existe `SoryntelApplication::calculate_ehph` → `EhPhUseCases::calculate(&self, req: &EhPhRequest) -> Result<EhPhResult, SoryntelError>` |
| 1752 | `db.search("H2O", 12)` | búsqueda | Localiza la especie H2O | `CatalogUseCases::search` |

### 2.8 `soryntel-desktop/src/phase_stability_ui.rs`

Tipos importados: `ThermoDatabase as SpeciesDb`; `soryntel_thermo::phase_stability::{AxisVariable,
PhaseStabilityDiagram, PhaseStabilityInput, TemperatureUnit}`, `Phase`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 163 | `db.search_by_elements(.., solid/liquid ..)` | búsqueda | Especies condensadas candidatas | `PhaseStabilityUseCases::candidates(&self, elements: &[String]) -> (Vec<ChemicalSpecies>, Vec<ChemicalSpecies>)` |
| 182 | `db.search_by_elements(.., gas ..)` | búsqueda | Gases candidatos | (ídem) |
| 299 | `calculate_phase_stability(db, &input)` | cálculo | Diagrama de predominancia | ya existe `SoryntelApplication::calculate_phase_stability` → `PhaseStabilityUseCases::calculate(&self, input: &PhaseStabilityInput) -> Result<PhaseStabilityDiagram, SoryntelError>` (misc) |

### 2.9 `soryntel-desktop/src/benson_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::benson::{MoleculeGraph, BensonParamsInput,
BensonEstimationResult, BlockCategory, BondOrder, ElementKind, PhaseThermoSummary}`, `Phase`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 120, 121, 359 | `find_structural_block(id)` | búsqueda | Catálogo de bloques estructurales | `BensonUseCases::block(id: &str) -> Option<…>` (misc) |
| 381 | `structural_blocks_for(cat)` | búsqueda | Bloques por categoría | `BensonUseCases::blocks_for(cat: BlockCategory)` |
| 122, 360, 387 | `MoleculeGraph::from_block(spec)` | cálculo | Molécula a partir de un bloque | `BensonUseCases::molecule_from_block(..)` |
| 333 | `MoleculeGraph::empty()` | infra | Molécula vacía | DTO |
| 184 | `soryntel_thermo::benson::decompose_molecule_to_benson(&mol)` | cálculo | Descompone en grupos de Benson | `BensonUseCases::decompose(mol: &MoleculeGraph) -> (Vec<(String, usize)>, BensonParamsInput)` |
| 1250 | `MoleculeGraph::from_smiles(smiles)` | cálculo | Parsea SMILES | ya existe `SoryntelApplication::parse_benson_smiles` → `BensonUseCases::parse_smiles(s: &str) -> Result<MoleculeGraph, String>` |
| 230–1092 | `&mut ThermoDatabase` en `ui` y `render_*` | escritura | Se pasa `&mut` para `SoryntelApplication::save_benson_species` (1021), que inserta en OwnDB | `BensonUseCases::save_species(&self, …)` con puerto de escritura |

### 2.10 `soryntel-desktop/src/excel_addins_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::excel_addins::{AddInUnits, ArgValue,
CellEvalValue, MineralRecord, AddInFunctionMeta, AddInOrigin, ADDIN_FUNCTION_CATALOG}`,
`soryntel_thermo::spreadsheet_export::{ExportCell, ExportSheet, …}`, `SpeciesRepository` (684, 689).

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 685 | `db.get_species(f)` en `has_all_species` | lectura | Comprueba si existen las especies de ejemplo | `AddinsUseCases::has_species(&self, fs: &[&str]) -> bool` |
| 691 | `db.get_species(c)` en `pick_existing_species` | lectura | Elige la primera especie de ejemplo que exista | `AddinsUseCases::pick_existing(&self, cands: &[&str]) -> String` |
| 1046 | `BridgeHandle::start(Arc::new(db.clone()), ..)` | infra | **Clona toda la base** para el bridge HTTP (copia fija, no se actualiza) | `AddinsUseCases::start_bridge(&self, port: u16) -> Result<BridgeHandle, String>` compartiendo `Arc<dyn SpeciesCatalogPort>` |
| 1480 | `db.search("CO2(g)", 1)`, `db.search("C", 1)` | búsqueda | Decide qué reacción de ejemplo mostrar | `AddinsUseCases::has_species` |
| 2160, 4144 | `db.len()` | info | Número de especies en el ribbon y en el backstage | `CatalogUseCases::species_count` |
| 3703 | `db.search(q, 40)` | búsqueda | Selector de especies | `CatalogUseCases::search` |
| 564, 1467–1473 | `format_cell_number(..)` | cálculo | Formato numérico de celdas | `AddinsUseCases::format_number(v: f64, dec: usize) -> String` (puro) |
| 904, 1346, 1688, 1711, 1734, 1771, 1830, 2449, 3725 | `parse_a1(..)` | cálculo | Parsea referencias A1 | puro (`addins::parse_a1`) |
| 1835, 3156 | `col_to_a1(c)` | cálculo | Columna → letra | puro |
| 1027, 1185, 1474, 2157, 2340, 2347, 2714, 3767, 3774, 3850, 4248, 4441 | `format_temp_unit_display(..)` | cálculo | Etiqueta de la unidad de temperatura | puro |
| 372–375 | `AddInUnits::…` | infra | Unidades por defecto | DTO |
| 4844–5048 | `with_open_defaults`, `load_external_hsc_readonly(p, false)`, `get_species`, `insert_species`, `len` | test | Base de test enriquecida (carga MainDB local si existe) | adaptador en memoria |

### 2.11 `soryntel-desktop/src/addins_bridge.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::excel_addins::{AddInOrigin, AddInUnits,
ArgValue, CellEvalValue, ADDIN_FUNCTION_CATALOG}`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 58, 123, 147, 176, 597, 653, 761, 839 | parámetros `Arc<ThermoDatabase>` / `&ThermoDatabase` | infra | El bridge, el bucle de accept y los handlers están tipados con el tipo concreto | `Arc<AddinsUseCases>` (o `Arc<dyn SpeciesCatalogPort + Send + Sync>`) |
| 135 | `db.len()` | info | Log de arranque | `CatalogUseCases::species_count` |
| 675 | `db.len()` | info | `GET /health` → `species_count` | `AddinsUseCases::health(&self) -> HealthInfo` |
| 12, 136 | `ADDIN_FUNCTION_CATALOG` | lectura | Catálogo de funciones (constante) | ya existe `SoryntelApplication::addin_catalog` |
| 621, 639, 795, 827, 892 | `SoryntelApplication::eval_addin_*` | — | Ya desacoplado | `AddinsUseCases::eval_*` |
| 929–1053 | `ThermoDatabase::with_open_defaults()` | test | Tests del bridge | adaptador en memoria |

### 2.12 `soryntel-desktop/src/reaction_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::{CalculationRequest, CalculationResponse,
ReactionCollectedSheet, ReactionWorkbookFile, ReactionYAxis}` (DTO).

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 72, 136, 200, 218, 454, 889, 967, 1092 | parámetro `db: &ThermoDatabase` | infra | Solo se pasa a `SoryntelApplication::calculate_reaction` y similares | `ReactionUseCases` inyectado |
| 682 | `db.len()` | info | Rótulo «Maindb6: N especies» | `CatalogUseCases::species_count` |
| 1956 | `with_open_defaults()` | test | — | adaptador en memoria |

### 2.13 `soryntel-desktop/src/balance_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::balance_engine::{BalanceWorkbook, StreamGroup,
SpeciesRow, …}`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 83–2933 | parámetro `db: &ThermoDatabase` | infra | Se pasa a `SoryntelApplication::evaluate_balance_workbook`, `solve_balance_adiabatic_temperature` (2862) y otras | `BalanceUseCases` inyectado |
| 435, 437, 886, 888, 2827, 2885 | `StreamGroup::new(..)` | infra | Crea corrientes | DTO / `BalanceUseCases::new_stream` |
| 2833 | `SpeciesRow::…` | infra | Fila de especie | DTO |
| 2795 | `db.search(q, 80)` | búsqueda | Diálogo «añadir especie» | `BalanceUseCases::search_species(&self, q: &str, limit: usize)` o `CatalogUseCases::search` |
| 2983 | `with_open_defaults()` | test | — | adaptador en memoria |

### 2.14 `soryntel-desktop/src/aqua_ui.rs`

Tipos importados: `ThermoDatabase`; `soryntel_thermo::aqua::{AquaCaseFile, OwnPitzerDb,
MergedPitzerCatalog, …}`, `aqua_pitzer_data::{PitzerDatabase, PitzerTempCurve, ValidityRange}`,
`aqua_density_data::DENSITY_SALTS`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 152–2552 | parámetro `db: &ThermoDatabase` | infra | Solo se pasa a `SoryntelApplication::aqua_calculate` (161, 243, 2517) | `AquaUseCases` inyectado |
| 153, 445, 449, 453, 2500 | `AquaCaseFile::preset_*()` | infra | Presets de casos | `AquaUseCases::preset(name) -> AquaCaseFile` |
| 423, 433 | `serde_json` de `AquaCaseFile` (+ `fs`) | archivo | Guardar/abrir caso | `AquaUseCases::serialize_case / deserialize_case` |
| 1936, 1948, 1957 | `serde_json` de `OwnPitzerDb` (+ `fs`) | archivo | Abrir/guardar la OwnDB de Pitzer | candidato a `PitzerOwnDbPort { load, save }` (paralelo a OwnDB de especies) |
| 2005–2045, 2703, 2718 | `PitzerTempCurve::constant / full` | infra | Edición de parámetros Pitzer | DTO |
| 2051 | `own_db.valid_binaries_prefix()` | cálculo | Validación de la OwnDB de Pitzer | `AquaUseCases::validate_own_db(..)` |
| 2250, 2347 | `DENSITY_SALTS` | lectura | Tabla de sales para la densidad | `AquaUseCases::density_salts() -> &'static [..]` |

### 2.15 `soryntel-desktop/src/elements_ui.rs`, `species_converter_ui.rs`, `water_ui.rs` (ya con puerto)

Los tres reciben `&dyn SpeciesCatalogPort`, que se importa desde `soryntel_thermo`. Conviene
importarlo de `soryntel_core`.

| Archivo:línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| elements_ui:1572 | `find_element_index_by_symbol("Os")` | búsqueda | Índice del elemento | `ElementsUseCases::element_index(sym) -> Option<usize>` (misc) |
| elements_ui:198 | `workbook.to_csv()` | export | Exporta la tabla periódica | `ElementsUseCases::export_csv(wb) -> String` |
| elements_ui:369, 1541 | `workbook.diagram_property_indices() / add_custom_row()` | cálculo | Modelo | métodos del modelo (aceptable) |
| species_converter_ui:257 | `soryntel_thermo::species_converter::SpeciesConverterResult { .. }` | infra | Construye un resultado vacío | DTO |
| species_converter_ui:321, 333 | `serde_json` de `Vec<ConverterSheet>` | archivo | Guardar/abrir hojas | `SpeciesConverterUseCases::serialize/deserialize` (misc) |
| water_ui:797, 829 | `sat_temp_gl_k(p)` | cálculo | T de saturación gas-líquido | `WaterUseCases::sat_temp_gl_k(p_bar) -> f64` |
| water_ui:1579 | `h2o_saturation_pressure_bar(t)` (de `balance`) | cálculo | Presión de saturación | `WaterUseCases::sat_pressure_bar(t_k) -> f64` |
| water_ui:1587 | `sat_pressure_gs_bar(t)` | cálculo | Presión de sublimación | `WaterUseCases::sat_pressure_gs_bar(t_k)` |
| water_ui:1595 | `sat_temp_ls_k(p)` | cálculo | T de fusión | `WaterUseCases::sat_temp_ls_k(p_bar)` |
| water_ui:1711 | `catalog.get_species(name)` | lectura | Especies de vapor para el gráfico de Cp | `WaterUseCases::steam_cp_curves(&self, …)` |
| water_ui:1716 | `evaluate_species_si(&sp, tk, ..)` | cálculo | Cp(T) de la especie | (dentro del anterior) |

### 2.16 `soryntel-desktop/src/heat_loss_ui.rs` (sin `ThermoDatabase`)

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 166, 398–414, 622–634 | `preset_hotwatertank/smelting1/radiator/simple_case/oil_furnace()` | infra | Presets de workbook | `HeatLossUseCases::preset(kind) -> HeatLossWorkbook` |
| 181–184 | `builtin_solids/fluids/convection_functions/emissivities()` | lectura | Tablas de materiales integradas | `HeatLossUseCases::builtin_tables() -> …` |
| 185 | `MaterialDatabase::with_hsc_defaults()` | infra | Base de materiales | candidato a `MaterialCatalogPort` si llega a tener persistencia |
| 3405 | `preset_oil_furnace_gas()` | infra | Composición del gas | `HeatLossUseCases::preset_gas()` |
| 4019, 4057 | `soryntel_thermo::heat_loss::target::apply_variable(..)` | cálculo | Aplica la variable objetivo al workbook | `HeatLossUseCases::apply_target_variable(wb, var, value)` |
| 167, 210, 489 | `ParticleSuspensionSpec::…`, `GasComposition {..}`, `HeatLossColumn::default()` | infra | DTO | — |

El resto de cálculos ya pasa por `SoryntelApplication::solve_heat_loss_*` / `generate_heat_loss_*`.

### 2.17 `soryntel-desktop/src/mass_balance_ui.rs` (sin `ThermoDatabase`)

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 212, 228 | `soryntel_thermo::sim::presets::empty_project()` | infra | Proyecto vacío | `SimulationUseCases::empty_project()` |
| 2406, 2477, 3565 | `evaluate_cell_uncertainty(..)` | cálculo | Incertidumbre de una celda | `MassBalanceUseCases::cell_uncertainty(..)` (simulation) |
| 3483, 4028 | `default_mineral_sg(name)` | lectura | Peso específico por defecto | `MassBalanceUseCases::default_sg(name)` |
| 3997 | `search_minerals_catalog(q, 35)` | búsqueda | Catálogo de minerales | `MassBalanceUseCases::search_minerals(q, limit)`; candidato a `MineralCatalogPort` |
| 729, 735, 929, 945, 954, 960, 1035, 1059, 1098, 1230, 1236, 1259, 1318, 3807 | `workbook.detect_stream_types / convert_elements_to_minerals / convert_sol_pct_to_water / estimate_missing_values / calculate_* / freeze_balanced / build_equations_text …` | cálculo | Lógica de balance de masa invocada desde la UI | `MassBalanceUseCases::{detect_stream_types, convert_elements_to_minerals, estimate_missing, …}(&self, wb: &mut MassBalanceWorkbook, …)` |
| 1286, 1352 | `workbook.export_summary_tsv()` | export | Exporta el resumen | `MassBalanceUseCases::export_summary_tsv(wb) -> String` |
| 652, 4159 | `serde_json` de `MassBalanceWorkbook` | archivo | Exportar/importar JSON | `MassBalanceUseCases::serialize/deserialize` |

### 2.18 `soryntel-desktop/src/lca_ui.rs` (sin `ThermoDatabase`)

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 147, 162, 169 | `ClassicLcaState/ClassicProcessInfo/BetaLcaState::default()` | infra | Estado inicial | DTO |
| 199, 208 | `beta.sync_from_project(..)` | cálculo | Sincroniza el LCA con el flowsheet | `LcaUseCases::sync_from_project(&self, st, project: &SimProject)` (simulation) |
| 374, 395, 915, 2322 | `refresh_indicators()` | cálculo | Recalcula indicadores | `LcaUseCases::refresh_indicators` |
| 1276, 1283 | `convert_stream_to_indicators / convert_indicators_back_to_stream` | cálculo | Conversión corriente ↔ indicadores | `LcaUseCases::…` |
| 1417 | `save_results_to_custom_sheet()` | export | Copia resultados a una hoja | `LcaUseCases::…` |
| 1647 | `calculate_results()` | cálculo | Evaluación LCIA | `LcaUseCases::calculate(&self, st: &mut ClassicLcaState)` |
| 1214, 1223, 1233 | `probe_openlca_ipc(..)` | infra (red) | Sondea el servidor IPC de openLCA | candidato a `OpenLcaPort` |
| 2431 | `classic.load_openlca_jsonld_zip_bytes(p, &bytes)` | archivo | Importa un ZIP JSON-LD de openLCA | `LcaUseCases::import_openlca_zip(&self, st, bytes: &[u8])` |
| 2524, 2530 | `import_external_sheet(..)` | archivo | Importa una hoja externa | `LcaUseCases::…` |
| 562, 621–623, 807–971, 1245–1381, 1579–1876, 2176, 2467–2473 | `normalize`, `LcaGroup::*_options`, `set_main_product`, `add/delete_manual_stream`, `add_indicator`, `set_counted`, `available_flows`, `add/delete_impact_category`, `delete_mapping`, `set_flow_type`, `set_location_and_provider`, `catalog_flows`, `add/rename/delete_custom_sheet`, `query_providers` | cálculo | Edición del modelo LCA | métodos del modelo o `LcaUseCases` |

### 2.19 `soryntel-desktop/src/units_ui.rs` (sin `ThermoDatabase`)

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 138, 139, 141, 794 | `workbook.find_quantity_index / find_unit_index` | búsqueda | Localiza magnitud y unidad | `UnitsUseCases::find_unit(..)` (misc) |
| 145, 2257 | `workbook.total_unit_count()` | info | Contador | métodos del modelo |
| 243–255 | `workbook.resolve_labeled_unit(..)` | búsqueda | Resuelve una etiqueta de unidad | `UnitsUseCases::resolve_unit(..)` |
| 1618, 1631 | `interpolate_mesh_to_mm / interpolate_mm_to_mesh` | cálculo | Conversión entre mesh y mm | `UnitsUseCases::mesh_to_mm(..)` |
| 2160, 2218 | `add_unit_to_quantity / add_quantity` | escritura (modelo) | Unidades del usuario | métodos del modelo |

El resto (`convert_unit_value`, humedad del aire) ya pasa por `SoryntelApplication`.

### 2.20 `soryntel-desktop/src/sim_ui/*`

| Archivo:línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| mod.rs:151, 424 | `soryntel_thermo::sim::presets::empty_project()` | infra | Flowsheet vacío | `SimulationUseCases::empty_project()` |
| state.rs:281 | `empty_project()` | infra | Ídem | ídem |
| mod.rs:10 / canvas.rs:4 / docks.rs:4, 477 / editors.rs:5 | `VISUALIZATION_METRICS`, `UNIT_ICON_GALLERY`, `DEFAULT_STREAM_VARIABLES` | lectura | Tablas constantes | `SimulationUseCases::unit_gallery()` etc., o aceptables como constantes de presentación |
| docks.rs:517 | `SimUnit::new(..)` | infra | Crea una unidad | `SimulationUseCases::new_unit(kind, …)` |
| canvas.rs:1133, 1199, 1208 / editors.rs:559 | `SimUnit::…` | infra | Constructores y ayudantes | DTO |
| mod.rs:171, 433 / editors.rs:1381 | `serde_json` de `SimProject` | archivo | Exportar/importar el proyecto | `SimulationUseCases::serialize/deserialize_project` |
| icons.rs:2 | tipo `UnitKind` | infra | Iconos | DTO |

La ejecución ya pasa por `SoryntelApplication::sim_*`.

### 2.21 `soryntel-desktop/src/ui_common.rs`

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 132 | `soryntel_solver::parse_term(formula)` | cálculo | `is_organic_formula`: heurística «contiene C y H» | `MiscUseCases::is_organic_formula(f: &str) -> bool` (o `SoryntelApplication::species_parse_elements`, que ya existe) |

### 2.22 Archivos sin acoplamientos relevantes

`excel_grid.rs` y `launcher_ui.rs` no usan nada (`launcher_ui` recibe el conteo como `usize`).

### 2.23 `soryntel-app/src/main.rs` (CLI + servidor HTTP)

Tipos importados: `ThermoDatabase`, `soryntel_solver::balance_reaction`,
`soryntel_thermo::{calculate, CalculationRequest}`.

| Línea | Llamada | Tipo | Qué hace | Caso de uso / firma sugerida |
|---|---|---|---|---|
| 32 | `ThermoDatabase::with_open_defaults()` | infra | Catálogo base | Raíz de composición |
| 49 | `db.load_external_hsc_readonly(&path, false)` | archivo | `--hsc-db`: carga MainDB **sin overwrite**, así que `OpenDefault` gana a MainDB (ver §4) | `CatalogUseCases::open_main_db(&self, path)`; decidir si la CLI debe usar la precedencia estricta |
| 58 | `balance_reaction(eq, 1.0)` | cálculo | `soryntel-app balance "<eq>"` | ya existe `SoryntelApplication::balance_reaction_equation` → `ReactionUseCases::balance(&self, eq: &str, smallest: f64) -> Result<String, SoryntelError>` |
| 82 | `calculate(&req, &db)` | cálculo | `soryntel-app calc …` | ya existe `SoryntelApplication::calculate_reaction` → `ReactionUseCases::calculate(&self, req: &CalculationRequest) -> Result<CalculationResponse, SoryntelError>` |
| 97, 102, 109, 114 | `Arc<ThermoDatabase>` en `run_server` / `handle_client` | infra | Servidor web | `Arc<ReactionUseCases>` + `Arc<CatalogUseCases>` |
| 106, 132 | `db.len()` | info | Log y `GET /api/info` | `CatalogUseCases::species_count` |
| 137 | `db.search(q, 50)` | búsqueda | `GET /api/species?q=` | `CatalogUseCases::search(&self, q, 50)` |
| 141 | `balance_reaction(&eq, smallest)` | cálculo | `POST /api/balance` | `ReactionUseCases::balance` |
| 160 | `calculate(&creq, db)` | cálculo | `POST /api/calculate` | `ReactionUseCases::calculate` |

---

## 3. Puertos y casos de uso necesarios

### 3.1 Puerto existente: `SpeciesCatalogPort` (soryntel-core)

Cubre `get_species`, `get_reference_element`, `search_species`, `search_species_by_elements`,
`insert_or_update_species`, `species_count`, `get_species_metadata` y `get_species_at`. Basta
para **todos los módulos de cálculo** (reaction, balance, equilibrium, aqua, water, diagrams,
ehph, phase_stability, estimate, exergy, benson y addins). Ajustes recomendados:

- Una forma de recorrer todo el catálogo sin `search("", 20_000)` (estimate_ui:1137), por
  ejemplo `fn iter_formulas(&self) -> Vec<String>` o paginación con `get_species_at`.
- Una decisión sobre la mutación (`insert_or_update_species(&mut self)`) con puertos compartidos
  (`Arc`): adaptador `RwLock<ThermoDatabase>` que implemente el puerto con `&self`, o un
  `SpeciesWriterPort` aparte.
- `Send + Sync` en el objeto trait, para el bridge HTTP y el servidor de `soryntel-app`.

### 3.2 Puertos nuevos (métodos de `ThermoDatabase` fuera de `SpeciesCatalogPort`)

Propuesta en `soryntel-core`. Los implementaría `soryntel-db` sobre `ThermoDatabase`.

```rust
/// Gestión de las capas MainDB / OwnDB y sus archivos .HSC/.CAT.
pub trait SpeciesDatabaseAdminPort: Send + Sync {
    fn load_main_db(&self, path: &Path, overwrite_open_defaults: bool) -> Result<(usize, usize), SoryntelError>;
    fn load_own_db(&self, path: &Path) -> Result<(usize, usize), SoryntelError>;
    fn save_own_db(&self, path: &Path) -> Result<usize, SoryntelError>;
    fn create_empty_own_db(&self, path: &Path) -> Result<(), SoryntelError>;
    fn save_main_db_copy(&self, path: &Path) -> Result<usize, SoryntelError>;
    fn refresh_catalogue(&self, source: DbSource) -> Result<PathBuf, SoryntelError>;
    fn sort_layer(&self, source: DbSource);
    fn status(&self) -> DatabaseStatusSummary;
}

/// Consultas del navegador de especies (por capa, con filtros y datasets alternativos).
pub trait SpeciesBrowserPort: Send + Sync {
    fn search_keyword(&self, q: &str, field: KeywordSearchField, source: Option<DbSource>,
                      filters: &PhaseFilterFlags, limit: usize) -> Vec<SpeciesCatalogEntry>;
    fn search_elements(&self, elems: &[String], mode: ElementSearchMode, source: Option<DbSource>,
                       filters: &PhaseFilterFlags, limit: usize) -> Vec<SpeciesCatalogEntry>;
    fn search_stoichiometry(&self, pattern: &str, fix_ratio: bool, source: Option<DbSource>,
                            filters: &PhaseFilterFlags, limit: usize) -> Result<Vec<SpeciesCatalogEntry>, SoryntelError>;
    fn datasets_for(&self, source: DbSource, formula: &str) -> Vec<SpeciesDataset>;
    fn select_dataset(&self, source: DbSource, formula: &str, idx: usize) -> bool;
    fn resolved_source_of(&self, formula: &str) -> Option<DbSource>;
}

/// Edición de la capa OwnDB e intercambio CSV/JSON.
pub trait OwnDbEditorPort: Send + Sync {
    fn upsert_own(&self, sp: ChemicalSpecies, meta: SpeciesMetadata, raw_phase_code: String);
    fn delete_own(&self, formula: &str) -> bool;
    fn import_csv_into_own(&self, csv: &str) -> Result<usize, SoryntelError>;
    fn export_csv(&self, source: Option<DbSource>) -> String;
    fn export_json(&self, source: Option<DbSource>) -> Result<String, SoryntelError>;
}

/// Diccionario bibliográfico MainDB6.REF.
pub trait ReferenceLibraryPort: Send + Sync {
    fn load_references(&self, path: &Path) -> Result<usize, SoryntelError>;
    fn citations_for(&self, ref_field: &str) -> Vec<ReferenceEntry>;
    fn search_references(&self, q: &str, limit: usize) -> Vec<ReferenceEntry>;
}

/// Herramientas de archivo sin estado (hoy son funciones asociadas de ThermoDatabase).
pub trait HscFileToolsPort: Send + Sync {
    fn merge_hsc_files(&self, src: &Path, dst: &Path, decisions: &[MergeConflictDecision],
                       inspect_only: bool) -> Result<(usize, Vec<DbMergeConflict>), SoryntelError>;
    fn convert_legacy_to_hsc6(&self, src: &Path, dst: &Path) -> Result<usize, SoryntelError>;
}

/// Ubicación de las bases por defecto (sustituye las rutas /home/nando/... de main.rs).
pub trait DatabaseLocatorPort: Send + Sync {
    fn default_main_db(&self) -> Option<PathBuf>;
    fn default_own_db(&self) -> Option<PathBuf>;
}
```

Requisito previo: mover a `soryntel-core` los tipos de estas firmas, que hoy están en
`soryntel_thermo::species` (`DbSource`, `SpeciesDataset`, `SpeciesCatalogEntry`,
`PhaseFilterFlags`, `KeywordSearchField`, `ElementSearchMode`, `DbMergeConflict`,
`MergeConflictDecision`) o en `soryntel_db` (`ReferenceEntry`, `DatabaseStatusSummary`).

Candidatos secundarios, fuera de `ThermoDatabase`, que solo merecen puerto si se persisten o
vienen de red:

- `PitzerOwnDbPort`: aqua_ui 1936–1957, OwnDB de Pitzer en JSON.
- `MineralCatalogPort`: mass_balance `search_minerals_catalog`, add-ins `lookup_addin_mineral`.
- `OpenLcaPort`: lca_ui `probe_openlca_ipc`, importación JSON-LD.
- `MaterialCatalogPort`: heat_loss `MaterialDatabase`.

### 3.3 Casos de uso por capacidad (resumen)

| Capacidad | Struct sugerido | Puertos inyectados | Consumidores UI | Operaciones nuevas detectadas (además de las de `SoryntelApplication`) |
|---|---|---|---|---|
| catalog | `CatalogUseCases` | `SpeciesCatalogPort`, `SpeciesDatabaseAdminPort`, `SpeciesBrowserPort`, `OwnDbEditorPort`, `ReferenceLibraryPort`, `HscFileToolsPort`, `DatabaseLocatorPort` | species_ui, desktop/main, soryntel-app, selectores de especies de todos los módulos | open/save/create OwnDB, open MainDB, status, browse_*, datasets_for/select_dataset, citations/references, merge, convert_legacy, import/export CSV, sort, refresh_catalogue, species_count, search |
| reaction | `ReactionUseCases` | `SpeciesCatalogPort` | reaction_ui, soryntel-app | `balance`, `calculate` para la CLI y el servidor |
| equilibrium | `EquilibriumUseCases` | `SpeciesCatalogPort` | equilibrium_ui | `cp_warnings`, `solve_sweep`, `resolve_species`, `sort_phases`, `serialize/deserialize_gem9` |
| balance | `BalanceUseCases` | `SpeciesCatalogPort` | balance_ui | `search_species` (o delegar en catalog) |
| heat_loss | `HeatLossUseCases` | — | heat_loss_ui | `preset`, `builtin_tables`, `apply_target_variable` |
| aqua | `AquaUseCases` | `SpeciesCatalogPort` (+ `PitzerOwnDbPort`) | aqua_ui | `preset`, `serialize/deserialize_case`, `load/save_own_pitzer_db`, `density_salts` |
| water | `WaterUseCases` | `SpeciesCatalogPort` | water_ui | `sat_*`, `steam_cp_curves` |
| simulation | `SimulationUseCases`, `MassBalanceUseCases`, `LcaUseCases` | — (+ `MineralCatalogPort`, `OpenLcaPort`) | sim_ui, mass_balance_ui, lca_ui | `empty_project`, `serialize/deserialize_project`, operaciones de balance de masa y LCA |
| addins | `AddinsUseCases` | `SpeciesCatalogPort` (compartido, sin clonar) | excel_addins_ui, addins_bridge, desktop/main (`--addin-eval`, `--addin-bridge`) | `start_bridge`, `health`, `has_species`/`pick_existing`, utilidades A1/formato |
| misc | `DiagramsUseCases`, `EhPhUseCases`, `PhaseStabilityUseCases`, `EstimateUseCases`, `ExergyUseCases`, `BensonUseCases`, `ElementsUseCases`, `UnitsUseCases`, `SpeciesConverterUseCases` | `SpeciesCatalogPort` (escritura en estimate y benson) | diagrams, ehph, phase_stability, estimate, exergy, benson, elements, units, species_converter, ui_common | ver tablas 2.4–2.9, 2.15, 2.19 y 2.21 |

### 3.4 Orden de migración sugerido

1. **Mecánico (sin puertos nuevos)**: cambiar `&ThermoDatabase` por `&dyn SpeciesCatalogPort`
   (o por el caso de uso) en reaction, balance, aqua, diagrams, ehph, phase_stability,
   equilibrium, exergy, estimate y benson, y sustituir las llamadas directas por las de
   `SoryntelApplication` que ya existen (hallazgo 3).
2. Compartir el catálogo con `Arc` en el bridge de add-ins, sin `db.clone()` (hallazgo 5).
3. `soryntel-app` sobre `ReactionUseCases` y `CatalogUseCases` (hallazgo 4).
4. Mover los tipos de `soryntel_thermo::species` y `soryntel_db` a core; definir los puertos de
   §3.2; migrar `species_ui` y la carga inicial de `main.rs` (con `DatabaseLocatorPort` en lugar
   de las rutas fijas).

---

## 4. Precedencia de capas (verificada por test)

`crates/soryntel-db/tests/precedence.rs` comprueba con archivos `.HSC` temporales, generados en
memoria:

- `load_main_hsc` (escritorio): **`OwnDB > MainDB > OpenDefault`**. MainDB sustituye a la
  especie integrada. OwnDB (cargado de `.HSC` o insertado en memoria) gana a ambas.
  `delete_own_species` vuelve a MainDB. Recargar MainDB no desplaza a OwnDB.
  `search_species` devuelve un único registro resuelto por fórmula.
- **Discrepancia**: `load_external_hsc_readonly(path, overwrite = false)`, que es lo que usa
  `soryntel-app/src/main.rs:49`, resuelve **`OwnDB > OpenDefault > MainDB`**. Las especies
  integradas conservan prioridad sobre MainDB «por paridad de regresión» (bandera
  `main_preserve_open_defaults`). Esto contradice la documentación de `ThermoDatabase`
  («precedencia estricta OwnDB > MainDB > OpenDefault»). El código no se ha modificado; el test
  `cargador_heredado_sin_overwrite_prioriza_open_default_sobre_main` deja fijado el
  comportamiento actual.
