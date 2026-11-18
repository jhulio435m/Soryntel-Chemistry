# Estado del proyecto (2026-11-18)

## Publicado
- **v0.1.1** en `fork/main` (jhulio435m/Soryntel-Chemistry): arquitectura hexagonal. CI y release en verde.

## Preparación local de 0.2.0
1. **Arquitectura hexagonal completa a nivel de dependencias.** `scripts/check_architecture.sh --strict` aplica las reglas a–g. Ver `docs/architecture/README.md` y los ADR 0001–0007.
2. **Corrección de errores.** Se corrigieron unos 99 errores en cuatro áreas (`docs/bugs/area-{a,b,c,d}.md`). Los cambios científicos tienen una revisión independiente en `docs/bugs/area-b-review.md`. Los cambios numéricos están en `CHANGELOG.md`.
3. **SoryntelDB v1.** Catálogo termoquímico versionado en formato `.sdb` (`docs/architecture/soryntel-db-format.md`, ADR 0007).
   - Contiene 21 089 especies y 21 210 juegos de datos, validados bit a bit contra el `.HSC`.
   - Precedencia: OwnDB > MainDB > SoryntelDB > OpenDefault.
   - Conversor: `cargo run -p soryntel-db --bin sdb-convert -- --main … --ref … --cat … --notice … --out …`.
4. **Empaquetado.** El release descarga `soryntel.sdb` desde el release `sdb-v2` del repo **privado** `jhulio435m/soryntel-data`, verifica su SHA-256 y lo incluye en los 5 paquetes (ver `docs/RELEASE.md` y `LICENSE-DATA.md`).
5. Tests: 334 en verde, en debug y en release, con y sin la MainDB real.

## Pendiente para publicar v0.2.0

- Publicar la copia con metadatos actualizados como `sdb-v2` en el repositorio privado y actualizar `SORYNTEL_SDB_SHA256` con su SHA-256.
- Verificar que `SORYNTEL_SDB_TOKEN` puede leer el archivo privado. Sigue los pasos de [publicación](RELEASE.md).
- Publicar la etiqueta `v0.2.0` y comprobar los cinco paquetes antes de sincronizar las descargas de la web.

## Pendiente técnico (siguiente iteración)
- Errores aplazados: B18, B22, B24 y los bloqueos .CAT/HSC antiguos (ver `docs/bugs/`).
- Anomalías en los datos del catálogo de entrada detectadas por el conversor: Cp/S° negativos, transiciones inconsistentes y referencias ausentes. Revisarlas con datos de referencia o una OwnDB de correcciones.
- Hexagonal al 100 %: la E/S de archivos de los workbooks sigue en la UI (`ProjectStoragePort` sin uso) y quedan métodos de cálculo de LCA/Mass Balance/units que se llaman desde la UI.
- Firma de código en Windows (SmartScreen): solicitar SignPath Foundation, con Certum como alternativa.
- Validar la migración de la interfaz a eframe/egui 0.37 en el próximo instalador.
- Localización: la UI mezcla inglés y español.
- Hablar con NnandoO: binarios y documentos de ingeniería inversa en el repo, y commits "WIP" que rompen CI.
