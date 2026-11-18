# Bugs del área D

Base auditada: v0.1.1, commit `6976afe`. Las ubicaciones apuntan al código de esa base. S1 corresponde a pánico, pérdida de datos, deadlock o seguridad. S2 corresponde a resultado incorrecto. S3 corresponde a un fallo menor o de experiencia de usuario.

## Hallazgos

| ID | Severidad | Archivo y línea | Reproducción | Esperado frente a real | Estado | Corrección sugerida |
|---|---|---|---|---|---|---|
| D01 | S1 | `crates/soryntel-app/src/http.rs:29–31` | Reservar un puerto efímero TCP en loopback y ejecutar `soryntel-app --port <puerto>` mientras el listener sigue abierto. | Esperado: error de arranque con diagnóstico y código de salida controlados. Real: pánico del hilo principal, salida 101 y `AddrInUse`. | CORREGIDO · `fa31565` | Propagar el error de bind al CLI e imprimir la dirección ocupada antes de salir con error. |
| D02 | S2 | `crates/soryntel-desktop/src/aqua_ui.rs:227–238` | En Aqua, preparar una fila de H₂O con temperatura `40,5` y cantidad `1,5` kmol y sincronizar para recalcular. | Esperado: calcular con 40,5 y 1,5, o detenerse y señalar el formato. Real: el caso recibe 25 y 0 sin señalar los campos. | CORREGIDO · `31b0e74, 65d02c4` | Parsear decimales según un formato explícito y validar las filas antes de reemplazar el caso de cálculo. |
| D03 | S1 | `crates/soryntel-desktop/src/excel_addins_ui.rs:1827–1835` | Poner `42` en C2 del Worksheet. Escribir `A0` en el cuadro de referencia y, desde el catálogo, pulsar «Insertar en celda activa del Worksheet». `ZZZZ0` sigue la misma ruta. | Esperado: rechazar la referencia y conservar C2. Real: inserta la fórmula en C2 y reporta éxito. | CORREGIDO · `fa31565` | Validar la referencia y los límites antes de escribir. Mostrar el error y conservar el contenido de la hoja. |
| D04 | S3 | `web/scripts/sync-release.mjs:39–61` | Ejecutar `sync-release.mjs --soft` con un manifiesto válido y una respuesta HTTP 200 de GitHub para una versión nueva con `assets: []`. | Esperado: conservar el manifiesto utilizable mientras se publican los paquetes. Real: éxito con lista vacía y desaparición de las descargas en el siguiente build del sitio. | CORREGIDO · `bd3c7bf` | En modo `--soft`, mantener el último manifiesto utilizable cuando la respuesta no contiene paquetes publicados. Mantener la validación estricta de `--check`. |
| D05 | S3 | `.github/workflows/release.yml:158–176,204–209` | Simular el job `publish` de `workflow_dispatch` en una rama con los cinco paquetes de versión en `dist/`. Ejecutar el bloque Bash real de preparación. | Esperado: el paso manual encuentra los cinco paquetes y su suma de verificación. Real: el bloque los mueve a `release/`, pero la subida conserva el patrón `dist/*`, que queda vacío en la prueba. | CORREGIDO · `bd3c7bf` | Subir desde `release/` y establecer `if-no-files-found: error` en el paso manual. |
| D06 | S3 | `soryntel/packaging/build-releases.sh:22–28,93–98` | Ejecutar `build-releases.sh window` con `DIST_DIR` en un directorio temporal. | Esperado: código distinto de cero y lista de destinos válidos. Real: sale con éxito, imprime «Artefactos» y deja el destino vacío. | CORREGIDO · `bd3c7bf` | Validar el destino antes de crear carpetas o lanzar comandos de build. |
| D07 | S2 | `crates/soryntel-desktop/src/excel_addins_ui.rs:1071–1145` | Guardar `1` en A20 de Worksheet y sustituirlo por `=SUM(A20;1)`. | Esperado: marcar referencia circular. Real: el resultado es 17 tras las 16 pasadas, a partir del valor que tenía la celda antes de introducir la fórmula. | CORREGIDO · `31b0e74` | Detectar dependencias circulares e invalidar sus celdas antes de publicar los resultados. |
| D08 | S3 | `crates/soryntel-desktop/src/lca_ui.rs:1225–1233`, `crates/soryntel-thermo/src/sim/lca.rs:1965–1987` | Configurar la sonda de LCA Beta con un servidor HTTP local que devuelve `200 OK` y el texto `OK` sin implementar openLCA. | Esperado: indicar que el endpoint no ofrece IPC de openLCA y mantener el catálogo local. Real: indica «Connected to openLCA IPC». | CORREGIDO · `bd3c7bf` | Validar respuesta HTTP y JSON-RPC antes de marcar conexión. Ejecutar la sonda fuera del hilo de UI. |
| D09 | S3 | `crates/soryntel-app/src/ui.html:254–286` | Calcular para una única temperatura, por ejemplo `Desde = Hasta = 25`, y abrir la gráfica. La prueba ejecuta el JavaScript de la página con una fila de resultados. | Esperado: un marcador visible y un eje legible. Real: una polilínea con un único vértice sin marcador y etiquetas repetidas en la misma coordenada horizontal. | CORREGIDO · `bd3c7bf` | Dibujar un marcador para resultados con un único punto y usar una escala horizontal con margen o una sola marca. |
| D10 | S2 | `crates/soryntel-application/src/use_cases/aqua.rs:21` | Cargar NaCl–HCl y fijar el agua a 301 °C. La entrada activa Na⁺/Cl⁻ declara 300 °C como límite superior. | Esperado: advertir al usuario. Antes: el resultado se mostraba sin advertencia por ese parámetro. Ahora: Aqua reporta que la temperatura supera el rango. | CORREGIDO · `65d02c4` | Avisar también si se incumple la fuerza iónica declarada. La corrección mantiene las ecuaciones y el resultado numérico. |

