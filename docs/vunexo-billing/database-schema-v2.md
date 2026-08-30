---
status: locked
round: 3
---

**Amendment (Round 6):** §5 and §9's `business.tax_regime_code` `CHECK` constraint is widened from `IN_GST` only to `IN_GST, VAT_STANDARD`, per Round 6 naming the second regime. This was explicitly deferred to this exact moment by the original text below ("widened when Round 4/6 names a second regime") — filled in now, not a scope reopening.

# Vunexo Billing — V2 Database Schema Deltas (Round 3)

This is an AI context file, same status as `docs/vunexo-billing/database-schema.md`. Read it before writing any V2 migration, repository, or domain code. It is designed against the locked flows in `docs/vunexo-billing/user-flows-v2.md` and `.ai/product-v2.md` — it does not relitigate them, only implements them. It is a **delta** document: everything in `database-schema.md` still applies unless explicitly overridden below; this file only covers what V2 adds or changes.

**Governing constraint for this round** (carried over verbatim from the review that unblocked it): don't let the schema become more generic than the product requires, especially around tax. A regime's *behavior* — how its rate/split/rounding works — lives in application/calculation-engine code (Round 4/6), matched against a small, named `tax_regime_code`. This document does **not** introduce `tax_regimes`, `tax_regime_rules`, or any other configurable-rules table. If V2 ever needs a third or fourth regime badly enough to justify a real rules engine, that's a future ADR, not a default extrapolation from "we support two."

## 1. New domain entities

- **Quote** — a transactional root, structurally a near-mirror of `invoices`: same snapshot discipline, same discount/tax computation shape, its own status lifecycle (§3).
- **QuoteLineItem** — belongs to exactly one quote; carries its own frozen product snapshot, identical shape to `invoice_line_items`.
- **QuoteNumberCounter** — its own counter table, parallel to `invoice_number_counters`, not a merge into a generalized "document counter." Reasoning: `invoice_number_counters` is a real table with real production data as of `v1.0.0` — renaming/generalizing it into a `document_number_counters(document_type, scope_key, ...)` shape would require a migration that rewrites an existing table's primary key structure for a benefit (avoiding one extra two-column table) too small to justify the risk to an already-shipped numbering sequence. A parallel table costs four lines of SQL and zero migration risk.

Deliberately **not** modeled as tables in V2, same discipline as V1 §1: no `statements` table, no `reports`/`report_items` table (§7 — both are read models over existing data), no `tax_regimes` table (see governing constraint above), no quote-status-history/audit-log table (state-transition timestamps live directly on the `quotes` row, same pattern V1 already uses for `issued_at`/`cancelled_at`).

## 2. Entity relationships (additions)

```
customers ──┐
            │ (nullable while draft; required to issue)
            ▼
        quotes ──────────────────────────────────────┐
            │  1:N                                   │
            ▼                                        ▼
    quote_line_items ◀──────────────────────── product_id (nullable ref)

        quotes
            │ 0..1  (source_quote_id, nullable, UNIQUE when set)
            ▼
        invoices        ── an invoice optionally traces back to the quote it was converted from
```

`invoices.source_quote_id` is a **display/traceability reference**, per Round 2 — never a live data dependency. Converting copies data into the new invoice row exactly the way duplication already does in V1; nothing about the resulting invoice is ever reconstructed by joining back to the quote.

## 3. Lifecycle / state rules — Quotes

- `quotes.status ∈ {DRAFT, ISSUED, ACCEPTED, DECLINED, CONVERTED, CANCELLED}` — a **stored** column. `EXPIRED` is never stored, same discipline as `OVERDUE` on invoices — computed at query time: `is_expired = valid_until < today AND status = 'ISSUED'` (per `user-flows-v2.md` §2).
- `DRAFT → ISSUED`: sets `quote_number`, `issued_at`, and the customer/business/tax-regime snapshot columns (§4) — same trigger condition as invoices.
- `ISSUED → ACCEPTED`: sets `accepted_at`.
- `ISSUED → DECLINED`: sets `declined_at`.
- `ACCEPTED → CONVERTED`: sets `converted_at`; simultaneously (same transaction) creates the new `DRAFT` invoice with `source_quote_id` pointing back. This is the one V2 transition that writes to two tables atomically.
- `DRAFT, ISSUED, ACCEPTED → CANCELLED`: sets `cancelled_at` and optional `cancel_reason` — identical shape to invoice cancellation. Per the locked flow fix, `ACCEPTED` is explicitly included here, not just `DRAFT`/`ISSUED`.
- `CONVERTED` and `CANCELLED` are both terminal: no further transitions, no edits.

