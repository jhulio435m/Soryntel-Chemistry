# ADR 0005 · Modelos públicos desde application

## Contexto

Los clientes importaban tipos de thermo y solver para construir solicitudes, mostrar resultados y almacenar estado de UI.

## Decisión

Reexportar los DTO y las constantes de presentación desde `soryntel_application::models`, agrupados por capacidad. Los núcleos científicos conservan la definición original y los casos de uso ofrecen las operaciones.

## Consecuencias

Los clientes comparten la misma identidad de tipos y representación serializada. El CLI ya consume ese contrato. La sustitución de imports directos en el escritorio sigue en migración.
