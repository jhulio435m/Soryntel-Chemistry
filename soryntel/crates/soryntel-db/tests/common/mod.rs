//! Utilidades compartidas por las pruebas de integración de `soryntel-db`.

use std::path::PathBuf;

/// Variable de entorno con la ruta a una `Maindb6.HSC` real para las pruebas que la necesitan.
pub const TEST_MAINDB_ENV: &str = "SORYNTEL_TEST_MAINDB";

/// Ruta de la MainDB real indicada en `SORYNTEL_TEST_MAINDB`. Si la variable no está
/// definida o el archivo no existe devuelve `None` y explica por stderr cómo activar la
/// prueba `test`, que debe saltarse sin fallar.
pub fn test_maindb(test: &str) -> Option<PathBuf> {
    let Some(path) = std::env::var_os(TEST_MAINDB_ENV).map(PathBuf::from) else {
        eprintln!(
            "[{test}] se omite: requiere una Maindb6.HSC real. \
             Defina {TEST_MAINDB_ENV}=/ruta/a/Maindb6.HSC para ejecutarla."
        );
        return None;
    };
    if !path.is_file() {
        eprintln!(
            "[{test}] se omite: {TEST_MAINDB_ENV}={} no es un archivo existente.",
            path.display()
        );
        return None;
    }
    Some(path)
}
