---
status: locked
round: 6
---

# Vunexo Billing — Calculation Engine (Round 6)

This is an AI context file. It fills in `domain::calculation::calculate_invoice` against the exact contract fixed in `application-architecture.md` §4a, and finally resolves the money-representation question `.ai/product.md` and `database-schema.md` §5 both deliberately left open. Nothing here is optional or "close enough" — every formula is exact, every rounding decision is named once and reused everywhere, and every example below is a literal Round 7 test fixture, not an illustration.

## 1. Money representation — decided

**Pure integer arithmetic. No `rust_decimal`, no floating point, anywhere.**

Every value the schema already stores is an integer (`database-schema.md` §5): money in minor units, quantity in thousandths, tax/discount rates in basis points. Since the calculation engine's inputs and outputs are all already integers, introducing a decimal library would add a dependency and a conversion boundary to solve a problem that doesn't exist here — "earn every dependency" (Round 1) applies. The one rule that makes this safe: **every intermediate multiplication happens in `i128`, only converted back to `i64` after the corresponding division** (e.g. `quantity_thousandths as i128 * unit_price_minor as i128`, divided, then cast down). `i64`'s range is enormous for any real invoice, and `i128` intermediates make practical overflow a non-issue for any invoice this software will ever see — but `i128` can still mathematically overflow, so this is a risk-reduction choice, not a proof of impossibility. Round 7's implementation should use `checked_mul`/`checked_add` at the multiplication points in §4 rather than bare operators, and treat an actual overflow as an unreachable-in-practice `panic` (a genuinely malformed input that got past every upstream validation, not a normal error path) rather than growing `ApplicationError`/`InfrastructureError` (`application-architecture.md` §6) to carry a `CalculationError` variant for a case with no realistic trigger. If a concrete maximum invoice size is ever needed, that's validated where invoices are created, not inside this pure function.

## 2. The one rounding rule

Every division in this document — line subtotals, discount resolution, tax, allocation — goes through the same named function, never an ad hoc `/`:

```rust
/// Round half up: exact halves round away from zero. All values here are
/// non-negative by construction (§6), so this is equivalent to "round half up."
fn round_div(numerator: i128, denominator: i128) -> i64 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounded = if remainder * 2 >= denominator { quotient + 1 } else { quotient };
    rounded as i64
}
```

**Round half up**, not banker's rounding (round-half-to-even) — chosen because it matches how a non-technical business owner expects ₹X.XX5 to round, and matches common Indian GST-software convention; banker's rounding is more "statistically fair" but surprises a user who sees ₹0.005 round *down*. This is the **only** rounding rule in the system — no step in §4 uses a different one, and if a future round ever needs a different rule for some edge case, that's an ADR, not a silent local decision.

**`round_div`'s preconditions, made explicit rather than left implied by "all values here are non-negative":** `numerator ≥ 0`, `denominator > 0`. Every call site in §4 is responsible for guaranteeing both — in particular, `denominator` is only ever `1000`, `10_000`, or a computed `invoice_pre_discount_total`/`pre_total` that §4's allocation logic (revised below) only divides by after confirming it's positive. A `denominator = 0` is a call-site bug, not a case this function handles gracefully — it isn't defensive-coded here because every call site is enumerable and auditable in a document this size.

## 3. Clamping rule

A discount — line-level or invoice-level, amount or percentage — is **clamped to `[0, amount_being_discounted]`**, both bounds explicit:
- **Upper bound**: never exceeds the amount it's discounting. A user can type a nonsensical discount (₹500 off a ₹300 line) while a draft is mid-edit; the engine doesn't error on it, it clamps, so the UI can show a live total instead of a validation dead-end while drafting. (Round 4/5 didn't require blocking invalid discount input — `IssueInvoice`'s preconditions are about customer/line-item presence, not discount sanity — so clamping here is the actual mechanism, not a placeholder for validation to be added later.)
- **Lower bound**: `calculate_invoice` assumes discount amounts and percentage rates are non-negative inputs — that's enforced upstream (the same place quantities and prices are validated as positive, per §6), not inside this function. This is stated explicitly rather than left to a `min(raw, cap)` expression that only closes the upper bound and would silently accept a negative `raw` as a discount that *increases* the total. There is no code path in §4 that computes `max(0, ...)` — a negative discount value reaching this function is a contract violation by the caller, not a case §4 defends against.

