//! Fuzz barato (sin nightly ni dependencias nuevas) de los códecs `.HSC` / `.REF` / CSV.
//!
//! Parte de un `.HSC` válido generado con `serialize_datasets_to_hsc` a partir de los datos
//! incluidos, lo muta con un PRNG propio de semilla fija y llama a los parsers dentro de
//! `catch_unwind`. Cualquier pánico se agrupa por mensaje y se informa con el input más corto.
//!
//! `SORYNTEL_FUZZ_ITERS` permite subir el número de iteraciones en local.

use soryntel_core::{DbSource, SpeciesDataset};
use soryntel_db::hsc_binary::{
    decode_cp1252, group_records_to_datasets, import_datasets_csv, export_datasets_csv,
    parse_hsc_binary_records, parse_ref_binary, serialize_datasets_to_hsc, infer_charge,
    upgrade_legacy_datasets,
};
use soryntel_db::ThermoDatabase;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

/// xorshift64* con semilla fija: reproducible y sin dependencias.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}

fn iterations(default: usize) -> usize {
    std::env::var("SORYNTEL_FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn seed_datasets() -> Vec<SpeciesDataset> {
    let db = ThermoDatabase::with_open_defaults();
    let json = db.export_json(None).expect("export json");
    serde_json::from_str(&json).expect("parse json")
}

const SPECIAL_F32: [f32; 10] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    -0.0,
    -1.0,
    f32::MAX,
    f32::MIN_POSITIVE,
    1e-45,
    298.15,
    0.0,
];
const F32_OFFSETS: [usize; 15] = [
    0x98, 0x9C, 0xA0, 0xA4, 0xA8, 0xAE, 0xB2, 0xB6, 0xBA, 0xBE, 0xC2, 0xC6, 0xCA, 0xFE, 0xFC,
];
const INTERESTING: [u8; 9] = [0x00, 0xFF, 0x80, 0x9F, 0xA0, 0x7F, 0x20, b'(', b')'];

