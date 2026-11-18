# Área B: bugs del núcleo científico

Alcance: `soryntel-core`, `soryntel-solver`, `soryntel-thermo` (reaction/calculate, species, equilibrium, ehph, diagrams, phase_stability, estimate, benson, balance y balance_engine, heat_loss, water, aqua, exergy, units, sim, excel_addins, spreadsheet_export) y `soryntel-application/use_cases`. Descubrimiento sobre `fork-main` (6976afe); corrección sobre `fork-main` (69f69c8) fusionado en la rama del área B.

Severidad: **S1**, pánico alcanzable o corrupción de datos; **S2**, resultado incorrecto; **S3**, menor. Diagnóstico: **CONFIRMADO** (había un test que fallaba), **PROBABLE** (deducido de la lectura del código) y **REQUIERE REVISIÓN EXPERTA**. Estado: **CORREGIDO** (con el commit), **APLAZADO** o **BLOQUEADO** (con el motivo).

Los tests de regresión ya no llevan `#[ignore]` y están en:

- `crates/soryntel-thermo/tests/bugs_area_b.rs` (B01-B10, B15-B17, B20, B21)
- `crates/soryntel-solver/tests/bugs_area_b.rs` (B11, B12) y el módulo de tests de `soryntel-solver/src/lib.rs` (B23)
- `crates/soryntel-application/tests/bugs_area_b.rs` (B01 con Ni(CO)4, B07 con NaCl, B13, B14; usan el catálogo OpenDefault)
- módulos de tests de `soryntel-core/src/serde_nonfinite.rs` (B04), `soryntel-core/src/lib.rs` (B06), `balance_engine.rs` (B19) y `estimate.rs` (B25)

Las líneas de la tabla se refieren a 6976afe.

## Resumen

| Severidad | Total | Corregidos | Aplazados / bloqueados |
|---|---|---|---|
| S1 | 5 (B04, B11, B13, B15, B16) | 5 | 0 |
| S2 | 11 (B01-B03, B05-B07, B09, B10, B14, B17, B18) | 10 | 1 (B18) |
| S3 | 9 (B08, B12, B19-B25) | 7 | 2 (B22, B24) |

Datos: D02 BLOQUEADO (el registro de e- está en `soryntel-db`); D04 documentado sin cambio (falta una referencia citada); D01, D03 y D05-D07 sin tocar por decisión.

## Tabla de bugs

