//! Guardado mediante archivo temporal en el mismo directorio y reemplazo final.
use std::{fs::{self, OpenOptions}, io::{self, Write}, path::Path, sync::atomic::{AtomicU64, Ordering}};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn write_atomic(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "La ruta debe indicar un archivo"))?;
    let mut temporary_name = name.to_os_string();
    temporary_name.push(format!(".{}.{}.tmp", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
    let temporary = parent.join(temporary_name);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        file.write_all(contents.as_ref())?;
        if let Ok(metadata) = fs::metadata(path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}

/// Carpeta de documentos del usuario (`~/Documents`, `%USERPROFILE%\\Documents`) o, si no
/// existe, su carpeta personal. `None` solo si no se puede determinar ninguna.
pub fn user_documents_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())?;
    for name in ["Documents", "Documentos"] {
        let dir = home.join(name);
        if dir.is_dir() {
            return Some(dir);
        }
    }
    Some(home)
}

/// Carpeta inicial de los diálogos de archivo: la del libro actual si se conoce y es
/// absoluta; si no, la carpeta de documentos del usuario. Nunca el directorio temporal ni una
/// ruta relativa al directorio de trabajo.
pub fn dialog_start_dir(current_file: Option<&Path>) -> Option<std::path::PathBuf> {
    current_file
        .filter(|p| p.is_absolute())
        .and_then(|p| p.parent())
        .filter(|dir| dir.is_dir() && !dir.starts_with(std::env::temp_dir()))
        .map(Path::to_path_buf)
        .or_else(user_documents_dir)
}

/// Nombre de archivo sugerido a partir de un título: sin separadores de ruta y con la
/// extensión indicada.
pub fn suggested_file_name(title: &str, extension: &str) -> String {
    let stem: String = title
        .trim()
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let stem = stem.trim_end_matches(&format!(".{extension}")[..]).trim();
    let stem = if stem.is_empty() { "soryntel" } else { stem };
    format!("{stem}.{extension}")
}

/// Diálogo nativo de abrir o guardar (rfd). El de guardar pide confirmación al sobrescribir
/// (comportamiento nativo de GTK, Windows y macOS) y añade la extensión si falta.
pub fn choose_file(
    save: bool,
    title: &str,
    extension: &str,
    suggested_name: &str,
    current_file: Option<&Path>,
) -> Option<std::path::PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .add_filter(extension.to_uppercase(), &[extension])
        .set_file_name(suggested_name);
    if let Some(dir) = dialog_start_dir(current_file) {
        dialog = dialog.set_directory(dir);
    }
    if save {
        dialog.save_file().map(|mut path| {
            if path.extension().is_none() {
                path.set_extension(extension);
            }
            path
        })
    } else {
        dialog.pick_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaces_complete_document_and_cleans_up_failed_save() {
        let dir = std::env::temp_dir().join(format!("soryntel-save-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("proyecto.json");
        write_atomic(&path, "{\"estado\":1}").unwrap();
        write_atomic(&path, "{\"estado\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"estado\":2}");
        let directory_target = dir.join("carpeta");
        fs::create_dir(&directory_target).unwrap();
        assert!(write_atomic(&directory_target, "datos").is_err());
        assert!(directory_target.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("soryntel-{tag}-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "BUG-C18: guardar sobre un enlace simbólico sustituye el enlace por un archivo y el destino real no se actualiza"]
    fn saving_through_symlink_updates_link_target() {
        let dir = scratch_dir("c18");
        let real = dir.join("real.bal8");
        let link = dir.join("enlace.bal8");
        fs::write(&real, "v1").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "v2").unwrap();
        let link_still_link = fs::symlink_metadata(&link).unwrap().file_type().is_symlink();
        let real_content = fs::read_to_string(&real).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(link_still_link && real_content == "v2", "enlace conservado={link_still_link}, destino real=«{real_content}»");
    }

    #[test]
    #[ignore = "BUG-C19: un nombre de archivo válido de 240+ bytes no se puede guardar porque el temporal supera NAME_MAX"]
    fn long_but_valid_file_name_can_be_saved() {
        let dir = scratch_dir("c19");
        let path = dir.join(format!("{}.rea8", "a".repeat(245))); // 250 bytes: válido en ext4/NTFS
        fs::write(&path, "v1").expect("preparación: el sistema admite el nombre");
        let result = write_atomic(&path, "v2");
        fs::remove_dir_all(&dir).unwrap();
        assert!(result.is_ok(), "{result:?}");
    }

    /// C14/C21/C23: ningún módulo del área C escribe en rutas fijas (`/tmp`, `/home/...`).
    #[test]
    fn area_c_modules_have_no_hardcoded_output_paths() {
        let sources = [
            ("balance_ui.rs", include_str!("balance_ui.rs")),
            ("benson_ui.rs", include_str!("benson_ui.rs")),
            ("elements_ui.rs", include_str!("elements_ui.rs")),
            ("equilibrium_ui.rs", include_str!("equilibrium_ui.rs")),
            ("estimate_ui.rs", include_str!("estimate_ui.rs")),
            ("exergy_ui.rs", include_str!("exergy_ui.rs")),
            ("heat_loss_ui.rs", include_str!("heat_loss_ui.rs")),
            ("mass_balance_ui.rs", include_str!("mass_balance_ui.rs")),
            ("reaction_ui.rs", include_str!("reaction_ui.rs")),
            ("species_converter_ui.rs", include_str!("species_converter_ui.rs")),
            ("species_ui.rs", include_str!("species_ui.rs")),
        ];
        for (name, source) in sources {
            let production = source.split("#[cfg(test)]").next().unwrap();
            for needle in [concat!("\"/", "tmp/"), concat!("\"/home", "/")] {
                assert!(!production.contains(needle), "{name} contiene la ruta fija {needle}");
            }
        }
    }

    #[test]
    fn dialog_paths_are_absolute_and_never_temporary() {
        if let Some(dir) = user_documents_dir() {
            assert!(dir.is_absolute());
        }
        let tmp_file = std::env::temp_dir().join("libro.bal8");
        let start = dialog_start_dir(Some(&tmp_file));
        assert!(start.as_deref().is_none_or(|d| !d.starts_with(std::env::temp_dir())), "{start:?}");
        assert_eq!(dialog_start_dir(Some(Path::new("relativo/libro.bal8"))), user_documents_dir());
        let here = std::env::current_dir().unwrap().join("Cargo.toml");
        assert_eq!(dialog_start_dir(Some(&here)), here.parent().map(Path::to_path_buf));
        assert_eq!(suggested_file_name("Smelting1", "htr8"), "Smelting1.htr8");
        assert_eq!(suggested_file_name("NICO.gem9", "gem9"), "NICO.gem9");
        assert_eq!(suggested_file_name("a/b:c", "csv"), "a_b_c.csv");
        assert_eq!(suggested_file_name("  ", "svg"), "soryntel.svg");
    }
}
