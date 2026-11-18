# Hallazgos del área A: `soryntel-db`, `chem_addin_core` y herramientas

Base del descubrimiento: `6976afe`. Para la corrección, `bugs/fix-a` se creó desde `worktree-agent-a113ee48317cdae95` e integró `fork-main` (`69f69c8`) en `4ef4795`.

**Severidad.** S1: pánico, aborto, corrupción o pérdida de datos. S2: resultado incorrecto. S3: menor o de experiencia de uso.

**Estado.** Las tablas conservan la reproducción original. CORREGIDO identifica el commit de la corrección. BLOQUEADO identifica un límite del formato o del alcance que sigue pendiente.

```bash
cd soryntel
cargo test --workspace
cargo test -p soryntel-db --test bugs_area_a -- --ignored
cargo test -p chem_addin_core --test bugs_area_a -- --ignored
SORYNTEL_FUZZ_ITERS=50000 cargo test -p soryntel-db --test fuzz_hsc --release
bash scripts/check_architecture.sh --strict
```

Las regresiones de A se ejecutan en la suite normal. Las dos invocaciones con `--ignored` quedan sin casos.

## Inventario de descubrimiento

| Severidad | Confirmados | Probables | Total |
|---|---|---|---|
| S1 | 7 | 1 | 8 |
| S2 | 13 | 0 | 13 |
| S3 | 11 | 7 | 18 |
| **Total** | **31** | **8** | **39** |

La revisión adicional confirmó los ocho hallazgos probables. Se corrigió el alcance verificable de siete y se añadió protección contra la pérdida de límites de dataset en A28. El formato HSC sigue sin un identificador explícito para esos casos.

## Hallazgos de `soryntel-db`

