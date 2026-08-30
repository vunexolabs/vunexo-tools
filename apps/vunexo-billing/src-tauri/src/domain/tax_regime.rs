//! calculation-engine-v2.md §1 — a deliberately small, closed set of tax
//! regimes. Regime *behavior* lives in code that matches on this enum
//! (calculate_invoice's core steps, split_gst, present_vat), never in a
//! configuration table — database-schema-v2.md §5's governing constraint.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxRegimeCode {
    InGst,
    VatStandard,
}

impl Default for TaxRegimeCode {
    /// V1's only regime, and the schema default — lets `#[serde(default)]`
    /// keep the existing V1 frontend (which doesn't send this field yet)
    /// working at the Tauri command boundary until it's updated.
    fn default() -> Self {
        TaxRegimeCode::InGst
    }
}

impl TaxRegimeCode {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "VAT_STANDARD" => TaxRegimeCode::VatStandard,
            _ => TaxRegimeCode::InGst,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            TaxRegimeCode::InGst => "IN_GST",
            TaxRegimeCode::VatStandard => "VAT_STANDARD",
        }
    }
}

/// application-architecture-v2.md §4b: `NULL` on an already-issued invoice
/// is a legacy pre-V2 state, not an ambiguous one — normalized to `IN_GST`
/// here, the one place this fallback is allowed to exist. Every other call
/// site receives an already-normalized `TaxRegimeCode`, never has to repeat
/// this `unwrap_or`.
pub fn normalize_legacy_snapshot(stored: Option<&str>) -> TaxRegimeCode {
    stored
        .map(TaxRegimeCode::from_db_str)
        .unwrap_or(TaxRegimeCode::InGst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_both_regimes() {
        for regime in [TaxRegimeCode::InGst, TaxRegimeCode::VatStandard] {
            assert_eq!(TaxRegimeCode::from_db_str(regime.as_db_str()), regime);
        }
    }

    #[test]
    fn legacy_null_normalizes_to_in_gst() {
        assert_eq!(normalize_legacy_snapshot(None), TaxRegimeCode::InGst);
    }

    #[test]
    fn unknown_db_string_falls_back_to_in_gst_rather_than_panicking() {
        assert_eq!(
            TaxRegimeCode::from_db_str("SOMETHING_ELSE"),
            TaxRegimeCode::InGst
        );
    }
}