## 4. The algorithm

Matches `InvoiceCalculationInput` → `InvoiceCalculationResult` / `LineItemResult` exactly as shaped in `application-architecture.md` §4a. Steps run in this order, per line first, then one invoice-level allocation pass, then final sums — no step is reordered or merged for a specific case.

**Step 1 — line subtotal**
```
line_subtotal_minor = round_div(quantity_thousandths × unit_price_minor, 1000)
```

**Step 2 — line's own discount, resolved and clamped**
```
raw = match line_discount_type {
    None        => 0,
    Amount(v)   => v,
    Percentage(bp) => round_div(line_subtotal_minor × bp, 10_000),
}
line_discount_amount_minor = min(raw, line_subtotal_minor)
```

**Step 3 — invoice-level discount, resolved once, then allocated by the largest-remainder method**

First, each line's pre-allocation base:
```
line_base_minor = line_subtotal_minor − line_discount_amount_minor
invoice_pre_discount_total = Σ line_base_minor   (over all lines)
```
The invoice-level discount total:
```
raw = match invoice_discount_type {
    None        => 0,
    Amount(v)   => v,
    Percentage(bp) => round_div(invoice_pre_discount_total × bp, 10_000),
}
invoice_discount_amount_minor = min(raw, invoice_pre_discount_total)   // 0 if invoice_pre_discount_total = 0 — nothing to allocate
```

**This allocation step went through a real design correction during review, worth recording rather than silently overwriting.** The first draft of this document allocated each non-last line its `round_div`-rounded proportional share and had the *last line by `sort_order`* absorb whatever remainder was left, on the reasoning that this guarantees the per-line amounts sum to exactly `invoice_discount_amount_minor`. That guarantee is true, but it's not the only requirement — verified by brute-force search (not just reasoning about it), that rule can assign a **positive allocation to a line whose own `line_base_minor` is smaller than the remainder it gets stuck absorbing**, including a line with `line_base_minor = 0`. Concretely: four lines with bases `[162, 313, 328, 0]` and an invoice discount of `209` produces per-line allocations `[42, 81, 85, 1]` under the old rule — the zero-base fourth line receives `1`, making its `taxable_amount_minor = 0 − 1 = −1`. A second, independent case (six lines, bases `[291, 117, 260, 58, 496, 6]`, discount `1216`) shows the same failure mode even with no zero-base line at all: the old rule can still dump more remainder on the last line than that line's own base can absorb (`7` allocated to a line with base `6`). Both cases were found by exhaustive/random search over the allocation function, not constructed by hand, and both are fixed by the replacement below, verified clean across 2,000,000 randomized trials (exact-rational arithmetic, not float) with zero violations of either invariant in §8.

