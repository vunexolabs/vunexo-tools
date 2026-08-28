//! Money representation and arithmetic primitives, locked in
//! docs/vunexo-billing/calculation-engine.md §1–2: pure integer arithmetic,
//! `i128` intermediates, one named rounding rule. No floating point, no
//! decimal library, anywhere in this module or its callers.

/// Widens two `i64` operands to `i128` and multiplies, via `checked_mul`.
/// Two `i64` values can never actually overflow an `i128` product
/// (`i64::MAX²` is far inside `i128::MAX`), so the `expect` below is
/// unreachable in practice — kept per calculation-engine.md §1's guidance
/// to use checked arithmetic rather than assert overflow is impossible.
pub fn checked_mul128(a: i64, b: i64) -> i128 {
    (a as i128)
        .checked_mul(b as i128)
        .expect("calculation overflow: intermediate value exceeded i128 range")
}

/// Round half up: exact halves round away from zero. Every call site in
/// `domain::calculation` guarantees `numerator >= 0` and `denominator > 0` —
/// see calculation-engine.md §2 for the exact precondition contract.
pub fn round_div(numerator: i128, denominator: i128) -> i64 {
    debug_assert!(numerator >= 0, "round_div: numerator must be non-negative");
    debug_assert!(denominator > 0, "round_div: denominator must be positive");
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounded = if remainder * 2 >= denominator {
        quotient + 1
    } else {
        quotient
    };
    rounded as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_div_exact_division() {
        assert_eq!(round_div(100, 10), 10);
    }

    #[test]
    fn round_div_rounds_half_up() {
        // calculation-engine.md §7, Vector 6: 25000 / 10000 = 2.5 exactly -> rounds to 3.
        assert_eq!(round_div(25_000, 10_000), 3);
    }

    #[test]
    fn round_div_rounds_down_below_half() {
        assert_eq!(round_div(24_999, 10_000), 2);
    }

    #[test]
    fn round_div_rounds_up_above_half() {
        assert_eq!(round_div(25_001, 10_000), 3);
    }

    #[test]
    fn round_div_zero_numerator() {
        assert_eq!(round_div(0, 10_000), 0);
    }
}
