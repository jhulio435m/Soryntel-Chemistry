//! Tests de reproducción del área B para el balanceador estequiométrico.
//! Hoy FALLAN; se ignoran para mantener CI en verde (ver `docs/bugs/area-b.md`).

use soryntel_solver::balance_reaction;
use std::panic::catch_unwind;

/// Subíndices enormes en la fórmula (entrada libre del usuario) desbordan la
/// aritmética `i128` de `Rat`: pánico en debug, resultado envuelto en release.
#[test]
fn b11_balance_reaction_does_not_panic_on_huge_subscripts() {
    let r = catch_unwind(|| {
        balance_reaction("C99999999999999999999999999999999999999999 + O2 = CO2", 1.0)
    });
    assert!(r.is_ok(), "balance_reaction no debe entrar en pánico; debe devolver Err");
}

/// `format_reaction` convierte coeficientes enteros con `as i64` (saturante):
/// un coeficiente de 1e20 se muestra como 9223372036854775807.
#[test]
fn b12_format_reaction_keeps_large_integer_coefficients() {
    let (eq, parsed) = balance_reaction("C100000000000000000000 + O2 = CO2", 1.0).unwrap();
    assert_eq!(parsed.products[0].coeff, 1e20);
    assert!(!eq.contains("9223372036854775807"), "ecuación formateada: {eq}");
}
