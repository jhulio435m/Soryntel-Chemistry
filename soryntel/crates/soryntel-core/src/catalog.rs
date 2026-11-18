//! Catalog support types shared by the species-database ports.
//!
//! Plain data types (and pure helpers) used by [`crate::ports`] to browse, edit
//! and administer layered species databases (`OwnDB > MainDB > SoryntelDB > OpenDefault`).
//! They live in core so the ports do not depend on any adapter crate; the
//! original paths (`soryntel_thermo::species::*`, `soryntel_db::*`) re-export
//! them unchanged.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::species::{ChemicalSpecies, Phase, SpeciesMetadata};

/// Originating database layer.
///
/// Precedence: `OwnDB > MainDB > SoryntelDB > OpenDefault`; `OpenDefault` only takes part
/// when no SoryntelDB is loaded. New variants are appended so the serde names of the
/// existing ones (`"Main"`, `"Own"`, `"OpenDefault"`) stay unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbSource {
    Main,
    Own,
    OpenDefault,
    /// SoryntelDB (`.sdb`), the base catalog shipped with Soryntel.
    Soryntel,
}

impl DbSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "MainDB",
            Self::Own => "OwnDB",
            Self::OpenDefault => "Built-in",
            Self::Soryntel => "SoryntelDB",
        }
    }
}

/// Element inclusion rule for `Search Mode` (`Some` = contains any; `All` = contains all).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementSearchMode {
    Some,
    All,
}

/// Target column for `Find Key Word`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeywordSearchField {
    Any,
    Formula,
    StructuralFormula,
    ChemicalName,
    CommonName,
    CasNumber,
}

/// Complete phase + organic filter matching HSC6 Ch. 28.5 (`Search Mode` + `List Names`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseFilterFlags {
    pub solid: bool,
    pub liquid: bool,
    pub gas: bool,
    pub aqua_ions: bool,
    pub aqua_neutrals: bool,
    /// When `false`, excludes `ai` (`(ia)`) neutral electrolytes.
    pub include_ai: bool,
    /// When `false`, excludes `ao` (`(a)`) undissociated aqueous species.
    pub include_ao: bool,
    /// When `false`, skips organic molecules (`H > 0` and `C > 2` per Ch. 28.5).
    pub organic: bool,
    /// Optional explicit carbon atom counts (`4, 6, 7` etc.; empty = unrestricted).
    pub carbon_counts: Vec<u32>,
}

impl Default for PhaseFilterFlags {
    fn default() -> Self {
        Self {
            solid: true,
            liquid: true,
            gas: true,
            aqua_ions: true,
            aqua_neutrals: true,
            include_ai: true,
            include_ao: true,
            organic: true,
            carbon_counts: Vec::new(),
        }
    }
}

impl PhaseFilterFlags {
    pub fn matches_phase(&self, phase: Phase, is_ai_electrolyte: bool) -> bool {
        match phase {
            Phase::Solid => self.solid,
            Phase::Liquid => self.liquid,
            Phase::Gas => self.gas,
            Phase::AqueousIon => self.aqua_ions,
            Phase::AqueousNeutral => {
                if !self.aqua_neutrals {
                    return false;
                }
                if is_ai_electrolyte {
                    self.include_ai
                } else {
                    self.include_ao
                }
            }
        }
    }

    pub fn matches_composition(&self, elements: &BTreeMap<String, f64>) -> bool {
        let c = elements.get("C").copied().unwrap_or(0.0);
        let h = elements.get("H").copied().unwrap_or(0.0);
        if !self.organic && h > 1e-9 && c > 2.0 + 1e-9 {
            return false;
        }
        if !self.carbon_counts.is_empty() {
            let matches_c = self
                .carbon_counts
                .iter()
                .any(|&target| (c - target as f64).abs() < 1e-6);
            if !matches_c {
                return false;
            }
        }
        true
    }
}

/// Complete database dataset block (used when a formula has multiple literature datasets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDataset {
    pub source: DbSource,
    pub species: ChemicalSpecies,
    pub metadata: SpeciesMetadata,
    pub dataset_index: usize,
    pub raw_phase_code: String,
}

