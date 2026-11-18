# Bugs del área D

Base auditada: v0.1.1, commit `6976afe`. Las ubicaciones apuntan al código de esa base. S1 corresponde a pánico, pérdida de datos, deadlock o seguridad. S2 corresponde a resultado incorrecto. S3 corresponde a un fallo menor o de experiencia de usuario.

## Hallazgos

| ID | Severidad | Archivo y línea | Reproducción | Esperado frente a real | Estado | Corrección sugerida |
|---|---|---|---|---|---|---|
| D01 | S1 | `crates/soryntel-app/src/http.rs:29–31` | Reservar un puerto efímero TCP en loopback y ejecutar `soryntel-app --port <puerto>` mientras el listener sigue abierto. | Esperado: error de arranque con diagnóstico y código de salida controlados. Real: pánico del hilo principal, salida 101 y `AddrInUse`. | CONFIRMADO | Propagar el error de bind al CLI e imprimir la dirección ocupada antes de salir con error. |
| D02 | S2 | `crates/soryntel-desktop/src/aqua_ui.rs:227–238` | En Aqua, preparar una fila de H₂O con temperatura `40,5` y cantidad `1,5` kmol y sincronizar para recalcular. | Esperado: calcular con 40,5 y 1,5, o detenerse y señalar el formato. Real: el caso recibe 25 y 0 sin señalar los campos. | CONFIRMADO | Parsear decimales según un formato explícito y validar las filas antes de reemplazar el caso de cálculo. |
| D03 | S1 | `crates/soryntel-desktop/src/excel_addins_ui.rs:1827–1835` | Poner `42` en C2 del Worksheet. Escribir `A0` en el cuadro de referencia y, desde el catálogo, pulsar «Insertar en celda activa del Worksheet». `ZZZZ0` sigue la misma ruta. | Esperado: rechazar la referencia y conservar C2. Real: inserta la fórmula en C2 y reporta éxito. | CONFIRMADO | Validar la referencia y los límites antes de escribir. Mostrar el error y conservar el contenido de la hoja. |
| D04 | S3 | `web/scripts/sync-release.mjs:39–61` | Ejecutar `sync-release.mjs --soft` con un manifiesto válido y una respuesta HTTP 200 de GitHub para una versión nueva con `assets: []`. | Esperado: conservar el manifiesto utilizable mientras se publican los paquetes. Real: éxito con lista vacía y desaparición de las descargas en el siguiente build del sitio. | CONFIRMADO | En modo `--soft`, mantener el último manifiesto utilizable cuando la respuesta no contiene paquetes publicados. Mantener la validación estricta de `--check`. |
| D05 | S3 | `.github/workflows/release.yml:158–176,204–209` | Simular el job `publish` de `workflow_dispatch` en una rama con los cinco paquetes de versión en `dist/`. Ejecutar el bloque Bash real de preparación. | Esperado: el paso manual encuentra los cinco paquetes y su suma de verificación. Real: el bloque los mueve a `release/`, pero la subida conserva el patrón `dist/*`, que queda vacío en la prueba. | CONFIRMADO | Subir desde `release/` y establecer `if-no-files-found: error` en el paso manual. |
| D06 | S3 | `soryntel/packaging/build-releases.sh:22–28,93–98` | Ejecutar `build-releases.sh window` con `DIST_DIR` en un directorio temporal. | Esperado: código distinto de cero y lista de destinos válidos. Real: sale con éxito, imprime «Artefactos» y deja el destino vacío. | CONFIRMADO | Validar el destino antes de crear carpetas o lanzar comandos de build. |
| D07 | S2 | `crates/soryntel-desktop/src/excel_addins_ui.rs:1071–1145` | Guardar `1` en A20 de Worksheet y sustituirlo por `=SUM(A20;1)`. | Esperado: marcar referencia circular. Real: el resultado es 17 tras las 16 pasadas, a partir del valor que tenía la celda antes de introducir la fórmula. | CONFIRMADO | Detectar dependencias circulares e invalidar sus celdas antes de publicar los resultados. |
| D08 | S3 | `crates/soryntel-desktop/src/lca_ui.rs:1225–1233`, `crates/soryntel-thermo/src/sim/lca.rs:1965–1987` | Configurar la sonda de LCA Beta con un servidor HTTP local que devuelve `200 OK` y el texto `OK` sin implementar openLCA. | Esperado: indicar que el endpoint no ofrece IPC de openLCA y mantener el catálogo local. Real: indica «Connected to openLCA IPC». | CONFIRMADO | Validar respuesta HTTP y JSON-RPC antes de marcar conexión. Ejecutar la sonda fuera del hilo de UI. |
| D09 | S3 | `crates/soryntel-app/src/ui.html:254–286` | Calcular para una única temperatura, por ejemplo `Desde = Hasta = 25`, y abrir la gráfica. La prueba ejecuta el JavaScript de la página con una fila de resultados. | Esperado: un marcador visible y un eje legible. Real: una polilínea con un único vértice sin marcador y etiquetas repetidas en la misma coordenada horizontal. | CONFIRMADO | Dibujar un marcador para resultados con un único punto y usar una escala horizontal con margen o una sola marca. |

