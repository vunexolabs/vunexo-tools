//! Money representation, locked in calculation-engine.md §1: integer minor
//! currency units, no binary floating point anywhere in the money path.
//!
//! Unlike Billing, this domain has no line-item math (no quantity × rate, no
//! discount allocation, no rounding rule) — `amount_minor`/`tax_amount_minor`
//! are entered directly by the user, and every report aggregation is a plain
//! integer `SUM`. `MinorUnits` exists purely to keep "this is minor units,
//! not a float, and not a bare `i64` that could be confused with an id"
//! visible at every call site (calculation-engine.md §1/§10).

/// `serde(transparent)` so the Tauri/JSON boundary sees a plain number, the
/// same shape the frontend already works with (`amount_minor: number`) —
/// wrapping it in `{ "0": 123450 }` would leak an implementation detail into
/// every command payload for no benefit.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct MinorUnits(pub i64);

impl MinorUnits {
    pub const ZERO: MinorUnits = MinorUnits(0);

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for MinorUnits {
    fn from(value: i64) -> Self {
        MinorUnits(value)
    }
}

impl From<MinorUnits> for i64 {
    fn from(value: MinorUnits) -> Self {
        value.0
    }
}

impl std::ops::Add for MinorUnits {
    type Output = MinorUnits;
    fn add(self, rhs: Self) -> Self::Output {
        MinorUnits(self.0 + rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_i64() {
        let value: MinorUnits = 123_450i64.into();
        assert_eq!(value, MinorUnits(123_450));
        assert_eq!(i64::from(value), 123_450);
    }

    #[test]
    fn serializes_as_a_plain_number_not_a_wrapped_object() {
        let value = MinorUnits(4200);
        assert_eq!(serde_json::to_string(&value).unwrap(), "4200");
        let back: MinorUnits = serde_json::from_str("4200").unwrap();
        assert_eq!(back, value);
    }
}
