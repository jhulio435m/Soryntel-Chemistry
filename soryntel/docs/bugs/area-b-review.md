# Revisión independiente de los cambios científicos del área B

Revisión de `74f6b97..1123ed8` (rama `worktree-agent-a8af5dec61ace6948`) frente a `docs/bugs/area-b.md`. No se ha cambiado código de producción. Los casos analíticos se ejecutaron en tests temporales que no se han commiteado: `soryntel-thermo/tests/zz_review_tmp.rs` y `soryntel-application/tests/zz_review_tmp.rs`, ya borrados.

## Veredictos

| Bug | Veredicto | Resumen |
|---|---|---|
| B01 | VERIFICADO | La fila de fase de WJD/RAND sin ȳ coincide con la derivación. Kp/K = 1,000000 en 13 casos nuevos: Δn = −1, Δn = +2, gas + 2 sólidos puros, P de 0,2 a 10 bar. |
| B02 | VERIFICADO | El término R·T0·ln(P/P0) aparece una sola vez para cualquier T y P, en la rama de gas ideal y en la de H2O(g). Error < 1e-5 MJ/kmol en 10 casos. |
| B03 | VERIFICADO | ex_ch = ΔfG° + Σν·ex_e (Szargut 1988) da CO2 19,86, H2O(l) 0,94, H2O(g) 9,50, CO 275,08, CH4 831,7 y N2 0,72 kJ/mol. Si falta el elemento, error explícito o NaN. Hay una nota sobre la coherencia de datos para CaCO3. |
| B06 | VERIFICADO (core y exportación) | 1 W/m² = 3,412142 BTU/h × 0,09290304 ft²/m² = 0,316998 BTU/(h·ft²). La tabla de la UI (`soryntel-desktop/src/heat_loss_ui.rs:2137`) sigue usando `from_watts_factor()`. El autor ya la marca como BLOQUEADA y sigue mostrando valores 10,76 veces mayores. |
| B07 | VERIFICADO | ε(T,P) de Bradley y Pitzer (1979) con U1-U9 correctos. A_phi a 0/25/50/100 °C = 0,37672/0,39145/0,41030/0,46057, idéntico a mi cálculo independiente con CODATA. γ± y φ de NaCl coinciden con Robinson y Stokes a 0,1, 1 y 3 m. |
| B17 | VERIFICADO | E = ΔrG/(ν_e·F) con ν_e con signo equivale a −ΔrG_red/(zF) en los dos sentidos de escritura. |

No he encontrado ningún cambio INCORRECTO. Hay tres observaciones menores, ninguna de las cuales invalida un fix. Están al final.

## B01: derivación de la fila de fase (WJD 1958, Smith y Missen 1982, cap. 6)

Para una fase de mezcla α con estimación y > 0, ȳ = Σ y_i y f_i = μ°_i/RT + ln(y_i/ȳ) + ln γ_i. La aproximación cuadrática de G/RT es

Q(x) = Σ x_i f_i + ½ Σ y_i (x_i/y_i − x̄/ȳ)².

La condición estacionaria con multiplicadores π_e (restricciones Σ a_ei x_i = b_e) da

x_i = y_i (π·a_i + u − f_i),  u = x̄/ȳ.

Si se suma sobre la fase, x̄ = Σ y_i π·a_i + u·ȳ − Σ y_i f_i. Como u·ȳ = x̄, queda

**Σ_e π_e Σ_i a_ei y_i = Σ_i y_i f_i**  (fila de fase, sin término en ȳ ni en u).

Las filas de elementos son Σ_e π_e Σ_i a_ei a_e'i y_i + u·Σ a_e'i y_i = b_e' + Σ a_e'i y_i f_i. Coinciden con `mat[e][e2]`, `mat[e][m+α]` y `rhs[e]` del código, y `mat[m+α][m+α] = 0` también coincide. Las fases puras activas imponen π·a_k = μ°_k/RT (+ ln a_fija).

Con el término extra ȳ en el lado derecho, el punto fijo x = y exige u + π·a_i − f_i = 1 para todo i. Al sumar sale u = 0, es decir, π·a_i = f_i + 1. Cada especie de mezcla se resuelve como si su μ° fuera μ° + RT, de modo que K_GEM = K·e^(−Δn_mezcla). Eso coincide con los síntomas documentados (e⁻¹, e³). Además, las fases puras no se desplazaban, así que el equilibrio sólido-gas también era erróneo: P_CO2 = K/e. El fix (r_val = 0) es exactamente la ecuación derivada.

Efecto lateral: `pi_sol` se usa en modo celda para E = |RT·π_dis/F|. Con el fix π tiene ya su significado físico (μ/RT), pero no hay ningún test de `cell_voltage_v`. Recomiendo añadir uno, por ejemplo una pila Daniell con E° conocido.

