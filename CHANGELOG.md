# Cambios

## 0.1.2 — corrección de errores

Se hizo una revisión completa en cuatro áreas: base de datos, núcleo científico, escritorio y red/CI. Cada error confirmado tiene un test que lo reproduce y ahora pasa. El detalle de cada error, con su commit, está en `soryntel/docs/bugs/area-{a,b,c,d}.md`. La revisión independiente de los cambios científicos está en `soryntel/docs/bugs/area-b-review.md`.

### ⚠️ Cambios en resultados numéricos

Algunos resultados cambian porque antes eran incorrectos. Los valores nuevos se verificaron con casos analíticos y referencias publicadas.

- **Equilibrio (GEM).** La constante de equilibrio salía multiplicada por e^(−Δn) en todo sistema en el que cambia el número de moles, también en el modo adiabático. Por ejemplo, en N2O4 ⇄ 2NO2 con ΔG° = 0 se obtenía Kp = 0,368 en lugar de 1, y en CaCO3 = CaO + CO2 se obtenía P_CO2 = K/e. Ahora Kp coincide con exp(−ΔG°/RT) (White, Johnson y Dantzig, 1958; Smith y Missen, 1982).
- **Aqua (Pitzer).** A_phi salía un 6 % bajo porque faltaba el término de presión de Bradley y Pitzer (1979). A 25 °C pasa de 0,367 a 0,3915. γ± de NaCl 1 m pasa de 0,685 a 0,656 (Robinson y Stokes: 0,657). Cambian todos los resultados de Aqua: γ, φ, aw, presión de vapor y puntos de congelación y ebullición.
- **Exergía física.** La corrección de presión se contaba dos veces. O2 a 5 bar daba 7,95 MJ/kmol en lugar de 3,96, y el estado muerto no daba 0.
- **Exergía química.** Si falta un elemento de referencia en el catálogo, ahora se muestra un error en lugar de un valor parcial.
- **Heat Loss.** La densidad de flujo en BTU/(h·ft²) salía 10,76 veces mayor; ahora se aplica el factor de área.
- **Potenciales redox.** E ya no depende de si la semirreacción se escribe como reducción o como oxidación (Fe3+/Fe2+ da +0,769 V en los dos casos).
- **Ellingham, Eh-pH y Phase Stability.** Si falta la referencia de un elemento, se avisa y no se dibuja G absoluto como si fuera ΔG.

### Pérdida de datos y estabilidad

- **OwnDB.**
  - Editar una especie ya no borra sus juegos de datos alternativos.
  - Fusionar archivos .HSC o importar CSV conserva todos los juegos de datos.
  - «Guardar en OwnDB» (Benson, Estimate) escribe siempre en el archivo.
- **Archivos.**
  - Las escrituras .HSC y .CAT son atómicas.
  - Un .HSC truncado da error en lugar de cargarse en silencio.
  - Las fórmulas que no caben en el formato ya no se truncan.
- **Workbooks.**
  - Abrir un libro es atómico: si el archivo no es válido, el libro abierto queda intacto.
  - Los valores infinitos (por ejemplo K muy grande) ya no dejan el archivo ilegible.
- **Pánicos y bloqueos.** Se corrigieron pánicos y cuelgues con:
  - entradas extremas: ejes con `inf`, libros sin hojas, fórmulas muy anidadas, desbordamientos en el balanceador y en Add-ins;
  - un puerto HTTP ya ocupado.
- **Add-in SQLite.** Al reabrir el archivo ya no se revierten las ediciones del usuario.

### Interfaz

- **Números.** Todos los campos aceptan coma decimal (`40,5`). Un valor inválido muestra un error y conserva el valor anterior, sin sustituirlo en silencio.
- **Especies no encontradas.** Se marcan con ⚠, sus valores calculados se muestran como «—» y aparece el aviso «Cálculo incompleto». Antes se mostraban ceros.
- **Fallos de carga de MainDB/OwnDB.** Se muestran en la ventana y no solo en la consola.
- **Hoja de cálculo.**
  - Una referencia A1 inválida se rechaza.
  - Una referencia circular devuelve `#CIRC!`.
- **Exportaciones.** Todas usan el diálogo de guardado del sistema. Ya no hay rutas fijas como `/home/...` o `/tmp/...`.
- **Varias ventanas.**
  - Heat Loss envía su resultado al Balance abierto.
  - El launcher no abre dos veces el mismo módulo.

### Bases de datos

- **Orden de búsqueda de MainDB.** `--hsc-db` > `SORYNTEL_MAIN_DB` > `<exe>/Databases/` > `Documentos/Soryntel/Databases/`. Si una ruta explícita falla, no se recurre a las rutas por defecto.

### Desarrollo y CI

- **Auditoría de arquitectura.** Se añade la regla (g): no se permiten rutas fijas en el código.
- **Tests con la MainDB real.** Se activan con `SORYNTEL_TEST_MAINDB`.
- **Tests de reproducción.** Hay fuzzing de los parsers .HSC/.REF/CSV y del parser de fórmulas.

### Pendiente

- **Errores aplazados.** B18 (unidades en `.IGI`), B22 (signo en el resumen de Balance) y B24 (Criss-Cobble por encima de 25 °C) quedan documentados en `area-b.md`. También los bloqueos de las áreas A y D.
- **Datos.** OpenDefault solo incluye 15 especies. Además, el electrón no sigue la convención que hace E(SHE) = 0 a cualquier temperatura. Se resolverá con SoryntelDB.
