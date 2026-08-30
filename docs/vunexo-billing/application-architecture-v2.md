---
status: locked
round: 4
---

**Amendment (Round 6):** `TaxRegimeCode` (§4a) gains its second variant, `VatStandard`, and the "regime dispatch" paragraph below is confirmed rather than left open — see `calculation-engine-v2.md` for the finding that `VatStandard` needs no core-arithmetic branch at all, only its own presentation function. Explicitly deferred to this moment by the original text ("Round 6 adds the second variant here") — filled in now, not a redesign.

# Vunexo Billing — V2 Application Architecture Deltas (Round 4)

This is an AI context file, same status as `docs/vunexo-billing/application-architecture.md`. It is a **delta** document — everything in that file still applies unless overridden below. Built against `docs/vunexo-billing/user-flows-v2.md` and `docs/vunexo-billing/database-schema-v2.md`; does not relitigate either, and does not decide arithmetic (Round 6) or screens (Round 5) — it decides *shape*: use cases, ports, module boundaries, transaction semantics for everything V2 adds.

**Governing principle carried into this round**: no god service. `QuoteService`, `InvoiceService`, `StatementService`, `ReportService`, `ReminderService`, and the tax-regime dispatch stay separate modules with explicit boundaries, mirroring how V1 already keeps `invoices.rs`/`customers.rs`/`products.rs`/`payments.rs` apart rather than one `DocumentService`. Quote→Invoice conversion is a **domain operation with its own transactional contract** (§4c below), not a `copyQuoteToInvoice()` utility method bolted onto either service.

## Module layout (additions)

```
domain/
├── quote.rs                (Quote, QuoteStatus)
├── quote_line_item.rs        (QuoteLineItem)
├── tax_regime.rs               (TaxRegimeCode, TaxCalculator dispatch — §4a)

application/
├── ports/
│   ├── quote_repository.rs
│   ├── quote_number_sequencer.rs
│   ├── statement_repository.rs
│   └── report_repository.rs
├── quotes.rs                (quote use cases, §2)
├── statements.rs              (GenerateCustomerStatement)
├── reports.rs                   (GenerateSalesReport, GenerateTaxSummaryReport)
└── reminders.rs                   (GenerateReminderMessage — no repository writes at all, see below)

infrastructure/
└── database/
    ├── sqlite_quote_repository.rs
    ├── sqlite_quote_number_sequencer.rs
    ├── sqlite_statement_repository.rs
    └── sqlite_report_repository.rs
```

No new module for reminders in `infrastructure/` — per Round 2/3's locked "no send-tracking" decision, `GenerateReminderMessage` reads an `Invoice` (existing `InvoiceRepository::get`) and `Settings.payment_reminder_template` (existing `SettingsRepository::get`), formats a string, and returns it. There is nothing to persist, so there is no port to add.

## 1. Domain types (additions)

`Quote` mirrors `Invoice` field-for-field per `database-schema-v2.md` §4, plus the lifecycle timestamps `accepted_at`/`declined_at`/`converted_at` and the `valid_until: NaiveDate` (business date, same bucket as `invoice_date`/`due_date` per the existing split) and `source` is **not** a field on `Quote` — the reference lives on `Invoice.source_quote_id` (one-directional, matching §2 of the schema doc).

```rust
// domain/quote.rs
pub enum QuoteStatus { Draft, Issued, Accepted, Declined, Converted, Cancelled }

pub struct Quote {
    pub id: QuoteId,
    pub quote_number: Option<String>,
    pub status: QuoteStatus,
    pub customer_id: Option<CustomerId>,
    pub customer_snapshot: Option<CustomerSnapshot>,
    pub business_snapshot: Option<BusinessSnapshot>,
    pub tax_regime_snapshot: Option<TaxRegimeCode>,
    pub is_interstate: bool,
    pub quote_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub discount: Option<(DiscountType, i64)>,
    pub subtotal: MinorUnits,
    pub discount_amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub total: MinorUnits,
    pub issued_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub declined_at: Option<DateTime<Utc>>,
    pub converted_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
}
```

