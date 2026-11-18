//! Serde helpers that keep non-finite `f64` values readable in JSON workbooks.
//!
//! `serde_json` writes `NaN`, `+inf` and `-inf` as `null` and then refuses to
//! read `null` back into an `f64`, so a single non-finite value made a whole
//! workbook unreadable. With `#[serde(with = "soryntel_core::serde_nonfinite")]`:
//!
//! - finite values are written as plain JSON numbers (unchanged format);
//! - `+inf`, `-inf` and `NaN` are written as the strings `"inf"`, `"-inf"` and `"NaN"`;
//! - on reading, numbers, those strings (case-insensitive, also `"Infinity"`) and
//!   `null` (files written by older versions) are accepted; `null` reads as `NaN`.
//!
//! The [`option`] submodule does the same for `Option<f64>` (`null` reads as `None`).

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

/// Serialize an `f64`, writing non-finite values as strings.
pub fn serialize<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.is_finite() {
        s.serialize_f64(*v)
    } else if v.is_nan() {
        s.serialize_str("NaN")
    } else if *v > 0.0 {
        s.serialize_str("inf")
    } else {
        s.serialize_str("-inf")
    }
}

/// Deserialize an `f64` from a number, a non-finite string or `null` (as `NaN`).
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    d.deserialize_any(F64Visitor)
}

/// Parse the textual forms accepted for non-finite values.
pub fn parse_nonfinite(s: &str) -> Option<f64> {
    match s.trim().to_ascii_lowercase().as_str() {
        "nan" => Some(f64::NAN),
        "inf" | "+inf" | "infinity" | "+infinity" => Some(f64::INFINITY),
        "-inf" | "-infinity" => Some(f64::NEG_INFINITY),
        _ => None,
    }
}

struct F64Visitor;

impl<'de> Visitor<'de> for F64Visitor {
    type Value = f64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number, \"inf\", \"-inf\", \"NaN\" or null")
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
        Ok(v)
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
        Ok(v as f64)
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
        Ok(v as f64)
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
        parse_nonfinite(v)
            .or_else(|| v.trim().parse::<f64>().ok())
            .ok_or_else(|| E::invalid_value(de::Unexpected::Str(v), &self))
    }
    fn visit_unit<E: de::Error>(self) -> Result<f64, E> {
        Ok(f64::NAN)
    }
    fn visit_none<E: de::Error>(self) -> Result<f64, E> {
        Ok(f64::NAN)
    }
}

/// Same convention for `Option<f64>`: `None` is `null`; `null` reads as `None`.
pub mod option {
    use super::*;

    pub fn serialize<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(x) => super::serialize(x, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
        struct OptVisitor;
        impl<'de> Visitor<'de> for OptVisitor {
            type Value = Option<f64>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an optional number, \"inf\", \"-inf\" or \"NaN\"")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Some(v as f64))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                F64Visitor.visit_str(v).map(Some)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
                super::deserialize(d).map(Some)
            }
        }
        d.deserialize_any(OptVisitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Row {
        #[serde(with = "crate::serde_nonfinite")]
        x: f64,
        #[serde(with = "crate::serde_nonfinite::option")]
        y: Option<f64>,
    }

    fn rt(x: f64, y: Option<f64>) -> Row {
        let s = serde_json::to_string(&Row { x, y }).unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn finite_values_keep_plain_number_format() {
        let s = serde_json::to_string(&Row { x: 1.5, y: Some(2.0) }).unwrap();
        assert_eq!(s, r#"{"x":1.5,"y":2.0}"#);
    }

    #[test]
    fn non_finite_values_roundtrip() {
        assert_eq!(rt(f64::INFINITY, None).x, f64::INFINITY);
        assert_eq!(rt(f64::NEG_INFINITY, Some(f64::INFINITY)).y, Some(f64::INFINITY));
        assert!(rt(f64::NAN, Some(f64::NAN)).x.is_nan());
        assert_eq!(
            serde_json::to_string(&Row { x: f64::INFINITY, y: Some(f64::NEG_INFINITY) }).unwrap(),
            r#"{"x":"inf","y":"-inf"}"#
        );
    }

    #[test]
    fn legacy_null_and_integers_are_accepted() {
        let r: Row = serde_json::from_str(r#"{"x":null,"y":null}"#).unwrap();
        assert!(r.x.is_nan());
        assert_eq!(r.y, None);
        let r: Row = serde_json::from_str(r#"{"x":3,"y":"Infinity"}"#).unwrap();
        assert_eq!(r.x, 3.0);
        assert_eq!(r.y, Some(f64::INFINITY));
    }
}
