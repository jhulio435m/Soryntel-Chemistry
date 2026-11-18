//! SoryntelDB v1 (`.sdb`): formato propio del catálogo base de Soryntel.
//!
//! Es un documento JSON UTF-8 con una cabecera versionada y el cuerpo `species[]` +
//! `references[]`. La conversión desde HSC 6 reutiliza los parsers de [`crate::hsc_binary`] y
//! guarda exactamente los `f64` que estos producen (sin redondear de nuevo), de modo que una
//! carga desde `.sdb` es idéntica bit a bit a una carga directa del `.HSC`. El esquema está
//! documentado en `docs/architecture/soryntel-db-format.md` y en el ADR 0007.
//!
//! La salida es determinista: el orden de especies, juegos de datos y referencias es el de los
//! archivos fuente y el único campo que depende del momento de la conversión es `created`.

mod sha256;

pub use sha256::sha256_hex;

use crate::hsc_binary::{group_records_to_datasets, parse_hsc_binary_records, parse_ref_binary};
use serde::{Deserialize, Serialize};
use soryntel_core::{
    CatalogError, ChemicalSpecies, DbSource, KelleyCoefficients, Phase, ReferenceEntry,
    SpeciesDataset, SpeciesMetadata, SpeciesThermoInterval,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Valor del campo `format`.
pub const SDB_FORMAT: &str = "soryntel-db";
/// Versión del esquema que lee y escribe este módulo.
pub const SDB_VERSION: u32 = 1;
/// Nombre de la fuente de los datos convertidos desde HSC 6.
pub const HSC6_SOURCE_NAME: &str = "HSC Chemistry 6 MainDB";
/// Nombre de la herramienta registrada en `generator`.
pub const GENERATOR_NAME: &str = "sdb-convert";

/// Documento `.sdb` completo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdbFile {
    /// Siempre [`SDB_FORMAT`].
    pub format: String,
    /// Versión del esquema ([`SDB_VERSION`]).
    pub version: u32,
    /// Fecha de la conversión en UTC (`AAAA-MM-DDThh:mm:ssZ`).
    pub created: String,
    pub generator: SdbGenerator,
    pub source: SdbSource,
    /// Texto de atribución de los datos.
    pub notice: String,
    /// Número de elementos de `species`.
    pub species_count: usize,
    /// Número total de juegos de datos (primarios y alternativos).
    pub dataset_count: usize,
    /// Número total de intervalos de temperatura (registros `.HSC` equivalentes).
    pub interval_count: usize,
    /// Número de elementos de `references`.
    pub reference_count: usize,
    pub species: Vec<SdbSpecies>,
    /// Diccionario bibliográfico (`MainDB6.REF`) en el orden del archivo.
    pub references: Vec<ReferenceEntry>,
}

/// Herramienta que generó el archivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdbGenerator {
    pub name: String,
    pub version: String,
}

/// Procedencia de los datos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdbSource {
    pub name: String,
    pub files: Vec<SdbSourceFile>,
}

/// Archivo fuente con su huella. Solo se guarda el nombre, nunca la ruta local.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdbSourceFile {
    /// `main`, `references` o `catalogue`.
    pub role: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

/// Especie: una fórmula con todos sus juegos de datos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdbSpecies {
    pub formula: String,
    /// Claves de `references[]` citadas por los intervalos, sin repetir y en orden de aparición.
    pub references: Vec<String>,
    /// Juegos de datos ordenados por `index`; el primero es el primario.
    pub datasets: Vec<SdbDataset>,
}

/// Juego de datos (primario `index = 0` o alternativo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdbDataset {
    pub index: usize,
    pub name: String,
    /// Fase a 298,15 K o en el primer intervalo.
    pub phase: Phase,
    /// Código de fase original de HSC (`s`, `l`, `g`, `a`, `ao`, `ai`…).
    pub raw_phase_code: String,
    /// g/mol.
    pub molecular_weight: f64,
    /// g/cm³.
    pub density_g_cm3: f64,
    pub charge: i32,
    pub metadata: SpeciesMetadata,
    pub intervals: Vec<SdbInterval>,
}

/// Intervalo de temperatura con coeficientes de Kelley en SI.
///
/// `Cp = a + b·1e-3·T + c·1e5/T² + d·1e-6·T²` en J/(mol·K). `h` (J/mol) y `s` (J/(mol·K))
/// son los valores de formación en el primer intervalo y los de transición en los siguientes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdbInterval {
    pub t_min: f64,
    pub t_max: f64,
    pub phase: Phase,
    pub h: f64,
    pub s: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    /// Campo de referencia original del registro HSC.
    pub reference: String,
}

/// Entradas de la conversión HSC 6 → `.sdb`.
#[derive(Debug, Clone)]
pub struct HscSources<'a> {
    pub main: &'a Path,
    pub references: Option<&'a Path>,
    pub catalogue: Option<&'a Path>,
}

