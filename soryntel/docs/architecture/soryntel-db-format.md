# Formato SoryntelDB v1 (`.sdb`)

SoryntelDB es el catálogo termoquímico de Soryntel Chemistry. El conversor `sdb-convert` importa un catálogo externo en formato heredado. El [ADR 0007](adr/0007-soryntel-db.md) describe la decisión y [`soryntel-db/src/sdb`](../../crates/soryntel-db/src/sdb/mod.rs) contiene la implementación.

Los datos fuente y los `.sdb` generados nunca se guardan en el repositorio. `.gitignore` excluye `*.sdb`.

## Principios

- **JSON UTF-8** con sangría de dos espacios y un salto de línea final. Es legible y se puede comparar con `diff`.
- **Versionado.** La cabecera lleva `format` y `version`. Un lector v1 rechaza cualquier otra versión con `CatalogError::Unsupported` y cualquier otro formato con `CatalogError::Backend`, incluso cuando el cuerpo tiene otro esquema.
- **Reproducible.** Las especies siguen el orden del `.HSC`, los juegos de datos el orden de su índice y las referencias el orden del `.REF`. El único campo que depende del momento de la conversión es `created`. Con las mismas entradas y el mismo `created`, dos conversiones dan archivos idénticos byte a byte.
- **Fidelidad numérica.** Se guardan exactamente los `f64` que produce el parser `.HSC`: el `f32` original ampliado a `f64` y, para las magnitudes energéticas, multiplicado por 4,184 (cal → J). No se redondea de nuevo. `serde_json` escribe el `f64` más corto que se relee igual y `soryntel-db` activa `float_roundtrip` para releerlo bit a bit. JSON no admite números no finitos, así que el conversor rechaza `NaN` e `inf` con `CatalogError::InvalidSpecies`.
- **Sin rutas locales.** En `source.files` solo se guarda el nombre de cada archivo, su tamaño y su SHA-256.

## Cabecera

| Campo | Tipo | Contenido |
|---|---|---|
| `format` | cadena | Siempre `"soryntel-db"`. |
| `version` | entero | `1`. |
| `created` | cadena | Fecha UTC `AAAA-MM-DDThh:mm:ssZ`. El conversor toma `--created`, después `SOURCE_DATE_EPOCH` y, si no hay ninguno, la hora actual. |
| `generator` | objeto | `name` (`"sdb-convert"`) y `version` (versión del workspace). |
| `source` | objeto | `name` (`"External thermochemical catalog"`) y `files[]`, con `role` (`main`, `references` o `catalogue`), `name`, `size` en bytes y `sha256` en hexadecimal. |
| `notice` | cadena | Texto de atribución que recibe el conversor con `--notice`. |
| `species_count` | entero | Número de elementos de `species`. |
| `dataset_count` | entero | Número total de juegos de datos, primarios y alternativos. |
| `interval_count` | entero | Número total de intervalos de temperatura, equivalente a los registros `.HSC` válidos. |
| `reference_count` | entero | Número de elementos de `references`. |

El lector comprueba que los cuatro contadores coinciden con el cuerpo.

## Cuerpo

### `species[]`

| Campo | Tipo | Contenido |
|---|---|---|
| `formula` | cadena | Fórmula HSC con sufijo de fase, por ejemplo `H2O(g)`. Es única en el archivo. |
| `references` | cadenas | Claves de `references[]` citadas por los intervalos, sin repetir y en orden de aparición. La resolución sigue la regla de `lookup_citations`: clave exacta sin distinguir mayúsculas o, para un campo truncado, la primera clave que empieza por él. Las citas que no están en el `.REF` no aparecen aquí, pero se conservan en el `reference` del intervalo. |
| `datasets` | objetos | Al menos un juego. El índice 0 es el primario. Los alternativos proceden de bloques no contiguos del `.HSC`. |

### `datasets[]`

| Campo | Tipo | Contenido |
|---|---|---|
| `index` | entero | Índice del juego, único dentro de la especie. |
| `name` | cadena | Nombre químico. |
| `phase` | cadena | `Solid`, `Liquid`, `Gas`, `AqueousIon` o `AqueousNeutral`. |
| `raw_phase_code` | cadena | Código de fase original de HSC (`s`, `l`, `g`, `a`, `ao`, `ai`…). |
| `molecular_weight` | número | g/mol. |
| `density_g_cm3` | número | g/cm³. |
| `charge` | entero | Carga inferida de la fórmula por el parser. |
| `metadata` | objeto | `structural_formula`, `common_name`, `cas_number`, `melting_point_k` (K), `boiling_point_k` (K), `color_code`, `solubility` y `reliability_class`. Es la serialización de `SpeciesMetadata`. |
| `intervals` | objetos | Al menos uno, ordenados por temperatura. |