`Invoice` (existing type) gains one field: `pub source_quote_id: Option<QuoteId>`, and its `tax_regime_snapshot: Option<TaxRegimeCode>` (nullable per the legacy-compatibility rule in §4b below — not `TaxRegimeCode` unwrapped).

## 2. Repository ports (additions)

`QuoteRepository` follows `InvoiceRepository`'s exact shape (`application-architecture.md` §3b) — explicit operations matching use cases one-to-one, no generic `save()`:

```rust
#[async_trait]
pub trait QuoteRepository: Send + Sync {
    async fn create_draft(&self, tx: &mut dyn Transaction, draft: NewQuoteDraft) -> Result<Quote, InfrastructureError>;
    async fn update_draft(&self, tx: &mut dyn Transaction, id: QuoteId, changes: DraftQuoteChanges) -> Result<Quote, InfrastructureError>;
    async fn issue(&self, tx: &mut dyn Transaction, id: QuoteId, issued: IssuedQuoteFields) -> Result<Quote, InfrastructureError>;
    async fn accept(&self, tx: &mut dyn Transaction, id: QuoteId) -> Result<Quote, InfrastructureError>;
    async fn decline(&self, tx: &mut dyn Transaction, id: QuoteId) -> Result<Quote, InfrastructureError>;
    async fn mark_converted(&self, tx: &mut dyn Transaction, id: QuoteId) -> Result<Quote, InfrastructureError>; // sets converted_at + status only — never touches invoices
    async fn cancel(&self, tx: &mut dyn Transaction, id: QuoteId, reason: Option<String>) -> Result<Quote, InfrastructureError>;
    async fn delete_draft(&self, tx: &mut dyn Transaction, id: QuoteId) -> Result<(), InfrastructureError>;

    async fn get(&self, id: QuoteId) -> Result<Option<QuoteWithLineItems>, InfrastructureError>;
    async fn list(&self, filter: QuoteFilter) -> Result<Vec<QuoteSummary>, InfrastructureError>;
}

#[async_trait]
pub trait QuoteNumberSequencer: Send + Sync {
    async fn preview_next(&self, format: &str, at: NaiveDate) -> Result<String, InfrastructureError>;
    async fn issue_next(&self, tx: &mut dyn Transaction, format: &str, at: NaiveDate) -> Result<String, InfrastructureError>;
}
```

`QuoteRepository::mark_converted` deliberately does **not** create the invoice — that write belongs to `InvoiceRepository::create_draft`, called from the same transaction by the `ConvertQuoteToInvoice` use case (§4c), not by the repository reaching sideways into another aggregate's table. Keeping each repository scoped to its own table is what makes the "no god service" principle enforceable at the repository layer too, not just the use-case layer.

**Statements and reports get purpose-built read ports**, same rule as `DashboardRepository` (`application-architecture.md` §3c) — SQL aggregates, never a Rust reduction over pulled rows:

```rust
#[async_trait]
pub trait StatementRepository: Send + Sync {
    async fn customer_statement(&self, customer_id: CustomerId, range: DateRange) -> Result<StatementResult, InfrastructureError>;
}

pub struct StatementResult {
    pub opening_balance: MinorUnits,
    pub entries: Vec<StatementEntry>,     // invoices + payments in range, chronological
    pub closing_balance: MinorUnits,
}

#[async_trait]
pub trait ReportRepository: Send + Sync {
    async fn sales_summary(&self, range: DateRange, group_by: SalesGrouping) -> Result<SalesSummaryResult, InfrastructureError>;
    async fn tax_summary(&self, range: DateRange) -> Result<TaxSummaryResult, InfrastructureError>; // grouped by tax_regime_snapshot, per database-schema-v2.md §7
}
```

`StatementRepository::customer_statement`'s SQLite implementation is exactly the two-query shape `database-schema-v2.md` §7 already specified (opening balance query, then in-range query) — this port exists so that shape has exactly one implementation, not one written fresh inside a use case.