fn backend(message: String) -> CatalogError {
    CatalogError::Backend(message)
}

fn read_file(path: &Path) -> Result<Vec<u8>, CatalogError> {
    fs::read(path).map_err(|e| backend(format!("Cannot read {}: {e}", path.display())))
}

fn source_file(role: &str, path: &Path, bytes: &[u8]) -> SdbSourceFile {
    SdbSourceFile {
        role: role.into(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        size: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    }
}

/// Convierte los archivos de HSC 6 en un documento `.sdb`.
///
/// `created` se copia tal cual en la cabecera; con el mismo valor y las mismas entradas el
/// resultado es idéntico.
pub fn convert_hsc6(
    sources: &HscSources<'_>,
    notice: &str,
    created: &str,
) -> Result<SdbFile, CatalogError> {
    let main_bytes = read_file(sources.main)?;
    let mut files = vec![source_file("main", sources.main, &main_bytes)];
    let references = match sources.references {
        Some(path) => {
            let bytes = read_file(path)?;
            files.push(source_file("references", path, &bytes));
            parse_ref_binary(&bytes)
        }
        None => Vec::new(),
    };
    if let Some(path) = sources.catalogue {
        let bytes = read_file(path)?;
        files.push(source_file("catalogue", path, &bytes));
    }
    let (records, _) = parse_hsc_binary_records(&main_bytes)
        .map_err(|e| backend(format!("{}: {e}", sources.main.display())))?;
    let (order, mut grouped) = group_records_to_datasets(records, DbSource::Soryntel);
    let datasets: Vec<Vec<SpeciesDataset>> = order
        .iter()
        .filter_map(|formula| grouped.remove(formula))
        .collect();
    Ok(build_sdb(
        datasets,
        references,
        SdbSource {
            name: HSC6_SOURCE_NAME.into(),
            files,
        },
        notice,
        created,
    ))
}

/// Construye un documento a partir de juegos de datos ya agrupados por fórmula (cada
/// elemento de `species` contiene los juegos de una fórmula, en orden de índice).
pub fn build_sdb(
    species: Vec<Vec<SpeciesDataset>>,
    references: Vec<ReferenceEntry>,
    source: SdbSource,
    notice: &str,
    created: &str,
) -> SdbFile {
    let resolver = ReferenceResolver::new(&references);
    let species: Vec<SdbSpecies> = species
        .into_iter()
        .filter(|datasets| !datasets.is_empty())
        .map(|datasets| {
            let formula = datasets[0].species.formula.clone();
            let cited = resolver.cited(
                datasets
                    .iter()
                    .flat_map(|d| d.species.intervals.iter().map(|i| i.reference.as_str())),
            );
            SdbSpecies {
                formula,
                references: cited,
                datasets: datasets.into_iter().map(dataset_to_sdb).collect(),
            }
        })
        .collect();
    let dataset_count = species.iter().map(|s| s.datasets.len()).sum();
    let interval_count = species
        .iter()
        .flat_map(|s| &s.datasets)
        .map(|d| d.intervals.len())
        .sum();
    SdbFile {
        format: SDB_FORMAT.into(),
        version: SDB_VERSION,
        created: created.into(),
        generator: SdbGenerator {
            name: GENERATOR_NAME.into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        source,
        notice: notice.into(),
        species_count: species.len(),
        dataset_count,
        interval_count,
        reference_count: references.len(),
        species,
        references,
    }
}

fn dataset_to_sdb(ds: SpeciesDataset) -> SdbDataset {
    let sp = ds.species;
    SdbDataset {
        index: ds.dataset_index,
        name: sp.name,
        phase: sp.phase,
        raw_phase_code: ds.raw_phase_code,
        molecular_weight: sp.molecular_weight,
        density_g_cm3: sp.density_g_cm3,
        charge: sp.charge,
        metadata: ds.metadata,
        intervals: sp
            .intervals
            .into_iter()
            .map(|i| SdbInterval {
                t_min: i.t_min,
                t_max: i.t_max,
                phase: i.phase,
                h: i.h_or_delta_h_tr,
                s: i.s_or_delta_s_tr,
                a: i.kelley.a,
                b: i.kelley.b,
                c: i.kelley.c,
                d: i.kelley.d,
                reference: i.reference,
            })
            .collect(),
    }
}

/// Resuelve los campos de referencia de los intervalos contra el diccionario `.REF` con la
/// misma regla que `ThermoDatabase::lookup_citations` (clave exacta o, si el campo está
/// truncado, la primera clave que empieza por él).
struct ReferenceResolver<'a> {
    exact: HashMap<String, &'a str>,
    ordered: Vec<(String, &'a str)>,
}

impl<'a> ReferenceResolver<'a> {
    fn new(entries: &'a [ReferenceEntry]) -> Self {
        let mut exact = HashMap::new();
        let mut ordered = Vec::new();
        for entry in entries {
            let lower = entry.key.to_ascii_lowercase();
            if !exact.contains_key(&lower) {
                exact.insert(lower.clone(), entry.key.as_str());
                ordered.push((lower, entry.key.as_str()));
            }
        }
        Self { exact, ordered }
    }

    fn cited<'b>(&self, fields: impl Iterator<Item = &'b str>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for field in fields {
            for token in field.split([',', ';']) {
                let token = token.trim().to_ascii_lowercase();
                if token.is_empty() {
                    continue;
                }
                let key = self.exact.get(&token).copied().or_else(|| {
                    self.ordered
                        .iter()
                        .find(|(lower, _)| lower.starts_with(&token))
                        .map(|(_, key)| *key)
                });
                if let Some(key) = key {
                    if seen.insert(key) {
                        out.push(key.to_string());
                    }
                }
            }
        }
        out
    }
}

/// Serializa el documento: JSON con sangría, UTF-8 y salto de línea final.
///
/// Falla si algún número no es finito, porque JSON no puede representarlo sin pérdida.
pub fn to_bytes(file: &SdbFile) -> Result<Vec<u8>, CatalogError> {
    check_finite(file)?;
    let mut bytes = serde_json::to_vec_pretty(file)
        .map_err(|e| backend(format!("SoryntelDB serialization failed: {e}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn check_finite(file: &SdbFile) -> Result<(), CatalogError> {
    for sp in &file.species {
        for ds in &sp.datasets {
            let m = &ds.metadata;
            let mut values = vec![
                ("molecular_weight", ds.molecular_weight),
                ("density_g_cm3", ds.density_g_cm3),
                ("melting_point_k", m.melting_point_k),
                ("boiling_point_k", m.boiling_point_k),
            ];
            for i in &ds.intervals {
                values.extend([
                    ("t_min", i.t_min),
                    ("t_max", i.t_max),
                    ("h", i.h),
                    ("s", i.s),
                    ("a", i.a),
                    ("b", i.b),
                    ("c", i.c),
                    ("d", i.d),
                ]);
            }
            if let Some((field, value)) = values.into_iter().find(|(_, v)| !v.is_finite()) {
                return Err(CatalogError::InvalidSpecies(format!(
                    "{} (dataset {}): {field} = {value} is not finite",
                    sp.formula, ds.index
                )));
            }
        }
    }
    Ok(())
}

/// Escribe el documento de forma atómica (archivo temporal + renombrado).
pub fn write_sdb(path: &Path, file: &SdbFile) -> Result<(), CatalogError> {
    let bytes = to_bytes(file)?;
    crate::atomic_write::write(path, &bytes)
        .map_err(|e| backend(format!("Cannot write {}: {e}", path.display())))
}

/// Lee y valida un archivo `.sdb`.
pub fn read_sdb(path: &Path) -> Result<SdbFile, CatalogError> {
    let bytes = read_file(path)?;
    parse_sdb(&bytes).map_err(|e| match e {
        CatalogError::Backend(m) => backend(format!("{}: {m}", path.display())),
        CatalogError::Unsupported(m) => CatalogError::Unsupported(format!("{}: {m}", path.display())),
        CatalogError::InvalidSpecies(m) => {
            CatalogError::InvalidSpecies(format!("{}: {m}", path.display()))
        }
        other => other,
    })
}

#[derive(Deserialize)]
struct HeaderProbe {
    format: Option<serde_json::Value>,
    version: Option<serde_json::Value>,
}

/// Interpreta y valida un documento `.sdb` en memoria.
///
/// Comprueba la cabecera (`format`, `version`) antes de aceptar el cuerpo y después la
/// coherencia de los contadores, fórmulas, índices e intervalos.
pub fn parse_sdb(bytes: &[u8]) -> Result<SdbFile, CatalogError> {
    let file: SdbFile = match serde_json::from_slice(bytes) {
        Ok(file) => file,
        Err(error) => {
            // Un cuerpo ilegible puede deberse a otro formato o a otra versión: se informa de
            // la cabecera antes que del detalle de serde.
            if let Ok(probe) = serde_json::from_slice::<HeaderProbe>(bytes) {
                check_header(probe.format.as_ref(), probe.version.as_ref())?;
            }
            return Err(backend(format!("invalid SoryntelDB document: {error}")));
        }
    };
    check_header(
        Some(&serde_json::Value::from(file.format.as_str())),
        Some(&serde_json::Value::from(file.version)),
    )?;
    validate(&file)?;
    Ok(file)
}

fn check_header(
    format: Option<&serde_json::Value>,
    version: Option<&serde_json::Value>,
) -> Result<(), CatalogError> {
    if format.and_then(|f| f.as_str()) != Some(SDB_FORMAT) {
        return Err(backend(format!(
            "not a SoryntelDB file: format = {}, expected \"{SDB_FORMAT}\"",
            format.map_or("(missing)".to_string(), |f| f.to_string())
        )));
    }
    match version.and_then(|v| v.as_u64()) {
        Some(v) if v == u64::from(SDB_VERSION) => Ok(()),
        Some(v) => Err(CatalogError::Unsupported(format!(
            "SoryntelDB version {v} (this build reads version {SDB_VERSION})"
        ))),
        None => Err(backend("SoryntelDB header without a numeric version".into())),
    }
}

fn validate(file: &SdbFile) -> Result<(), CatalogError> {
    let invalid = |m: String| Err(CatalogError::InvalidSpecies(m));
    if file.species_count != file.species.len() {
        return invalid(format!(
            "species_count = {} but the file holds {} species",
            file.species_count,
            file.species.len()
        ));
    }
    if file.reference_count != file.references.len() {
        return invalid(format!(
            "reference_count = {} but the file holds {} references",
            file.reference_count,
            file.references.len()
        ));
    }
    let mut formulas = HashSet::new();
    let (mut datasets, mut intervals) = (0usize, 0usize);
    for sp in &file.species {
        if sp.formula.is_empty() {
            return invalid("species with an empty formula".into());
        }
        if !formulas.insert(sp.formula.as_str()) {
            return invalid(format!("duplicate species {}", sp.formula));
        }
        if sp.datasets.is_empty() {
            return invalid(format!("{} has no datasets", sp.formula));
        }
        let mut indices = BTreeSet::new();
        for ds in &sp.datasets {
            if !indices.insert(ds.index) {
                return invalid(format!("{}: duplicate dataset index {}", sp.formula, ds.index));
            }
            if ds.intervals.is_empty() {
                return invalid(format!("{} (dataset {}) has no intervals", sp.formula, ds.index));
            }
            for i in &ds.intervals {
                if !(i.t_min.is_finite() && i.t_max.is_finite() && i.t_min >= 0.0 && i.t_max > i.t_min) {
                    return invalid(format!(
                        "{} (dataset {}): invalid interval {}..{} K",
                        sp.formula, ds.index, i.t_min, i.t_max
                    ));
                }
            }
            intervals += ds.intervals.len();
        }
        datasets += sp.datasets.len();
    }
    if file.dataset_count != datasets || file.interval_count != intervals {
        return invalid(format!(
            "dataset_count/interval_count = {}/{} but the file holds {datasets}/{intervals}",
            file.dataset_count, file.interval_count
        ));
    }
    Ok(())
}

/// Juegos de datos de la capa SoryntelDB: orden de fórmulas y juegos por fórmula.
pub type SdbLayer = (Vec<String>, HashMap<String, Vec<SpeciesDataset>>);

/// Convierte el cuerpo del documento en juegos de datos con origen [`DbSource::Soryntel`].
pub fn into_datasets(species: Vec<SdbSpecies>) -> SdbLayer {
    let mut order = Vec::with_capacity(species.len());
    let mut map = HashMap::with_capacity(species.len());
    for sp in species {
        let formula = sp.formula;
        let mut list: Vec<SpeciesDataset> = sp
            .datasets
            .into_iter()
            .map(|ds| SpeciesDataset {
                source: DbSource::Soryntel,
                dataset_index: ds.index,
                raw_phase_code: ds.raw_phase_code,
                metadata: ds.metadata,
                species: ChemicalSpecies {
                    formula: formula.clone(),
                    name: ds.name,
                    phase: ds.phase,
                    molecular_weight: ds.molecular_weight,
                    density_g_cm3: ds.density_g_cm3,
                    charge: ds.charge,
                    intervals: ds
                        .intervals
                        .into_iter()
                        .map(|i| SpeciesThermoInterval {
                            t_min: i.t_min,
                            t_max: i.t_max,
                            phase: i.phase,
                            kelley: KelleyCoefficients {
                                a: i.a,
                                b: i.b,
                                c: i.c,
                                d: i.d,
                            },
                            h_or_delta_h_tr: i.h,
                            s_or_delta_s_tr: i.s,
                            reference: i.reference,
                        })
                        .collect(),
                },
            })
            .collect();
        list.sort_by_key(|ds| ds.dataset_index);
        order.push(formula.clone());
        map.insert(formula, list);
    }
    (order, map)
}

/// Fecha UTC `AAAA-MM-DDThh:mm:ssZ` correspondiente a `secs` segundos desde 1970-01-01.
pub fn utc_timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Algoritmo civil_from_days de H. Hinnant.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_timestamp_formats_known_instants() {
        assert_eq!(utc_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_timestamp(1_795_000_000), "2026-11-18T11:06:40Z");
    }
}