## Tabla de casos analíticos (tests temporales)

Datos con Cp constante (tipo NIST): ΔfH/kJ·mol⁻¹, S°/J·mol⁻¹·K⁻¹, Cp. K = exp(−ΔrG°/RT) se calcula con `evaluate_species_si` (estado estándar 1 bar). Kp_GEM se forma con las fracciones molares y P que devuelve `solve_single_point`.

| Caso | T (K) | P (bar) | Esperado (K o valor) | Obtenido | Cociente |
|---|---|---|---|---|---|
| CO + Cl2 = COCl2 (Δn = −1) | 700 | 1 | 7,164901 | 7,164901 | 1,000000 |
| CO + Cl2 = COCl2 | 750 | 5 | 2,011870 | 2,011870 | 1,000000 |
| CO + Cl2 = COCl2 | 800 | 0,2 | 0,6603258 | 0,6603258 | 1,000000 |
| CH4 + H2O = CO + 3H2 (Δn = +2) | 850 | 1 | 0,3200334 | 0,3200334 | 1,000000 |
| CH4 + H2O = CO + 3H2 | 900 | 3 | 1,996527 | 1,996527 | 1,000000 |
| CH4 + H2O = CO + 3H2 | 950 | 10 | 10,44111 | 10,44111 | 1,000000 |
| CaCO3(s) = CaO(s) + CO2(g), con N2 inerte: P_CO2 = K | 1000 | 1 | 0,07789942 | 0,07789942 | 1,000000 |
| ídem | 1000 | 0,5 | 0,07789942 | 0,07789942 | 1,000000 |
| ídem | 1050 | 1 | 0,2134716 | 0,2134716 | 1,000000 |
| ídem | 1050 | 2 | 0,2134716 | 0,2134716 | 1,000000 |
| Ex_phys N2(g), MJ/kmol: Cp(T−T0) − T0[Cp ln(T/T0) − R ln(P/P0)] | 600 | 5 | 6,67527 | 6,67527 | 1 |
| ídem | 1000 | 0,5 | 8,18021 | 8,18021 | 1 |
| ídem | 298,15 | 20 | 7,39366 | 7,39366 | 1 |
| ídem (P = 1,0 exacto, rama antes excluida) | 400 | 1 | 0,38184 | 0,38184 | 1 |
| ídem | 800 | 1,01325 | 6,04448 | 6,04448 | 1 |
| ídem | 298,15 | 1 | −0,03263 | −0,03263 | 1 |
| Ex_phys H2O(g), corrección H2O activada | 500 | 5 | 5,55992 | 5,55992 | 1 |
| ídem | 600 | 1 | 3,10372 | 3,10372 | 1 |
| Ex_phys H2O(g), corrección desactivada | 500 | 1 | 1,57019 | 1,57019 | 1 |
| ídem | 700 | 10 | 10,62748 | 10,62748 | 1 |
| ex_ch CO2(g), kJ/mol (Szargut 1988) | 298,15 | — | 19,87 | 19,86 | — |
| ex_ch H2O(l) | 298,15 | — | 0,9 | 0,94 | — |
| ex_ch H2O(g) | 298,15 | — | 9,5 | 9,50 | — |
| ex_ch CO(g) | 298,15 | — | 275,1 | 275,08 | — |
| ex_ch CH4(g) | 298,15 | — | 831,2 | 831,7 | — |
| ex_ch N2(g) | 298,15 | — | 0,72 | 0,72 | — |
| ex_ch CaO | 298,15 | — | 110,2 | 111,1 | — |
| ex_ch CaCO3 | 298,15 | — | 1,0 | −0,46 | — |
| A_phi (kg/mol)^½ | 273,15 | 1,013 | 0,3767 (B-P 1979 recalculado) | 0,37672 | — |
| A_phi | 298,15 | 1,013 | 0,3915 (Pitzer 1991) | 0,39145 | — |
| A_phi | 323,15 | 1,013 | 0,4103 | 0,41030 | — |
| A_phi | 373,15 | P_sat | 0,4606 (B-P 1979 recalculado) | 0,46057 | — |
| NaCl 0,1 m, 25 °C: γ± / φ | 298,15 | — | 0,778 / 0,932 (R&S) | 0,7769 / 0,9321 | — |
| NaCl 1 m: γ± / φ | 298,15 | — | 0,657 / 0,936 | 0,6556 / 0,9359 | — |
| NaCl 3 m: γ± / φ | 298,15 | — | 0,714 / 1,045 | 0,7131 / 1,0457 | — |

Los valores de NaCl también coinciden a 1e-4 con Pitzer y Mayorga (1973) calculado a mano (β0 = 0,0765, β1 = 0,2664, Cφ = 0,00127, A_phi = 0,3915): 0,7768/0,9321, 0,6555/0,9359 y 0,7130/1,0457.