| State | Invariant |
|---|---|
| `DRAFT` | `quote_number`, `issued_at`, every snapshot column are `NULL`. |
| `ISSUED` / `ACCEPTED` / `DECLINED` / `CONVERTED` | `quote_number`, `issued_at`, `customer_id`, every snapshot column are `NOT NULL`. |
| `ACCEPTED` | `accepted_at` is `NOT NULL`. |
| `DECLINED` | `declined_at` is `NOT NULL`. |
| `CONVERTED` | `converted_at` is `NOT NULL`; exactly one row in `invoices` has `source_quote_id` equal to this quote's `id` (enforced by the partial unique index in §8, not a trigger). |
| `CANCELLED` | `cancelled_at` is `NOT NULL`; no further edits. |

These are service-layer invariants (Round 4), not DB triggers — same "trigger machinery is unnecessary weight" call V1 already made for its own invariants (`database-schema.md` §3).

## 4. Snapshot strategy — Quotes

Identical mechanism to invoices (`database-schema.md` §4): denormalized nullable columns directly on `quotes`, frozen at Issue, `NULL` until then. Customer snapshot columns are the same five fields (`name`/`phone`/`email`/`address`/`gstin`); business snapshot columns are the same eight fields V1 already freezes onto invoices, **plus** the new `tax_regime_snapshot` (§5).

