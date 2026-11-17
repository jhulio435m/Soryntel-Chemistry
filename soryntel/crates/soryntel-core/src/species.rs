//! Domain models for chemical species and their thermochemical data.
//!
//! These types are pure data (plus a few closed-form Kelley-polynomial helpers)
//! and carry no dependency on any adapter. Their serde representation is part of
//! the public contract: field and variant names are serialized verbatim, without
//! any `#[serde(rename)]`.

use serde::{Deserialize, Serialize};

/// Physical phase of aggregation for a chemical species.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Condensed solid phase.
    Solid,
    /// Condensed liquid phase.
    Liquid,
    /// Ideal-gas phase.
    Gas,
    /// Charged species in aqueous solution.
    AqueousIon,
    /// Neutral species in aqueous solution.
    AqueousNeutral,
}

/// Kelley heat-capacity coefficients in SI units.
///
/// The heat capacity is
/// `Cp(T) = a + b·1e-3·T + c·1e5/T² + d·1e-6·T²` in J/(mol·K), with `T` in K.
/// The scale factors (`1e-3`, `1e5`, `1e-6`) follow the classic Kelley/HSC
/// tabulation convention.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KelleyCoefficients {
    /// Constant term, J/(mol·K).
    pub a: f64,
    /// Linear term coefficient (multiplied by `1e-3·T`).
    pub b: f64,
    /// Inverse-square term coefficient (multiplied by `1e5/T²`).
    pub c: f64,
    /// Quadratic term coefficient (multiplied by `1e-6·T²`).
    pub d: f64,
}

impl KelleyCoefficients {
    /// Heat capacity `Cp(T)` in J/(mol·K) at temperature `t_k` (K).
    pub fn cp(&self, t_k: f64) -> f64 {
        self.a + self.b * 1e-3 * t_k + self.c * 1e5 / (t_k * t_k) + self.d * 1e-6 * t_k * t_k
    }

    /// Enthalpy change `∫ Cp dT` from `t1` to `t2` (K), in J/mol.
    pub fn integrate_h(&self, t1: f64, t2: f64) -> f64 {
        self.a * (t2 - t1) + 0.5 * self.b * 1e-3 * (t2 * t2 - t1 * t1)
            - self.c * 1e5 * (1.0 / t2 - 1.0 / t1)
            + self.d * 1e-6 / 3.0 * (t2 * t2 * t2 - t1 * t1 * t1)
    }

    /// Entropy change `∫ Cp/T dT` from `t1` to `t2` (K), in J/(mol·K).
    pub fn integrate_s(&self, t1: f64, t2: f64) -> f64 {
        self.a * (t2 / t1).ln() + self.b * 1e-3 * (t2 - t1)
            - 0.5 * self.c * 1e5 * (1.0 / (t2 * t2) - 1.0 / (t1 * t1))
            + 0.5 * self.d * 1e-6 * (t2 * t2 - t1 * t1)
    }

    /// Returns `true` if any temperature-dependent term (`b`, `c`, `d`) is
    /// non-negligible (|x| > 1e-12), i.e. `Cp` is not constant.
    pub fn has_temp_terms(&self) -> bool {
        self.b.abs() > 1e-12 || self.c.abs() > 1e-12 || self.d.abs() > 1e-12
    }
}

/// One temperature interval of a species' thermochemical description.
///
/// For the first interval, `h_or_delta_h_tr` / `s_or_delta_s_tr` hold the
/// standard formation enthalpy and absolute entropy at the reference
/// temperature; for subsequent intervals they hold the enthalpy/entropy of the
/// phase transition at `t_min`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesThermoInterval {
    /// Lower temperature bound of the interval, K.
    pub t_min: f64,
    /// Upper temperature bound of the interval, K.
    pub t_max: f64,
    /// Phase of the species within this interval.
    pub phase: Phase,
    /// Heat-capacity coefficients valid in this interval.
    pub kelley: KelleyCoefficients,
    /// Formation enthalpy (first interval) or transition enthalpy, J/mol.
    pub h_or_delta_h_tr: f64,
    /// Absolute entropy (first interval) or transition entropy, J/(mol·K).
    pub s_or_delta_s_tr: f64,
    /// Bibliographic source of the data.
    pub reference: String,
}