## B07: A_phi

- Las constantes U1-U9 coinciden con Bradley y Pitzer (1979), Tabla I. La forma ε = ε1000 + C·ln((B+P)/(B+1000)) es correcta.
- El prefactor 1,400608e6 coincide con (1/3)·√(2πN_A·1000)·(e²/4πε0k)^1,5 = 1,400595e6 (CODATA 2018) a 1e-5 relativo.
- ε recalculado: 87,856 (0 °C), 78,384 (25 °C), 69,877 (50 °C) y 55,459 (100 °C), frente a 87,90/78,38/69,91/55,51 tabulados.
- Las referencias del encargo de 0,3774 (0 °C) y 0,4592 (100 °C) no coinciden con Bradley y Pitzer recalculado (0,3767 y 0,4606). Las diferencias, de 0,0007 y 0,0014, están dentro de la dispersión entre correlaciones de ε y ρ (Archer y Wang 1990 da 0,3764 a 0 °C y 0,4606 a 100 °C). No las considero un fallo del fix.
- Observación menor, previa al fix: por encima de 100 °C ε se evalúa a P_sat, mientras que la densidad de Kell es la de 1 atm. El efecto es < 0,1 % hasta 150 °C.

## B02/B03: exergía

- Rama de gas ideal: s_t − s_dead = S°(T) − S°(T0) − R ln(P/1) + R ln(P0/1) = ΔS° − R ln(P/P0). El término de presión entra una vez para cualquier T y P, también para P = 1,0 bar exacto, que antes quedaba fuera de la rama. La tabla lo confirma.
- Rama del H2O vapor: pressure_ds = −R ln(P/P0) y dead_state_ds = 0, lo que es equivalente. Rama líquida: V·(P−P0) en H y nada en S, correcto para un líquido incompresible.
- `exergy.rs` (workbook EXE) tiene su propio cálculo: (H−H0) − T0[(S°(T) − R ln(P/P0)) − S°(T0)]. Ya era correcto y no se ha tocado.
- B03: los llamadores de `species_chemical_exergy_kj_mol` son `balance.rs`, `exergy.rs` (NaN + aviso) y `diagrams.rs` (omite la serie con aviso). Todos tratan el `Result`.
- Nota de datos, no del fix: con datos de catálogo no idénticos a los de Szargut aparecen desviaciones de ~1 kJ/mol (CaCO3 −0,46 frente a 1,0; CaO 111,1 frente a 110,2). La tabla de ex_e de Szargut se derivó con ΔfG antiguos y combinarla con otro ΔfG mueve ex_ch de las especies de referencia (CaCO3 es la referencia del Ca). Es una limitación conocida del método de Szargut, no un error de implementación.

## B17

Los términos llevan el signo (−coef en reactivos, +coef en productos).
- Reducción Fe³⁺ + e⁻ = Fe²⁺: ν_e = −1 y ΔG = −74,2 kJ, así que E = +0,769 V.
- Oxidación: ν_e = +1 y ΔG = +74,2 kJ, así que E = +0,769 V.
- Con e⁻ a ambos lados se usa el neto, que es correcto.
- Sin e⁻ no hay potencial, igual que antes.

## B06

El factor es correcto y está cubierto en `soryntel-core` y en la exportación (`heat_loss/model.rs:724`). Para kcal/h el área ya está en m², así que el factor 1 es correcto. Falta la UI: `soryntel-desktop/src/heat_loss_ui.rs:2137` debe usar `q_unit.flux_from_w_m2_factor()`. Ya está documentado como BLOQUEADO.

## Llamadores y regresiones

- `pitzer_a_phi`: solo la usa `eval_activity_and_osmotic` (todos los métodos de Aqua). El `water_dielectric_constant` de `ehph.rs` (Malmberg-Maryott) es una función distinta y no se ha tocado.
- `evaluate_balance_point_si`: la usa `balance_engine.rs` en dos sitios (filas y T adiabática). La T adiabática solo usa h_tot y no se ve afectada por el NaN de ex_ch.
- `flux_from_w_m2_factor`: se usa en core y en la exportación, falta la UI (ver B06).
- GEM: el cambio está en `run_wjd_active_set`, del que dependen todos los modos (NTP, NHP, celda, actividad fija).
- `cargo test --workspace`: todo verde en debug y en `--release`, con los totales de la sección siguiente.

## Resultado de los tests

- `cargo test --workspace` (debug): 236 pasados, 0 fallidos, 24 binarios de test.
- `cargo test --workspace --release`: 236 pasados, 0 fallidos, 24 binarios de test.
- Tests temporales de esta revisión: 6 de 6 pasan (5 en thermo, 1 en application). No se han commiteado.
