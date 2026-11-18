# ADR 0003 · Precedencia unificada del catálogo

## Contexto

La carga anterior del CLI conservaba OpenDefault por encima de MainDB. El escritorio daba prioridad a MainDB.

## Decisión

Aplicar OwnDB > MainDB > OpenDefault en ambos clientes. El CLI carga MainDB a través del puerto de administración con `OverrideOpenDefault`.

## Consecuencias

Una especie repetida se resuelve con la misma política en ambos clientes. Los resultados pueden cambiar cuando MainDB contiene datos diferentes. En la prueba con un HSC sintético, el cambio de capa reduce aproximadamente 10 kJ la entalpía y la energía de Gibbs de Al₂O₃ a 25 °C.

## Prioridad de las rutas

La ruta explícita de `--hsc-db` tiene prioridad sobre `SORYNTEL_MAIN_DB` y las rutas por defecto. Si falla la carga, se informa de la ruta intentada y MainDB queda sin cargar. OpenDefault sigue disponible y OwnDB se carga por separado, primero desde `SORYNTEL_OWN_DB` si está definida y luego desde las rutas por defecto si no lo está.

Escritorio y CLI resuelven las rutas en el mismo orden (`DatabaseDirLocator::standard`); se usa la primera que exista:

MainDB:

1. `--hsc-db <ruta>`.
2. `SORYNTEL_MAIN_DB`.
3. `<directorio del ejecutable>/Databases/Maindb6.HSC`.
4. Carpeta de datos del usuario: `<Documentos>/Soryntel/Databases/Maindb6.HSC`, donde `<Documentos>` es `~/Documents` o `~/Documentos` si existe y, si no, la carpeta personal (`HOME` o `USERPROFILE`).

OwnDB:

1. `SORYNTEL_OWN_DB`.
2. `<directorio del ejecutable>/Databases/OWNDB6.HSC`.
3. `<Documentos>/Soryntel/Databases/OWNDB6.HSC`.

Una ruta de los pasos 1 y 2 de MainDB (o 1 de OwnDB) que no carga no recurre a los siguientes. Si no se encuentra MainDB, el escritorio avisa de que solo está disponible el catálogo abierto e indica estas ubicaciones. Para crear una OwnDB nueva, el escritorio usa `<directorio del ejecutable>/Databases/OWNDB6.HSC`. No hay rutas fijas de ninguna máquina concreta ni rutas relativas al directorio de trabajo.
