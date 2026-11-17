# ADR 0004 · Catálogo compartido con el bridge

## Contexto

El bridge abierto desde la UI recibe una copia y no observa las ediciones posteriores de OwnDB.

## Decisión

Compartir la instancia mediante `Arc<RwLock<ThermoDatabase>>` desde la composición del escritorio. Entregar un préstamo del puerto a application durante la operación. El bridge toma un guard de lectura por petición y la UI usa un guard de escritura para las ediciones.

## Consecuencias

Las peticiones posteriores observan los cambios confirmados en el catálogo. Los guards deben liberarse antes de adquirir otro acceso incompatible. El diseño está implementado en T7 (`f36ddb8`) y su cableado en el escritorio sigue en migración en la base de esta guía.

## Limitación conocida

Add-ins y Species DB se abren en procesos distintos. El bridge comparte el catálogo con Add-ins dentro de su proceso, por lo que no ve los guardados de OwnDB de Species DB. Como seguimiento, se propone detectar cambios del archivo activo y recargar OwnDB antes de atender `/eval`.
