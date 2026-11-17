# ADR 0002 · Catálogo por parámetro

## Contexto

El mismo caso de uso se ejecuta desde el escritorio, el CLI, HTTP o una prueba con datos en memoria.

## Decisión

Recibir el puerto por parámetro. Usar un préstamo compartido para consultas y un préstamo mutable para operaciones de escritura. Las implementaciones de sincronización pertenecen al código que compone o invoca el caso de uso.

## Consecuencias

Cada llamada declara el acceso que necesita. Un cliente puede usar un adaptador en memoria y el escritorio puede prestar el puerto bajo un guard. Los puntos de entrada son responsables de resolver qué instancia prestar.
