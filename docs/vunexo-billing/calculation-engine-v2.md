---
status: locked
round: 6
---

# Vunexo Billing — V2 Calculation Engine Deltas (Round 6)

This is an AI context file, same status as `docs/vunexo-billing/calculation-engine.md`. It is a **delta** document — every rule in that file (money representation, the one rounding rule, clamping, the algorithm in its §4, the property invariants in its §8) still applies unchanged. This round names V2's second tax regime and fills in exactly what's new: the `calculate_invoice` signature's `regime` parameter (committed in `application-architecture-v2.md` §4a but left unimplemented), and that regime's presentation function.

## 1. The second regime: `VAT_STANDARD`

A deliberately narrow flat-rate VAT model — not a claim of covering any country's actual VAT law. Named to make that boundary impossible to misread later (`database-schema-v2.md` §5's amendment already locks the DB-level name):

**In scope:**
- Tax computed per line, from that line's own `tax_rate_basis_points` (the same field GST already uses).
- Tax is applied to the taxable amount exactly as §4 of `calculation-engine.md` already computes it.
- One aggregate `tax_amount_minor` at the invoice level.

**Explicitly not in scope, named so a future reader doesn't assume otherwise:** jurisdiction/nexus rules, exemptions, reverse charge, tax-inclusive pricing, multiple simultaneous rates per jurisdiction, destination/origin sourcing. `VAT_STANDARD` means exactly "flat per-line rate, tax-exclusive, single aggregate" — the same shape GST already has, minus the CGST/SGST/IGST split. If a real deployment ever needs any of the excluded items, that's a new named regime (or an ADR revisiting this one), never a silent expansion of what `VAT_STANDARD` is assumed to cover.

## 2. The finding: no core-arithmetic change

Steps 1–7 of `calculation-engine.md` §4 (line subtotal, line discount, invoice-discount allocation, taxable amount, line tax, line total, invoice sums) reference `tax_rate_basis_points` generically — **nothing in that algorithm is GST-specific**. GST's only regime-specific behavior in the entire existing document is §5, the CGST/SGST/IGST split, which is a separate function called *after* `calculate_invoice` returns, operating on the already-computed `tax_amount_minor` — not a step inside the algorithm itself.

Verified by hand against your own worked example before writing anything down: 1 line, qty 2 × ₹5,000/unit, 20% rate, ₹1,000 invoice-level discount, run through §4 of `calculation-engine.md` **completely unmodified**:

```
line_subtotal_minor = round_div(2000 × 500000, 1000) = 1,000,000        (₹10,000)
invoice_discount_amount_minor = min(100000, 1000000) = 100,000           (₹1,000 — single eligible line, gets it all)
taxable_amount_minor = 1,000,000 − 0 − 100,000 = 900,000                 (₹9,000)
line_tax_minor = round_div(900000 × 2000, 10000) = 180,000               (₹1,800)
line_total_minor = 900,000 + 180,000 = 1,080,000                         (₹10,800)
```

Matches exactly. This means `VAT_STANDARD` needs **zero changes** to `calculate_invoice`'s steps 1–7, its rounding rule, its clamping rule, or its discount-allocation logic. Every property invariant in `calculation-engine.md` §8 (conservation, bounds, non-negativity, `Σ line_total_minor == total_minor`) already holds for `VAT_STANDARD` for free, by virtue of it being the same code path.

**Consequence for the function signature `application-architecture-v2.md` §4a already committed to:**

```rust
pub fn calculate_invoice(
    input: InvoiceCalculationInput,
    regime: TaxRegimeCode,
) -> InvoiceCalculationResult
```

`regime` is accepted but **not matched on anywhere inside steps 1–7** — both `TaxRegimeCode::InGst` and `TaxRegimeCode::VatStandard` run the identical code path. This is deliberately *not* implemented as `match regime { InGst => calculate_gst(input), VatStandard => calculate_vat(input) }` with two near-identical function bodies — that would create two copies of the same formula with no behavioral difference between them, which is exactly the "two implementations of the same arithmetic quietly drifting apart" failure mode `application-architecture.md` §4a already names as the reason V1 has exactly one invoice-math implementation. `regime` stays a parameter on the function — not deleted — for two reasons: it's part of the contract Round 4 already locked, and a future regime with genuinely different core arithmetic (tax-inclusive pricing, say) would need it to branch inside §4 for the first time. Until that day, the parameter is honest about being currently inert for the core computation, not a placeholder pretending to do more than it does.

**Stated precisely, since "regime-agnostic" alone invites the wrong reading**: the calculation result is regime-neutral *in computation*, for the two regimes V2 actually has — not merely neutral in output shape while secretly computing differently underneath. That equivalence is a fact about `IN_GST` and `VAT_STANDARD` specifically (both being flat-rate, tax-exclusive, per-line models), not a general law that any future regime will also turn out to need no branch. Round 7 should not read this as "regime never affects arithmetic" — only as "these two regimes, as scoped, don't."