fn mutate_binary(rng: &mut Rng, base: &[u8]) -> Vec<u8> {
    let mut buf = base.to_vec();
    let n_ops = 1 + rng.below(8);
    for _ in 0..n_ops {
        match rng.below(9) {
            0 if !buf.is_empty() => {
                let i = rng.below(buf.len());
                buf[i] ^= 1 << rng.below(8);
            }
            1 if !buf.is_empty() => {
                let i = rng.below(buf.len());
                buf[i] = rng.byte();
            }
            2 if !buf.is_empty() => {
                let i = rng.below(buf.len());
                buf[i] = INTERESTING[rng.below(INTERESTING.len())];
            }
            3 => {
                let len = rng.below(buf.len() + 1);
                buf.truncate(len);
            }
            4 => {
                let extra = rng.below(300);
                for _ in 0..extra {
                    buf.push(rng.byte());
                }
            }
            5 if buf.len() >= 256 => {
                // Duplica un registro completo en otra posición (mismo formula, otro bloque).
                let n = buf.len() / 256;
                let src = rng.below(n) * 256;
                let rec: Vec<u8> = buf[src..src + 256].to_vec();
                let dst = rng.below(n + 1) * 256;
                let dst = dst.min(buf.len());
                buf.splice(dst..dst, rec);
            }
            6 if buf.len() >= 256 => {
                let n = buf.len() / 256;
                let rec = rng.below(n) * 256;
                let off = F32_OFFSETS[rng.below(F32_OFFSETS.len())];
                if rec + off + 4 <= buf.len() {
                    let v = SPECIAL_F32[rng.below(SPECIAL_F32.len())];
                    buf[rec + off..rec + off + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
            7 if buf.len() >= 256 => {
                // Rellena un campo de texto completo con bytes no ASCII / sin terminador.
                let n = buf.len() / 256;
                let rec = rng.below(n) * 256;
                let (a, b) = [(0x00, 0x18), (0x38, 0x6C), (0xAC, 0xAE), (0xCC, 0xDA), (0xDA, 0xFE)]
                    [rng.below(5)];
                let fill = rng.byte();
                for x in &mut buf[rec + a..rec + b] {
                    *x = fill;
                }
            }
            _ => {
                let len = rng.below(600);
                buf = (0..len).map(|_| rng.byte()).collect();
            }
        }
    }
    buf
}

const CSV_INSERTS: [&str; 20] = [
    ",", "\"", "\n", "\r\n", ";", "é", "\u{feff}", "0", "-", "NaN", "inf", "1e309", "(", ")",
    "+", " ", "\"\"", "ÿ", "·", "\u{0}",
];

fn mutate_text(rng: &mut Rng, base: &str) -> String {
    let mut chars: Vec<char> = base.chars().collect();
    let n_ops = 1 + rng.below(6);
    for _ in 0..n_ops {
        match rng.below(4) {
            0 => {
                let at = rng.below(chars.len() + 1);
                let ins = CSV_INSERTS[rng.below(CSV_INSERTS.len())];
                chars.splice(at..at, ins.chars());
            }
            1 if !chars.is_empty() => {
                let at = rng.below(chars.len());
                chars.remove(at);
            }
            2 => {
                let len = rng.below(chars.len() + 1);
                chars.truncate(len);
            }
            _ if !chars.is_empty() => {
                let at = rng.below(chars.len());
                chars[at] = char::from_u32(rng.below(0x3000) as u32).unwrap_or('?');
            }
            _ => {}
        }
    }
    chars.into_iter().collect()
}

#[derive(Default)]
struct Panics {
    /// mensaje -> (input más corto en hex/texto, longitud, ocurrencias)
    by_msg: BTreeMap<String, (String, usize, usize)>,
}

impl Panics {
    fn record(&mut self, stage: &str, msg: String, input_repr: String, len: usize) {
        let key = format!("[{stage}] {msg}");
        let e = self.by_msg.entry(key).or_insert((input_repr.clone(), len, 0));
        e.2 += 1;
        if len < e.1 {
            e.0 = input_repr;
            e.1 = len;
        }
    }
    fn report(&self) -> String {
        let mut s = String::new();
        for (msg, (input, len, count)) in &self.by_msg {
            s.push_str(&format!("{msg}  x{count}  (input mínimo {len} B): {input}\n"));
        }
        s
    }
}

fn panic_msg(e: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "<pánico sin mensaje>".into()
    }
}

fn hex(b: &[u8]) -> String {
    if b.len() > 64 {
        format!("{}… ({} B)", b[..64].iter().map(|x| format!("{x:02x}")).collect::<String>(), b.len())
    } else {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}

/// Recorre la cadena completa de lectura de un buffer binario: parse, agrupar, reserializar,
/// re-parsear, normalizar legado, CAT/CSV y el texto decodificado.
fn exercise_binary(buf: &[u8]) {
    let _ = parse_ref_binary(buf);
    let _ = decode_cp1252(&buf[..buf.len().min(64)]);
    if let Ok((recs, _)) = parse_hsc_binary_records(buf) {
        for r in &recs {
            let _ = infer_charge(&r.formula);
        }
        let (order, map) = group_records_to_datasets(recs, DbSource::Own);
        let flat: Vec<SpeciesDataset> =
            order.iter().filter_map(|f| map.get(f).cloned()).flatten().collect();
        let bytes = serialize_datasets_to_hsc(&flat);
        let _ = parse_hsc_binary_records(&bytes).expect("reserializado debe parsear");
        let csv = export_datasets_csv(&flat);
        let _ = import_datasets_csv(&csv, DbSource::Own);
        let _ = upgrade_legacy_datasets(flat);
    }
}

#[test]
fn fuzz_parsers_binarios_hsc_y_ref_no_entran_en_panico() {
    let base = serialize_datasets_to_hsc(&seed_datasets());
    assert!(base.len() >= 256 * 10);
    let mut rng = Rng(0x5EED_A001_C0FF_EE11);
    let mut panics = Panics::default();
    let iters = iterations(5_000);
    for _ in 0..iters {
        let input = mutate_binary(&mut rng, &base);
        let r = catch_unwind(AssertUnwindSafe(|| exercise_binary(&input)));
        if let Err(e) = r {
            panics.record("binario", panic_msg(e), hex(&input), input.len());
        }
    }
    assert!(panics.by_msg.is_empty(), "pánicos encontrados:\n{}", panics.report());
}

/// Ruta completa a través de `ThermoDatabase`: carga desde archivo (reconstruye la vista resuelta
/// y la caché de composición), búsquedas, guardado y fusión.
#[test]
fn fuzz_carga_de_owndb_y_busquedas_no_entran_en_panico() {
    let base = serialize_datasets_to_hsc(&seed_datasets());
    let dir: PathBuf =
        std::env::temp_dir().join(format!("soryntel-fuzz-own-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut rng = Rng(0xA11C_E5EE_D000_0002);
    let mut panics = Panics::default();
    let iters = iterations(5_000) / 10;
    let filters = soryntel_core::PhaseFilterFlags::default();
    for i in 0..iters {
        let input = mutate_binary(&mut rng, &base);
        let src = dir.join("fuzz.HSC");
        std::fs::write(&src, &input).unwrap();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut db = ThermoDatabase::with_open_defaults();
            if db.load_own_hsc(&src).is_ok() {
                let _ = db.search_advanced(
                    "a",
                    soryntel_core::KeywordSearchField::Any,
                    None,
                    &filters,
                    usize::MAX,
                );
                let _ = db.search_elements_filtered(
                    &["O".to_string(), "H".to_string()],
                    soryntel_core::ElementSearchMode::Some,
                    Some(DbSource::Own),
                    &filters,
                    usize::MAX,
                );
                let _ = db.search_stoichiometry("Al2O3", i % 2 == 0, None, &filters, usize::MAX);
                let _ = db.export_csv(None);
                let _ = db.save_own_hsc(dir.join("out.HSC"));
            }
            let _ = db.load_main_hsc(&src);
            let _ = ThermoDatabase::merge_hsc_files(&src, dir.join("merged.HSC"), &BTreeMap::new(), false);
            let _ = ThermoDatabase::convert_legacy_to_hsc6(&src, dir.join("conv.HSC"));
        }));
        if let Err(e) = r {
            panics.record("ThermoDatabase", panic_msg(e), hex(&input), input.len());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(panics.by_msg.is_empty(), "pánicos encontrados:\n{}", panics.report());
}

#[test]
fn fuzz_import_csv_no_entra_en_panico() {
    let base = export_datasets_csv(&seed_datasets());
    let mut rng = Rng(0xC5F0_0000_0000_0003);
    let mut panics = Panics::default();
    let iters = iterations(5_000);
    for _ in 0..iters {
        let input = mutate_text(&mut rng, &base);
        let r = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(ds) = import_datasets_csv(&input, DbSource::Own) {
                let _ = serialize_datasets_to_hsc(&ds);
                let _ = upgrade_legacy_datasets(ds);
            }
            let mut db = ThermoDatabase::default();
            let _ = db.import_csv_into_own(&input);
        }));
        if let Err(e) = r {
            panics.record("csv", panic_msg(e), format!("{input:?}"), input.len());
        }
    }
    assert!(panics.by_msg.is_empty(), "pánicos encontrados:\n{}", panics.report());
}