| ID | Sev. | Archivo:línea | Reproducción | Esperado / real | Estado | Fix sugerido |
|---|---|---|---|---|---|---|
| A01 | S1 | `soryntel-db/src/hsc_binary.rs:676` | `merge_hsc_files` sobre un destino con `Al2O3` en dos datasets alternativos y un origen que solo trae `ZrO2`. | Esperado: el destino conserva sus 2 datasets de Al2O3. Real: queda 1. El `BTreeMap` está indexado por fórmula y cada dataset pisa al anterior. | CORREGIDO · `fa272c9` | Indexar por `(fórmula, dataset_index)` o guardar un `Vec` por fórmula. |
| A24 | S1 | `soryntel-db/src/hsc_binary.rs:679-697` | Origen con 2 datasets de `Al2O3` y destino vacío, con la decisión `TakeSource`. | Esperado: 2 datasets fusionados. Real: 1. El segundo pisa al primero y se informan 2 «conflictos» falsos. | CORREGIDO · `fa272c9` | Misma causa que A01. Agrupar los datasets por fórmula antes de decidir. |
| A02 | S1 | `soryntel-db/src/lib.rs:215` | Cargar un OwnDB con 2 datasets de `Al2O3` y llamar a `insert_species` (editor). | La doc dice «replaces primary OwnDB dataset». Real: se borran **todos** los datasets alternativos de esa fórmula. | CORREGIDO · `fa272c9` | Sustituir solo el índice activo dentro del `Vec` y conservar los demás. |
| A03 | S1 | `soryntel-db/src/lib.rs:563-570` | `import_csv_into_own` con un CSV que contiene 2 datasets de la misma fórmula. | Devuelve `Ok(2)`, pero `get_all_datasets_for` da 1 porque cada `upsert` sustituye el anterior. | CORREGIDO · `fa272c9`, `8eddda0` | Insertar el `Vec` agrupado completo, sin llamar a `upsert` dataset a dataset. |
| A11 | S1 | `soryntel-db/src/hsc_binary.rs:384` | Serializar `C12H22O11*Ca(NO3)2*4H2O(a)` y `…(g)` (más de 24 bytes) y volver a leer. | Real: ambas se truncan a `C12H22O11*Ca(NO3)2*4H2O(`. Dos especies distintas acaban fusionadas en silencio. Pasa igual con el nombre (52 B), el CAS (12 B) y la referencia (36 B). | CORREGIDO · `fa272c9` | Devolver un error (o al menos un aviso) al serializar un campo que no cabe y validarlo en el editor. |
| A25 | S1 | `soryntel-db/src/lib.rs:346, 360, 376, 507, 536` y `hsc_binary.rs:484` | Un corte de energía o un disco lleno durante `save_own_hsc`, `merge_hsc_files` o `convert_legacy_to_hsc6`. | `fs::write` trunca el destino y lo reescribe directamente. Un fallo a mitad deja el OWNDB truncado y A20 hace que se cargue sin aviso. | CORREGIDO dentro de DB · `fa272c9`, `5398531`. **BLOQUEADO** para escritores externos al alcance. | Escribir en `*.tmp`, hacer `sync_all` y luego `rename` atómico. Opcionalmente, guardar una copia `.bak`. |
| A04 | S2 | `soryntel-db/src/hsc_binary.rs:534` | Exportar una especie cuyo nombre contiene `\n` (`csv_escape` lo entrecomilla) y volver a importar ese CSV. | Esperado: ida y vuelta. Real: `Err("CSV line 2 has 2 columns")`. Se parte por `lines()` antes de respetar las comillas. | CORREGIDO · `8eddda0` | Usar un lector CSV que admita saltos de línea dentro de comillas, o escapar `\n` y `\r` al exportar. |
| A05 | S2 | `soryntel-db/src/hsc_binary.rs:536` | Un CSV con BOM UTF-8 (lo guarda Excel) o con una línea en blanco antes de la cabecera. | Real: se importa una especie `"\u{feff}Formula"` (T1/T2 quedan por defecto en 298.15/1000). | CORREGIDO · `8eddda0` | Quitar el BOM y detectar la cabecera en la primera línea no vacía. |
| A06 | S2 | `soryntel-db/src/hsc_binary.rs:552-593` | Una fila con `"298,15"`, `"2327,0"` y `"-400,5"` (coma decimal). | Esperado: error. Real: `Ok` con T = 298.15..1000 K y H = 0 inventados mediante `unwrap_or`. | CORREGIDO · `8eddda0` | Devolver `ParseError` con línea y columna cuando un campo numérico no vacío no se pueda leer. |
| A07 | S2 | `soryntel-db/src/hsc_binary.rs:539` | Una fila separada por `;` con la referencia `"Barin 77, JANAF 98"` entrecomillada. | Real: se elige `,` como separador porque la línea contiene una coma y la importación falla. | CORREGIDO · `8eddda0` | Detectar el separador una sola vez en la cabecera, o contar los separadores fuera de comillas. |
| A08 | S2 | `soryntel-db/src/lib.rs:526` | `convert_legacy_to_hsc6` con un CSV de tamaño múltiplo de 256 bytes. | Real: se lee como binario y produce una especie basura con la fórmula `Formula,StructuralFormul`. | CORREGIDO · `8eddda0` | Decidir por la extensión o la cabecera (`Formula,` o `;`), o validar el binario antes de aceptarlo. |
| A09 | S2 | `soryntel-db/src/hsc_binary.rs:51, 74` | Ida y vuelta de un nombre con `ÿ` (U+00FF). | Esperado: `"Aÿb óxido"`. Real: `"A"`. `encode` escribe 0xFF y `decode` lo toma como terminador. | CORREGIDO · `8eddda0` | Escribir `?` en lugar de 0xFF, o dejar de usar 0xFF como terminador en los campos de texto. |
| A12 | S2 | `soryntel-db/src/hsc_binary.rs:382` y `lib.rs:354` | Guardar un OwnDB con una especie sin intervalos (recién creada en el editor). | `save_own_hsc` devuelve `Ok(1)`, pero al recargar hay 0 especies. Se descarta en silencio. | CORREGIDO · `8eddda0` | Rechazar el guardado o avisar, y devolver el número de especies realmente escritas. |
| A13 | S2 | `soryntel-db/src/hsc_binary.rs:465` | `write_catalogue_file` con un dataset sin intervalos seguido de `ZnO`. | La cubeta `Zn` apunta al registro 2, pero el `.HSC` solo tiene 1 registro, así que el índice queda desplazado. | CORREGIDO · `8eddda0` | Contar `intervals.len()` sin `.max(1)` (igual que el serializador). |
| A14 | S2 | `soryntel-db/src/hsc_binary.rs:188` | Un registro con `T1 = NaN`. | Esperado: se descarta. Real: se acepta, porque `NaN <= x` y `NaN < 0` son `false`. | CORREGIDO · `8eddda0` | Añadir `!t1.is_finite() \|\| !t2.is_finite()` al filtro, y quizá también para H, S y Cp. |
| A20 | S2 | `soryntel-db/src/hsc_binary.rs:175` | `.HSC` truncado (con 100 bytes de menos). | Real: `Ok`, sin aviso, y se pierde el último registro. Combinado con A25 esconde la corrupción. | CORREGIDO · `8eddda0` | Devolver un error o un aviso cuando `len % 256 != 0`, e informarlo en `status_summary`. |
| A21 | S2 | `soryntel-db/src/hsc_binary.rs:277` y `:168` | Ida y vuelta de `NH4+` con fase `AqueousIon`, código `a` y carga +1. | Real: `(AqueousNeutral, 0)`. La carga no se guarda y se vuelve a deducir del sufijo HSC `(+a)`. | CORREGIDO · `8eddda0` | Validar al insertar que la fórmula de un ion lleve el sufijo HSC, o usar `compute_molecular_weight` para obtener la carga. |
| A10 | S3 | `soryntel-db/src/hsc_binary.rs:87, 207` | Ida y vuelta de la solubilidad `"0,1 g/L 20°C"`. | Real: `"-"`. Se codifica en CP1252 pero se decodifica como ASCII imprimible o `-`. | CORREGIDO · `5398531` | Decodificar con `decode_cp1252` y mapear a `-` solo los bytes de control. |
| A15 | S3 | `soryntel-db/src/lib.rs:99, 118` | Comparar `get_species_at(i)` entre dos instancias de `with_open_defaults()`. | Real: orden distinto en cada instancia (iteración de `HashMap`). Afecta a la paginación, a `export_csv(None)`, a `export_json` y al índice del CLI. | CORREGIDO · `5398531` | Iterar `open_defaults` en un orden estable (`BTreeMap`, o un `Vec` con el orden de `open_reference_species`). |
| A16 | S3 | `soryntel-db/src/lib.rs:448` | `lookup_citations("Barin")` con las claves `Barin 77`, `89` y `93`. | Real: en 32 ejecuciones devolvió cada una de las 3, según la semilla del `HashMap`. | CORREGIDO · `5398531` | Recorrer `reference_keys_ordered` y elegir la primera coincidencia (o todas). |
| A17 | S3 | `soryntel-db/src/lib.rs:312-314` | Cargar MainDB con `.REF` y después otro MainDB sin `.REF`. | Real: siguen activas las referencias y `ref_path` del archivo anterior. | CORREGIDO · `5398531` | Limpiar `references` y `ref_path` cuando no exista un `.REF` acompañante. |
| A18 | S3 | `soryntel-db/src/lib.rs:307, 334` | `select_dataset(Own, "Al2O3", 1)` y después volver a cargar el OwnDB. | Real: sigue activo el set 1 del archivo nuevo. `active_dataset_choice` no se reinicia. | CORREGIDO · `5398531` | Al cargar una capa, borrar sus entradas de `active_dataset_choice`. |
| A19 | S3 | `soryntel-db/src/lib.rs:353, 368, 379, 513, 539` | `save_own_hsc` cuando `own.CAT` es un directorio y no se puede escribir. | Real: `Ok`. El error del `.CAT` se descarta con `let _ =`. | CORREGIDO · `5398531` | Propagar el error, o devolverlo como aviso en el resultado. |
| A22 | S3 | `soryntel-db/src/hsc_binary.rs:109` | Coeficiente `d = 1e40` (finito en f64). | Real: se guarda `+inf`. La comprobación de finitud se hace en f64, no en f32. | CORREGIDO · `5398531` | Comprobar `(val as f32).is_finite()` y devolver un error de rango. |
| A23 | S3 | `soryntel-db/src/lib.rs:719, 733-751` | Buscar `"óxido"` en `ChemicalName` cuando el nombre es `"Óxido de aluminio"`. | Real: 0 resultados (`to_ascii_lowercase`). `search` usa `to_lowercase`, así que las dos búsquedas no son coherentes. | CORREGIDO · `5398531` | Usar `to_lowercase()` tanto en la consulta como en los campos. |
| A26 | S3 | `soryntel-db/src/hsc_binary.rs:469-479` | `.CAT` con los datasets sin ordenar, o con `Ca…` antes que `C…`. | La doc dice «102 cubetas», pero hay 26. `starts_with("C")` también acepta `Ca`, `Cl` y `Cu`. Una cubeta ausente apunta a 1. Compatibilidad dudosa con el CAT de HSC 6. | CORREGIDO para prefijos · `5398531`. **BLOQUEADO** para validar el esquema completo de HSC 6 sin referencia verificable. | Verificar el formato real del CAT con HSC 6 y ordenar antes de indexar. |
| A27 | S3 | `soryntel-db/src/lib.rs:313` | `.REF` acompañante sin permiso de lectura. | El error se descarta (`let _ =`) y el usuario ve una biblioteca vacía sin explicación. | CORREGIDO · `5398531` | Devolver un aviso junto al recuento de MainDB. |
| A28 | S3 | `soryntel-db/src/hsc_binary.rs:270` | Un dataset con un hueco de más de 1 K entre intervalos, o dos sets alternativos contiguos. | Tras la ida y vuelta, el primero se divide en dos datasets y los segundos se fusionan. El agrupado depende de una heurística de temperatura. | **BLOQUEADO** para representar límites ambiguos en HSC. `5398531` rechaza el guardado con pérdida. CSV tiene `DatasetIndex` desde `8eddda0`. | Documentar la heurística, o marcar el inicio de set de forma explícita (por ejemplo, con `reliability_class` o el campo de referencia). |
| A29 | S3 | `soryntel-db/src/hsc_binary.rs:188` | Lint `clippy::overly_complex_bool_expr`. | `(formula == "0" && t2 <= t1)` es redundante. Un registro con la fórmula `"0"` y T válidas se acepta como especie `"0"`. | CORREGIDO · `8eddda0` | Filtrar el centinela de forma explícita (`formula == "0"`), o simplificar la expresión. |
| A39 | S3 | `soryntel-db/src/hsc_binary.rs:509-517` | Exportar a CSV con `t_min = 298.123` o H con más de 4 decimales en kcal. | La ida y vuelta CSV pierde precisión (`{:.2}` K y `{:.4}` kcal, unos 0.4 J). No está documentado. | CORREGIDO · `8eddda0` | Exportar con `{}` (precisión completa de f64) o documentar la precisión del CSV. |