## 3. Use cases (application layer)

**Quotes** (`application/quotes.rs`) — same precondition/transaction discipline as `application/invoices.rs`:

- `CreateDraftQuote` — no required fields; single insert, not transactional.
- `UpdateDraftQuote` — precondition: status = `Draft` (a `ISSUED`+ quote is immutable per the locked flow — this use case rejects with `ApplicationError::Conflict` rather than silently no-op'ing on anything past `Draft`). Runs `calculate_invoice` (§4a) against the *current* `business.tax_regime_code` (see §4c below on Draft regime handling) and persists lines + totals in one transaction.
- `IssueQuote` — preconditions: `customer_id` set, ≥1 line item. One transaction: validate → load customer + business → `QuoteNumberSequencer::issue_next` → `calculate_invoice` → `QuoteRepository::issue` (writes `quote_number`, snapshots incl. `tax_regime_snapshot`, `status = Issued`, `issued_at`). Structurally identical to `IssueInvoice` (`application-architecture.md` §4c) — same rollback-never-burns-a-number guarantee, on the quote counter instead of the invoice counter.
- `AcceptQuote` — precondition: status = `Issued`. Sets `accepted_at`, `status = Accepted`. Single-column write, not transactional. **Does not check `valid_until` server-side** — an expired quote can still be accepted if the business chooses to honor it; `is_expired` is a display badge (per Round 2/3), never a hard block on any transition, matching how V1's `is_overdue` never blocks a payment.
- `DeclineQuote` — precondition: status = `Issued`. Sets `declined_at`, `status = Declined`. Not transactional.
- `CancelQuote` — precondition: status ∈ `{Draft, Issued, Accepted}` (per the locked flow fix). Sets `cancelled_at` + optional `cancel_reason`. Not transactional (single row).
- `ConvertQuoteToInvoice` — precondition: status = `Accepted`. The one two-table transactional use case in this delta — spelled out in full in §4c.
- `DuplicateQuote` — copies customer, line items, quote-level discount, notes, terms into a new `DRAFT` Quote; never copies `accepted_at`/`converted_at`/payments-equivalent state (a Quote has none) or the quote number.
- `DeleteDraftQuote` — precondition: status = `Draft`.

**Statements** (`application/statements.rs`)
- `GenerateCustomerStatement` — thin pass-through to `StatementRepository::customer_statement`; no validation beyond "customer exists," no transaction (read-only).

**Reports** (`application/reports.rs`)
- `GenerateSalesReport`, `GenerateTaxSummaryReport` — same shape, pass-through to `ReportRepository`.

**Reminders** (`application/reminders.rs`)
- `GenerateReminderMessage` — precondition: invoice exists and `is_overdue` (per `database-schema.md` §8's existing predicate — reusing it, not redefining a second "is this invoice reminder-eligible" rule). Reads the invoice + `Settings.payment_reminder_template` (falling back to a built-in `const DEFAULT_REMINDER_TEMPLATE: &str` when `NULL`, per `database-schema-v2.md` §8), substitutes placeholders, returns a `String`. No repository write, no transaction — the whole use case is two reads and a format call.

**Queries** — `GetQuote`, `ListQuotes`, named explicitly per the existing convention (`application-architecture.md` §4's "Queries" bullet).

## 4. New/changed cross-cutting mechanics

### 4a. Tax-regime dispatch — the calculation-engine boundary

Round 3 ruled out a `tax_regimes` configuration table; the corresponding application-layer decision is that regime *behavior* is a small, closed Rust dispatch, not a runtime-configurable strategy registry:

```rust
// domain/tax_regime.rs
pub enum TaxRegimeCode {
    InGst,
    VatStandard, // Round 6 — see calculation-engine-v2.md; CHECK-constraint widened in database-schema-v2.md §5
}

// domain/calculation.rs — existing signature gains one parameter
pub fn calculate_invoice(
    input: InvoiceCalculationInput,
    regime: TaxRegimeCode,
) -> InvoiceCalculationResult;
```

`calculate_invoice` stays pure (§4a of the existing doc, unchanged rule) — `regime` is dispatched internally via a `match`, not a trait object looked up from configuration. `InvoiceCalculationResult`/`LineItemResult` (existing shape) are **not** widened with regime-specific fields — V1 already established the right pattern here and V2 keeps it: `tax_amount_minor` is regime-agnostic (just "how much tax"), and the regime-specific *display* breakdown (GST's CGST/SGST/IGST split today; whatever the second regime needs) is derived separately at render/report time, the same way `domain::invoice_pdf::split_gst` already works, not stored and not part of the calculation contract. Round 6 adds the second regime's equivalent `split_*`-style function alongside its arithmetic; this round only fixes that the *pattern* — compute the total purely, derive the presentation separately — extends unchanged.

### 4b. Legacy `NULL` tax_regime_snapshot — normalized once, not per call site

Per the review that required this to be explicit rather than left to scattered repository code: **`NULL` is a legacy-compatibility state, valid only on invoices issued before this schema column existed.** Normalization happens at exactly one boundary — `SqliteInvoiceRepository::get`/`list`'s row-mapping code — not in every use case or every UI read:

```rust
// infrastructure/database/sqlite_invoice_repository.rs, row → domain mapping
let tax_regime = row.tax_regime_snapshot
    .map(TaxRegimeCode::from_db_code)
    .unwrap_or(TaxRegimeCode::InGst); // legacy pre-V2 invoice — IN_GST is the only regime that existed when it was issued
```

Every V2-issued document (`IssueInvoice`, `IssueQuote`) writes a non-`NULL` `tax_regime_snapshot` — enforced by the same invariant table pattern `application-architecture.md` §3b already uses (`issue()` writes it as part of the same fields it already guarantees `NOT NULL` post-issue). `NULL` reaching the mapping layer past V2's ship date means "this row predates V2," a fact about *when* the row was written, not an ambiguous state a caller has to reason about each time it reads one.

### 4c. `ConvertQuoteToInvoice`, spelled out

The single most important transactional contract this round adds — named explicitly because the failure modes are asymmetric and dangerous if half-applied:

```
BEGIN
  load quote (must be status = Accepted, else Conflict)
  build NewInvoiceDraft from the quote's customer, line items, quote-level discount, notes, terms
                                                    ── copied, per the snapshot-independence rule in user-flows-v2.md §2
  InvoiceRepository::create_draft(tx, ...)          ── new row, status = Draft, source_quote_id = quote.id
  QuoteRepository::mark_converted(tx, quote.id)     ── status = Converted, converted_at = now
COMMIT
```

**The contract this locks, stated as the two states that must never occur:**
- Never `quote.status = Converted` with no corresponding invoice row (an orphaned conversion).
- Never `quote.status = Accepted` with an invoice row already pointing at it via `source_quote_id` (a conversion that "half-happened").

Both are prevented by the same mechanism `IssueInvoice` already relies on (`application-architecture.md` §4c): every write in the sequence above runs against one shared `Transaction`; any failure rolls back everything, including the invoice insert. The database-level partial unique index on `invoices.source_quote_id` (`database-schema-v2.md` §9) is the second, independent layer of defense — it makes "two invoices from one quote" impossible even if a future bug somehow ran the sequence twice outside a transaction, but it is a backstop, not a substitute for the transaction boundary itself.

Unlike `IssueInvoice`, this use case does **not** call `InvoiceNumberSequencer` — the resulting invoice is a `Draft` (per `user-flows-v2.md` §3: "user lands on the new Draft Invoice"), and numbering only happens when *that* invoice is later, separately, issued through the ordinary `IssueInvoice` use case. Conflating the two would mean a converted-but-never-issued invoice silently burns a number, which is exactly the waste `IssueInvoice`'s own transaction was designed to prevent.

### 4d. Draft tax-regime handling — resolved, no new column

The open question flagged during Round 3 review — "current business regime ≠ Draft's regime ≠ issued snapshot" — resolves without a schema change, by extending a rule V1 already established rather than inventing a new one: **a Draft has no persisted regime of its own.** `UpdateDraftInvoice`/`UpdateDraftQuote` always calls `calculate_invoice` against `business.tax_regime_code` **as read at the moment of that save** — the same "a Draft always reflects its last save, not a continuously-live view" principle `database-schema.md` §4 already uses to justify recomputing and persisting Draft totals on every "Save Draft," now extended to cover which regime those totals are computed under.

**Consequence, stated plainly**: a Draft left open (in the UI, unsaved) across a business's regime switch shows stale totals until the next save — not a bug, the same staleness a Draft already has with respect to a mid-edit customer address change today. The line items' own frozen fields (`tax_rate_basis_points`, per-line discount) never change; only the invoice/quote-level presentation and computed totals do, and only upon the next explicit save. This is a UX nicety for Round 5 to consider (e.g. a "recalculated under your new tax settings" toast), not a data-model gap — nothing here requires a Draft-scoped regime column, so Round 3's schema is confirmed sufficient without revisiting it.

## 5. Tauri command surface (additions)

`create_draft_quote`, `update_draft_quote`, `issue_quote`, `accept_quote`, `decline_quote`, `cancel_quote`, `convert_quote_to_invoice`, `duplicate_quote`, `delete_draft_quote`, `get_quote`, `list_quotes`, `generate_customer_statement`, `generate_sales_report`, `generate_tax_summary_report`, `generate_reminder_message`. `update_business` and `update_settings` (existing commands) gain new optional fields in their payload (`tax_regime_code`, `quote_number_format`/`payment_reminder_template` respectively) — no new commands needed for those, since they're field additions to an existing use case's input, not new use cases.

## 6. Error handling

Unchanged — `InfrastructureError`/`ApplicationError` (`application-architecture.md` §6) cover every new use case without extension. The one new pattern worth naming: `ConvertQuoteToInvoice`'s precondition failure ("quote is not Accepted") is `ApplicationError::Conflict`, matching how `CancelInvoice`'s precondition failure is already categorized — a state-machine violation, not a validation error on user input.

## 7. Transaction boundaries (additions)

- `IssueQuote`: number generation + snapshot (incl. `tax_regime_snapshot`) + totals + status write — same shape and same "rollback never burns a number" guarantee as `IssueInvoice`.
- `UpdateDraftQuote`: line items + recalculated totals written together — same shape as `UpdateDraftInvoice`.
- `ConvertQuoteToInvoice`: the new invoice-draft insert + the quote's `mark_converted` write — see §4c. **This is the only V2 use case that writes to two different aggregate tables in one transaction** — worth flagging because it's the one place a future refactor might be tempted to split it into "two separate calls the frontend sequences," which would reopen exactly the half-applied-conversion risk §4c exists to prevent.
- `DeleteDraftQuote`: the quote delete + its cascaded line items, same pattern as `DeleteDraftInvoice`.

## 8. Verification guide (additions)

| # | Fix | Verify now (design review) | Test to write in Round 7 | What it catches if the fix were fake |
|---|---|---|---|---|
| 1 | `ConvertQuoteToInvoice` atomicity (§4c) | Both writes happen inside one `Transaction`; no code path calls `InvoiceRepository::create_draft` and `QuoteRepository::mark_converted` as two independent top-level calls. | Force a failure between the two writes (fault-inject after the invoice insert, before `mark_converted`); assert after rollback the quote is still `Accepted` **and** no invoice row with that `source_quote_id` exists. Then also test the reverse injection point (fault after `mark_converted`, before commit) to confirm the invoice insert rolls back too. | A "looks atomic" implementation that actually does two separate commits could leave a quote stuck `Accepted` forever with a dangling invoice, or `Converted` with no invoice at all — exactly the two forbidden states §4c names. |
| 2 | `invoices.source_quote_id` uniqueness (`database-schema-v2.md` §9) as backstop | The partial unique index exists in the migration; `ConvertQuoteToInvoice`'s use-case code does not itself re-check "does this quote already have an invoice" before inserting (redundant application-layer check would mask what the DB constraint is actually for). | Attempt to call `InvoiceRepository::create_draft` twice with the same `source_quote_id` outside the normal use case (simulating a hypothetical bug); assert the second insert fails with `InfrastructureError::ConstraintViolation`. | Confirms the "exactly once" invariant survives even a bug that bypasses the use-case layer, not just normal-path testing. |
| 3 | Legacy `tax_regime_snapshot` normalization (§4b) | The `unwrap_or(TaxRegimeCode::InGst)` fallback exists in exactly one place (`SqliteInvoiceRepository`'s row mapping) — grep for `tax_regime_snapshot` across `application/` and confirm no use case re-implements the fallback. | Seed a fixture invoice row with `tax_regime_snapshot = NULL` (simulating a pre-V2 row); load it through the repository; assert the domain `Invoice.tax_regime_snapshot` reads as `Some(TaxRegimeCode::InGst)`, never `None` past this boundary. | If normalization is scattered, a future call site that forgets it would treat a legacy invoice as regime-less, breaking any report/statement logic that switches on regime. |
| 4 | Draft regime recalculation (§4d) | `UpdateDraftInvoice`/`UpdateDraftQuote` read `business.tax_regime_code` fresh on every call — no caching of "the regime this draft was created under." | Create a Draft under regime A, switch `business.tax_regime_code` to a fixture regime B, call `UpdateDraftInvoice` again (e.g. re-saving with an unchanged line item), assert the recalculated totals reflect regime B's rules, not regime A's. | Confirms the "no Draft-scoped regime" design decision actually holds in the implementation, not just on paper — a cached-regime bug would silently keep computing under the stale regime. |
| 5 | Statement balance reconciliation (`database-schema-v2.md` §7) | `StatementRepository::customer_statement`'s SQL matches the two-query shape in the schema doc exactly — opening balance excludes `Draft`/`Cancelled` by construction, not by an extra `WHERE` a future edit could drop. | Property test: for a fixture customer with random invoices/payments across three consecutive date ranges, assert `closing_balance(range N) == opening_balance(range N+1)` for every adjacent pair. | This is the exact invariant flagged during Round 3 review as worth a test — confirms the query shape is actually reconciling, not just plausible-looking SQL. |
| 6 | No god service (governing principle) | `application/quotes.rs`, `application/statements.rs`, `application/reports.rs`, `application/reminders.rs` each depend only on the repository ports their own use cases need — none of them import `InvoiceRepository` and `QuoteRepository` and `StatementRepository` all at once inside a single struct. | A CI-level static check (per-file import list) as a cheap first line of defense, same style as the existing "grep for `invoice` inside the customer/product repository files" check (`application-architecture.md` §8, row 8). | The whole point of keeping these modules separate is defeated if a later refactor quietly merges them into one `DocumentService` "for convenience" — this is the regression test that catches that the moment it happens. |

## Round 4 (V2) definition of done

Every use case named above has a home, a layer, and a stated pre/postcondition; `QuoteRepository`/`QuoteNumberSequencer`/`StatementRepository`/`ReportRepository` are named with representative signatures; the tax-regime dispatch boundary is fixed (a closed Rust enum matched in `calculate_invoice`, not a configuration table); the legacy-`NULL` normalization point is pinned to exactly one location; `ConvertQuoteToInvoice`'s atomicity contract is spelled out with its two forbidden states named explicitly; Draft regime handling is resolved without a schema change. Round 5 (UI/UX) designs screens against this command surface; Round 6 (calculation engine) names the second tax regime and implements `calculate_invoice`'s regime-dispatched arithmetic; Round 7 (implementation) writes the Rust code following this layout and the test list in §8.