`quote_line_items` mirrors `invoice_line_items` exactly, field-for-field, including the discount/taxable-amount breakdown columns (`database-schema.md` §4's worked example applies unchanged) — the one rename is `invoice_discount_amount_minor` → `quote_discount_amount_minor`, since it's now a line's allocated share of a *quote*-level discount, not an invoice-level one. Computed totals are persisted at save time, same as invoices, same rationale (a historical quote must keep showing the numbers it showed the day it was issued, even if a future app version changes rounding rules).

## 5. Tax regime representation — the cross-cutting V2 change

This is the one schema change that touches both `business` and `invoices`, not just the new Quote tables.

- **`business.tax_regime_code TEXT NOT NULL DEFAULT 'IN_GST'`** — the business's *current*, forward-looking tax configuration (per `user-flows-v2.md` §1: a single business-level setting, changed like an address, never retroactive). No foreign key to a `tax_regimes` table — deliberately a plain code matched against a small enum the application/calculation-engine code knows about (see governing constraint above). **`CHECK (tax_regime_code IN ('IN_GST', 'VAT_STANDARD'))`** — widened per the Round 6 amendment at the top of this document; `VAT_STANDARD` is Round 6's name for a deliberately narrow flat-rate VAT model (per-line rate, no jurisdiction/exemption/reverse-charge logic — see `calculation-engine-v2.md`), not a claim of covering every country's VAT rules.
- **`invoices.tax_regime_snapshot TEXT`** — added to the existing `invoices` table, nullable, frozen at Issue exactly like every other snapshot column (`NULL` while `DRAFT`, `NOT NULL` from `ISSUED` onward — added to the existing invariant table in `database-schema.md` §3). This is the column that makes the critical invariant enforceable: **a later change to `business.tax_regime_code` must never make an already-issued invoice appear to have been calculated under a different regime.** Without this column, an issued invoice's effective tax regime would have to be inferred from "whatever `business.tax_regime_code` says today," which breaks the instant a business switches regimes — this column is what prevents that.
- **`quotes.tax_regime_snapshot TEXT`** — same column, same rule, on the new `quotes` table.
- **`invoices.is_interstate`** (existing V1 column) and its new **`quotes.is_interstate`** twin stay exactly as V1 defined them: meaningful only when the document's `tax_regime_snapshot = 'IN_GST'`. **`VAT_STANDARD` has no equivalent column** (Round 6 amendment) — it has no split to decide (§ below), so the flag simply isn't read for a `VAT_STANDARD` document rather than being repurposed for a different meaning.
- **`tax_rates`** (existing V1 table) is **not** given a `tax_regime_code` column in this round. Per `user-flows-v2.md` §1, "regime-scoped tax rates" is named as a UX behavior, but nothing in the locked flows requires a business to hold rates for two regimes *simultaneously* — a business has exactly one active `tax_regime_code` at a time, so today's flat `tax_rates` list is unambiguous as long as it's understood to belong to whichever regime is currently active. If Round 4/6 concludes a business must keep historical rates from a regime it's since switched away from (for editing an old draft, say), that's the trigger to revisit this — not assumed here.

## 6. Numbering — Quotes

Same rules as invoice numbering (`database-schema.md` §7), on the new parallel table:

- `quote_number_counters(scope_key TEXT PRIMARY KEY, last_value INTEGER NOT NULL DEFAULT 0)`.
- A fresh Draft Quote previews its next number without incrementing; the counter increments and the number is written atomically at Issue.
- `quotes.quote_number` is `UNIQUE` whenever set, via the same partial-unique-index pattern.
- `settings.quote_number_format` (new column, e.g. default `'QUO-{year}-{seq:04d}'`) becomes read-only once the first quote has been issued — same rule, same rationale, as `settings.invoice_number_format`, checked independently (a business could issue its first invoice and first quote in either order; the two formats lock independently of each other).
- A converted quote's `quote_number` is untouched by conversion — it keeps it permanently. The resulting invoice gets a normal freshly-generated `invoice_number` at its own Issue, drawn from `invoice_number_counters` exactly like any other Draft.

## 7. Statements & reports — query model, not tables

Per the locked constraint that these are read models: no persisted rows, only SQL over `customers`/`invoices`/`payments` (and `invoice_line_items` for the sales-by-product report). The correctness-critical piece is the statement's opening balance — specified precisely here since a wrong opening balance is a wrong document handed to a customer:

```sql
-- Opening balance as of range_start, for one customer:
opening_balance_minor =
    (SELECT COALESCE(SUM(total_minor), 0) FROM invoices
     WHERE customer_id = :customer_id
       AND status != 'CANCELLED'
       AND issued_at < :range_start)
  -
    (SELECT COALESCE(SUM(p.amount_minor), 0) FROM payments p
     JOIN invoices i ON p.invoice_id = i.id
     WHERE i.customer_id = :customer_id
       AND p.paid_on < :range_start)

-- Activity within [range_start, range_end):
invoices_in_range: same WHERE, with issued_at >= :range_start AND issued_at < :range_end
payments_in_range: same JOIN, with p.paid_on >= :range_start AND p.paid_on < :range_end

closing_balance_minor = opening_balance_minor
                       + SUM(invoices_in_range.total_minor)
                       - SUM(payments_in_range.amount_minor)
```

`DRAFT` invoices are excluded by construction (`issued_at` is `NULL` for a Draft, so it can never satisfy `issued_at < :range_start` or fall inside the range) — nothing not yet issued was ever "owed." `CANCELLED` invoices are excluded explicitly, same reasoning as V1's balance/status logic. This query shape is what makes closing balance for period N equal opening balance for period N+1 by construction — worth a unit test asserting exactly that once Round 7 implements it.

**Sales/tax summary reports** aggregate `invoice_line_items`/`invoices` by period and by `tax_regime_snapshot` (per `user-flows-v2.md` §5's mixed-regime edge case) — grouped, not silently summed across regimes when a report's date range spans a regime switch.

## 8. Payment reminders — one new column, no new table

Per Round 2's explicit "no sent-history tracking" decision: nothing to persist about an individual reminder. The one piece of state that *is* configuration, not history: **`settings.payment_reminder_template TEXT`**, nullable — `NULL` means "use the built-in default template," an application-code constant, not a row that has to exist before the feature works.

## 9. Indexes & constraints (additions)

- `quotes`: index on `customer_id`, index on `status`, index on `quote_date`, index on `valid_until` (drives the `is_expired` query, mirrors `idx_invoices_due_date`), partial unique index on `quote_number`.
- `quote_line_items`: index on `quote_id`, index on `product_id`.
- `invoices`: **new** partial unique index on `source_quote_id` (`WHERE source_quote_id IS NOT NULL`) — this is the constraint that actually enforces "a quote converts to at most one invoice," not just application-layer discipline.
- Foreign keys: `quotes.customer_id → customers.id` **RESTRICT** (mirrors `invoices.customer_id`); `quote_line_items.quote_id → quotes.id` **CASCADE** (line items owned by their quote); `quote_line_items.product_id → products.id` **RESTRICT**; `quote_line_items.tax_rate_id → tax_rates.id` **SET NULL**; `invoices.source_quote_id → quotes.id` **RESTRICT** (a quote that produced an invoice is never deleted anyway — see §10 — but RESTRICT is the same defensive backstop pattern V1 already uses throughout).
- `CHECK` constraints: `quotes.status IN ('DRAFT','ISSUED','ACCEPTED','DECLINED','CONVERTED','CANCELLED')`, `quotes.discount_type IN ('AMOUNT','PERCENTAGE')`, `quote_line_items.quantity_thousandths > 0`, `business.tax_regime_code IN ('IN_GST', 'VAT_STANDARD')` (Round 6 amendment, per §5).

## 10. Delete / archive behavior (additions)

| Entity | Zero references | Referenced |
|---|---|---|
| Quote (`DRAFT`) | hard delete (cascades to its line items) | — |
| Quote (`ISSUED`/`ACCEPTED`/`DECLINED`) | never deleted | cancel instead |
| Quote (`CONVERTED`) | never deleted | terminal — has produced an invoice |
| Quote (`CANCELLED`) | never deleted | terminal |

Same discipline as V1's invoice table: only a never-issued `DRAFT` can be hard-deleted; every other state is permanent history.

## 11. Migration strategy

V1 shipped `0001_init.sql` as the real, live schema (`v1.0.0` is a published release with real installs) — the "replace the placeholder migration" move `database-schema.md` §12 used is no longer available. V2 ships as a genuinely additive `0002_v2_quotes_and_tax_regime.sql`: `CREATE TABLE quotes`, `CREATE TABLE quote_line_items`, `CREATE TABLE quote_number_counters`, `ALTER TABLE invoices ADD COLUMN source_quote_id`, `ALTER TABLE invoices ADD COLUMN tax_regime_snapshot`, `ALTER TABLE business ADD COLUMN tax_regime_code TEXT NOT NULL DEFAULT 'IN_GST'`, `ALTER TABLE settings ADD COLUMN quote_number_format` / `payment_reminder_template`. Every existing V1 row backfills correctly under these defaults with no data migration needed: existing businesses get `tax_regime_code = 'IN_GST'` (matches their actual, only-ever regime), existing invoices' `tax_regime_snapshot` stays `NULL` (they predate the concept — the application layer should treat a `NULL` snapshot on an already-issued invoice as "assume `IN_GST`," the only regime that existed when it was issued, rather than treating `NULL` as an error state).

## Round 3 (V2) definition of done

Every table, column, relationship, index, and delete/archive rule needed by the locked `user-flows-v2.md` flows is specified above, additive to the shipped V1 schema, with the same exact-integer money/quantity/tax discipline. The tax-regime representation makes the "no retroactive recalculation" invariant enforceable at the schema level, not just a UI convention. Round 4 (application architecture) designs the repository/service layer against this delta, including the two-table-atomic Quote→Invoice conversion; Round 6 (calculation engine) names the second tax regime and fills in its arithmetic.