Confirmados por severidad: **S1 2 · S2 2 · S3 5**.

## Verificación

Desde `soryntel/`:

```bash
cargo test --workspace
cargo test -p soryntel-app --test area_d -- --ignored
cargo test -p soryntel-desktop area_d_regressions -- --ignored
```

La primera ejecución omite las nueve regresiones. Las dos ejecuciones explícitas deben fallar en esta revisión, cinco en app y cuatro en escritorio. Las pruebas de app para D04 y D09 requieren Node.js con `--import`. D05 y D06 usan Bash y las utilidades de empaquetado invocadas por los bloques bajo prueba. D01 y D08 usan sockets reales en `127.0.0.1` con puertos efímeros. D05 reproduce localmente el bloque Bash y comprueba el patrón de subida del YAML sin ejecutar GitHub Actions.

`npm run lint` y `npm run build` pasan en `web/`. Se cotejaron nombres, tamaños y URLs de los cinco instaladores y `SHA256SUMS.txt` con la API de GitHub para v0.1.1. Las seis URLs devolvieron HTTP 200 con HEAD tras seguir redirecciones. [Release v0.1.1](https://github.com/jhulio435m/Soryntel-Chemistry/releases/tag/v0.1.1)

## Revisión pendiente de confirmación

- **HTTP.** CLI y bridge enlazan a loopback. El CLI incrusta el HTML y el bridge despacha endpoints fijos, sin un servidor de archivos estáticos que traduzca rutas HTTP al sistema de archivos. El bridge usa CORS permisivo para clientes de hojas de cálculo, un tope nominal de 1 MiB, timeouts por operación y procesamiento serial. El CLI lee una sola vez por conexión con un buffer fijo y un hilo por conexión. Revisar encuadre, límites, plazo total, concurrencia y origen permitido antes de ampliar el uso de los servicios. Los errores de sintaxis JSON en las rutas examinadas tienen ruta de error. La revisión de seguridad fue estática.
- **Locks.** Excel Add-Ins mantiene un guard de lectura del catálogo durante el frame, incluidas las rutas hacia los diálogos internos. El bridge usa otro guard de lectura y puede tomar el mutex de unidades bajo ese guard. Ambos lectores pueden coexistir. El bridge recupera el contenido de un mutex envenenado, mientras que algunas rutas de la UI descartan ese acceso. Falta medir la espera con escritores y operaciones de archivo. La inspección no confirma un deadlock.
- **Cálculo y UI.** Falta una reproducción con referencia científica para Pitzer e IF97 fuera del dominio, y una verificación de equivalencia al cambiar unidades. El probe de LCA sigue en la ruta del frame y usa timeouts de conexión y E/S de 250 ms. D08 verifica la clasificación falsa del endpoint, sin medir la latencia del frame. En el canvas de simulación, la ruta examinada comprueba que haya al menos dos puntos antes de los `unwrap` de los extremos.
- **Release y empaquetado.** Se revisaron los inputs de caché y el script NSIS. El workflow genera SHA-256 y carece de paso de firma. Falta validar firma en el runner, construir y ejecutar NSIS con rutas con espacios, y comprobar los artefactos instalados. `makensis` no estaba disponible en esta máquina.
