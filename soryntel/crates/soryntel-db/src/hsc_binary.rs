//! Binary `.HSC` / `.REF` / `.CAT` codecs and legacy/CSV/JSON interchange helpers (HSC 2.0–6.0).

use soryntel_core::{SoryntelError, CAL_TO_JOULE};
use soryntel_thermo::species::{
    compute_molecular_weight, formula_sort_key, SpeciesDataset,
};
use soryntel_thermo::{
    ChemicalSpecies, KelleyCoefficients, Phase, SpeciesMetadata, SpeciesThermoInterval,
};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub use soryntel_core::catalog::ReferenceEntry;

/// Decoded raw 256-byte `.HSC` binary record before interval grouping.
#[derive(Debug, Clone)]
pub struct RawHscRecord {
    pub formula: String,
    pub structural_formula: String,
    pub name: String,
    pub common_name: String,
    pub cas_number: String,
    pub mw: f64,
    pub mp_k: f64,
    pub bp_k: f64,
    pub t1: f64,
    pub t2: f64,
    pub raw_phase: String,
    pub phase: Phase,
    pub h_j: f64,
    pub s_j: f64,
    pub kelley_j: KelleyCoefficients,
    pub density: f64,
    pub color_code: i16,
    pub solubility: String,
    pub reference: String,
    pub reliability_class: i16,
}

/// Windows-1252 bytes `0x80..=0x9F` mapped to exact Unicode code points.
const CP1252_80_9F: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Decode a Windows-1252 (CP1252 / Latin-1) fixed-width field trimmed at NUL and spaces (all-0xFF fields are blank).
pub fn decode_cp1252(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    if bytes[..end].iter().all(|&byte| byte == 0xFF) {
        return String::new();
    }
    let mut out = String::with_capacity(end);
    for &b in &bytes[..end] {
        let ch = if (0x80..=0x9F).contains(&b) {
            CP1252_80_9F[(b - 0x80) as usize]
        } else {
            b as char
        };
        out.push(ch);
    }
    out.trim().to_string()
}

/// Encode a UTF-8 string into fixed-width CP1252 space-padded bytes (`0x20`).
pub fn encode_cp1252_fixed(text: &str, width: usize) -> Vec<u8> {
    let mut buf = vec![b' '; width];
    let mut idx = 0usize;
    for ch in text.chars() {
        if idx >= width {
            break;
        }
        let b = if (ch as u32) < 0x80 {
            ch as u8
        } else if (0xA0..=0xFF).contains(&(ch as u32)) {
            ch as u8
        } else if let Some(pos) = CP1252_80_9F.iter().position(|&c| c == ch) {
            0x80 + pos as u8
        } else {
            b'?'
        };
        buf[idx] = b;
        idx += 1;
    }
    buf
}

fn decode_solubility(bytes: &[u8]) -> String {
    let text = decode_cp1252(bytes);
    if text.is_empty() || text.chars().any(char::is_control) {
        "-".into()
    } else {
        text
    }
}

