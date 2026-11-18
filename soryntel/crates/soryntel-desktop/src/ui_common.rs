use eframe::egui;

/// Cross-platform path inside the OS temporary directory (`/tmp/<name>` on Linux, `%TEMP%\<name>` on Windows).
pub fn temp_file_path(filename: &str) -> String {
    std::env::temp_dir()
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

/// Parser canónico de números escritos por el usuario en la interfaz.
///
/// Recorta espacios y acepta punto o coma decimal: la coma se interpreta como separador
/// decimal solo si el texto no contiene ningún punto (`"40,5"` → 40.5, `"1,000.5"` → error).
/// Rechaza textos vacíos, `inf`, `NaN` y cualquier valor no finito (p. ej. `1e400`).
/// Quien lo use debe mostrar el error y conservar el valor anterior, sin valores por defecto.
pub fn parse_user_number(text: &str) -> Result<f64, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Introduzca un número.".into());
    }
    let normalized = if trimmed.contains('.') {
        std::borrow::Cow::Borrowed(trimmed)
    } else {
        std::borrow::Cow::Owned(trimmed.replace(',', "."))
    };
    let invalid = || format!("«{trimmed}» no es un número válido.");
    // Solo dígitos, signo, separador decimal y exponente: evita que `inf` o `nan` se acepten.
    if !normalized
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
    {
        return Err(invalid());
    }
    let value = normalized.parse::<f64>().map_err(|_| invalid())?;
    if !value.is_finite() {
        return Err(format!("«{trimmed}» está fuera del rango representable."));
    }
    Ok(value)
}

/// Acumula los errores al leer varios campos numéricos de un formulario.
///
/// Cada lectura devuelve un valor provisional para poder seguir construyendo la petición,
/// pero si algún campo no es válido [`NumberFields::check`] devuelve el error y quien llama
/// no debe usar el resultado ni cambiar su estado.
#[derive(Default)]
pub struct NumberFields {
    errors: std::cell::RefCell<Vec<String>>,
}

impl NumberFields {
    /// Campo obligatorio.
    pub fn req(&self, text: &str) -> f64 {
        parse_user_number(text).unwrap_or_else(|e| {
            self.errors.borrow_mut().push(e);
            0.0
        })
    }

    /// Campo opcional: vacío significa `default`; cualquier otro texto debe ser un número.
    pub fn or(&self, text: &str, default: f64) -> f64 {
        if text.trim().is_empty() {
            default
        } else {
            self.req(text)
        }
    }

    /// `Ok` si todos los campos leídos eran válidos; si no, los errores unidos.
    pub fn check(&self) -> Result<(), String> {
        let errors = self.errors.borrow();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" "))
        }
    }
}

/// 4-column periodic-style element grid matching HSC6/HSC9 Tpp/Lpp (100 elements).
pub const ELEMENT_GRID_4COL: &[&str] = &[
    "Ac", "Dy", "Mn", "Ru",
    "Ag", "Er", "Mo", "S",
    "Al", "Es", "N", "Sb",
    "Am", "Eu", "Na", "Sc",
    "Ar", "F", "Nb", "Se",
    "As", "Fe", "Nd", "Si",
    "At", "Fm", "Ne", "Sm",
    "Au", "Fr", "Ni", "Sn",
    "B", "Ga", "Np", "Sr",
    "Ba", "Gd", "O", "Ta",
    "Be", "Ge", "Os", "Tb",
    "Bi", "H", "P", "Tc",
    "Bk", "He", "Pa", "Te",
    "Br", "Hf", "Pb", "Th",
    "C", "Hg", "Pd", "Ti",
    "Ca", "Ho", "Pm", "Tl",
    "Cd", "I", "Po", "Tm",
    "Ce", "In", "Pr", "U",
    "Cf", "Ir", "Pt", "V",
    "Cl", "K", "Pu", "W",
    "Cm", "Kr", "Ra", "Xe",
    "Co", "La", "Rb", "Y",
    "Cr", "Li", "Re", "Yb",
    "Cs", "Lu", "Rh", "Zn",
    "Cu", "Mg", "Rn", "Zr",
];

/// Draw a dashed or dotted line segment between `p0` and `p1`.
pub fn draw_dashed_line(
    painter: &egui::Painter,
    p0: egui::Pos2,
    p1: egui::Pos2,
    dash_len: f32,
    gap_len: f32,
    stroke: egui::Stroke,
) {
    let delta = p1 - p0;
    let total = delta.length();
    if total <= 1.0 {
        return;
    }
    let dir = delta / total;
    let mut dist = 0.0_f32;
    while dist < total {
        let end = (dist + dash_len).min(total);
        painter.line_segment([p0 + dir * dist, p0 + dir * end], stroke);
        dist += dash_len + gap_len;
    }
}