### `intervals[]`

| Campo | Unidad | Contenido |
|---|---|---|
| `t_min`, `t_max` | K | Límites del intervalo, con `0 ≤ t_min < t_max`. |
| `phase` | — | Fase en el intervalo. |
| `h` | J/mol | Entalpía de formación a 298,15 K en el primer intervalo. En los siguientes, entalpía de transición. |
| `s` | J/(mol·K) | Entropía absoluta en el primer intervalo. En los siguientes, entropía de transición. |
| `a`, `b`, `c`, `d` | J/(mol·K) | Coeficientes de Kelley: Cp = a + b·10⁻³·T + c·10⁵/T² + d·10⁻⁶·T². |
| `reference` | — | Campo de referencia original del registro HSC, de hasta 36 caracteres. |

### `references[]`

Copia del diccionario `MainDB6.REF` en el orden del archivo: `key` (36 caracteres como máximo), `citation` y `flag` (byte de marca original).

## Ejemplo abreviado

```json
{
  "format": "soryntel-db",
  "version": 1,
  "created": "2026-01-01T00:00:00Z",
  "generator": { "name": "sdb-convert", "version": "0.1.2" },
  "source": {
    "name": "External thermochemical catalog",
    "files": [{ "role": "main", "name": "Maindb6.HSC", "size": 256, "sha256": "…" }]
  },
  "notice": "…",
  "species_count": 1,
  "dataset_count": 1,
  "interval_count": 1,
  "reference_count": 0,
  "species": [{
    "formula": "X",
    "references": [],
    "datasets": [{
      "index": 0, "name": "…", "phase": "Solid", "raw_phase_code": "s",
      "molecular_weight": 1.0, "density_g_cm3": 1.0, "charge": 0,
      "metadata": { "structural_formula": "", "common_name": "", "cas_number": "",
                    "melting_point_k": 0.0, "boiling_point_k": 0.0, "color_code": 0,
                    "solubility": "-", "reliability_class": 1 },
      "intervals": [{ "t_min": 298.1499938964844, "t_max": 1000.0, "phase": "Solid",
                      "h": 0.0, "s": 0.0, "a": 0.0, "b": 0.0, "c": 0.0, "d": 0.0,
                      "reference": "" }]
    }]
  }],
  "references": []
}
```

El ejemplo es una plantilla sintética con un solo intervalo y no contiene datos reales.

## Generación

```bash
cargo run --release -p soryntel-db --bin sdb-convert -- \
  --main <Maindb6.HSC> --ref <MainDB6.REF> --cat <Maindb6.cat> \
  --notice "<texto de atribución>" --out <soryntel.sdb>
```

`--created <AAAA-MM-DDThh:mm:ssZ>` o `SOURCE_DATE_EPOCH` fijan la fecha para una compilación reproducible. Al terminar, el conversor imprime los contadores, el tamaño y el SHA-256 del archivo.

## Carga

`ThermoDatabase::load_soryntel_db(path)` y `SpeciesDatabaseAdminPort::load_soryntel_db` cargan la capa `DbSource::Soryntel`. Los errores de E/S, de cabecera y de validación llegan como `CatalogError`. Si no hay un `.REF` activo, se usa el diccionario incluido en el `.sdb`, que también se conserva al cargar una MainDB sin `.REF` compañero. Los clientes buscan el archivo como se describe en el [ADR 0007](adr/0007-soryntel-db.md).

## Validación

`soryntel-db/tests/sdb.rs` usa archivos sintéticos generados en memoria. `soryntel-db/tests/sdb_real.rs` solo se ejecuta con `SORYNTEL_TEST_MAINDB`. Convierte la MainDB dos veces y compara los archivos. Después comprueba especies, juegos de datos e intervalos, y que H, S, Cp y G de todos los juegos sean idénticos bit a bit a 298,15, 500 y 1000 K (o en el límite más cercano de su rango) entre la carga directa del `.HSC` y la del `.sdb`. También contrasta H2O(g) y CO2(g) con NIST-JANAF.
