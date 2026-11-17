# ADR 0003 · Precedencia unificada del catálogo

## Contexto

La carga anterior del CLI conservaba OpenDefault por encima de MainDB. El escritorio daba prioridad a MainDB.

## Decisión

Aplicar OwnDB > MainDB > OpenDefault en ambos clientes. El CLI carga MainDB a través del puerto de administración con `OverrideOpenDefault`.

## Consecuencias

Una especie repetida se resuelve con la misma política en ambos clientes. Los resultados pueden cambiar cuando MainDB contiene datos diferentes. En la prueba con un HSC sintético, el cambio de capa reduce aproximadamente 10 kJ la entalpía y la energía de Gibbs de Al₂O₃ a 25 °C.

## Prioridad de las rutas

La ruta explícita de `--hsc-db` tiene prioridad sobre `SORYNTEL_MAIN_DB` y las rutas por defecto. Si falla la carga, se informa de la ruta intentada y MainDB queda sin cargar. OpenDefault sigue disponible y OwnDB se carga por separado, primero desde `SORYNTEL_OWN_DB` si está definida y luego desde las rutas por defecto si no lo está.
