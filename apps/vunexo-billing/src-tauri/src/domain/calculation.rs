//! The invoice calculation engine. Implements
//! docs/vunexo-billing/calculation-engine.md §4 (the algorithm) and §5 (GST
//! split) exactly — every step, every rounding call, and the largest-remainder
//! discount allocation are line-for-line translations of that locked
//! document, not a reinterpretation of it. Pure: no I/O, no clock, no
//! randomness — every input it needs is passed in.
//!
//! `sort_order` (calculation-engine.md §4's tie-break) is the position of a
//! line item within `InvoiceCalculationInput::line_items` itself — the Vec's
//! index order *is* the sort order, per `application-architecture.md` §4a's
//! "same order as input line_items" contract. No separate field is needed.

use super::invoice::DiscountType;
use super::money::{checked_mul128, round_div};

#[derive(Debug, Clone, Copy)]
pub struct LineItemInput {
    pub quantity_thousandths: i64,
    pub unit_price_minor: i64,
    pub tax_rate_basis_points: i64,
    pub line_discount: Option<(DiscountType, i64)>,
}

#[derive(Debug, Clone)]
pub struct InvoiceCalculationInput {
    pub line_items: Vec<LineItemInput>,
    pub invoice_discount: Option<(DiscountType, i64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineItemResult {
    pub line_subtotal_minor: i64,
    pub line_discount_amount_minor: i64,
    pub invoice_discount_amount_minor: i64,
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceCalculationResult {
    pub lines: Vec<LineItemResult>,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GstSplit {
    pub cgst: i64,
    pub sgst: i64,
    pub igst: i64,
}

/// A discount resolved against `base`, clamped to `[0, base]` on the upper
/// bound only (calculation-engine.md §3) — `discount`'s value/rate is assumed
/// non-negative by the caller; this function does not enforce that lower
/// bound itself.
fn resolve_discount(discount: Option<(DiscountType, i64)>, base: i64) -> i64 {
    let raw = match discount {
        None => 0,
        Some((DiscountType::Amount, value)) => value,
        Some((DiscountType::Percentage, basis_points)) => {
            round_div(checked_mul128(base, basis_points), 10_000)
        }
    };
    raw.min(base)
}

struct LineWorking {
    line_subtotal_minor: i64,
    line_discount_amount_minor: i64,
    line_base_minor: i64,
    tax_rate_basis_points: i64,
}

/// Step 3's allocation, in isolation: the largest-remainder (Hamilton)
/// method, restricted to lines with a positive base. Returns one allocation
/// per line in `working`, in the same order, summing to exactly
/// `invoice_discount_amount_minor`.
fn allocate_largest_remainder(
    working: &[LineWorking],
    invoice_discount_amount_minor: i64,
) -> Vec<i64> {
    let mut allocation = vec![0i64; working.len()];
    if invoice_discount_amount_minor == 0 {
        return allocation;
    }

    let eligible: Vec<usize> = (0..working.len())
        .filter(|&i| working[i].line_base_minor > 0)
        .collect();
    if eligible.is_empty() {
        return allocation;
    }

    let pre_total: i128 = eligible
        .iter()
        .map(|&i| working[i].line_base_minor as i128)
        .sum();

    let mut remainders: Vec<(usize, i128)> = Vec::with_capacity(eligible.len());
    let mut total_floor: i64 = 0;
    for &i in &eligible {
        let numerator = checked_mul128(invoice_discount_amount_minor, working[i].line_base_minor);
        let floor_share = (numerator / pre_total) as i64;
        let remainder = numerator % pre_total;
        allocation[i] = floor_share;
        total_floor += floor_share;
        remainders.push((i, remainder));
    }

    let extra_units = (invoice_discount_amount_minor - total_floor) as usize;
    // Largest fractional remainder first; ties broken by ascending index,
    // i.e. ascending sort_order — see the module doc comment.
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for &(i, _) in remainders.iter().take(extra_units) {
        allocation[i] += 1;
    }

    allocation
}

/// The whole algorithm, calculation-engine.md §4, Steps 1–7.
pub fn calculate_invoice(input: &InvoiceCalculationInput) -> InvoiceCalculationResult {
    // Steps 1–2: per-line subtotal and the line's own discount.
    let working: Vec<LineWorking> = input
        .line_items
        .iter()
        .map(|line| {
            let line_subtotal_minor = round_div(
                checked_mul128(line.quantity_thousandths, line.unit_price_minor),
                1000,
            );
            let line_discount_amount_minor =
                resolve_discount(line.line_discount, line_subtotal_minor);
            let line_base_minor = line_subtotal_minor - line_discount_amount_minor;
            LineWorking {
                line_subtotal_minor,
                line_discount_amount_minor,
                line_base_minor,
                tax_rate_basis_points: line.tax_rate_basis_points,
            }
        })
        .collect();

    // Step 3: invoice-level discount, resolved once, then allocated.
    let invoice_pre_discount_total: i64 = working.iter().map(|w| w.line_base_minor).sum();
    let invoice_discount_amount_minor =
        resolve_discount(input.invoice_discount, invoice_pre_discount_total);
    let invoice_discount_allocation =
        allocate_largest_remainder(&working, invoice_discount_amount_minor);

    // Steps 4–6: taxable amount, tax, and total, per line.
    let lines: Vec<LineItemResult> = working
        .iter()
        .zip(invoice_discount_allocation.iter())
        .map(|(w, &allocated)| {
            let taxable_amount_minor = w.line_base_minor - allocated;
            let line_tax_minor = round_div(
                checked_mul128(taxable_amount_minor, w.tax_rate_basis_points),
                10_000,
            );
            let line_total_minor = taxable_amount_minor + line_tax_minor;
            LineItemResult {
                line_subtotal_minor: w.line_subtotal_minor,
                line_discount_amount_minor: w.line_discount_amount_minor,
                invoice_discount_amount_minor: allocated,
                taxable_amount_minor,
                line_tax_minor,
                line_total_minor,
            }
        })
        .collect();

    // Step 7: invoice totals — plain sums of already-rounded line values.
    let subtotal_minor: i64 = lines.iter().map(|l| l.line_subtotal_minor).sum();
    let discount_amount_minor: i64 = lines
        .iter()
        .map(|l| l.line_discount_amount_minor + l.invoice_discount_amount_minor)
        .sum();
    let tax_amount_minor: i64 = lines.iter().map(|l| l.line_tax_minor).sum();
    let total_minor = subtotal_minor - discount_amount_minor + tax_amount_minor;

    InvoiceCalculationResult {
        lines,
        subtotal_minor,
        discount_amount_minor,
        tax_amount_minor,
        total_minor,
    }
}

/// calculation-engine.md §5 — a single blended split at the invoice level,
/// not per line and not per tax rate.
pub fn split_gst(tax_amount_minor: i64, is_interstate: bool) -> GstSplit {
    if is_interstate {
        GstSplit {
            igst: tax_amount_minor,
            cgst: 0,
            sgst: 0,
        }
    } else {
        let cgst = tax_amount_minor / 2;
        let sgst = tax_amount_minor - cgst;
        GstSplit {
            igst: 0,
            cgst,
            sgst,
        }
    }
}

#[cfg(test)]
mod vector_tests {
    //! calculation-engine.md §7 — each test here is one worked vector from
    //! that document, transcribed, not reinterpreted.
    use super::*;

    fn line(qty_thousandths: i64, unit_price_minor: i64, tax_bp: i64) -> LineItemInput {
        LineItemInput {
            quantity_thousandths: qty_thousandths,
            unit_price_minor,
            tax_rate_basis_points: tax_bp,
            line_discount: None,
        }
    }

    #[test]
    fn vector_1_simple_single_line_intrastate() {
        let input = InvoiceCalculationInput {
            line_items: vec![line(2000, 100_000, 1800)],
            invoice_discount: None,
        };
        let result = calculate_invoice(&input);

        assert_eq!(result.lines[0].line_subtotal_minor, 200_000);
        assert_eq!(result.lines[0].taxable_amount_minor, 200_000);
        assert_eq!(result.lines[0].line_tax_minor, 36_000);
        assert_eq!(result.lines[0].line_total_minor, 236_000);
        assert_eq!(result.subtotal_minor, 200_000);
        assert_eq!(result.discount_amount_minor, 0);
        assert_eq!(result.tax_amount_minor, 36_000);
        assert_eq!(result.total_minor, 236_000);

        let split = split_gst(result.tax_amount_minor, false);
        assert_eq!(split.cgst, 18_000);
        assert_eq!(split.sgst, 18_000);
        assert_eq!(split.igst, 0);
    }

    #[test]
    fn vector_2_same_as_vector_1_interstate() {
        let input = InvoiceCalculationInput {
            line_items: vec![line(2000, 100_000, 1800)],
            invoice_discount: None,
        };
        let result = calculate_invoice(&input);
        assert_eq!(result.total_minor, 236_000);

        let split = split_gst(result.tax_amount_minor, true);
        assert_eq!(split.igst, 36_000);
        assert_eq!(split.cgst, 0);
        assert_eq!(split.sgst, 0);
    }

    #[test]
    fn vector_3_invoice_discount_largest_remainder_with_tie() {
        let input = InvoiceCalculationInput {
            line_items: vec![
                line(1000, 100_000, 0),
                line(1000, 100_000, 0),
                line(1000, 100_000, 0),
            ],
            invoice_discount: Some((DiscountType::Amount, 1000)),
        };
        let result = calculate_invoice(&input);

        // per-line, in order — tie broken by ascending index (line 1 wins).
        assert_eq!(
            result
                .lines
                .iter()
                .map(|l| l.invoice_discount_amount_minor)
                .collect::<Vec<_>>(),
            vec![334, 333, 333]
        );
        assert_eq!(
            result
                .lines
                .iter()
                .map(|l| l.taxable_amount_minor)
                .collect::<Vec<_>>(),
            vec![99_666, 99_667, 99_667]
        );
        assert_eq!(result.subtotal_minor, 300_000);
        assert_eq!(result.discount_amount_minor, 1000);
        assert_eq!(result.tax_amount_minor, 0);
        assert_eq!(result.total_minor, 299_000);
        assert_eq!(
            result.lines.iter().map(|l| l.line_total_minor).sum::<i64>(),
            result.total_minor
        );
    }

    #[test]
    fn vector_4_discount_exceeding_line_clamped() {
        let input = InvoiceCalculationInput {
            line_items: vec![LineItemInput {
                quantity_thousandths: 1000,
                unit_price_minor: 5000,
                tax_rate_basis_points: 1800,
                line_discount: Some((DiscountType::Amount, 10_000)),
            }],
            invoice_discount: None,
        };
        let result = calculate_invoice(&input);

        assert_eq!(result.lines[0].line_subtotal_minor, 5000);
        assert_eq!(result.lines[0].line_discount_amount_minor, 5000);
        assert_eq!(result.lines[0].taxable_amount_minor, 0);
        assert_eq!(result.lines[0].line_tax_minor, 0);
        assert_eq!(result.lines[0].line_total_minor, 0);
        assert_eq!(result.total_minor, 0);
    }

    #[test]
    fn vector_5_zero_tax_rate() {
        let input = InvoiceCalculationInput {
            line_items: vec![line(1000, 10_000, 0)],
            invoice_discount: None,
        };
        let result = calculate_invoice(&input);

        assert_eq!(result.lines[0].line_tax_minor, 0);
        assert_eq!(result.lines[0].line_total_minor, 10_000);
    }

    #[test]
    fn vector_6_rounding_boundary_exact_half_rounds_up() {
        // taxable_amount_minor = 100 directly, via a zero-price line with the
        // tax computed against a nonzero taxable base is awkward to construct
        // through the public API alone, so this exercises round_div directly
        // (calculation-engine.md §7 states the vector in terms of round_div).
        assert_eq!(round_div(checked_mul128(100, 250), 10_000), 3);
    }

    #[test]
    fn vector_7_invoice_discount_with_zero_base_line() {
        let make = |bases_order: [i64; 3]| {
            let items: Vec<LineItemInput> = bases_order
                .iter()
                .map(|&base_price| LineItemInput {
                    quantity_thousandths: 1000,
                    unit_price_minor: 10_000,
                    tax_rate_basis_points: 0,
                    line_discount: if base_price == 0 {
                        Some((DiscountType::Amount, 10_000))
                    } else {
                        None
                    },
                })
                .collect();
            InvoiceCalculationInput {
                line_items: items,
                invoice_discount: Some((DiscountType::Amount, 1000)),
            }
        };

        // Line 1 is the fully-discounted (zero-base) line.
        let normal = calculate_invoice(&make([0, 10_000, 10_000]));
        assert_eq!(
            normal
                .lines
                .iter()
                .map(|l| l.invoice_discount_amount_minor)
                .collect::<Vec<_>>(),
            vec![0, 500, 500]
        );

        // Same lines, zero-base line moved last by sort_order (the arrangement
        // that broke the first draft's "last line absorbs remainder" rule).
        let reordered = calculate_invoice(&make([10_000, 10_000, 0]));
        assert_eq!(
            reordered
                .lines
                .iter()
                .map(|l| l.invoice_discount_amount_minor)
                .collect::<Vec<_>>(),
            vec![500, 500, 0]
        );
    }

    #[test]
    fn vector_8_odd_total_tax_gst_split_still_exact() {
        let split = split_gst(101, false);
        assert_eq!(split.cgst, 50);
        assert_eq!(split.sgst, 51);
        assert_eq!(split.cgst + split.sgst, 101);
    }
}

#[cfg(test)]
mod property_tests {
    //! calculation-engine.md §8 — the five invariants, checked against
    //! randomized input rather than the fixed vectors above.
    use super::*;
    use proptest::prelude::*;

    fn discount_strategy() -> impl Strategy<Value = Option<(DiscountType, i64)>> {
        prop::option::of((
            prop_oneof![Just(DiscountType::Amount), Just(DiscountType::Percentage)],
            0i64..=50_000_000,
        ))
    }

    fn line_item_strategy() -> impl Strategy<Value = LineItemInput> {
        (
            1i64..=100_000,
            0i64..=50_000_000,
            0i64..=10_000,
            discount_strategy(),
        )
            .prop_map(
                |(quantity_thousandths, unit_price_minor, tax_rate_basis_points, line_discount)| {
                    LineItemInput {
                        quantity_thousandths,
                        unit_price_minor,
                        tax_rate_basis_points,
                        line_discount,
                    }
                },
            )
    }

    fn invoice_input_strategy() -> impl Strategy<Value = InvoiceCalculationInput> {
        (
            prop::collection::vec(line_item_strategy(), 1..8),
            discount_strategy(),
        )
            .prop_map(|(line_items, invoice_discount)| InvoiceCalculationInput {
                line_items,
                invoice_discount,
            })
    }

    proptest! {
        #[test]
        fn invariant_1_allocation_conservation(input in invoice_input_strategy()) {
            let result = calculate_invoice(&input);
            let sum_alloc: i64 = result.lines.iter().map(|l| l.invoice_discount_amount_minor).sum();

            let pre_total: i64 = result
                .lines
                .iter()
                .map(|l| l.line_subtotal_minor - l.line_discount_amount_minor)
                .sum();
            let expected = match input.invoice_discount {
                None => 0,
                Some((DiscountType::Amount, v)) => v.min(pre_total),
                Some((DiscountType::Percentage, bp)) => {
                    round_div(checked_mul128(pre_total, bp), 10_000).min(pre_total)
                }
            };
            prop_assert_eq!(sum_alloc, expected);
        }

        #[test]
        fn invariant_2_allocation_bounds(input in invoice_input_strategy()) {
            let result = calculate_invoice(&input);
            for line in &result.lines {
                let base = line.line_subtotal_minor - line.line_discount_amount_minor;
                prop_assert!(line.invoice_discount_amount_minor >= 0);
                prop_assert!(line.invoice_discount_amount_minor <= base);
            }
        }

        #[test]
        fn invariant_3_non_negative_downstream(input in invoice_input_strategy()) {
            let result = calculate_invoice(&input);
            for line in &result.lines {
                prop_assert!(line.taxable_amount_minor >= 0);
                prop_assert!(line.line_tax_minor >= 0);
                prop_assert!(line.line_total_minor >= 0);
            }
        }

        #[test]
        fn invariant_4_invoice_line_conservation(input in invoice_input_strategy()) {
            let result = calculate_invoice(&input);
            let sum_line_totals: i64 = result.lines.iter().map(|l| l.line_total_minor).sum();
            prop_assert_eq!(sum_line_totals, result.total_minor);
        }

        #[test]
        fn invariant_5_gst_split_conservation(tax_amount_minor in 0i64..=1_000_000_000, is_interstate: bool) {
            let split = split_gst(tax_amount_minor, is_interstate);
            if is_interstate {
                prop_assert_eq!(split.igst, tax_amount_minor);
                prop_assert_eq!(split.cgst, 0);
                prop_assert_eq!(split.sgst, 0);
            } else {
                prop_assert_eq!(split.cgst + split.sgst, tax_amount_minor);
                prop_assert_eq!(split.igst, 0);
            }
        }
    }
}