/// A chemical species with its thermochemical intervals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalSpecies {
    /// Chemical formula, including phase suffix if any (e.g. `H2O(g)`).
    pub formula: String,
    /// Human-readable name.
    pub name: String,
    /// Phase at the reference temperature.
    pub phase: Phase,
    /// Molecular weight, g/mol.
    pub molecular_weight: f64,
    /// Density, g/cm³.
    pub density_g_cm3: f64,
    /// Electric charge (non-zero for ions).
    pub charge: i32,
    /// Ordered temperature intervals of thermochemical data.
    pub intervals: Vec<SpeciesThermoInterval>,
}

/// Thermodynamic state of a species at a single temperature, in SI units.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermoPointSI {
    /// Temperature, K.
    pub t_k: f64,
    /// Heat capacity, J/(mol·K).
    pub cp: f64,
    /// Enthalpy, J/mol.
    pub h: f64,
    /// Entropy, J/(mol·K).
    pub s: f64,
    /// Gibbs energy, J/mol.
    pub g: f64,
    /// If the value was extrapolated beyond the data range, the temperature
    /// (K) of the boundary it was extrapolated from.
    pub extrapolated_from_k: Option<f64>,
}

/// Descriptive (non-thermochemical) metadata of a species.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpeciesMetadata {
    /// Structural formula.
    pub structural_formula: String,
    /// Common or trade name.
    pub common_name: String,
    /// CAS registry number.
    pub cas_number: String,
    /// Melting point, K.
    pub melting_point_k: f64,
    /// Boiling point, K.
    pub boiling_point_k: f64,
    /// Display colour code used by the catalog source.
    pub color_code: i16,
    /// Free-text solubility information.
    pub solubility: String,
    /// Data reliability class as given by the catalog source.
    pub reliability_class: i16,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn sample() -> KelleyCoefficients {
        KelleyCoefficients {
            a: 10.0,
            b: 2.0,
            c: 1.0,
            d: 3.0,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < EPS,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn cp_matches_hand_calculation() {
        // Cp(500) = 10 + 2e-3*500 + 1e5/500² + 3e-6*500²
        //         = 10 + 1 + 0.4 + 0.75 = 12.15
        assert_close(sample().cp(500.0), 12.15);
    }

    #[test]
    fn cp_constant_only() {
        let k = KelleyCoefficients {
            a: 29.1,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        };
        assert_close(k.cp(300.0), 29.1);
        assert_close(k.cp(1500.0), 29.1);
    }

    #[test]
    fn integrate_h_matches_hand_calculation() {
        // ΔH(300→500) = 10*200 + 0.5*2e-3*(500²-300²) - 1e5*(1/500-1/300)
        //               + 3e-6/3*(500³-300³)
        //             = 2000 + 160 + 400/3 + 98 = 2391.333...
        assert_close(sample().integrate_h(300.0, 500.0), 2391.0 + 1.0 / 3.0);
    }

    #[test]
    fn integrate_h_is_antisymmetric_and_zero_on_empty_range() {
        let k = sample();
        assert_close(k.integrate_h(400.0, 400.0), 0.0);
        assert_close(k.integrate_h(500.0, 300.0), -k.integrate_h(300.0, 500.0));
    }

    #[test]
    fn integrate_s_matches_hand_calculation() {
        // ΔS(300→500) = 10*ln(5/3) + 2e-3*200 - 0.5*1e5*(1/500²-1/300²)
        //               + 0.5*3e-6*(500²-300²)
        //             = 5.108256237659907 + 0.4 + 16/45 + 0.24
        let expected = 5.108_256_237_659_907 + 0.4 + 16.0 / 45.0 + 0.24;
        assert_close(sample().integrate_s(300.0, 500.0), expected);
    }

    #[test]
    fn integrate_s_constant_cp_is_a_ln_ratio() {
        let k = KelleyCoefficients {
            a: 20.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        };
        // 20 * ln(2) = 13.862943611198906
        assert_close(k.integrate_s(300.0, 600.0), 13.862_943_611_198_906);
    }

    #[test]
    fn has_temp_terms_detects_each_coefficient() {
        let base = KelleyCoefficients {
            a: 5.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        };
        assert!(!base.has_temp_terms());
        assert!(KelleyCoefficients { b: 1.0, ..base }.has_temp_terms());
        assert!(KelleyCoefficients { c: -1.0, ..base }.has_temp_terms());
        assert!(KelleyCoefficients { d: 1e-6, ..base }.has_temp_terms());
        // Below the 1e-12 threshold counts as no temperature dependence.
        assert!(!KelleyCoefficients { b: 1e-13, ..base }.has_temp_terms());
    }

    fn sample_species() -> ChemicalSpecies {
        ChemicalSpecies {
            formula: "H2O(g)".to_string(),
            name: "Water vapour".to_string(),
            phase: Phase::Gas,
            molecular_weight: 18.015,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 298.15,
                t_max: 1100.0,
                phase: Phase::Gas,
                kelley: KelleyCoefficients {
                    a: 30.0,
                    b: 10.7,
                    c: 0.33,
                    d: 0.0,
                },
                h_or_delta_h_tr: -241_826.0,
                s_or_delta_s_tr: 188.835,
                reference: "test".to_string(),
            }],
        }
    }

    #[test]
    fn chemical_species_serde_roundtrip() {
        let sp = sample_species();
        let json = serde_json::to_string(&sp).unwrap();
        let back: ChemicalSpecies = serde_json::from_str(&json).unwrap();

        assert_eq!(back.formula, sp.formula);
        assert_eq!(back.name, sp.name);
        assert_eq!(back.phase, sp.phase);
        assert_eq!(back.molecular_weight, sp.molecular_weight);
        assert_eq!(back.density_g_cm3, sp.density_g_cm3);
        assert_eq!(back.charge, sp.charge);
        assert_eq!(back.intervals.len(), 1);
        let (a, b) = (&back.intervals[0], &sp.intervals[0]);
        assert_eq!(a.t_min, b.t_min);
        assert_eq!(a.t_max, b.t_max);
        assert_eq!(a.phase, b.phase);
        assert_eq!(a.kelley.a, b.kelley.a);
        assert_eq!(a.kelley.b, b.kelley.b);
        assert_eq!(a.kelley.c, b.kelley.c);
        assert_eq!(a.kelley.d, b.kelley.d);
        assert_eq!(a.h_or_delta_h_tr, b.h_or_delta_h_tr);
        assert_eq!(a.s_or_delta_s_tr, b.s_or_delta_s_tr);
        assert_eq!(a.reference, b.reference);
    }

    #[test]
    fn chemical_species_serde_field_names_are_stable() {
        let v = serde_json::to_value(sample_species()).unwrap();
        let obj = v.as_object().unwrap();
        for key in [
            "formula",
            "name",
            "phase",
            "molecular_weight",
            "density_g_cm3",
            "charge",
            "intervals",
        ] {
            assert!(obj.contains_key(key), "missing key {key}");
        }
        assert_eq!(obj.len(), 7);
        assert_eq!(v["phase"], "Gas");
        let iv = v["intervals"][0].as_object().unwrap();
        for key in [
            "t_min",
            "t_max",
            "phase",
            "kelley",
            "h_or_delta_h_tr",
            "s_or_delta_s_tr",
            "reference",
        ] {
            assert!(iv.contains_key(key), "missing interval key {key}");
        }
        for key in ["a", "b", "c", "d"] {
            assert!(v["intervals"][0]["kelley"].get(key).is_some());
        }
    }

    #[test]
    fn phase_variant_names_are_stable() {
        let cases = [
            (Phase::Solid, "\"Solid\""),
            (Phase::Liquid, "\"Liquid\""),
            (Phase::Gas, "\"Gas\""),
            (Phase::AqueousIon, "\"AqueousIon\""),
            (Phase::AqueousNeutral, "\"AqueousNeutral\""),
        ];
        for (phase, json) in cases {
            assert_eq!(serde_json::to_string(&phase).unwrap(), json);
            assert_eq!(serde_json::from_str::<Phase>(json).unwrap(), phase);
        }
    }
}
