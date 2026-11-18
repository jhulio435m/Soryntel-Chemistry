//! `sdb-convert`: genera SoryntelDB (`.sdb`) a partir de la MainDB de HSC Chemistry 6.
//!
//! ```text
//! sdb-convert --main Maindb6.HSC --ref MainDB6.REF --cat Maindb6.cat \
//!             --notice "<texto de atribución>" --out soryntel.sdb [--created AAAA-MM-DDThh:mm:ssZ]
//! ```
//!
//! `created` toma, por orden, `--created`, `SOURCE_DATE_EPOCH` o la hora actual. Con las
//! mismas entradas y el mismo `created` la salida es idéntica byte a byte.

use soryntel_db::sdb::{self, HscSources};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "uso: sdb-convert --main <Maindb6.HSC> --ref <MainDB6.REF> --cat <Maindb6.cat> \
--notice <texto> --out <archivo.sdb> [--created <AAAA-MM-DDThh:mm:ssZ>]";

#[derive(Debug, Default)]
struct Args {
    main: Option<PathBuf>,
    reference: Option<PathBuf>,
    catalogue: Option<PathBuf>,
    notice: Option<String>,
    out: Option<PathBuf>,
    created: Option<String>,
}

fn parse_args(raw: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut iter = raw.into_iter();
    while let Some(flag) = iter.next() {
        if flag == "-h" || flag == "--help" {
            return Err(USAGE.into());
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("falta el valor de {flag}\n{USAGE}"))?;
        let slot_taken = match flag.as_str() {
            "--main" => args.main.replace(value.into()).is_some(),
            "--ref" => args.reference.replace(value.into()).is_some(),
            "--cat" => args.catalogue.replace(value.into()).is_some(),
            "--notice" => args.notice.replace(value).is_some(),
            "--out" => args.out.replace(value.into()).is_some(),
            "--created" => args.created.replace(value).is_some(),
            other => return Err(format!("argumento desconocido: {other}\n{USAGE}")),
        };
        if slot_taken {
            return Err(format!("{flag} aparece más de una vez"));
        }
    }
    Ok(args)
}

fn created_timestamp(explicit: Option<String>) -> Result<String, String> {
    if let Some(created) = explicit {
        return Ok(created);
    }
    let secs = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) => value
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("SOURCE_DATE_EPOCH no es un entero: {value}"))?,
        Err(_) => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    };
    Ok(sdb::utc_timestamp(secs))
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args().skip(1))?;
    let missing = |name: &str| format!("falta {name}\n{USAGE}");
    let main = args.main.ok_or_else(|| missing("--main"))?;
    let reference = args.reference.ok_or_else(|| missing("--ref"))?;
    let catalogue = args.catalogue.ok_or_else(|| missing("--cat"))?;
    let notice = args.notice.ok_or_else(|| missing("--notice"))?;
    let out = args.out.ok_or_else(|| missing("--out"))?;
    if notice.trim().is_empty() {
        return Err("--notice no puede estar vacío".into());
    }
    let created = created_timestamp(args.created)?;
    let sources = HscSources {
        main: &main,
        references: Some(&reference),
        catalogue: Some(&catalogue),
    };
    let file = sdb::convert_hsc6(&sources, &notice, &created).map_err(|e| e.to_string())?;
    sdb::write_sdb(&out, &file).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "{}: {} especies, {} juegos de datos, {} intervalos, {} referencias, {} bytes, sha256 {}",
        out.display(),
        file.species_count,
        file.dataset_count,
        file.interval_count,
        file.reference_count,
        bytes.len(),
        sdb::sha256_hex(&bytes)
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sdb-convert: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_all_flags_and_rejects_unknown_or_repeated_ones() {
        let args = parse_args(strings(&[
            "--main", "M.HSC", "--ref", "M.REF", "--cat", "M.cat", "--notice", "aviso", "--out",
            "o.sdb",
        ]))
        .unwrap();
        assert_eq!(args.main, Some(PathBuf::from("M.HSC")));
        assert_eq!(args.notice.as_deref(), Some("aviso"));
        assert!(parse_args(strings(&["--bogus", "x"])).is_err());
        assert!(parse_args(strings(&["--main"])).is_err());
        assert!(parse_args(strings(&["--out", "a", "--out", "b"])).is_err());
    }

    #[test]
    fn explicit_created_is_kept_verbatim() {
        assert_eq!(
            created_timestamp(Some("2026-01-01T00:00:00Z".into())).unwrap(),
            "2026-01-01T00:00:00Z"
        );
    }
}