#[inline]
fn read_f32(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

#[inline]
fn read_i16(buf: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([buf[offset], buf[offset + 1]])
}

#[inline]
fn write_f32(buf: &mut [u8], offset: usize, val: f64) {
    let v = if val.is_finite() { val as f32 } else { 0.0_f32 };
    buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline]
fn write_i16(buf: &mut [u8], offset: usize, val: i16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

/// Infer ionic formal charge from HSC formula suffixes like `(+a)`, `(-2a)`, `(+3g)`, `e-`.
pub fn infer_charge(formula: &str) -> i32 {
    if formula == "e-" || formula == "e(-)" {
        return -1;
    }
    if let Some(open) = formula.rfind('(') {
        if formula.ends_with(')') {
            let suf = &formula[open + 1..formula.len() - 1];
            let trimmed = suf.trim_end_matches(|c| c == 'a' || c == 'g');
            if trimmed == "+" {
                return 1;
            } else if trimmed == "-" {
                return -1;
            } else if trimmed.starts_with('+') || trimmed.starts_with('-') {
                if let Ok(v) = trimmed.parse::<i32>() {
                    return v;
                }
            }
        }
    }
    if let Some(index) = formula.rfind(['+', '-']) {
        let (base, suffix) = formula.split_at(index);
        if !base.is_empty() && !base.ends_with(['(', '[', '*']) {
            if suffix == "+" || suffix == "-" {
                // A bare sign after a polyatomic formula leaves its subscripts intact (NH4+).
                let element = base.trim_end_matches(|ch: char| ch.is_ascii_digit());
                let magnitude = if !element.is_empty()
                    && element.as_bytes()[0].is_ascii_uppercase()
                    && element.as_bytes()[1..].iter().all(u8::is_ascii_lowercase)
                    && element.len() <= 2
                    && element.len() < base.len()
                {
                    base[element.len()..].parse::<i32>().unwrap_or(0)
                } else {
                    1
                };
                return if suffix == "-" { -magnitude } else { magnitude };
            }
            if let Ok(charge) = suffix.parse::<i32>() {
                return charge;
            }
        }
    }
    0
}

/// Map HSC two-byte phase code (`s`, `l`, `g`, `ao`, `ai`, `a`) to domain `Phase`.
pub fn map_phase(p: &str, formula: &str) -> Phase {
    match p.trim() {
        "g" => Phase::Gas,
        "l" => Phase::Liquid,
        "ao" | "a" => {
            if infer_charge(formula) != 0 {
                Phase::AqueousIon
            } else {
                Phase::AqueousNeutral
            }
        }
        "ai" => Phase::AqueousNeutral,
        _ => Phase::Solid,
    }
}

/// Canonical two-byte `.HSC` phase code for serialization.
pub fn phase_to_hsc_code(phase: Phase, raw_hint: &str) -> &'static str {
    let hint = raw_hint.trim();
    if matches!(phase, Phase::AqueousNeutral) && (hint == "ai" || hint == "ao") {
        return if hint == "ai" { "ai" } else { "ao" };
    }
    match phase {
        Phase::Solid => "s",
        Phase::Liquid => "l",
        Phase::Gas => "g",
        Phase::AqueousIon | Phase::AqueousNeutral => "ao",
    }
}

/// Decode all valid 256-byte `.HSC` records from raw file bytes.
/// Returns `(valid_records, total_256_slots)`.
pub fn parse_hsc_binary_records(buf: &[u8]) -> Result<(Vec<RawHscRecord>, usize), SoryntelError> {
    let n_recs = buf.len() / 256;
    let offset = n_recs * 256;
    if buf.len() != offset {
        return Err(SoryntelError::DatabaseError(format!(
            "Incomplete HSC record at offset {offset}: {} bytes available, expected 256 (file size {})",
            buf.len() - offset, buf.len()
        )));
    }
    if n_recs == 0 {
        return Err(SoryntelError::DatabaseError(
            "File is smaller than a single 256-byte HSC record".into(),
        ));
    }
    let mut records = Vec::with_capacity(n_recs);
    for i in 0..n_recs {
        let rec = &buf[i * 256..(i + 1) * 256];
        let formula = decode_cp1252(&rec[0x00..0x18]);
        let t1 = read_f32(rec, 0xA4) as f64;
        let t2 = read_f32(rec, 0xA8) as f64;
        // HSC OWNDB6.HSC uses sentinel `'0'` records when empty
        if formula.is_empty() || formula == "0" || !t1.is_finite() || !t2.is_finite() || t2 <= t1 || t1 < 0.0 {
            continue;
        }
        let structural_formula = decode_cp1252(&rec[0x18..0x38]);
        let name = decode_cp1252(&rec[0x38..0x6C]);
        let common_name = decode_cp1252(&rec[0x6C..0x8C]);
        let cas_number = decode_cp1252(&rec[0x8C..0x98]);
        let mw = read_f32(rec, 0x98) as f64;
        let mp_k = read_f32(rec, 0x9C) as f64;
        let bp_k = read_f32(rec, 0xA0) as f64;
        let raw_phase = decode_cp1252(&rec[0xAC..0xAE]);
        let h_kcal = read_f32(rec, 0xAE) as f64;
        let s_cal = read_f32(rec, 0xB2) as f64;
        let a_cal = read_f32(rec, 0xB6) as f64;
        let b_cal = read_f32(rec, 0xBA) as f64;
        let c_cal = read_f32(rec, 0xBE) as f64;
        let d_cal = read_f32(rec, 0xC2) as f64;
        let density = read_f32(rec, 0xC6) as f64;
        let color_code = read_i16(rec, 0xCA);
        let solubility = decode_solubility(&rec[0xCC..0xDA]);
        let reference = decode_cp1252(&rec[0xDA..0xFE]);
        let reliability_class = read_i16(rec, 0xFE);

        records.push(RawHscRecord {
            phase: map_phase(&raw_phase, &formula),
            formula,
            structural_formula,
            name,
            common_name,
            cas_number,
            mw,
            mp_k,
            bp_k,
            t1,
            t2,
            raw_phase,
            h_j: h_kcal * 1000.0 * CAL_TO_JOULE,
            s_j: s_cal * CAL_TO_JOULE,
            kelley_j: KelleyCoefficients {
                a: a_cal * CAL_TO_JOULE,
                b: b_cal * CAL_TO_JOULE,
                c: c_cal * CAL_TO_JOULE,
                d: d_cal * CAL_TO_JOULE,
            },
            density,
            color_code,
            solubility,
            reference,
            reliability_class,
        });
    }
    Ok((records, n_recs))
}

/// Group decoded flat records into continuous interval `SpeciesDataset`s, preserving non-contiguous
/// alternative literature datasets (e.g. `Mg(+2a)` set 0 and set 1) instead of discarding them.
pub fn group_records_to_datasets(
    records: Vec<RawHscRecord>,
    source: soryntel_thermo::species::DbSource,
) -> (Vec<String>, HashMap<String, Vec<SpeciesDataset>>) {
    let mut by_formula: HashMap<String, Vec<RawHscRecord>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in records {
        by_formula
            .entry(r.formula.clone())
            .or_insert_with(|| {
                order.push(r.formula.clone());
                Vec::new()
            })
            .push(r);
    }

    let mut datasets_map: HashMap<String, Vec<SpeciesDataset>> = HashMap::new();
    for formula in &order {
        let recs = &by_formula[formula];
        if recs.is_empty() {
            continue;
        }
        // Split on temperature discontinuities `(r.t1 - prev_t2).abs() > 1.0`
        let mut chunks: Vec<&[RawHscRecord]> = Vec::new();
        let mut start = 0usize;
        for idx in 1..recs.len() {
            if (recs[idx].t1 - recs[idx - 1].t2).abs() > 1.0 {
                chunks.push(&recs[start..idx]);
                start = idx;
            }
        }
        chunks.push(&recs[start..]);

        let ds_list = chunks.into_iter().enumerate()
            .map(|(index, chunk)| build_dataset(chunk, source, index))
            .collect();
        datasets_map.insert(formula.clone(), ds_list);
    }

    (order, datasets_map)
}

fn build_dataset(
    chunk: &[RawHscRecord],
    source: soryntel_thermo::species::DbSource,
    dataset_index: usize,
) -> SpeciesDataset {
    let first = &chunk[0];
    let meta = merge_chunk_metadata(chunk);
    let intervals: Vec<SpeciesThermoInterval> = chunk
        .iter()
        .map(|r| SpeciesThermoInterval {
            t_min: r.t1,
            t_max: r.t2,
            phase: r.phase,
            kelley: r.kelley_j,
            h_or_delta_h_tr: r.h_j,
            s_or_delta_s_tr: r.s_j,
            reference: r.reference.clone(),
        })
        .collect();
    let sp = ChemicalSpecies {
        formula: first.formula.clone(),
        name: first.name.clone(),
        phase: first.phase,
        molecular_weight: first.mw,
        density_g_cm3: first.density,
        charge: infer_charge(&first.formula),
        intervals,
    };
    SpeciesDataset {
        source,
        species: sp,
        metadata: meta,
        dataset_index,
        raw_phase_code: first.raw_phase.clone(),
    }
}

fn merge_chunk_metadata(recs: &[RawHscRecord]) -> SpeciesMetadata {
    let mut meta = SpeciesMetadata::default();
    for r in recs {
        if meta.structural_formula.is_empty() && !r.structural_formula.is_empty() {
            meta.structural_formula = r.structural_formula.clone();
        }
        if meta.common_name.is_empty() && !r.common_name.is_empty() {
            meta.common_name = r.common_name.clone();
        }
        if meta.cas_number.is_empty() && !r.cas_number.is_empty() {
            meta.cas_number = r.cas_number.clone();
        }
        if meta.melting_point_k == 0.0 && r.mp_k.is_finite() && r.mp_k != 0.0 {
            meta.melting_point_k = r.mp_k;
        }
        if meta.boiling_point_k == 0.0 && r.bp_k.is_finite() && r.bp_k != 0.0 {
            meta.boiling_point_k = r.bp_k;
        }
        if meta.color_code == 0 && r.color_code != 0 {
            meta.color_code = r.color_code;
        }
        if (meta.solubility.is_empty() || meta.solubility == "-")
            && !r.solubility.is_empty()
            && r.solubility != "-"
        {
            meta.solubility = r.solubility.clone();
        }
        if meta.reliability_class == 0 && r.reliability_class != 0 {
            meta.reliability_class = r.reliability_class;
        }
    }
    if meta.solubility.is_empty() {
        meta.solubility = "-".into();
    }
    meta
}

/// Build an empty 256-byte sentinel `'0'` `.HSC` buffer identical to HSC6 `OWNDB6.HSC`.
pub fn build_empty_hsc_bytes() -> Vec<u8> {
    let mut rec = vec![b' '; 256];
    rec[0x00] = b'0';
    rec[0x18] = b'0';
    rec[0x38] = b'0';
    rec[0x6C] = b'0';
    rec[0x8C] = b'0';
    for off in [0x98, 0x9C, 0xA0, 0xA4, 0xA8] {
        write_f32(&mut rec, off, 0.0);
    }
    rec[0xAC] = b'0';
    for off in [0xAE, 0xB2, 0xB6, 0xBA, 0xBE, 0xC2, 0xC6] {
        write_f32(&mut rec, off, 0.0);
    }
    write_i16(&mut rec, 0xCA, 0);
    rec[0xDA] = b'0';
    write_i16(&mut rec, 0xFE, 1);
    rec
}

/// Serialize a list of `SpeciesDataset`s into binary `.HSC` 256-byte LE/CP1252 format.
/// Emits a valid sentinel `'0'` record if no intervals exist so HSC 6.0 always opens the file cleanly.
/// Validate all fixed-width fields before building bytes for a persistent write.
pub fn serialize_datasets_checked(datasets: &[SpeciesDataset]) -> Result<Vec<u8>, SoryntelError> {
    let mut invalid = std::collections::BTreeSet::new();
    for ds in datasets {
        let sp = &ds.species;
        let meta = &ds.metadata;
        if sp.intervals.is_empty() {
            invalid.insert(format!("{} (no intervals)", sp.formula));
        }
        if sp.phase == Phase::AqueousIon && (sp.charge == 0 || infer_charge(&sp.formula) != sp.charge) {
            invalid.insert(format!("{} (ionic charge cannot be recovered from formula)", sp.formula));
        }
        for interval in &sp.intervals {
            let numbers = [
                interval.t_min, interval.t_max,
                interval.h_or_delta_h_tr / (1000.0 * CAL_TO_JOULE),
                interval.s_or_delta_s_tr / CAL_TO_JOULE,
                interval.kelley.a / CAL_TO_JOULE, interval.kelley.b / CAL_TO_JOULE,
                interval.kelley.c / CAL_TO_JOULE, interval.kelley.d / CAL_TO_JOULE,
                sp.molecular_weight, sp.density_g_cm3, meta.melting_point_k, meta.boiling_point_k,
            ];
            if numbers.into_iter().any(|value| !(value as f32).is_finite())
                || interval.t_min < 0.0 || (interval.t_min as f32) >= (interval.t_max as f32)
            {
                invalid.insert(format!("{} (numeric values outside HSC range)", sp.formula));
            }
        }
        let fields = [
            ("formula", sp.formula.as_str(), 24),
            ("structural formula", meta.structural_formula.as_str(), 32),
            ("name", sp.name.as_str(), 52),
            ("common name", meta.common_name.as_str(), 32),
            ("CAS", meta.cas_number.as_str(), 12),
            ("solubility", meta.solubility.as_str(), 14),
        ];
        for (field, value, width) in fields.into_iter().chain(
            sp.intervals.iter().map(|interval| ("reference", interval.reference.as_str(), 36)),
        ) {
            if value.chars().count() > width || value.chars().any(|ch| {
                ch.is_control() || !(ch.is_ascii() || ('\u{00a0}'..='ÿ').contains(&ch)
                    || CP1252_80_9F.contains(&ch))
            }) {
                invalid.insert(format!("{} ({field}: max {width} CP1252 bytes)", sp.formula));
            }
        }
    }
    if !invalid.is_empty() {
        return Err(SoryntelError::DatabaseError(format!(
            "Unrepresentable HSC fields: {}", invalid.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    Ok(serialize_datasets_to_hsc(datasets))
}

pub fn serialize_datasets_to_hsc(datasets: &[SpeciesDataset]) -> Vec<u8> {
    let mut out = Vec::new();
    for ds in datasets {
        let sp = &ds.species;
        let meta = &ds.metadata;
        for (iv_idx, iv) in sp.intervals.iter().enumerate() {
            let mut rec = vec![b' '; 256];
            rec[0x00..0x18].copy_from_slice(&encode_cp1252_fixed(&sp.formula, 24));
            rec[0x18..0x38].copy_from_slice(&encode_cp1252_fixed(&meta.structural_formula, 32));
            rec[0x38..0x6C].copy_from_slice(&encode_cp1252_fixed(&sp.name, 52));
            rec[0x6C..0x8C].copy_from_slice(&encode_cp1252_fixed(&meta.common_name, 32));
            rec[0x8C..0x98].copy_from_slice(&encode_cp1252_fixed(&meta.cas_number, 12));
            write_f32(&mut rec, 0x98, sp.molecular_weight);
            write_f32(&mut rec, 0x9C, meta.melting_point_k);
            write_f32(&mut rec, 0xA0, meta.bp_k_or_zero(meta.boiling_point_k));
            write_f32(&mut rec, 0xA4, iv.t_min);
            write_f32(&mut rec, 0xA8, iv.t_max);
            let ph_code = phase_to_hsc_code(iv.phase, &ds.raw_phase_code);
            rec[0xAC..0xAE].copy_from_slice(&encode_cp1252_fixed(ph_code, 2));
            write_f32(&mut rec, 0xAE, iv.h_or_delta_h_tr / (1000.0 * CAL_TO_JOULE));
            write_f32(&mut rec, 0xB2, iv.s_or_delta_s_tr / CAL_TO_JOULE);
            write_f32(&mut rec, 0xB6, iv.kelley.a / CAL_TO_JOULE);
            write_f32(&mut rec, 0xBA, iv.kelley.b / CAL_TO_JOULE);
            write_f32(&mut rec, 0xBE, iv.kelley.c / CAL_TO_JOULE);
            write_f32(&mut rec, 0xC2, iv.kelley.d / CAL_TO_JOULE);
            write_f32(&mut rec, 0xC6, if iv_idx == 0 { sp.density_g_cm3 } else { 0.0 });
            write_i16(&mut rec, 0xCA, meta.color_code);
            let sol_str = if meta.solubility == "-" { "" } else { meta.solubility.as_str() };
            rec[0xCC..0xDA].copy_from_slice(&encode_cp1252_fixed(sol_str, 14));
            rec[0xDA..0xFE].copy_from_slice(&encode_cp1252_fixed(&iv.reference, 36));
            write_i16(&mut rec, 0xFE, meta.reliability_class.max(1));
            out.extend_from_slice(&rec);
        }
    }
    if out.is_empty() {
        return build_empty_hsc_bytes();
    }
    out
}

trait MetaHelper {
    fn bp_k_or_zero(&self, val: f64) -> f64;
}
impl MetaHelper for SpeciesMetadata {
    fn bp_k_or_zero(&self, val: f64) -> f64 {
        if val.is_finite() { val } else { 0.0 }
    }
}

/// Decode `MainDB6.REF` (`293`-byte records: 36 B key + 256 B full citation + 1 B flag).
pub fn parse_ref_binary(bytes: &[u8]) -> Vec<ReferenceEntry> {
    const REC_LEN: usize = 293;
    let n = bytes.len() / REC_LEN;
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &bytes[i * REC_LEN..(i + 1) * REC_LEN];
        let key = decode_cp1252(&rec[0..36]);
        if key.is_empty() {
            continue;
        }
        let citation = decode_cp1252(&rec[36..292]);
        let flag = rec[292];
        entries.push(ReferenceEntry { key, citation, flag });
    }
    entries
}

/// Discover companion `.REF` file beside `hsc_path` (`MainDB6.REF`, `<stem>.REF`, `<stem>.ref`).
pub fn discover_companion_ref(hsc_path: &Path) -> Option<PathBuf> {
    let dir = hsc_path.parent()?;
    let stem = hsc_path.file_stem()?.to_string_lossy();
    let candidates = [
        dir.join(format!("{stem}.REF")),
        dir.join(format!("{stem}.ref")),
        dir.join("MainDB6.REF"),
        dir.join("Maindb6.ref"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Generate a companion HSC `*.CAT` index file next to `hsc_path` (102 alphabetical element buckets).
pub fn write_catalogue_file(hsc_path: &Path, datasets: &[SpeciesDataset]) -> Result<PathBuf, SoryntelError> {
    // Compute 1-based record start offsets for each dataset
    let mut flat: Vec<(String, usize)> = Vec::new();
    let mut rec_idx = 1usize;
    for ds in datasets {
        let key = formula_sort_key(&ds.species.formula).0;
        if !ds.species.intervals.is_empty() {
            flat.push((key, rec_idx));
            rec_idx += ds.species.intervals.len();
        }
    }

    // Standard HSC6 element prefix buckets + ASCII CAT output
    const BUCKETS: [&str; 26] = [
        "Ac", "B", "C", "Ca", "Cl", "Cu", "F", "Fe", "H", "Hg", "I", "K", "Li", "Mg", "Mn",
        "N", "Na", "Ni", "O", "P", "Pb", "S", "Si", "Ti", "W", "Zn",
    ];
    let mut lines = Vec::new();
    for prefix in BUCKETS {
        let pos = flat
            .iter()
            .find(|(k, _)| k.starts_with(prefix))
            .map(|(_, r)| *r)
            .unwrap_or(1);
        lines.push(format!(" {pos}"));
    }
    let cat_path = hsc_path.with_extension("CAT");
    let body = lines.join("\r\n") + "\r\n";
    crate::atomic_write::write(&cat_path, body.as_bytes()).map_err(|e| {
        SoryntelError::DatabaseError(format!("Failed writing {}: {e}", cat_path.display()))
    })?;
    Ok(cat_path)
}

/// Export datasets to UTF-8 CSV containing all 19 HSC record columns (calories/Kelvin canonical units).
pub fn export_datasets_csv(datasets: &[SpeciesDataset]) -> String {
    let mut out = String::from(
        "Formula,StructuralFormula,ChemicalName,CommonName,CAS,MW,Mp_K,Bp_K,T1_K,T2_K,Phase,H_kcal,S_cal,CpA_cal,CpB_cal,CpC_cal,CpD_cal,Density_g_cm3,Color,Solubility,Reference,Reliability,DatasetIndex\n",
    );
    for ds in datasets {
        let sp = &ds.species;
        let meta = &ds.metadata;
        for (idx, iv) in sp.intervals.iter().enumerate() {
            let ph = phase_to_hsc_code(iv.phase, &ds.raw_phase_code);
            let row = [
                csv_escape(&sp.formula),
                csv_escape(&meta.structural_formula),
                csv_escape(&sp.name),
                csv_escape(&meta.common_name),
                csv_escape(&meta.cas_number),
                (sp.molecular_weight).to_string(),
                (meta.melting_point_k).to_string(),
                (meta.boiling_point_k).to_string(),
                (iv.t_min).to_string(),
                (iv.t_max).to_string(),
                ph.to_string(),
                (iv.h_or_delta_h_tr / (1000.0 * CAL_TO_JOULE)).to_string(),
                (iv.s_or_delta_s_tr / CAL_TO_JOULE).to_string(),
                (iv.kelley.a / CAL_TO_JOULE).to_string(),
                (iv.kelley.b / CAL_TO_JOULE).to_string(),
                (iv.kelley.c / CAL_TO_JOULE).to_string(),
                (iv.kelley.d / CAL_TO_JOULE).to_string(),
                (if idx == 0 { sp.density_g_cm3 } else { 0.0 }).to_string(),
                meta.color_code.to_string(),
                csv_escape(&meta.solubility),
                csv_escape(&iv.reference),
                meta.reliability_class.max(1).to_string(),
                ds.dataset_index.to_string(),
            ];
            out.push_str(&row.join(","));
            out.push('\n');
        }
    }
    out
}

/// Import datasets from CSV exported by `export_datasets_csv` (or HSC 2.0–style comma/semicolon table).
pub fn import_datasets_csv(csv_text: &str, source: soryntel_thermo::species::DbSource) -> Result<Vec<SpeciesDataset>, SoryntelError> {
    let csv_text = csv_text.trim_start_matches('\u{feff}');
    let sep = csv_separator(csv_text.trim_start());
    let mut records = Vec::new();
    let mut explicit = BTreeMap::<(String, usize), Vec<RawHscRecord>>::new();
    let mut explicit_order = Vec::new();
    for (position, (line_no, cols)) in csv_rows(csv_text, sep)?.into_iter().enumerate() {
        if position == 0 && cols[0].trim().trim_start_matches('\u{feff}').eq_ignore_ascii_case("formula") {
            continue;
        }
        let invalid = |column| SoryntelError::ParseError(format!(
            "CSV line {line_no}, column {}: invalid number", column + 1
        ));
        let number = |column: usize, default: f64| {
            match cols.get(column).map(|value| value.trim()).filter(|value| !value.is_empty()) {
                None => Ok(default),
                Some(value) => value.parse::<f64>().ok().filter(|n| n.is_finite())
                    .ok_or_else(|| invalid(column)),
            }
        };
        let integer = |column: usize, default: i16| {
            match cols.get(column).map(|value| value.trim()).filter(|value| !value.is_empty()) {
                None => Ok(default),
                Some(value) => value.parse::<i16>().map_err(|_| invalid(column)),
            }
        };
        if cols.len() < 17 {
            return Err(SoryntelError::ParseError(format!(
                "CSV line {} has {} columns (expected >= 17)",
                line_no,
                cols.len()
            )));
        }
        let formula = cols[0].trim().to_string();
        if formula.is_empty() {
            continue;
        }
        let t1 = number(8, 298.15)?;
        let t2 = number(9, 1000.0)?;
        if t1 < 0.0 || t2 <= t1 {
            return Err(SoryntelError::ParseError(format!("CSV line {line_no}, columns 9–10: invalid temperature interval")));
        }
        let mw_parsed = number(5, 0.0)?;
        let mw = if mw_parsed > 0.0 {
            mw_parsed
        } else {
            compute_molecular_weight(&formula).map(|(m, _, _)| m).unwrap_or(0.0)
        };
        let raw_phase = cols[10].trim().to_string();
        let record = RawHscRecord {
            phase: map_phase(&raw_phase, &formula),
            formula,
            structural_formula: cols[1].trim().to_string(),
            name: cols[2].trim().to_string(),
            common_name: cols[3].trim().to_string(),
            cas_number: cols[4].trim().to_string(),
            mw,
            mp_k: number(6, 0.0)?,
            bp_k: number(7, 0.0)?,
            t1,
            t2,
            raw_phase,
            h_j: number(11, 0.0)? * 1000.0 * CAL_TO_JOULE,
            s_j: number(12, 0.0)? * CAL_TO_JOULE,
            kelley_j: KelleyCoefficients {
                a: number(13, 0.0)? * CAL_TO_JOULE,
                b: number(14, 0.0)? * CAL_TO_JOULE,
                c: number(15, 0.0)? * CAL_TO_JOULE,
                d: number(16, 0.0)? * CAL_TO_JOULE,
            },
            density: number(17, 0.0)?,
            color_code: integer(18, 0)?,
            solubility: cols
                .get(19)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "-".into()),
            reference: cols.get(20).map(|s| s.trim().to_string()).unwrap_or_default(),
            reliability_class: integer(21, 1)?,
        };
        if let Some(raw) = cols.get(22).filter(|raw| !raw.trim().is_empty()) {
            let index = raw.trim().parse::<usize>().map_err(|_| invalid(22))?;
            let key = (record.formula.clone(), index);
            explicit.entry(key.clone()).or_insert_with(|| {
                explicit_order.push(key);
                Vec::new()
            }).push(record);
        } else {
            records.push(record);
        }
    }
    if !records.is_empty() && !explicit.is_empty() {
        return Err(SoryntelError::ParseError("CSV mixes rows with and without DatasetIndex".into()));
    }
    if !explicit.is_empty() {
        return Ok(explicit_order.into_iter().map(|key| build_dataset(&explicit[&key], source, key.1)).collect());
    }
    let (order, map) = group_records_to_datasets(records, source);
    let mut out = Vec::new();
    for f in order {
        if let Some(mut list) = map.get(&f).cloned() {
            out.append(&mut list);
        }
    }
    Ok(out)
}

/// Repair & normalize legacy HSC 2.0–5.0 records into canonical HSC 6.0 format, sorted + indexed.
pub fn upgrade_legacy_datasets(mut datasets: Vec<SpeciesDataset>) -> Vec<SpeciesDataset> {
    for ds in &mut datasets {
        if ds.species.molecular_weight <= 0.0 {
            if let Ok((mw, _, z)) = compute_molecular_weight(&ds.species.formula) {
                ds.species.molecular_weight = mw;
                ds.species.charge = z;
            }
        }
        if ds.metadata.solubility.trim().is_empty() {
            ds.metadata.solubility = "-".into();
        }
        if ds.metadata.reliability_class <= 0 {
            ds.metadata.reliability_class = 1;
        }
        // In HSC 2.0, neutral aqueous electrolytes ended with `(a)` + phase `ai`; HSC 6.0 uses `(ia)`
        if ds.raw_phase_code == "ai"
            && ds.species.formula.ends_with("(a)")
            && ds.species.charge == 0
        {
            let stem = &ds.species.formula[..ds.species.formula.len() - 3];
            ds.species.formula = format!("{stem}(ia)");
        }
    }
    datasets.sort_by(|a, b| formula_sort_key(&a.species.formula).cmp(&formula_sort_key(&b.species.formula)));
    datasets
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains(['\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv_separator(text: &str) -> char {
    let mut quoted = false;
    let mut commas = 0;
    let mut semicolons = 0;
    for ch in text.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => commas += 1,
            ';' if !quoted => semicolons += 1,
            '\n' | '\r' if !quoted => break,
            _ => {}
        }
    }
    if semicolons > commas { ';' } else { ',' }
}

fn csv_rows(text: &str, separator: char) -> Result<Vec<(usize, Vec<String>)>, SoryntelError> {
    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut closed_quote = false;
    let mut line = 1;
    let mut row_start = 1;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' if quoted => {
                quoted = false;
                closed_quote = true;
            }
            '"' if field.trim().is_empty() && !closed_quote => {
                field.clear();
                quoted = true;
            }
            ch if quoted => {
                if ch == '\n' {
                    line += 1;
                }
                field.push(ch);
            }
            ch if ch == separator => {
                fields.push(std::mem::take(&mut field));
                closed_quote = false;
            }
            '\n' | '\r' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if !fields.is_empty() || !field.trim().is_empty() {
                    fields.push(std::mem::take(&mut field));
                    rows.push((row_start, std::mem::take(&mut fields)));
                } else {
                    field.clear();
                }
                line += 1;
                row_start = line;
                closed_quote = false;
            }
            ch if ch == '"' || (closed_quote && !ch.is_whitespace()) => {
                return Err(SoryntelError::ParseError(format!(
                    "CSV line {line}, column {}: unexpected character after or inside quotes", fields.len() + 1
                )));
            }
            ch if !closed_quote => field.push(ch),
            _ => {}
        }
    }
    if quoted {
        return Err(SoryntelError::ParseError(format!(
            "CSV line {row_start}, column {}: unclosed quote", fields.len() + 1
        )));
    }
    if !fields.is_empty() || !field.trim().is_empty() {
        fields.push(field);
        rows.push((row_start, fields));
    }
    Ok(rows)
}

/// Merge two flat dataset vectors ordered canonically; returns `(merged, conflicts)`.
pub fn detect_and_merge_datasets(
    source_ds: &[SpeciesDataset],
    target_ds: &[SpeciesDataset],
    decisions: &BTreeMap<String, soryntel_thermo::species::MergeConflictDecision>,
) -> (
    Vec<SpeciesDataset>,
    Vec<soryntel_thermo::species::DbMergeConflict>,
) {
    let mut by_formula: BTreeMap<((String, String), usize), SpeciesDataset> = BTreeMap::new();
    for ds in target_ds {
        by_formula.insert((formula_sort_key(&ds.species.formula), ds.dataset_index), ds.clone());
    }
    let mut conflicts = Vec::new();
    for src in source_ds {
        let key = (formula_sort_key(&src.species.formula), src.dataset_index);
        if let Some(dst) = by_formula.get(&key).cloned() {
            conflicts.push(soryntel_thermo::species::DbMergeConflict {
                formula: src.species.formula.clone(),
                source_dataset: src.clone(),
                target_dataset: dst.clone(),
            });
            let dec = decisions
                .get(&src.species.formula)
                .copied()
                .unwrap_or(soryntel_thermo::species::MergeConflictDecision::TakeSource);
            if matches!(dec, soryntel_thermo::species::MergeConflictDecision::TakeSource) {
                by_formula.insert(key, src.clone());
            }
        } else {
            by_formula.insert(key, src.clone());
        }
    }
    (by_formula.into_values().collect(), conflicts)
}