/// Compute a human-friendly axis tick step near `raw`.
pub fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let mag = 10_f64.powf(exp);
    let norm = raw / mag;
    let mult = if norm <= 1.5 {
        1.0
    } else if norm <= 3.0 {
        2.0
    } else if norm <= 7.0 {
        5.0
    } else {
        10.0
    };
    mult * mag
}

/// Upper bound on the number of ticks `nice_ticks` returns.
pub const MAX_TICKS: usize = 50;

/// Generate nice axis tick values within `[min, max]`.
///
/// Total for any input: non-finite bounds give no ticks, a degenerate range gives a single
/// tick, and the result never holds more than [`MAX_TICKS`] values even when the step is too
/// small to advance in floating point.
pub fn nice_ticks(min: f64, max: f64, approx_count: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() {
        return Vec::new();
    }
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    let span = max - min;
    if !span.is_finite() {
        return Vec::new();
    }
    if span <= 1e-12 {
        return vec![min];
    }
    let step = nice_step(span / approx_count.clamp(2, MAX_TICKS) as f64);
    let start = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut i = 0usize;
    while ticks.len() < MAX_TICKS {
        let v = start + i as f64 * step;
        if !v.is_finite() || v > max + 1e-9 * step {
            break;
        }
        if ticks.last().is_some_and(|&last: &f64| v <= last) {
            // El paso no avanza en coma flotante (valores enormes con rango pequeño).
            break;
        }
        ticks.push(v);
        i += 1;
    }
    ticks
}

/// Format an axis tick cleanly without `-0`.
pub fn format_tick(v: f64) -> String {
    if v.abs() < 1e-9 {
        "0".into()
    } else if (v - v.round()).abs() < 1e-6 {
        format!("{:.0}", v.round())
    } else {
        format!("{v:.1}")
    }
}

/// Draw a 90-degree counter-clockwise rotated Y-axis title.
pub fn draw_rotated_y_title(
    painter: &egui::Painter,
    x: f32,
    center_y: f32,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let mut shape = egui::epaint::TextShape::new(
        egui::pos2(x, center_y + 0.5 * galley.size().x),
        galley,
        color,
    );
    shape.angle = -std::f32::consts::FRAC_PI_2;
    painter.add(egui::Shape::Text(shape));
}

/// Return true if a chemical formula contains both C and H (organic heuristic).
pub fn is_organic_formula(formula: &str) -> bool {
    soryntel_application::use_cases::EquilibriumUseCases::parse_species_term(formula)
        .map(|p| p.elements.contains_key("C") && p.elements.contains_key("H"))
        .unwrap_or(false)
}

/// Shared tab action returned by `render_chart_tabs`.
pub enum ChartTabAction {
    None,
    SelectChart(usize),
    CloseChart(usize),
    SelectTable,
}

/// Render a shared `Chart 1 ×  Chart 2 ×  ...  Table` tab bar.
pub fn render_chart_tabs(
    ui: &mut egui::Ui,
    chart_names: &[&str],
    active_chart: Option<usize>,
    table_selected: bool,
) -> ChartTabAction {
    let mut action = ChartTabAction::None;
    ui.horizontal_wrapped(|ui| {
        for (idx, &name) in chart_names.iter().enumerate() {
            let selected = active_chart == Some(idx) && !table_selected;
            if ui.selectable_label(selected, name).clicked() {
                action = ChartTabAction::SelectChart(idx);
            }
            if chart_names.len() > 1 && ui.small_button("×").clicked() {
                action = ChartTabAction::CloseChart(idx);
            }
        }
        if ui.selectable_label(table_selected, "Table").clicked() {
            action = ChartTabAction::SelectTable;
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user_number_accepts_point_and_comma_and_rejects_non_finite() {
        assert_eq!(parse_user_number(" 40.5 "), Ok(40.5));
        assert_eq!(parse_user_number("40,5"), Ok(40.5));
        assert_eq!(parse_user_number("-1,25e3"), Ok(-1250.0));
        assert_eq!(parse_user_number("7"), Ok(7.0));
        for bad in ["", "  ", "abc", "inf", "-inf", "NaN", "infinity", "1e400", "1,000.5", "1,5,6"] {
            assert!(parse_user_number(bad).is_err(), "{bad:?} debería rechazarse");
        }
    }

    #[test]
    fn nice_ticks_is_total_and_bounded() {
        assert!(nice_ticks(0.0, f64::INFINITY, 6).is_empty());
        assert!(nice_ticks(f64::NAN, 1.0, 6).is_empty());
        assert!(nice_ticks(-f64::MAX, f64::MAX, 6).len() <= MAX_TICKS);
        assert_eq!(nice_ticks(3.0, 3.0, 6), vec![3.0]);
        let big = nice_ticks(1e17, 1e17 + 16.0, 6);
        assert!(!big.is_empty() && big.len() <= MAX_TICKS);
        assert!(big.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(nice_ticks(0.0, 10.0, 5), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(nice_ticks(10.0, 0.0, 5), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert!(nice_ticks(0.0, 1.0, 10_000).len() <= MAX_TICKS);
    }
}
