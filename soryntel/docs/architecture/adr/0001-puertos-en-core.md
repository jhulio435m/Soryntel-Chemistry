# ADR 0001 · Puertos de catálogo en soryntel-core

## Contexto

El cálculo y la aplicación necesitan especies, mientras que el formato HSC y sus archivos pertenecen a infraestructura.

## Decisión

Definir los puertos de catálogo, edición, administración, referencias, herramientas HSC y localización en `soryntel-core`. `soryntel-db` proporciona las implementaciones de producción.

## Consecuencias

El cálculo compila sin enlazar la base de datos concreta. Los contratos compartidos deben expresar datos y operaciones del catálogo sin incorporar detalles de la interfaz ni de su almacenamiento.