**Verificado sin hallazgos.** No hay pánicos en `parse_hsc_binary_records`, `parse_ref_binary`, `group_records_to_datasets`, `import_datasets_csv`, `upgrade_legacy_datasets` ni `infer_charge`. Tampoco en la ruta completa `load_own_hsc`, búsquedas, `save_own_hsc`, `merge_hsc_files` y `convert_legacy_to_hsc6`. No hay reservas de memoria guiadas por longitudes leídas del archivo, porque todos los campos tienen offset fijo. La precedencia OwnDB > MainDB > OpenDefault y la vuelta a MainDB tras `delete_own_species` se comportan como describe el ADR 0003, y ya las cubren `tests/precedence.rs` y `tests/ports.rs`.

## Hallazgos de `chem_addin_core`

| ID | Sev. | Archivo:línea | Reproducción | Esperado / real | Estado | Fix sugerido |
|---|---|---|---|---|---|---|
| A30 | S1 | `chem_addin_core/src/parser.rs:486` | `parse_species` con 200 000 `(` + `H` + 200 000 `)`. | Esperado: `Err`. Real: desbordamiento de pila y SIGABRT (código 134), que tumba el proceso anfitrión del Add-in (bridge o Excel). | CORREGIDO · `fa272c9` | Limitar la profundidad (por ejemplo, 64) en `parse_sequence` y `split_*`, o hacerlo iterativo. |
| A35 | S1 | `chem_addin_core/src/db.rs:92, 155, 167` | `open_file`, cambiar `H2O(l).h298` a −123, cerrar y reabrir. | Esperado: −123. Real: −285.83. Cada apertura vuelve a sembrar los datos con `ON CONFLICT DO UPDATE` y pisa las ediciones del usuario. | CORREGIDO · `fa272c9` | Sembrar solo si la tabla está vacía (o una versión de esquema), con `ON CONFLICT DO NOTHING`. |
| A32 | S2 | `chem_addin_core/src/parser.rs:374` | `parse_species("Fe2+")`. | Real: `Fe2` con carga +1. `bal("Fe2+ + Ce4+ = Fe3+ + Ce3+")` devuelve `1.5 Fe2+ + 1.5 Ce4+ = Fe3+ + 2 Ce3+`, sin sentido químico. | CORREGIDO · `8eddda0`, `5398531` | Rechazar por ambigua la notación «dígito+signo» al final, o interpretarla como carga (`Fe2+` = Fe con 2+). |
| A33 | S2 | `chem_addin_core/src/parser.rs:9, 17` | `parse_species("Na(+a)")` y `parse_species("SO4(-2a)")`. | Real: `InvalidFormula`. La notación HSC de iones, que usa el resto del workspace, no se reconoce en el motor de Add-ins. | CORREGIDO · `8eddda0` | Aceptar `\(([+-]\d*)a\)$` como carga más fase acuosa. |
| A31 | S3 | `chem_addin_core/src/parser.rs:89` | `mw("e-")`. | Esperado: 0.000548579909. Real: 0, porque la condición excluye el electrón libre. | CORREGIDO · `5398531` | Para una composición vacía con carga −1, devolver `atomic_weight("e-")`. |
| A34 | S3 | `chem_addin_core/src/parser.rs:567` | `parse_species("C0")` y `parse_species("H0")`. | Esperado: error. Real: especie válida con MW 0. | CORREGIDO · `5398531` | Rechazar los subíndices `== 0` (`val <= 0.0`). |
| A36 | S3 | `chem_addin_core/src/thermo.rs:253, 276` | `H("H2O(l)", −100 °C)`, cuando los datos cubren 273.15–373.15 K. | Real: `Ok(−295.14)`, una extrapolación silenciosa. `soryntel-core` sí expone `extrapolated_from_k`. | CORREGIDO · `5398531` | Devolver un error o un indicador de extrapolación fuera de `[t_min(primero), t_max(último)]`. |
| A37 | S3 | `chem_addin_core/src/parser.rs:9` | `parse_species("Fe(S)")` y `parse_species("Co(G)")`. | La regex de fase no distingue mayúsculas: `Fe(S)` se toma como Fe sólido, no como FeS, y `Co(G)` como Co gaseoso. | CORREGIDO · `5398531` | Aceptar solo los sufijos de fase en minúscula. |
| A38 | S3 | `chem_addin_core/src/db.rs:272` | `elements_wt_json` corrupto en la tabla `minerals`. | `unwrap_or_default` devuelve un mapa vacío sin avisar. | CORREGIDO · `5398531` | Propagar el error de deserialización. |