| ID | Sev. | Archivo:línea | Reproducción | Esperado frente a real | Diagnóstico | Fix sugerido | Estado |
|---|---|---|---|---|---|---|---|
| B01 | S2 | `soryntel-thermo/src/equilibrium/gem_solver.rs:702` | GEM con N2O4(g) y NO2(g), ambos con G°(T) = 0 (ΔG° = 0), 500 K y P = 0,1, 1 y 10 bar. Test `b01_*`. Con datos reales, Ni + 4CO(g) = Ni(CO)4(g) a 300-450 K da K_GEM / K_reacción = 20,09 = e³. | Kp = 1. Hoy Kp = e⁻¹ = 0,3679 a cualquier presión. En general K_GEM = K·e^(−Δn_mezcla): cada especie de una fase de mezcla converge con μ_i/RT = π·a_i − 1. Afecta a todos los equilibrios con Δn ≠ 0, incluido el modo adiabático N,H,P. | CONFIRMADO | La fila de fase de WJD debe ser Σ_e π_e b_e^α = Σ_i y_i f_i (White, Johnson y Dantzig 1958; Smith y Missen 1982, método RAND). Quitar `y_phase_tot[alpha]` del RHS (`r_val = 0.0`) y comprobar u = x̄/ȳ. | CORREGIDO (76f6740) |
| B02 | S2 | `soryntel-thermo/src/balance.rs:246` y `:262` | `evaluate_balance_point_si(O2(g), 298,15 K, p, corr = false)`. Test `b02_*`. | Ex_phys = R·T0·ln(P/P0): 3,957 MJ/kmol a 5 bar y 0 a 1,01325 bar. Hoy da 7,947 y 0,0326 porque `s_t` ya incluye −R·ln(P/1 bar) y se le vuelve a sumar `ds_pressure_gas` = −R·ln(P/P0). Lo mismo ocurre con H2O vapor si `h2o_pressure_corr = false` y P ≠ 1 bar. | CONFIRMADO | Aplicar una sola corrección, siempre respecto a P0 = 1,01325 bar del estado muerto (Szargut, Morris y Steward 1988). | CORREGIDO (32d984e). P0 = 1,01325 bar (constante `P_REF_BAR`): O2 a 5 bar da R·T0·ln(5/1,01325) = 3,957 MJ/kmol; con P0 = 1 bar sería R·T0·ln 5 = 3,990. |
| B03 | S2 | `soryntel-thermo/src/balance.rs:194` | `species_chemical_exergy_kj_mol(Al2O3)` con un catálogo sin Al, como OpenDefault. Test `b03_*`. | ex_ch(Al2O3) = 200,4 kJ/mol (Szargut 1988). Hoy da 183,6: se omite G(Al) en ΔfG sin avisar. TiO2 da 12,3 frente a 21,4. Afecta a Balance (columna Exergy), a Diagrams ExExergy y a Exergy. | CONFIRMADO | Si falta la referencia de un elemento, devolver «no disponible» (`Option`/`Result`, o 0,0 según la convención ya usada para un elemento sin exergía) y avisar en la UI. | CORREGIDO (c3c1ffe) |
| B04 | S1 | `soryntel-thermo/src/lib.rs:402` y `serialize_rea8_workbook` (`lib.rs:690`) | Recoger una hoja con log K > 308, p. ej. 4Al + 3O2(g) = 2Al2O3 a 25 °C (log K ≈ 554), y guardar el .rea8. Test `b04_*`. | El .rea8 se vuelve a abrir. Hoy `k_eq = +inf` se escribe como `null` y la carga falla con «invalid type: null, expected f64»: el workbook queda ilegible. Pasa lo mismo con cualquier f64 no finito en .bal8 o .exe8 (p. ej. `temp_display` = 1e300 en Balance). | CONFIRMADO | Serializar los no finitos de forma explícita (`serde_with`, cadena "inf" o `Option<f64>`) o no persistir `k_eq` (se deriva de `log10_k`). Validar los no finitos al guardar. | CORREGIDO (7bb299a). Helper común `soryntel_core::serde_nonfinite` en todos los `f64`/`Option<f64>` serializables de thermo; K se guarda como "inf" y se relee (no se pasó a recalcular desde log K porque el helper ya cubre todos los campos). Quedan fuera los `f32` de la vista de Sim y los `f64` dentro de `Vec`/tuplas. |
| B05 | S2 | `soryntel-thermo/src/sim/mass_balance.rs:324` y `:350` | `serde_json::to_string_pretty(&preset_rougher_flotation_mb())`, igual que *Backup → Export (.HSCMas JSON)* (`mass_balance_ui.rs:649`). Test `b05_*`. | Se genera el JSON. Hoy da `Err("key must be a string")`: los mapas `BTreeMap<CellKey, _>` y `BTreeMap<(usize, usize), _>` no se pueden serializar a JSON y la copia muestra «Serialize error». | CONFIRMADO | Serializar los mapas como listas de pares (como `cell_map_serde` en `balance_engine.rs`) o con `serde_with::As<Vec<(_, _)>>`. | CORREGIDO (fe4459a) |
| B06 | S2 | `soryntel-core/src/lib.rs:275` y `soryntel-thermo/src/heat_loss/model.rs:690` (también la UI de `heat_loss_ui.rs`, fila 19) | Heat Loss, unidad BTU/h: el flujo se muestra o exporta en «BTU/(h·ft²)». Test `b06_*`. | 1 W/m² = 0,3170 BTU/(h·ft²). Hoy se multiplica solo por el factor de potencia: el valor está en BTU/(h·m²) y sale 10,76 veces mayor (1671,9 frente a 155,3). | CONFIRMADO | Añadir `HeatFlowUnit::flux_from_w_m2_factor()` con el factor de área (0,09290304 m²/ft² para BTU) y usarlo en la exportación y en la UI. | CORREGIDO (daf4981) en core y en la exportación. BLOQUEADO en la UI: la fila 19 de `soryntel-desktop/src/heat_loss_ui.rs` debe usar `HeatFlowUnit::flux_from_w_m2_factor()`. |
| B07 | S2 | `soryntel-thermo/src/aqua.rs:899` | `pitzer_a_phi(298,15)`; NaCl 1 mol/kg a 25 °C con Pitzer o Harvie. Tests `b07_*` en thermo y application. | A_phi = 0,3915 y, para NaCl 1 m, γ± = 0,657 y φ = 0,936 (Robinson y Stokes 1959; Pitzer y Mayorga 1973). Hoy A_phi = 0,3669, γ± = 0,685 y φ = 0,947. `water_dielectric_constant` solo usa ε1000 de Bradley y Pitzer (1979): da ε(25 °C) = 81,8 en lugar de 78,4. Afecta a todos los métodos de Aqua, también a «Davies». | CONFIRMADO | Añadir el término de presión ε = ε1000 + C·ln((B+P)/(B+1000)) con U4-U9 de Bradley y Pitzer (1979), a P = 1 bar o P_sat. | CORREGIDO (f5f1d56) |
| B08 | S3 | `soryntel-thermo/src/balance_engine.rs:670` | `set_cell_value(wb, "FEED!D3", 5)` sin una hoja de usuario llamada FEED. Test `b08_*`. | `Err(InvalidCellReference)`. Hoy `parse().unwrap_or(1)` hace que cualquier hoja desconocida apunte al área 1: escribe 5 kmol en IN1!D3. Lo puede provocar una fila de Target o un diagrama mal escrito. | CONFIRMADO | Exigir los prefijos IN/OUT seguidos de un número válido y devolver un error en otro caso. | CORREGIDO (9fa0da2) |
| B09 | S2 | `soryntel-thermo/src/balance_engine.rs:558` | `solve_adiabatic_temperature` con una especie de salida que no está en el catálogo. Test `b09_*`. | `Err(SpeciesNotFound)`. Hoy devuelve `Ok(1000 °C)`, calculado como si esa especie (1 kmol) no existiera. | CONFIRMADO | Fallar, o al menos avisar, cuando `get_species` devuelve `None` en `eval_f`. | CORREGIDO (cf28d69) |
| B10 | S2 | `soryntel-thermo/src/balance_engine.rs:341` | Evaluar un workbook, cambiar una fórmula por otra inexistente y volver a evaluar. Test `b10_*`. | Los valores calculados de esa fila se anulan. Hoy `heat_content`, `total_h`, `amount_kg`, `ex_*`, etc. conservan los valores de la especie anterior y entran en los totales de IN/OUT. También se conservan si falla `evaluate_balance_point_si`, porque el `if let Ok` no tiene `else`. | CONFIRMADO | Poner a cero los campos calculados cuando falta la especie o falla la evaluación. | CORREGIDO (cf28d69) |
| B11 | S1 | `soryntel-solver/src/lib.rs:344-354` (`Rat::sub/mul/div`) y `:328` | `balance_reaction("C99999999999999999999999999999999999999999 + O2 = CO2", 1)`. Test `b11_*`. | `Err(BalanceError)`. Hoy hay pánico «attempt to multiply with overflow» en debug, y en release (sin overflow-checks) la aritmética i128 se envuelve en silencio. Con subíndices realistas de seis decimales no se ha reproducido. | CONFIRMADO | Usar `checked_mul/checked_sub` y devolver un error, o pasar a racionales de precisión arbitraria. Limitar los subíndices en el parser. | CORREGIDO (be23568) |
| B12 | S3 | `soryntel-solver/src/lib.rs:551` | `balance_reaction("C100000000000000000000 + O2 = CO2", 1)`. Test `b12_*`. | La ecuación se muestra con 100000000000000000000O2. Hoy `t.coeff.round() as i64` satura y muestra 9223372036854775807O2, aunque el coeficiente guardado es 1e20. | CONFIRMADO | Formatear el f64 directamente (`{:.0}`) o usar i128. | CORREGIDO (be23568) |
| B13 | S1 | `soryntel-thermo/src/excel_addins.rs:948` y `:953` | Fórmula de Add-in `=Species(2,1e20)`, que llega por el bridge HTTP. Test `b13_*`. | Error de celda (#VALUE!/#N/A). Hoy `pos as usize` satura a `usize::MAX` y `pos + 8` desborda: pánico en debug y envoltura silenciosa en release. | CONFIRMADO | Validar `pos` (finito, de 1 a `species_count`) y usar `saturating_add`. | CORREGIDO (a70a6f4); también la conversión de columnas A1 (`checked_mul`) |
| B14 | S2 | `soryntel-thermo/src/diagrams.rs:232` | Diagrama ΔG (Ellingham) de Al2O3 con OpenDefault (sin Al). Test `b14_*`. | ΔfG(298,15 K) = −1582,3 kJ/mol (JANAF), o un aviso. Hoy `unwrap_or((0,0,0,0))` dibuja G absoluto (−1690,9 kJ/mol) con la etiqueta ΔG y sin aviso. El mismo patrón aparece en `ehph.rs:237-240` (ΔfG de especies Eh-pH), `ehph.rs:446-449` (ΔfG del H2O fijo en −56,678 kcal a cualquier T si falta H2O) y `phase_stability.rs:661-666`. Esos tres casos son PROBABLES. | CONFIRMADO | Propagar el error de `sum_element_references` como aviso y omitir la serie, en lugar de sustituirlo por cero. | CORREGIDO (1ceb327), también ehph y phase_stability. En Eh-pH, si falta H2O se usa -56,678 kcal/mol solo con aviso visible (un test del escritorio calcula a 80 °C con OpenDefault). |
| B15 | S1 | `soryntel-thermo/src/heat_loss/conduction.rs:33` y `:37` | `calculation_grid` enorme en una capa sólida (.htr8 editado o entrada de UI). Test `b15_*`. | Error de validación. Hoy `Vec::with_capacity(m)` entra en pánico con «capacity overflow» (o agota la memoria con valores grandes pero representables). `calculation_grid` no tiene cota superior. | CONFIRMADO | Validar `1 ≤ calculation_grid ≤ N` (p. ej. 10 000) al cargar y al resolver. | CORREGIDO (e620831) |
| B16 | S1 | `soryntel-thermo/src/diagrams.rs:168-172` | `DiagramRequest` con `t_min_k = 1e16`, `t_max_k = 1,0000000000000004e16` y `step_k = 1`. | Error de rango. Hoy `t += step` no cambia `t` (ulp = 2), el bucle no termina y el `Vec` crece hasta agotar la memoria. Con rangos grandes pero finitos (`t_max_k = 1e9`) se generan 1e9 puntos. No hay límite de puntos, a diferencia de `calculate()`, que limita a 100 000. | PROBABLE (no se ejecuta en test porque se colgaría) | Calcular el número de pasos con `floor((t_max − t_min)/step)`, limitarlo (p. ej. a 100 001) y generar los puntos por índice. | CORREGIDO (e0ac392) |
| B17 | S2 | `soryntel-thermo/src/lib.rs:374` y `:410` | «Fe(+2a) = Fe(+3a) + e-» frente a «Fe(+3a) + e- = Fe(+2a)». | Ambas formas deberían dar el mismo potencial de reducción, que es lo que dice el título del gráfico («Potencial estándar de reducción»). Hoy `n_electrons` suma \|coef\| sin signo y E = −ΔG/(nF), así que una oxidación da −E_red. Si hay e- en ambos lados, n es incorrecto. | PROBABLE / REQUIERE REVISIÓN EXPERTA (convención HSC) | Usar n con signo, Σ ν(e-) con ν negativo en reactivos: E = ΔG/(n_signed·F). | CORREGIDO (afefbab), con la cita IUPAC |
| B18 | S2 | `soryntel-thermo/src/equilibrium/gem_solver.rs:925-943` | `.IGI` con `input_scale_to_mol = 1` y cantidades en kg o Nm3. | Las unidades deberían ser coherentes. Hoy kmol usa `scale_to_mol`, pero kg y Nm3 multiplican siempre por 1000 (mol reales), así que en escala molar se mezclan órdenes de magnitud. g/l y mol/l suponen 1 L sin usar el volumen de solución. | REQUIERE REVISIÓN EXPERTA | Documentar la convención de cada unidad y aplicar `scale_to_mol` de forma uniforme. | APLAZADO: no hay referencia de la convención de unidades de los `.IGI`; cambiarla sin ella alteraría resultados. |
| B19 | S3 | `soryntel-thermo/src/balance_engine.rs:641` | `brent_root` tras 100 iteraciones, o con f(a) o f(b) igual a NaN (`fa*fb > 0` es falso con NaN). | Debería devolver un error de no convergencia. Hoy devuelve `Ok(b)` en silencio. También afecta a `iterate_target_rows` y a la temperatura adiabática. El análogo en `heat_loss/target.rs:244-250` devuelve `Ok` con `iterations = max_iter` y sin indicador de convergencia. | PROBABLE | Devolver `Err(TargetNotBracketed/CircularConvergenceFailed)` y rechazar NaN. | CORREGIDO (9fa0da2) |
| B20 | S3 | `soryntel-thermo/src/balance_engine.rs:780` | `get_cell_value(wb, "BALANCE1!XYZ")`. | `Err(InvalidCellReference)`. Hoy devuelve el balance de H total como si fuera la celda pedida. | PROBABLE | Devolver un error en el brazo `_`. | CORREGIDO (9fa0da2) |
| B21 | S3 | `soryntel-thermo/src/balance_engine.rs:1085-1088` | Fórmula `=1/0` en una celda de cantidad. | #DIV/0! o un error. Hoy da 0 sin avisar y la cantidad queda en 0 kmol. | PROBABLE | Devolver `FormulaSyntaxError("división por cero")`. | CORREGIDO (9fa0da2) |
| B22 | S3 | `soryntel-thermo/src/balance_engine.rs:444` frente a `:420-430` | Resumen de un área. | El signo debería ser el mismo en todo el resumen. Hoy los totales usan OUT − IN (como indica el comentario HSC) y el balance de elementos usa IN − OUT. | REQUIERE REVISIÓN EXPERTA (convención HSC) | Unificar el signo o rotularlo en la UI. | APLAZADO: convención de signo de HSC sin referencia; cambiar el signo alteraría la presentación de los presets. |
| B23 | S3 | `soryntel-solver/src/lib.rs:87-99` | «Na+ + e- = Na»: `split_side` corta en el «+» de la carga. | Debería dar un error de sintaxis o interpretar la carga. Hoy «Na+» se convierte en «Na» neutro y la ecuación da un error engañoso («requires moving a species»). «Na+» sola sí da un error claro. | PROBABLE | Rechazar los términos que terminan en «+» o «-» fuera de paréntesis, o aceptar la notación iónica. | CORREGIDO (8251e1b) |
| B24 | S3 | `soryntel-thermo/src/lib.rs:152-158` | Criss-Cobble (`criss_cobble = true`) para iones acuosos por encima de 25 °C. | Criss y Cobble (1964) tabulan a(T) y b(T) a 60, 100, 150 y 200 °C, con valores distintos para cationes, aniones simples, oxianiones y oxianiones ácidos. Hoy se interpola linealmente hasta 300 °C con solo dos familias, y para aniones b(300 °C) = 0,35. Además, el Cp mostrado es el Cp medio, no Cp(T). | REQUIERE REVISIÓN EXPERTA | Usar la tabla original de Criss y Cobble (J. Am. Chem. Soc. 86, 5385 y 5390, 1964) con las cuatro familias. | APLAZADO: requiere las tablas originales de Criss y Cobble (1964) por familia de iones; no se cambia la fórmula sin ellas. |
| B25 | S3 | `soryntel-thermo/src/estimate.rs:1026` | `generate_cp_curve` con una fila que no se puede estimar. | Debería avisar u omitir el punto. Hoy usa Cp = 40 J/(mol·K) en silencio, y `fit_kelley_cp` ajusta Kelley sobre ese valor ficticio. | PROBABLE | Omitir los puntos sin estimación y avisar. | CORREGIDO (e0fed5f) |

## Clasificación de `unwrap`/`expect`

En `soryntel-thermo` hay 228 ocurrencias, pero 212 están en módulos `#[cfg(test)]`. El código de producción de área B tiene 22: 16 en thermo, 6 en solver y ninguna en core ni en application.

**Alcanzables desde la entrada del usuario: 0.** Los pánicos alcanzables encontrados no vienen de `unwrap` sino de aritmética y reservas de memoria sin comprobar: B11 (i128), B13 (usize), B15 (`capacity overflow`) y B16 (bucle infinito, probable).

| Ubicación | Clasificación |
|---|---|
| `soryntel-solver/src/lib.rs:257, 261, 274, 287` (pila de `parse_group`) | Invariante seguro: tamaño comprobado antes. |
| `soryntel-thermo/src/species.rs:242, 246, 259, 274` (misma pila) | Invariante seguro. |
| `soryntel-solver/src/lib.rs:473` (`expect` con nulidad 1) | Invariante seguro. |
| `soryntel-solver/src/lib.rs:495` (`min()` de `ints`) | Invariante seguro: hay al menos 2 columnas. |
| `soryntel-thermo/src/lib.rs:308` (`partial_cmp().unwrap()` sobre `t_min`) | Seguro con datos válidos. Solo fallaría con un `t_min` NaN en un OwnDB corrupto (no se ha probado). |
| `soryntel-thermo/src/heat_loss/conduction.rs:68` | Invariante seguro: va después de `solid_data.is_none()`. |
| `soryntel-thermo/src/heat_loss/diagram.rs:181` | Invariante seguro: el bucle se ejecuta al menos una vez. |
| `soryntel-thermo/src/heat_loss/gas_radiation.rs:48` | Seguro: `NaN.max(1e-6)` devuelve 1e-6. |
| `soryntel-thermo/src/sim/lca.rs:1474, 1485, 1514, 1548` (`to_string_pretty`) | Seguro: serde_json no falla con `Value` ni con f64 no finitos. |
| `soryntel-thermo/src/sim/lca.rs:2449` | Invariante seguro: `ref_count == 1`. |
| `soryntel-thermo/src/sim/solver.rs:118, 122` (Tarjan) | Invariante seguro. |
| `soryntel-thermo/src/sim/mass_balance.rs:1027` | Invariante seguro: reinsertar una cota que ya estaba activa. |

Además, una exploración por mutación (todas las hojas numéricas de los presets de Balance, Heat Loss, Water, Exergy, Aqua, Sim y Estimate puestas a 0, −1, ±1e300 y 1e-300, y cadenas extrañas en parsers, SMILES, `eval_cp_expression` y todas las funciones de Add-ins) solo encontró, como pánicos, B13 y los no finitos que hacen ilegibles los ficheros (B04).

## Casos de referencia (datos integrados OpenDefault, 298,15 K salvo indicación)

| Caso | Soryntel | Referencia | Desviación | Clasificación |
|---|---|---|---|---|
| Al2O3: ΔfH, S | −1675,692 kJ/mol; 50,949 J/(mol·K) | JANAF: −1675,692; 50,950 | 0 | Correcto |
| Al2O3: Cp | 77,24 J/(mol·K) | JANAF 79,01 | −2,2 % | DATO D03 |
| O2(g): S, Cp | 205,029; 29,322 | JANAF 205,147; 29,376 | −0,06 %; −0,2 % | DATO D06 (menor) |
| H2(g): S, Cp | 130,570; 28,818 | JANAF 130,680; 28,836 | −0,08 % | DATO D06 (menor) |
| Mg(g): ΔfH, ΔfG (formato Delta) | 147,101; 112,483 kJ/mol | JANAF 147,1; 112,5 | < 0,05 % | Correcto |
| Zn, Cd, Mg: continuidad de G en la fusión | ΔG_tr = 1,2; −0,8; 0,06 J/mol | 0 | Redondeo | Correcto |
| TiO2: ΔG en la fusión (2143 K) | −410,9 J/mol | 0 (ΔS_tr = ΔH_tr/T_tr) | — | DATO D04 |
| Ni + 4CO(g) = Ni(CO)4(g), 300 K (calculate) | log K = 6,58 | Con ΔrH = −160,8 kJ y ΔrS = −410,1 J/K (NIST): 6,58 | 0 | Correcto |
| El mismo equilibrio por GEM | K_GEM = 20,09·K | K | ×e³ | B01 |
| P_sat del agua a 100, 200 y 300 °C | 1,01418; 15,549; 85,879 bar | IAPWS-IF97: 1,01418; 15,547; 85,879 | < 0,02 % | Correcto |
| Agua: Δh de 300 °C/25 bar (gas) a 25 °C/1 bar (líquido) | −2912,4 kJ/kg | IAPWS-IF97 −2904,7 | 0,26 % | Correcto: modelo aproximado documentado en `water.rs` |
| Vapor a 300 °C/25 bar: ρ, cp | 9,67 kg/m³; 2,44 kJ/(kg·K) | IAPWS-IF97: 10,11; ≈2,71 | −4 %; −10 % | Limitación documentada del modelo virial, no bug |
| NaCl 1 mol/kg (Pitzer): γ±, φ | 0,685; 0,947 | Robinson y Stokes: 0,657; 0,936 | +4,2 %; +1,2 % | B07 |
| Ex_phys de O2 a 5 bar y T0 | 7,947 MJ/kmol | R·T0·ln(P/P0) = 3,957 | ×2 | B02 |
| ex_ch(Al2O3) | 183,6 kJ/mol | Szargut 200,4 | −8 % | B03 |
| 1 atm a bar; 25 °C a K; 212 °F a °C | 1,01325; 298,15; 100 | Valores exactos | 0 | Correcto |
| 1 cal(th) a J; 1 kWh a J; 1 Btu(IT) a J | 4,1840006; 3 599 997; 1055,0604 | 4,184; 3 600 000; 1055,0559 | 0,15-4 ppm | DATO D07 |
| Ida y vuelta de las 90 magnitudes del libro de unidades | Sin errores (tolerancia 1e-9) | — | — | Correcto |
| ΔfH, S y Cp de H2O(l), H2O(g) y CO2(g); ΔfG del H2O | No calculable | JANAF | — | DATO D01 |

## DATOS (causa en los datos, no en el código)

| ID | Dato | Efecto | Sugerencia |
|---|---|---|---|
| D01 | OpenDefault solo trae 15 especies (Al2O3, TiO2, Mg, Mg(g), Zn, Zn(g), Cd, Cd(g), Ni, CO(g), Ni(CO)4(g), O2(g), H2(g), H(+a), e-). Faltan H2O(l/g), CO2(g), C, N2(g), Al, Ti, etc. | Los casos de referencia pedidos (agua, CO2, ΔfG del H2O) no se pueden calcular sin MainDB. El formato Delta falla con Al2O3, TiO2 y Ni(CO)4(g) por falta de la referencia del elemento (Al, Ti, C). Hace visibles B03 y B14 con la configuración por defecto. | Añadir al menos C, Al, Ti, N2(g), H2O(l), H2O(g) y CO2(g) de JANAF o CODATA. |
| D02 | `e-` en OpenDefault tiene H = S = Cp = 0, igual que `H(+a)`. | H2(g) = 2H(+a) + 2e- da E = −0,2017 V frente a SHE (debería ser 0 por definición), porque ΔG = −G_abs(H2) = +38,9 kJ. Todo potencial con e- queda desplazado 0,20 V a 25 °C (el signo depende del lado de e-; varía con T). | Definir e- con la convención G(e-) = ½·G(H2(g)) − G(H(+a)) (p. ej. S(e-) = ½·S(H2) = 65,34 J/(mol·K) con H(+a) = 0) o fijar la convención en el código. REQUIERE REVISIÓN EXPERTA. | **BLOQUEADO**: el registro está en `soryntel-db/src/lib.rs` (`open_reference_species`), fuera del área. Cambio propuesto, solo en ese registro: `cal_interval(298.15, 3000.0, Phase::AqueousIon, 0.0, 15.6035, 3.26, 0.39, 0.06, 0.0, "Convención SHE: S°, Cp = ½·H2(g)")`, es decir, S°(e-) = ½·S°(H2) y Cp(e-) = ½·Cp(H2) con H+(a) = 0, de modo que ΔrG(H2 = 2H+ + 2e-) = 0 y E(SHE) = 0 a cualquier T (IUPAC Green Book 2007, §2.13; convención de Pourbaix/HSC). El test pedido (0 V a 25, 100 y 200 °C) se añade con ese cambio. |
| D03 | Las referencias de Al2O3 se citan como «Barin 77, JANAF 98», pero el Cp ajustado a 298 K es 77,24 (JANAF 79,01). | Error de Cp del 2 % a 298 K. | Reajustar Kelley con JANAF. |
| D04 | TiO2 en la fusión: ΔH_tr = 16,000 kcal y ΔS_tr = 7,512 cal/K, pero ΔH/T_tr = 7,466. | G salta −411 J/mol en 2143 K (debería ser continuo). | Corregir ΔS_tr = ΔH_tr/T_tr. Sin cambio: no hay una referencia citada para elegir entre ΔH y ΔS (y el registro está en `soryntel-db`). |
| D05 | Al2O3: el primer intervalo empieza en 100 K y el segundo tiene ΔH_tr = 0 en 800 K (cambio de ajuste, no de fase). | Salto de Cp de 118,3 a 120,1 J/(mol·K) en 800 K. Es aceptable. | Ninguna. |
| D06 | S°(O2) = 49,003 cal y S°(H2) = 31,207 cal (205,03 y 130,57 J/(mol·K)) frente a JANAF 205,15 y 130,68. | Desviación de ≈0,1 J/(mol·K) en ΔS de las reacciones con O2 y H2. | Usar los valores CODATA (205,152 y 130,680). |
| D07 | Coeficientes de `units_data.rs` redondeados de HSC6: cal(th) 0,2390057, kWh 2,77778e-7, Btu(IT) 0,000947813 y curie 2,7e-11 (exacto: 1/3,7e10 = 2,7027e-11). Además, «dyne» y «poundal» aparecen en la magnitud Masa. | Errores de 0,15 a 4 ppm en energía; 0,1 % en curie. Unidades de fuerza listadas como masa. | Usar valores exactos (1/4,184, 1/3,6e6, 1/1055,05585262, 1/3,7e10) y mover dyne y poundal a Fuerza. |

## Cambios numéricos

Valores que cambian con las correcciones (antes → después). Ningún test previo de `scientific_contract.rs` ni de los módulos de equilibrio cambió su valor esperado: todos siguieron pasando sin modificarlos, porque sus aserciones (por ejemplo, NH3 > 1,9 mol a 298 K o la descomposición completa de CaCO3 a 1300 K) no dependían del factor e^(−Δn).

| Bug | Caso | Antes | Después | Comprobación |
|---|---|---|---|---|
| B01 | N2O4 = 2NO2 con ΔG° = 0, 500 K, 0,1-10 bar: Kp | 0,3679 (e⁻¹) | 1,0000 | Analítico: K = exp(−ΔG°/RT) = 1 |
| B01 | N2O4 = 2NO2 con datos NIST, 298-400 K y 1-10 bar: Kp_GEM/K | e⁻¹ | 1 (± 1e-4) | exp(−ΔG°/RT) con los mismos datos |
| B01 | Ni + 4CO = Ni(CO)4, 300-450 K, 1 bar: K_GEM/K | 20,09 (e³) | 1 (± 1e-3) | K del módulo de reacciones |
| B01 | Modo N,H,P (N2O4 + 30 kJ): K en la T de equilibrio | e⁻¹·K | K (± 1e-3); ΔH = 30,00 kJ | exp(−ΔG°/RT) a T_eq |
| B02 | O2(g) a T0 y 5 bar: Ex_phys | 7,947 MJ/kmol | 3,957 MJ/kmol | R·T0·ln(5/1,01325) |
| B02 | Gas a T0 y P0 = 1,01325 bar: Ex_phys | 0,0326 MJ/kmol | 0 | Estado muerto |
| B03 | Al2O3 sin Al en el catálogo (OpenDefault): ex_ch | 183,6 kJ/mol | Error «elemento de referencia Al»; en Balance, Ex químico/total = NaN con aviso | Szargut 200,4 (se obtiene 200,4 si se añade Al) |
| B06 | Flujo exportado en BTU/(h·ft²) (preset simple) | 1671,9 | 155,3 | ×0,316998 por W/m² |
| B07 | A_phi a 25 °C | 0,3669 | 0,3915 | Pitzer (1991): 0,3915 |
| B07 | A_phi a 50 °C | — | 0,4103 | ≈0,410 (Bradley y Pitzer 1979) |
| B07 | NaCl 1 mol/kg, 25 °C: γ±, φ | 0,685; 0,947 | 0,656; 0,936 | Robinson y Stokes: 0,657; 0,936 |
| B07 | Todos los resultados de Aqua (γ, φ, aw, presión de vapor, puntos de congelación y ebullición) | A_phi un 6 % bajo | Corregidos | — |
| B09 | Temperatura adiabática con una especie de salida ausente | 1000 °C (sin la especie) | Error SpeciesNotFound | — |
| B10 | Fila con especie ausente tras un cambio de fórmula | Valores de la especie anterior | 0 con aviso | — |
| B14 | Ellingham ΔG(Al2O3) con OpenDefault, 298 K | −1690,9 kJ/mol (G absoluto) | Serie omitida con aviso | JANAF ΔfG = −1582,3 |
| B14 | Eh-pH a T ≠ 25 °C sin H2O en el catálogo | ΔfG(H2O) = −56,678 kcal/mol en silencio | Mismo valor con aviso visible | — |
| B14 | Tabla de Phase Stability con referencia ausente: ΔfG | G absoluto | NaN | — |
| B17 | Fe2+ = Fe3+ + e- (escrita como oxidación): E | −0,769 V | +0,769 V (igual que la reducción) | IUPAC: E = −ΔrG/(zF) de la reducción |
| B21 | Fórmula `=1/0` en una cantidad | 0 kmol | Cantidad anterior conservada y aviso #DIV/0! | — |
| B25 | Cp estimado sin estimación disponible | 40 J/(mol·K) por punto | Punto omitido | — |
