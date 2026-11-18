# Publicación de instaladores con SoryntelDB

El workflow descarga el archivo `soryntel.sdb` del release indicado por `SORYNTEL_SDB_RELEASE` en el repositorio privado `jhulio435m/soryntel-data`. Configura estos secretos de GitHub Actions en el fork que publica las versiones:

| Secreto | Valor |
| --- | --- |
| `SORYNTEL_SDB_TOKEN` | Token de acceso personal de grano fino, limitado a `jhulio435m/soryntel-data` con permiso de lectura de Contents. |
| `SORYNTEL_SDB_SHA256` | SHA-256 del archivo, expresado como 64 dígitos hexadecimales. |

Una ejecución sobre una etiqueta falla si falta un secreto, si la descarga falla o si el archivo no supera la comprobación de integridad. Desde una rama, `workflow_dispatch` emite un aviso y genera paquetes sin la base en esos casos. En el repositorio de la aplicación, el archivo verificado se distribuye únicamente dentro de los paquetes, junto con su [licencia de datos](../../LICENSE-DATA.md).

## Actualizar la base

1. Publica un release con una etiqueta nueva en el repositorio privado `jhulio435m/soryntel-data` y adjunta el archivo como `soryntel.sdb`.
2. Actualiza `SORYNTEL_SDB_RELEASE` en [release.yml](../../.github/workflows/release.yml) con la nueva etiqueta. Calcula el hash del archivo adjunto con `sha256sum soryntel.sdb` y actualiza el secreto `SORYNTEL_SDB_SHA256` antes de publicar la versión de la aplicación.
3. Renueva el token antes de su vencimiento y reemplaza el valor de `SORYNTEL_SDB_TOKEN` en el fork que ejecuta el workflow.