D01–D09 corregidos por severidad: **S1 2 · S2 2 · S3 5**. La revisión adicional confirmó y corrigió D10 (S2).

## Verificación

Desde `soryntel/`:

```bash
cargo test --workspace
bash scripts/check_architecture.sh --strict
```

Las nueve regresiones originales están activas. Se añadieron casos para el puerto ocupado del bridge, formatos numéricos, un ciclo indirecto y su recuperación al romperlo, contención del catálogo, temperatura de Pitzer y límites numéricos de agua. Las pruebas D04 y D09 ejecutan Node.js. D05 y D06 ejecutan Bash. D05 verifica localmente el bloque de preparación y que el patrón de subida incluya los paquetes y `SHA256SUMS.txt`. D08 comprueba respuestas positivas y negativas con sockets TCP de loopback y un servicio simulado. [Contrato JSON-RPC de openLCA](https://greendelta.github.io/openLCA-ApiDoc/data/get_descriptors.html)

En `web/` pasan `npm run lint` y `npm run build`. En la fase de descubrimiento se cotejaron los cinco instaladores y `SHA256SUMS.txt` con la API de GitHub y sus seis URLs devolvieron HTTP 200. [Release v0.1.1](https://github.com/jhulio435m/Soryntel-Chemistry/releases/tag/v0.1.1)

## Comprobaciones adicionales y límites

- **Contención del catálogo.** Ocho clientes TCP enviaron `GET /health` mientras otro hilo retenía `AppServices::catalog_mut`. Las respuestas llegaron al soltar el guard según los plazos de la prueba. Esta cobertura prueba el orden escritor→lectores bajo esa carga.
- **Agua.** La UI ya avisa al exceder sus límites numéricos y conserva el resultado anterior. La prueba usa 2300 K frente al límite de 2273,15 K. El módulo documenta que usa aproximaciones y que la correlación de saturación procede de IAPWS. Los límites numéricos no certifican el dominio físico de todas las correlaciones.
- **Pitzer — BLOQUEADO en molalidad.** D10 comprueba temperatura y fuerza iónica declaradas. Falta definir qué molalidad corresponde al parámetro cuando el electrolito está en una mezcla de varias sales. La prueba de D10 confirma 300 °C frente a 301 °C para Na⁺/Cl⁻.
- **Validación A1.** Las ediciones del área D usan `AddinsUseCases::parse_a1` y su variante con límites de hoja. Unificar con el parser interno de expresiones de `soryntel-thermo` queda **BLOQUEADO** por el alcance de archivos de esta rama.
- **Validación decimal.** `excel_grid::parse_decimal_input` se comparte en escritorio dentro del área D. Unificarlo con los demás módulos queda **BLOQUEADO** por el alcance sobre `ui_common.rs`. Las celdas que admiten texto y fórmulas conservan esa semántica.
- **Release.** La prueba de D05 reproduce el paso localmente. Queda verificar una ejecución real de GitHub Actions y el instalador NSIS con rutas que contengan espacios.
