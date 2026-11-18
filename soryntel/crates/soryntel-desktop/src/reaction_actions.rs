//! Adaptadores de archivos e imagen del módulo de reacciones.
use std::path::PathBuf;

pub fn choose_file(
    save: bool,
    title: &str,
    extension: &str,
    filename: &str,
    current_file: Option<&std::path::Path>,
) -> Option<PathBuf> {
    crate::storage::choose_file(save, title, extension, filename, current_file)
}

pub fn render_svg(svg: &str) -> Result<resvg::tiny_skia::Pixmap, String> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixels = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| "No se pudo reservar memoria para la imagen".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    Ok(pixels)
}

pub fn rgba(pixels: &resvg::tiny_skia::Pixmap) -> Vec<u8> {
    pixels
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect()
}

pub fn csv_from_tsv(tsv: &str) -> String {
    tsv.lines()
        .map(|line| {
            line.split('\t')
                .map(|cell| format!("\"{}\"", cell.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_quotes_commas_quotes_and_empty_cells() {
        assert_eq!(
            csv_from_tsv("valor\treferencia\n1\tA, B \"C\"\n"),
            "\"valor\",\"referencia\"\r\n\"1\",\"A, B \"\"C\"\"\"\r\n"
        );
    }
    #[test]
    fn svg_produces_real_png_and_unpremultiplied_rgba() {
        let image = render_svg(r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="#ff0000"/></svg>"##).unwrap();
        assert_eq!((image.width(), image.height()), (640, 400));
        assert_eq!(&rgba(&image)[..4], &[255, 0, 0, 255]);
        assert!(image
            .encode_png()
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