## 3. VAT presentation function

Symmetric with `split_gst` (`calculation-engine.md` §5), even though the transformation is closer to a passthrough — kept as its own named function rather than inlined at each call site, so the UI/PDF/report layer's dispatch point stays "one function per regime," matching the `useTaxRegimeFields`-style single-switch-point discipline `ui-ux-v2.md` §3 already established on the frontend side:

```rust
pub struct VatPresentation {
    pub vat_amount_minor: i64,
}

fn present_vat(tax_amount_minor: i64) -> VatPresentation {
    VatPresentation { vat_amount_minor: tax_amount_minor }
}
```

No split, no allocation — `VAT_STANDARD` has nothing analogous to GST's intrastate/interstate distinction (per the `database-schema-v2.md` §5 amendment: no `is_interstate`-equivalent column). A `VAT_STANDARD` PDF/report renders a single `VAT` line at `tax_amount_minor`, the same way a `IN_GST` interstate invoice renders a single `IGST` line — structurally the simpler of the two cases GST already has, not a new shape.

## 4. Test vectors

Two new vectors, plus an explicit note on what's *not* duplicated:

**Vector V1 — VAT, single line, no discount (mirrors `calculation-engine.md` Vector 1 exactly, same inputs, different regime)**
- Input: 1 line, qty `2.000` (2000), unit price ₹1,000 (100000), VAT rate 1800bp (18%). No discounts. `regime = VatStandard`.
- Identical arithmetic to Vector 1: `line_subtotal_minor = 200000`, `taxable_amount_minor = 200000`, `line_tax_minor = 36000`, `line_total_minor = 236000`.
- Invoice: `subtotal=200000, discount=0, tax=36000, total=236000` — **byte-identical to Vector 1's totals.**
- `present_vat(36000) = VatPresentation { vat_amount_minor: 36000 }`. Contrast with Vector 1's `split_gst(36000, false) = { cgst: 18000, sgst: 18000 }` — same input, deliberately different presentation, to make the "core same, presentation diverges" claim a literal assertion in a test rather than only a design-doc claim.

**Vector V2 — VAT, invoice-level discount (your worked example, hand-verified above and independently by script before this document was written)**
- Input: 1 line, qty `2.000` (2000), unit price ₹5,000 (500000), VAT rate 2000bp (20%). Invoice discount: `Amount`, ₹1,000 (100000). `regime = VatStandard`.
- `line_subtotal_minor = 1,000,000`, `invoice_discount_amount_minor = 100,000`, `taxable_amount_minor = 900,000`, `line_tax_minor = 180,000`, `line_total_minor = 1,080,000`.
- Invoice: `subtotal=1000000, discount=100000, tax=180000, total=1080000` (₹10,000 / ₹1,000 / ₹1,800 / ₹10,800).
- `present_vat(180000) = VatPresentation { vat_amount_minor: 180000 }`.

**Not duplicated, and why that's correct rather than a coverage gap:** `calculation-engine.md`'s Vectors 3, 4, 6, 7 (largest-remainder allocation with a tie, discount-exceeding-line clamping, the round-half-up boundary case, and the zero-base-line allocation fix) all exercise steps that never read `regime` — re-running them under `VatStandard` would produce identical numbers by construction (§2 above), so a duplicate vector would test the test harness, not the code. Round 7 should still add a single parameterized-test pass (loop Vectors 1, 3, 4, 6, 7 under both `TaxRegimeCode` values, assert identical `InvoiceCalculationResult` for every one) as a cheap, direct confirmation of the "no branch in steps 1–7" claim — one property test, not five hand-written duplicate vectors.

## Round 6 (V2) definition of done

`VAT_STANDARD` is named and scoped precisely (what it computes, what it explicitly doesn't). The finding that it requires no change to `calculate_invoice`'s core steps is verified against a hand-worked example, not asserted. The `regime` parameter's role is stated honestly: present in the signature per Round 4's contract, currently inert for the core computation, reserved for a future regime that might need it. `present_vat` is defined, symmetric with `split_gst`, and confirmed to need no allocation logic. Two new test vectors exist, plus a stated (not hand-duplicated) plan for confirming the unchanged vectors hold under the new regime too via a parameterized test. `database-schema-v2.md` and `application-architecture-v2.md`'s two deliberately-deferred enum/CHECK widenings are filled in as amendments to those locked documents, not left dangling. Round 7 (implementation) writes `domain/tax_regime.rs`, adds the `VatStandard` match arm (a no-op alongside `InGst` for the core algorithm), implements `present_vat`, and turns §4 into `#[test]` functions plus the parameterized cross-regime property test.