## Verificación y límites

- `validation.rs`, `precedence.rs` y `ports.rs` se ejecutan sin cambios. Las pruebas nuevas comprueban la edición de un dataset alternativo, las colisiones por índice, los límites del parser y el balance de carga.
- Los guardados de DB validan CP1252 y los campos antes de escribir. Los archivos HSC y CAT se reemplazan individualmente con temporal en el mismo directorio, sincronización y renombrado. Si el CAT es un directorio, se rechaza antes de escribir el HSC. La pareja HSC/CAT no constituye una transacción de dos archivos. La API infalible de codificación binaria se conserva para consumidores existentes que generan fixtures. Los guardados de DB usan la variante validada.
- **A25 externo BLOQUEADO.** En estos crates `.REF` se lee y CSV/JSON se devuelven como texto. Sus consumidores de fuera del área A quedan fuera de esta corrección.
- **A26 formato completo BLOQUEADO.** La prueba demuestra que el bucket `C` ya no apunta a `Ca` cuando existe un registro de `C`. Se conserva el orden de los registros HSC y las 26 entradas del escritor existente. Falta una referencia comprobable para determinar si HSC 6 requiere otras entradas o un centinela distinto.
- **A28 BLOQUEADO para HSC legado.** Un límite de dataset inferido por temperaturas no distingue alternativas contiguas ni intervalos discontinuos del mismo dataset. Se rechaza una exportación HSC nueva si esa inferencia cambiaría su identidad. Los archivos HSC existentes siguen el mismo criterio de lectura. CSV conserva identidades explícitas y sigue leyendo archivos legados sin la columna nueva.
- El editor y la fusión trabajan con la identidad fórmula/índice. La interfaz existente de decisiones de fusión recibe una política por fórmula. Una operación que necesite escoger políticas distintas para dos conflictos de la misma fórmula requeriría ampliar el contrato que comparte `soryntel-core`, fuera del área A.