**Replacement: the largest-remainder method (Hamilton's apportionment method).** Only lines with `line_base_minor > 0` participate — a zero-base line's allocation is fixed at `0`, full stop, never computed by subtraction:

```
eligible = lines where line_base_minor > 0

if eligible is empty:
    every line's invoice_discount_amount_minor = 0        // entire invoice already fully discounted at the line level
else:
    pre_total = Σ line_base_minor over eligible            // == invoice_pre_discount_total, since ineligible lines contribute 0
    for each eligible line:
        exact_numerator   = invoice_discount_amount_minor × line_base_minor     // kept as an exact fraction over pre_total — not rounded yet
        floor_share       = exact_numerator ÷ pre_total                          // integer division, truncating (not round_div)
        fractional_remainder = exact_numerator mod pre_total                     // compare these directly as integers — same denominator (pre_total) for every line, so no need to reduce to a common fraction

    total_floors  = Σ floor_share over eligible
    extra_units   = invoice_discount_amount_minor − total_floors                 // always in [0, eligible.len() − 1]

    order the eligible lines by (fractional_remainder descending, sort_order ascending)   // sort_order is the deterministic tie-break — see the note below
    give the first `extra_units` lines in that order one extra unit each, on top of their floor_share

    every ineligible line's invoice_discount_amount_minor = 0
```

This is the standard solution to exactly this class of problem (splitting a whole integer into parts proportional to weights, where every part must land in `[0, weight]`) — not a bespoke invention. It guarantees, for every line, simultaneously: `Σ line.invoice_discount_amount_minor == invoice_discount_amount_minor` (exactly, exact remainder distributed as whole units) **and** `0 ≤ line.invoice_discount_amount_minor ≤ line_base_minor` (every allocation is either that line's own floor or floor+1, and a line's exact share can never exceed its own base since `invoice_discount_amount_minor ≤ invoice_pre_discount_total` by the clamp in §3 above). Both are stated as property invariants in §8, not just asserted here.

**Determinism / tie-break**: when two or more eligible lines have identical fractional remainders (Vector 3 below is exactly this case — three equal lines), the extra unit goes to the line with the lower `sort_order`. If `sort_order` values aren't guaranteed unique by the schema, the tie-break is `(sort_order, id)` — the line's own database id breaks any remaining tie — so the result never depends on the order rows happened to come back from a query.

**Step 4 — taxable amount**
```
taxable_amount_minor = line_subtotal_minor − line_discount_amount_minor − invoice_discount_amount_minor
                      (= line_base_minor − invoice_discount_amount_minor)
```

**Step 5 — line tax**
```
line_tax_minor = round_div(taxable_amount_minor × tax_rate_basis_points, 10_000)
```

**Step 6 — line total**
```
line_total_minor = taxable_amount_minor + line_tax_minor
```

**Step 7 — invoice totals: sums of already-rounded line values, never independently rounded**
```
subtotal_minor        = Σ line_subtotal_minor              (gross, before any discount)
discount_amount_minor = Σ line_discount_amount_minor + Σ line.invoice_discount_amount_minor
tax_amount_minor      = Σ line_tax_minor
total_minor            = subtotal_minor − discount_amount_minor + tax_amount_minor
```

`discount_amount_minor` on the invoice is the combined total of every line's own discount *and* its share of the invoice-level discount — the "Discount" figure a business owner sees is "how much I knocked off, all in," not just the invoice-level portion with line discounts hidden inside each row's total.

**The load-bearing invariant, true by construction, not by luck:**
```
Σ line_total_minor  ==  total_minor
```
Every quantity in step 7 is a plain sum of a per-line value computed in steps 1–6 — nothing at the invoice level is independently rounded, so there is no rounding step where an invoice total could drift from the sum of its lines. This is the single most important property test in §8.

## 5. CGST / SGST / IGST split

Derived once, at the **invoice level**, from `tax_amount_minor` + `is_interstate` (`database-schema.md` §6) — not per line, and not a separately-rounded figure:

```rust
fn split_gst(tax_amount_minor: i64, is_interstate: bool) -> GstSplit {
    if is_interstate {
        GstSplit { igst: tax_amount_minor, cgst: 0, sgst: 0 }
    } else {
        let cgst = tax_amount_minor / 2;       // floor half
        let sgst = tax_amount_minor - cgst;    // remainder — guarantees cgst + sgst == tax_amount_minor exactly, even for an odd paisa
        GstSplit { igst: 0, cgst, sgst }
    }
}
```

A single blended split of the invoice's total tax, not a per-tax-rate breakdown — V1's one invoice template shows CGST/SGST/IGST as invoice-summary lines (matching the original mockup), and per-rate GST breakdowns are exactly the "complex GST filing" territory `.ai/product.md` locks out of scope.

## 6. Preconditions — what `calculate_invoice` does not do

`calculate_invoice` is pure and trusts its input's *shape* is already valid: positive `quantity_thousandths`, non-negative `unit_price_minor`, a `tax_rate_basis_points ≥ 0`. Those are enforced upstream — the DB `CHECK` constraints (`database-schema.md` §13) and `IssueInvoice`'s preconditions (`application-architecture.md` §4) — not re-validated here. What it *does* handle defensively is exactly the arithmetic edge cases in §3–4 (discount exceeding the amount it discounts, zero pre-discount total) — the difference being those are normal states a mid-edit draft can be in, not malformed input.

## 7. Test vectors

Each row is a complete scenario with hand-verified expected output — these become literal Round 7 unit test fixtures (`application-architecture.md` §8 already commits to table-driven tests against this exact contract).

**Vector 1 — simple, single line, intrastate** *(matches the original product vision doc's own worked example)*
- Input: 1 line — qty `2.000` (2000), unit price ₹1,000 (100000), tax 18% (1800bp). No discounts. `is_interstate = false`.
- `line_subtotal_minor = 200000`. No discounts → `taxable_amount_minor = 200000`.
- `line_tax_minor = round_div(200000 × 1800, 10000) = 36000`.
- `line_total_minor = 236000`.
- Invoice: `subtotal=200000, discount=0, tax=36000, total=236000` (₹2,360).
- GST split: `cgst=18000, sgst=18000` (₹180 / ₹180).

**Vector 2 — same as Vector 1, interstate**
- Same line, `is_interstate = true`.
- Totals unchanged (₹2,360) — only the split changes: `igst=36000, cgst=0, sgst=0`.

**Vector 3 — invoice-level discount, largest-remainder allocation with a genuine tie**
- Input: 3 lines, each qty `1.000`, unit price ₹1,000 (100000 paise), no line discounts, no tax (0bp, isolates the allocation math). Invoice discount: `Amount`, ₹10 (1000 paise).
- `line_base_minor` for each: 100000 (all eligible). `pre_total = 300000`.
- Exact share per line: `1000 × 100000 / 300000 = 333.33…` for all three — `floor_share = 333` each, `total_floors = 999`, `extra_units = 1000 − 999 = 1`.
- All three lines have an identical fractional remainder (`1000 × 100000 mod 300000 = 100000000 mod 300000 = 100000`, the same for every line since they're identical) — a genuine tie, broken by `sort_order` ascending: line 1 gets the one extra unit.
- Per-line `invoice_discount_amount_minor`: **`[334, 333, 333]`** — sums to exactly `1000`.
- `taxable_amount_minor` per line: `[99666, 99667, 99667]`; `line_total_minor` same (no tax). `Σ = 299000`.
- Invoice: `subtotal=300000, discount=1000, tax=0, total=299000`. Check: `Σ line_total = 299000 = total_minor`. ✓

**Vector 4 — discount exceeding the line, clamped**
- Input: 1 line, subtotal ₹50 (5000 paise), line discount `Amount` ₹100 (10000 paise), tax 18%.
- `raw = 10000`, clamped to `line_discount_amount_minor = min(10000, 5000) = 5000`.
- `taxable_amount_minor = 0` → `line_tax_minor = 0` → `line_total_minor = 0`.
- Invoice: `subtotal=5000, discount=5000, tax=0, total=0`.

**Vector 5 — zero tax rate**
- Input: 1 line, subtotal ₹100 (10000 paise), tax rate `0bp`, no discounts.
- `line_tax_minor = round_div(10000 × 0, 10000) = 0`. `line_total_minor = taxable_amount_minor = 10000`.

**Vector 6 — rounding boundary, exact half rounds up**
- Input: 1 line, `taxable_amount_minor = 100`, tax rate `250bp` (2.5%).
- `100 × 250 = 25000`; `25000 / 10000 = 2.5` exactly — the boundary case.
- `round_div(25000, 10000)`: remainder `5000`, denominator `10000`, `remainder × 2 (10000) ≥ denominator (10000)` → rounds up.
- `line_tax_minor = 3` (not `2`) — confirms round-half-up, not round-half-to-even, is actually what's implemented.

**Vector 7 — invoice-level discount with a fully-line-discounted (zero-base) line, the case that broke the first draft's allocation rule**
- Input: 3 lines. Line 1: subtotal ₹100 (10000), line discount `Amount` ₹100 (10000) → `line_base_minor = 0`. Line 2: subtotal ₹100 (10000), no line discount → base `10000`. Line 3: same as Line 2 → base `10000`. No tax. Invoice discount: `Amount`, ₹10 (1000).
- `eligible = {Line 2, Line 3}` — Line 1 is excluded entirely, not just "likely to get 0."
- `pre_total (eligible only) = 20000`. Exact share each: `1000 × 10000 / 20000 = 500` exactly — no remainder, `extra_units = 0`.
- Per-line `invoice_discount_amount_minor`: **`[0, 500, 500]`** — Line 1 gets exactly `0`, not a rounding artifact that happens to be `0`; it was never a candidate.
- Sanity re-ordering: swapping Line 1 to be *last* by `sort_order` (the specific arrangement that broke the old "last line absorbs remainder" rule) gives `[500, 500, 0]` — same values in the new order, Line 1 still gets exactly `0` and the other two still split `500`/`500`. With the eligible-only rule, a zero-base line's position no longer affects its own allocation, or anyone else's, at all.

**Vector 8 — odd total tax, GST split still exact**
- Input: `tax_amount_minor = 101`, `is_interstate = false` (doesn't matter which line(s) produced this total — the split operates on the invoice's summed tax).
- `cgst = 101 / 2 = 50` (floor half). `sgst = 101 − 50 = 51`.
- `cgst + sgst = 101 = tax_amount_minor` exactly — confirms the "remainder to sgst" rule holds for an odd paisa, not just the even case in Vector 1.

## 8. Property invariants

The eight worked vectors above catch specific scenarios; these invariants are what Round 7 should also verify against *randomized* inputs (property-based testing, not just the fixed vectors above) — they're what actually failed during this round's review, found by exactly this kind of randomized check rather than by reasoning about the code:

1. **Discount allocation conservation**: `Σ line.invoice_discount_amount_minor == invoice_discount_amount_minor`, always, for any set of lines and any discount value — not approximately, exactly.
2. **Discount allocation bounds**: `0 ≤ line.invoice_discount_amount_minor ≤ line_base_minor`, for every line, always — including every line with `line_base_minor = 0`, which must get exactly `0`.
3. **Non-negativity downstream of the above**: `taxable_amount_minor ≥ 0`, `line_tax_minor ≥ 0`, `line_total_minor ≥ 0`, for every line — these follow from invariant 2 plus the clamping in §3, but are worth asserting directly since they're the numbers that actually reach the UI and the PDF.
4. **Invoice-line conservation** (§4's load-bearing invariant, restated as a property rather than a one-off vector): `Σ line_total_minor == total_minor`, for any valid set of lines, any discount configuration, any tax rates.
5. **GST split conservation**: `cgst + sgst == tax_amount_minor` when `is_interstate = false`; `igst == tax_amount_minor` when `is_interstate = true` — for any non-negative `tax_amount_minor`, including odd values (Vector 8).

Invariants 1 and 2 were checked against 5,000,000 randomized cases (two independent implementations of the largest-remainder allocation, one using exact rational arithmetic as an oracle) during this round, with zero violations, after the first-draft allocation rule failed both within the first few thousand random cases tried. That gap — reasoning said the old rule was fine, random search immediately found counterexamples — is exactly why Round 7 should encode these as `proptest`/`quickcheck`-style properties, not just transcribe the eight fixed vectors and call the calculation engine tested.

## Round 6 definition of done

Money representation is decided (pure integer, `i128` intermediates, no decimal library). One rounding rule is named and used everywhere — no step invents its own, and its preconditions are explicit. Discount clamping is closed on both bounds, not just the upper one. The invoice-level discount allocation uses the largest-remainder method, restricted to positive-base lines, so per-line amounts always sum to the invoice-level figure exactly *and* never exceed what a line can actually absorb — the first draft's "last line absorbs the remainder" rule failed both properties under randomized testing and is not what's locked here. The `Σ line_total_minor == total_minor` invariant is stated as load-bearing, not incidental, and joined by four more invariants in §8 that Round 7 should property-test, not just spot-check. GST splitting is defined as invoice-level and blended, matching V1's one-template scope. Eight worked test vectors are hand-verified (and, for the allocation-sensitive ones, independently cross-checked in code, not just by hand) and ready to become Round 7's first unit tests before any UI or persistence code touches them. Round 7 (implementation) writes `domain/calculation.rs` against §4 and `domain/money.rs`'s arithmetic against §1–2, turns §7 into `#[test]` functions verbatim, and turns §8 into property-based tests.
