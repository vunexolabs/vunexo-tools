---
status: locked
round: 6
---

# Vunexo Expense Manager — Calculation Engine (Round 6)

Builds on Rounds 1–5. Much lighter than Billing's calculation engine (`docs/vunexo-billing/calculation-engine.md`) because this domain has no line-item math, no discount allocation, and no multi-tax split — an expense's `amount_minor`/`tax_amount_minor` are entered directly by the user, not derived from quantity × rate.

## 1. Money representation — decided

`MinorUnits(i64)`, same newtype Billing uses, storing integer minor currency units. No rounding rule is needed for expense entry itself: a user-entered amount is already an integer number of minor units by construction (the UI parses "₹1,234.50" into `123450` minor units and never performs a floating-point calculation on it).

## 2. The one place summation happens: reports

Every report (`generate_category_summary`, `generate_period_summary`, `generate_deductible_summary`, `generate_tax_itc_summary`, `generate_top_vendors`) is a `SUM(amount_minor)` / `SUM(tax_amount_minor)` grouped by category/period/deductible-flag/itc-flag/vendor, computed in SQL (per Round 4's `report_repository` discipline — aggregation in SQL, not summed in a Rust loop). Summing integers is exact; no rounding rule is needed for this step either, unlike Billing's line-item tax math.

## 3. Deductible / non-deductible summary

`SUM(amount_minor) WHERE deductible = 1` and `SUM(amount_minor) WHERE deductible = 0`, grouped by whatever period/category filter the user picked. Reads `expenses.deductible` (the expense's own stored flag, per Round 3 — never re-derives it from the category's current `default_deductible`).

## 4. Tax / ITC summary

Two independent sums, per Round 1/3's "tax paid and ITC-eligibility are separate facts":

- **Tax paid total**: `SUM(tax_amount_minor)` over the filtered range, regardless of ITC flag.
- **ITC-eligible total**: `SUM(tax_amount_minor) WHERE itc_eligible = 1` — the full tax amount of ITC-flagged expenses, since V1 has no partial-ITC-amount column (Round 3 §6).

The report presents both numbers side by side, with the same disclosure Round 1 locked into the product spec: this is what the user recorded, not a statutory determination.

## 5. Top vendors

`SUM(amount_minor) GROUP BY vendor_id ORDER BY SUM(amount_minor) DESC LIMIT N`, reading `vendor_name_snapshot` for display (so a vendor renamed after some expenses were recorded still shows a coherent ranking using whatever name was current at each expense's creation — if that produces split rows for a renamed vendor, that's the correct, historically-accurate behavior per Round 1's immutability principle, not a bug to "fix" by grouping on the live name instead).

## 6. Preconditions — what these reports do not do

- Do not compute or infer tax amounts — always reads the stored `tax_amount_minor`.
- Do not determine legal deductibility or statutory ITC eligibility — always reads the stored flags.
- Do not convert currency — V1 is single-currency (Round 3 §5).

## 7. Test vectors

- Three expenses in one category, amounts 10000/25000/5000 minor units → category summary total 40000.
- One expense `deductible=1` amount 10000, one `deductible=0` amount 5000 → deductible summary shows 10000/5000, not 15000/0 or any recomputation from category defaults.
- One expense `itc_eligible=1` tax_amount 1800, one `itc_eligible=0` tax_amount 900 → tax-paid total 2700, ITC-eligible total 1800.
- Vendor renamed after 2 of its 3 expenses were recorded (with `vendor_name_snapshot` fixed at old name for those 2) → top-vendors report shows two rows for that vendor's two name snapshots, each with its own partial total — not one merged row under the current name.

## Round 6 definition of done

- Every report in Round 4/5 has its exact SQL aggregation shape specified above.
- No rounding-rule ambiguity remains — user-entered amounts are integers by construction, and every aggregation is an exact integer sum.
- The deductible/ITC-eligible fields are confirmed read-only-from-snapshot in report logic, consistent with Round 3's schema decision.
