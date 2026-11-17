# ADR 0006 · Retirar la fachada de aplicación

Fecha: 2026-11-17

## Contexto

La fachada obsoleta delegaba a los casos de uso y mantenía una segunda forma de invocarlos. El escritorio, el CLI y el bridge ya migraron a la API por capacidad.

## Decisión

Eliminar `SoryntelApplication` y migrar las pruebas de contrato científico a `use_cases`. Activar la auditoría de arquitectura en modo estricto en CI.

## Consecuencias

Los consumidores deben importar el caso de uso correspondiente. Las pruebas científicas siguen verificando los mismos valores esperados. Las infracciones de los límites auditados bloquean CI según las reglas del modo estricto.
