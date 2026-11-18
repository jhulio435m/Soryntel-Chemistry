//! Enlace entre las ventanas Heat Loss y Balance, que se ejecutan en procesos distintos.
//!
//! Balance publica cuántas áreas tiene el libro abierto; Heat Loss envía la pérdida de calor a
//! un área y Balance responde con un acuse que Heat Loss muestra al usuario. Todo va por
//! archivos pequeños en el directorio temporal del usuario, escritos de forma atómica.
use std::io;
use std::path::{Path, PathBuf};

pub struct BalanceLink {
    dir: PathBuf,
    prefix: String,
}

impl BalanceLink {
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
    fn heat_loss_round_trip_with_presence_and_ack() {
        let dir = std::env::temp_dir().join(format!("soryntel-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let link = BalanceLink::new(&dir, "prueba");
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