/// Summary entry for browsing, listing, and resolving species metadata cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesCatalogEntry {
    pub formula: String,
    pub source: DbSource,
    pub species: ChemicalSpecies,
    pub metadata: SpeciesMetadata,
    pub dataset_count: usize,
    pub raw_phase_code: String,
}

/// Duplicate conflict emitted during `Merge HSC1 -> HSC2` when `Show Duplicates` is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMergeConflict {
    pub formula: String,
    pub source_dataset: SpeciesDataset,
    pub target_dataset: SpeciesDataset,
}

/// Merge resolution decision for duplicate species.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeConflictDecision {
    TakeSource,
    TakeTarget,
}

/// One entry in `MainDB6.REF` (293 bytes fixed-length record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEntry {
    pub key: String,
    pub citation: String,
    pub flag: u8,
}

/// Summary statistics for the active databases (`OwnDB`, `MainDB`, SoryntelDB, `MainDB6.REF`).
#[derive(Debug, Clone, Default)]
pub struct DatabaseStatusSummary {
    pub main_path: Option<PathBuf>,
    pub own_path: Option<PathBuf>,
    pub ref_path: Option<PathBuf>,
    /// Active SoryntelDB (`.sdb`) file.
    pub soryntel_path: Option<PathBuf>,
    pub soryntel_unique_species: usize,
    /// Temperature intervals (equivalent `.HSC` records) held by SoryntelDB.
    pub soryntel_binary_records: usize,
    pub main_unique_species: usize,
    pub main_binary_records: usize,
    pub own_unique_species: usize,
    pub own_binary_records: usize,
    pub resolved_total_species: usize,
    pub references_count: usize,
}

/// Precedence of `MainDB` over the built-in (`OpenDefault`) species when loading it.
///
/// `OwnDB` always wins over both, whichever policy is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MainDbPrecedence {
    /// `OwnDB > MainDB > OpenDefault` (desktop behaviour, `load_main_hsc`).
    OverrideOpenDefault,
    /// `OwnDB > OpenDefault > MainDB` (legacy read-only loader kept for regression parity).
    PreserveOpenDefault,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_filter_organic_and_carbon_counts() {
        let propane: BTreeMap<String, f64> =
            [("C".to_string(), 3.0), ("H".to_string(), 8.0)].into();
        let ethylene: BTreeMap<String, f64> =
            [("C".to_string(), 2.0), ("H".to_string(), 4.0)].into();
        let mut flags = PhaseFilterFlags::default();
        assert!(flags.matches_composition(&propane));
        flags.organic = false;
        assert!(!flags.matches_composition(&propane));
        assert!(flags.matches_composition(&ethylene));
        flags.carbon_counts = vec![3];
        assert!(!flags.matches_composition(&ethylene));
    }

    #[test]
    fn phase_filter_aqueous_neutrals() {
        let mut flags = PhaseFilterFlags::default();
        assert!(flags.matches_phase(Phase::AqueousNeutral, true));
        flags.include_ai = false;
        assert!(!flags.matches_phase(Phase::AqueousNeutral, true));
        assert!(flags.matches_phase(Phase::AqueousNeutral, false));
    }

    #[test]
    fn serde_representation_is_stable() {
        assert_eq!(
            serde_json::to_string(&DbSource::OpenDefault).unwrap(),
            "\"OpenDefault\""
        );
        for (source, name) in [
            (DbSource::Main, "\"Main\""),
            (DbSource::Own, "\"Own\""),
            (DbSource::Soryntel, "\"Soryntel\""),
        ] {
            assert_eq!(serde_json::to_string(&source).unwrap(), name);
            assert_eq!(serde_json::from_str::<DbSource>(name).unwrap(), source);
        }
        assert_eq!(
            serde_json::to_string(&KeywordSearchField::CasNumber).unwrap(),
            "\"CasNumber\""
        );
        let r = ReferenceEntry {
            key: "Barin 89".into(),
            citation: "I. Barin".into(),
            flag: 1,
        };
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            r#"{"key":"Barin 89","citation":"I. Barin","flag":1}"#
        );
    }
}
