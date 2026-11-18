//! Enlaces entre ventanas de módulos, que se ejecutan en procesos distintos.
//!
//! Heat Loss → Balance: Balance publica cuántas áreas tiene el libro abierto; Heat Loss envía la pérdida de calor a
//! un área y Balance responde con un acuse que Heat Loss muestra al usuario. Todo va por
//! archivos pequeños en el directorio temporal del usuario, escritos de forma atómica.
use std::io;
use std::path::{Path, PathBuf};

/// Antigüedad máxima de una fórmula enviada de Species a Reaction: si ninguna ventana de
/// Reaction la recoge a tiempo, se descarta en lugar de aparecer días después.
pub const FORMULA_MAX_AGE_SECS: u64 = 15;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Codifica una fórmula enviada en el instante `sent_at` (segundos Unix).
pub fn encode_formula(formula: &str, sent_at: u64) -> String {
    format!("{sent_at}\n{}", formula.trim())
}

/// Decodifica un envío y lo descarta si es más antiguo que `max_age` o no tiene marca.
pub fn decode_formula(content: &str, now: u64, max_age: u64) -> Option<String> {
    let (stamp, formula) = content.split_once('\n')?;
    let sent_at: u64 = stamp.trim().parse().ok()?;
    let formula = formula.trim();
    (now.saturating_sub(sent_at) <= max_age && !formula.is_empty()).then(|| formula.to_string())
}

pub struct ModuleLink {
    dir: PathBuf,
    prefix: String,
}

impl ModuleLink {
    /// Enlace en el directorio temporal, con un prefijo por usuario para no mezclar sesiones
    /// de usuarios distintos que comparten `/tmp`.
    pub fn in_temp_dir() -> Self {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>();
        Self::new(std::env::temp_dir(), &user)
    }

    pub fn new(dir: impl Into<PathBuf>, user: &str) -> Self {
        let prefix = if user.is_empty() {
            "soryntel".to_string()
        } else {
            format!("soryntel_{user}")
        };
        Self { dir: dir.into(), prefix }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}_{name}", self.prefix))
    }

    fn take(path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        let _ = std::fs::remove_file(path);
        Some(content)
    }

    /// Species: envía una fórmula a la ventana de Reaction abierta.
    pub fn send_formula(&self, formula: &str) -> io::Result<()> {
        crate::storage::write_atomic(self.path("import_formula.txt"), encode_formula(formula, now_secs()))
    }

    /// Reaction: recoge una fórmula reciente; los envíos caducados se borran sin aplicarse.
    pub fn take_formula(&self) -> Option<String> {
        let content = Self::take(&self.path("import_formula.txt"))?;
        decode_formula(&content, now_secs(), FORMULA_MAX_AGE_SECS)
    }

    /// Heat Loss: envía `q_w` (W) al área `area_idx` (base 0).
    pub fn send_heat_loss(&self, area_idx: usize, q_w: f64) -> io::Result<()> {
        crate::storage::write_atomic(self.path("heat_loss_export.txt"), format!("{area_idx} {q_w}"))
    }

    /// Balance: recoge un envío pendiente. `Err` si el archivo no tiene el formato esperado.
    pub fn take_heat_loss(&self) -> Option<Result<(usize, f64), String>> {
        let content = Self::take(&self.path("heat_loss_export.txt"))?;
        let parts: Vec<&str> = content.split_whitespace().collect();
        Some(match parts.as_slice() {
            [area, q] => match (area.parse::<usize>(), q.parse::<f64>()) {
                (Ok(area), Ok(q)) if q.is_finite() => Ok((area, q)),
                _ => Err(format!("Envío de Heat Loss no válido: «{}»", content.trim())),
            },
            _ => Err(format!("Envío de Heat Loss no válido: «{}»", content.trim())),
        })
    }

    /// Balance: publica el número de áreas del libro abierto.
    pub fn publish_presence(&self, area_count: usize) -> io::Result<()> {
        crate::storage::write_atomic(self.path("balance_session.txt"), area_count.to_string())
    }

    /// Balance: retira la publicación al cerrar la ventana.
    pub fn clear_presence(&self) {
        let _ = std::fs::remove_file(self.path("balance_session.txt"));
    }

    /// Heat Loss: número de áreas del Balance abierto, o `None` si no hay ninguno.
    pub fn balance_area_count(&self) -> Option<usize> {
        std::fs::read_to_string(self.path("balance_session.txt"))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// Balance: confirma a Heat Loss el resultado de la importación.
    pub fn write_ack(&self, ok: bool, message: &str) -> io::Result<()> {
        crate::storage::write_atomic(
            self.path("heat_loss_ack.txt"),
            format!("{}\n{message}", if ok { "ok" } else { "error" }),
        )
    }

    /// Heat Loss: recoge el acuse de Balance, si lo hay.
    pub fn take_ack(&self) -> Option<(bool, String)> {
        let content = Self::take(&self.path("heat_loss_ack.txt"))?;
        let (flag, message) = content.split_once('\n').unwrap_or((content.as_str(), ""));
        Some((flag == "ok", message.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_malformed_formula_is_ignored() {
        assert_eq!(decode_formula(&encode_formula(" Fe2O3 ", 100), 105, 15).as_deref(), Some("Fe2O3"));
        assert_eq!(decode_formula(&encode_formula("Fe2O3", 100), 200, 15), None, "envío caducado");
        assert_eq!(decode_formula("Fe2O3", 100, 15), None, "formato antiguo sin marca de tiempo");
        assert_eq!(decode_formula(&encode_formula("", 100), 100, 15), None);
        let dir = std::env::temp_dir().join(format!("soryntel-formula-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = ModuleLink::new(&dir, "prueba");
        link.send_formula("CuSO4").unwrap();
        assert_eq!(link.take_formula().as_deref(), Some("CuSO4"));
        assert_eq!(link.take_formula(), None, "se consume una vez");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn heat_loss_round_trip_with_presence_and_ack() {
        let dir = std::env::temp_dir().join(format!("soryntel-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let link = ModuleLink::new(&dir, "prueba");
        assert_eq!(link.balance_area_count(), None);
        link.publish_presence(3).unwrap();
        assert_eq!(link.balance_area_count(), Some(3));
        assert!(link.take_heat_loss().is_none());
        link.send_heat_loss(2, 1500.5).unwrap();
        assert_eq!(link.take_heat_loss(), Some(Ok((2, 1500.5))));
        assert!(link.take_heat_loss().is_none(), "el envío se consume una vez");
        std::fs::write(link.path("heat_loss_export.txt"), "x y").unwrap();
        assert!(matches!(link.take_heat_loss(), Some(Err(_))));
        link.write_ack(false, "no existe el Area 4").unwrap();
        assert_eq!(link.take_ack(), Some((false, "no existe el Area 4".to_string())));
        link.clear_presence();
        assert_eq!(link.balance_area_count(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
