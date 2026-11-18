# ADR 0007 · SoryntelDB, base termoquímica propia

Fecha: 2026-11-18

## Contexto

El catálogo abierto integrado tiene cobertura limitada. SoryntelDB permite distribuir un catálogo termoquímico más amplio en un formato versionado y verificable.

## Decisión

1. Crear SoryntelDB v1 (`.sdb`), un documento JSON UTF-8 versionado que se genera con `sdb-convert` a partir de `Maindb6.HSC`, `MainDB6.REF` y `Maindb6.cat`. Guarda todos los juegos de datos de cada especie, sus metadatos, las referencias citadas y el diccionario `.REF`. Guarda también la procedencia (nombre, tamaño y SHA-256 de cada archivo fuente) y un texto de atribución. El esquema está en [soryntel-db-format.md](../soryntel-db-format.md).
2. Conservar exactamente los `f64` que produce el parser `.HSC`. La conversión es determinista: solo `created` depende del momento de la conversión.
3. Añadir la capa `DbSource::Soryntel`. Las variantes anteriores conservan su nombre serde. La precedencia pasa a ser **OwnDB > MainDB > SoryntelDB > OpenDefault**, y OpenDefault solo se usa si no hay SoryntelDB cargada. MainDB sigue disponible como base externa opcional.
4. Exponer `load_soryntel_db` en `ThermoDatabase` y en `SpeciesDatabaseAdminPort`, y `default_soryntel_db` en `DatabaseLocatorPort`.
5. Cargar SoryntelDB al arrancar el escritorio y el CLI. La ruta se resuelve en este orden:
   1. `SORYNTEL_SDB`. Si la ruta está definida y no carga, no se recurre a las siguientes.
   2. `<directorio del ejecutable>/Databases/soryntel.sdb`.
   3. `<Documentos>/Soryntel/Databases/soryntel.sdb`, con la misma carpeta de datos del usuario que el [ADR 0003](0003-precedencia-unificada.md).

   Si no se encuentra o no carga, el escritorio muestra en el banner de avisos «SoryntelDB no encontrada; catálogo limitado» y el CLI escribe el aviso en stderr. La falta de MainDB ya no genera aviso.
6. No incluir nunca en el repositorio datos fuente del catálogo externo ni `.sdb` generados, tampoco en pruebas o fixtures. Las pruebas con datos reales leen `SORYNTEL_TEST_MAINDB` y se omiten si no está definida. El resto genera archivos sintéticos en memoria.

## Consecuencias

- Con SoryntelDB instalada, el catálogo resuelto tiene las mismas especies que la catálogo externo de origen y los resultados no cambian respecto a la carga directa del `.HSC`. La validación con la MainDB real lo comprueba bit a bit.
- Las especies que solo existen en OpenDefault dejan de estar disponibles cuando hay SoryntelDB. Hoy todas las especies de OpenDefault están también en la catálogo externo de origen.
- El `.sdb` ocupa unos 30 MB, frente a los 9,4 MB del `.HSC`. En release carga en un tiempo del mismo orden que el `.HSC` (≈0,2 s).
- El empaquetado debe instalar `soryntel.sdb` en `Databases/` junto al ejecutable. La atribución en la UI y `LICENSE-DATA` se tratan aparte.
- Un cambio incompatible del esquema requiere `version: 2` y un lector nuevo. Un lector v1 rechaza esos archivos con un error explícito.
