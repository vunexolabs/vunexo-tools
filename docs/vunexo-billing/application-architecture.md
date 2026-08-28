---
status: locked
round: 4
---

# Vunexo Billing — Application Architecture (Round 4)

This is an AI context file. It fills in the `commands → application → domain → infrastructure` layering locked in `docs/vunexo-billing/architecture.md` with concrete use cases, ports, and module boundaries, built against the locked flows (`user-flows.md`) and schema (`database-schema.md`). It does not implement anything (that's Round 7) and does not decide calculation arithmetic (that's Round 6) — it decides *shape*: what the use cases are, what they depend on, what's transactional, and how errors cross layer boundaries.

## Revised dependency graph

```
                 FRONTEND
                    │
                    ▼
             Tauri Commands            (transport translation only — §5)
                    │
                    ▼
             APPLICATION
          ┌─────────┴─────────┐
          │                   │
       Use Cases          Repository Ports         (incl. TransactionManager — §3a)
          │                   │
          ▼                   │
        DOMAIN                │                    (pure calculation — §2, §4)
          │                   │
          └─────────┬─────────┘
                    ▼
             INFRASTRUCTURE
          ┌─────────┼─────────┐
          │         │         │
        SQLite   Filesystem    PDF
          │
          ▼
       SQLx/SQLite
```

```
Domain          ❌ Tauri   ❌ SQLx   ❌ filesystem
Application     ❌ SQLx    ❌ Tauri
Infrastructure  ✅ SQLx    ✅ filesystem   ✅ Tauri integration where appropriate
```

## 1. Module layout

`application/`, `domain/`, and `infrastructure/database/` (all currently single stub files from Round 1) become directories grouped by aggregate, mirroring the schema's master-data / transactional split:

```
domain/
├── mod.rs                 (re-exports)
├── business.rs             (Business)
├── settings.rs             (Settings)
├── tax_rate.rs              (TaxRate)
├── customer.rs               (Customer, CustomerStatus)
├── product.rs                 (Product, ProductStatus)
├── invoice.rs                   (Invoice, InvoiceStatus, DiscountType)
├── invoice_line_item.rs          (InvoiceLineItem)
├── payment.rs                     (Payment, PaymentMethod)
├── calculation.rs                  (CalculateInvoice + its input/output types — §4a)
└── money.rs                         (MinorUnits newtype — arithmetic ops land in Round 6)

application/
├── mod.rs
├── ports/                  (repository traits — the "Ports / Interfaces" from architecture.md)
│   ├── mod.rs
│   ├── transaction.rs        (TransactionManager — §3a)
│   ├── infrastructure_error.rs (InfrastructureError — §6)
│   ├── business_repository.rs
│   ├── settings_repository.rs
│   ├── tax_rate_repository.rs
│   ├── customer_repository.rs
│   ├── product_repository.rs
│   ├── invoice_repository.rs
│   ├── payment_repository.rs
│   ├── invoice_number_sequencer.rs
│   └── dashboard_repository.rs   (§4d)
├── invoices.rs              (invoice use cases, §4)
├── customers.rs              (customer use cases, §4)
├── products.rs                 (product use cases, §4)
├── payments.rs                   (payment use cases, §4)
├── dashboard.rs                    (GetDashboardMetrics)
├── backup.rs                        (BackupDatabase, RestoreBackup — §4e)
└── error.rs                           (ApplicationError, §6)

infrastructure/
├── database/
│   ├── mod.rs               (pool + migrations — already exists from Round 1)
│   ├── transaction.rs        (SQLx-backed TransactionManager impl)
│   ├── sqlite_business_repository.rs
│   ├── sqlite_settings_repository.rs
│   ├── sqlite_tax_rate_repository.rs
│   ├── sqlite_customer_repository.rs
│   ├── sqlite_product_repository.rs
│   ├── sqlite_invoice_repository.rs
│   ├── sqlite_payment_repository.rs
│   ├── sqlite_invoice_number_sequencer.rs
│   └── sqlite_dashboard_repository.rs
├── filesystem/              (backup/restore archive handling — §4e)
└── pdf/                     (Round 5/6 concern, unchanged shape)
```

Each `sqlite_*_repository.rs` implements the matching trait from `application/ports/`, per the dependency-inversion rule already locked in Round 1: `application` owns the trait, `infrastructure` owns the implementation.

## 2. Domain types

Plain data + enums, no I/O, no Tauri, no SQLx (per the Round 1 zero-dependency rule) — the same shapes the schema stores, but as Rust types rather than rows. Date/time types are fixed now rather than left as a placeholder, split by what they actually represent:

- **Business dates** (calendar dates a person picks or reasons about, no time-of-day meaning): `invoice_date`, `due_date`, `paid_on` → `chrono::NaiveDate`.
- **Timestamps** (a specific instant something happened, stored/compared, shown to the user converted to local time by the UI — not by `domain`/`application`): `issued_at`, `cancelled_at`, `created_at`, `updated_at` → `chrono::DateTime<Utc>`.

```rust
// domain/money.rs
pub struct MinorUnits(pub i64); // arithmetic ops added in Round 6

// domain/invoice.rs
pub enum InvoiceStatus { Draft, Issued, PartiallyPaid, Paid, Cancelled }
pub enum DiscountType { Amount, Percentage }

pub struct Invoice {
    pub id: InvoiceId,
    pub invoice_number: Option<String>,
    pub invoice_number_is_custom: bool,
    pub status: InvoiceStatus,
    pub customer_id: Option<CustomerId>,
    pub customer_snapshot: Option<CustomerSnapshot>,
    pub business_snapshot: Option<BusinessSnapshot>,
    pub is_interstate: bool,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub discount: Option<(DiscountType, i64)>,
    pub subtotal: MinorUnits,
    pub discount_amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub total: MinorUnits,
    pub issued_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
}
```

`Customer`, `Product`, `TaxRate`, `Business`, `Settings`, `InvoiceLineItem`, `Payment` follow the same pattern: one field per schema column, enums for the `CHECK`-constrained columns, the date/timestamp split above. This document doesn't spell out every field — the schema (§13 of `database-schema.md`) is the source of truth for exactly what they contain.

## 3. Repository ports

### 3a. Transaction boundary

Repository ports don't each get their own implicit connection — a use case that needs several repository calls to succeed or fail together (§7 lists exactly which ones) must run them against one shared transaction, not one pool checkout per call. That requires a port for it:

```rust
// application/ports/transaction.rs
#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn Transaction>, InfrastructureError>;
}

#[async_trait]
pub trait Transaction: Send {
    async fn commit(self: Box<Self>) -> Result<(), InfrastructureError>;
    async fn rollback(self: Box<Self>) -> Result<(), InfrastructureError>;
}
```

Every repository method that participates in a transactional use case (§7) takes the active transaction explicitly rather than reaching for a pool of its own — shown below as a `tx: &mut dyn Transaction` parameter. The **rule** this locks is: *any repository call inside a listed transaction boundary must be threaded through the same `Transaction`, never open an independent connection.* The exact Rust encoding (a trait-object transaction with infrastructure downcasting it back to a concrete `sqlx::Transaction<'_, Sqlite>` internally, versus a generic parameter threaded through every port) is deliberately left to Round 7 — V1 has exactly one infrastructure implementation (SQLite), so this doesn't need to be a fully backend-agnostic abstraction, just an honest one.

### 3b. Aggregate repositories

One trait per aggregate. Representative signatures (not exhaustive — the pattern repeats), now returning `InfrastructureError` (not `ApplicationError` — see §6) and taking a transaction where the operation is meant to participate in one:

```rust
#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn create(&self, tx: &mut dyn Transaction, customer: NewCustomer) -> Result<Customer, InfrastructureError>;
    async fn update(&self, tx: &mut dyn Transaction, id: CustomerId, changes: CustomerChanges) -> Result<Customer, InfrastructureError>;
    async fn archive(&self, tx: &mut dyn Transaction, id: CustomerId) -> Result<(), InfrastructureError>;
    async fn restore(&self, tx: &mut dyn Transaction, id: CustomerId) -> Result<(), InfrastructureError>;
    async fn delete(&self, tx: &mut dyn Transaction, id: CustomerId) -> Result<(), InfrastructureError>; // surfaces ConstraintViolation if referenced by any invoice
    async fn find_by_id(&self, id: CustomerId) -> Result<Option<Customer>, InfrastructureError>;         // plain reads don't need a transaction
    async fn list(&self, filter: CustomerFilter) -> Result<Vec<Customer>, InfrastructureError>;
}
```

`InvoiceRepository` replaces the earlier generic `save()` with explicit operations matching the use cases in §4 one-to-one — the repository persists what the use case has already decided, it doesn't infer intent from a blob:

```rust
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn create_draft(&self, tx: &mut dyn Transaction, draft: NewInvoiceDraft) -> Result<Invoice, InfrastructureError>;
    async fn update_draft(&self, tx: &mut dyn Transaction, id: InvoiceId, changes: DraftInvoiceChanges) -> Result<Invoice, InfrastructureError>;
    async fn issue(&self, tx: &mut dyn Transaction, id: InvoiceId, issued: IssuedInvoiceFields) -> Result<Invoice, InfrastructureError>;
    async fn update_issued(&self, tx: &mut dyn Transaction, id: InvoiceId, changes: IssuedInvoiceChanges) -> Result<Invoice, InfrastructureError>;
    async fn cancel(&self, tx: &mut dyn Transaction, id: InvoiceId, reason: Option<String>) -> Result<Invoice, InfrastructureError>;
    async fn set_status(&self, tx: &mut dyn Transaction, id: InvoiceId, status: InvoiceStatus) -> Result<(), InfrastructureError>; // used only by the payment recalculation step
    async fn delete_draft(&self, tx: &mut dyn Transaction, id: InvoiceId) -> Result<(), InfrastructureError>; // repository trusts the caller already checked status = Draft

    async fn get(&self, id: InvoiceId) -> Result<Option<InvoiceWithLineItems>, InfrastructureError>;
    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<InvoiceSummary>, InfrastructureError>;
}

#[async_trait]
pub trait InvoiceNumberSequencer: Send + Sync {
    /// Read-only: what the next number *would* be, without reserving it.
    async fn preview_next(&self, format: &str, at: NaiveDate) -> Result<String, InfrastructureError>;
    /// Atomically increments the counter and returns the number to assign. Only ever
    /// called as part of the same transaction as IssueInvoice — see §7.
    async fn issue_next(&self, tx: &mut dyn Transaction, format: &str, at: NaiveDate) -> Result<String, InfrastructureError>;
}
```

`PaymentRepository`, `ProductRepository`, `TaxRateRepository`, `BusinessRepository` (singleton `get`/`update`, no `create`/`delete` — `business` is a fixed single row), `SettingsRepository` (same singleton shape) follow the same shape as `CustomerRepository`.

### 3c. Dashboard repository

Dashboard metrics get their own purpose-built port rather than being assembled in the application layer from `InvoiceRepository::list()` — that would mean pulling every invoice into Rust to count/sum what SQLite can already aggregate directly:

```rust
#[async_trait]
pub trait DashboardRepository: Send + Sync {
    async fn today_sales(&self, today: NaiveDate) -> Result<MinorUnits, InfrastructureError>;
    async fn month_sales(&self, month: (i32, u32)) -> Result<MinorUnits, InfrastructureError>;
    async fn outstanding_total(&self) -> Result<MinorUnits, InfrastructureError>;
    async fn paid_total(&self, month: (i32, u32)) -> Result<MinorUnits, InfrastructureError>;
    async fn overdue_summary(&self, today: NaiveDate) -> Result<OverdueSummary, InfrastructureError>; // count + total, using the is_overdue rule from database-schema.md §8
    async fn recent_invoices(&self, limit: u32) -> Result<Vec<InvoiceSummary>, InfrastructureError>;
}
```

Implemented by `infrastructure/database/sqlite_dashboard_repository.rs` as `COUNT`/`SUM`/`GROUP BY` queries, never as an in-Rust reduction over a full invoice list.

## 4. Use cases (application layer)

One function/struct per entry below; each takes its repositories by `Arc<dyn Trait>` (constructor-injected) and returns `Result<_, ApplicationError>` — translating any `InfrastructureError` it gets back from a repository call into the right `ApplicationError` variant (§6). Pre/postconditions are the Round 2 rules already locked — restated here only as a checklist so Round 7 has a direct checklist to implement against.

**Invoices** (`application/invoices.rs`)
- `CreateDraftInvoice` — no required fields; persists an empty (or partially filled) `DRAFT`. Not transactional (single insert).
- `UpdateDraftInvoice` — replaces customer/dates/notes/terms/discount/line items on a `DRAFT`; runs `domain::calculation::calculate_invoice` (§4a) and persists its result alongside the line items in one transaction, per §4 of `database-schema.md`.
- `IssueInvoice` — preconditions: `customer_id` set, ≥1 line item. One transaction (spelled out in full in §7): validate → load customer + business → `InvoiceNumberSequencer::issue_next` → `calculate_invoice` → `InvoiceRepository::issue` (writes number, snapshots, totals, `status = Issued`, `issued_at`).
- `EditIssuedInvoice` — allowed on `Issued`/`PartiallyPaid`/`Paid`; re-snapshots and recomputes at save time via `InvoiceRepository::update_issued`, in one transaction; never touches `payments`.
- `DuplicateInvoice` — copies customer, line items, discount, notes, terms into a new `DRAFT`; never copies payments, status, or the invoice number.
- `CancelInvoice` — preconditions: status ∈ `{Issued, PartiallyPaid, Paid}`. Sets `cancelled_at` + optional `cancel_reason` via `InvoiceRepository::cancel`. Terminal.
- `DeleteDraftInvoice` — preconditions: status = `Draft` (checked by the use case, not the repository). Cascades to line items (DB-level `ON DELETE CASCADE` already handles this).

**Customers** (`application/customers.rs`) / **Products** (`application/products.rs`)
- `CreateCustomer` / `CreateProduct`, `UpdateCustomer` / `UpdateProduct`.
- `ArchiveCustomer` / `ArchiveProduct` — used when a delete is attempted on a referenced row (see below), or directly from the UI.
- `RestoreCustomer` / `RestoreProduct` — `Archived → Active`.
- `DeleteCustomer` / `DeleteProduct` — attempts a hard delete; the repository's `ON DELETE RESTRICT` backstop surfaces as `InfrastructureError::ConstraintViolation`, which the use case translates into `ApplicationError::Conflict` ("archive instead?") rather than letting a raw DB error reach the UI.

**Invariant, stated explicitly because it isn't obvious from the schema alone:** `UpdateCustomer` and `UpdateProduct` only ever write to `customers`/`products`. Neither may reach into `invoices` or `invoice_line_items` to "fix up" existing snapshots — that would silently rewrite history and is exactly what the snapshot design in Round 2/3 exists to prevent. This isn't a technical constraint the database enforces (nothing stops a bug from doing it); it's a rule Round 7's implementation and review must hold itself to.

**Payments** (`application/payments.rs`)
- `RecordPayment`, `UpdatePayment`, `DeletePayment` — each, in one transaction (§7): confirm the parent invoice isn't `Cancelled`, write the payment change, `SUM` payments for the invoice, and call `InvoiceRepository::set_status` with the recalculated status.

**Calculation** (`domain/calculation.rs` — not `application/`, since it's pure business logic with no I/O; see §4a for the contract)
- `calculate_invoice(input: InvoiceCalculationInput) -> InvoiceCalculationResult` — the one function that produces every `*_minor` value the schema persists. Called by `UpdateDraftInvoice`/`IssueInvoice`/`EditIssuedInvoice`, never duplicated inline in a use case or a Tauri command.

**Dashboard** (`application/dashboard.rs`)
- `GetDashboardMetrics` — assembles its response entirely from `DashboardRepository` (§3c); does no invoice iteration of its own.

**Backup** (`application/backup.rs`) — see §4e for the full `RestoreBackup` sequence.
- `BackupDatabase` — delegates to `infrastructure/filesystem` to produce the `.vbx` archive (§9 of `database-schema.md`); the application layer's job is just "ask for a backup," not archive-format details.
- `RestoreBackup` — delegates to `infrastructure/filesystem`, with the atomicity contract in §4e.

### 4a. Calculation engine contract

Round 6 owns the exact arithmetic; Round 4 fixes the *shape* of what goes in and what comes out, because Round 3 already committed to persisting `invoice_discount_amount_minor` and `taxable_amount_minor` per line — the contract has to be able to produce them:

```rust
pub struct InvoiceCalculationInput {
    pub line_items: Vec<LineItemInput>,      // quantity_thousandths, unit_price_minor, tax_rate_basis_points, line_discount
    pub invoice_discount: Option<(DiscountType, i64)>,
}

pub struct LineItemResult {
    pub line_subtotal_minor: i64,
    pub line_discount_amount_minor: i64,
    pub invoice_discount_amount_minor: i64,   // this line's allocated share
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
}

pub struct InvoiceCalculationResult {
    pub lines: Vec<LineItemResult>,           // same order as input line_items
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
}
```

`calculate_invoice` is pure (no repository access, no clock, no randomness) — every input it needs is passed in, which is what makes it independently testable ahead of any UI or database work landing in Round 7.

### 4b. Master-data mutation invariant

Stated once, applies everywhere: **no master-data use case (`UpdateCustomer`, `UpdateProduct`, `UpdateBusiness`, changing a `TaxRate`) ever writes to `invoices` or `invoice_line_items`.** The snapshot exists precisely so these mutations don't need to.

### 4c. `IssueInvoice`, spelled out

```
BEGIN
  validate draft (customer_id set, ≥1 line item)
  load customer, load business
  issue_next(format, today)          ── via InvoiceNumberSequencer, same tx
  calculate_invoice(...)             ── pure, no I/O
  InvoiceRepository::issue(...)      ── writes number, snapshots, totals, status=Issued, issued_at
COMMIT
```

If anything fails, **ROLLBACK — including the counter increment.** `InvoiceNumberSequencer::issue_next` participates in the same transaction as everything else specifically so a failed issue never burns a number; only a *committed* issue consumes one. This is the same "never reused, never wasted" numbering guarantee from Round 2/3, extended to cover failure, not just abandoned drafts.

### 4d. Payment recalculation, spelled out

```
BEGIN
  validate invoice is not Cancelled
  validate payment
  INSERT / UPDATE / DELETE the payment row
  SUM(payments) for the invoice
  determine new status (Round 2 rule: 0 → Issued, 0<paid<total → PartiallyPaid, paid≥total → Paid)
  InvoiceRepository::set_status(...)
COMMIT
```

### 4e. `RestoreBackup`, spelled out

Restore is materially more dangerous than every other use case here — it replaces the user's live data — so it gets its own explicit sequence rather than being "just another repository call":

```
validate archive (readable, well-formed .vbx)
        ↓
validate metadata.json's format_version is one this app version understands
        ↓
validate database.sqlite passes an integrity check (SQLite's own PRAGMA integrity_check)
        ↓
close the active database connection pool
        ↓
atomically replace the live database file + assets/ (write to a temp path, then rename — never delete-then-copy)
        ↓
reopen the pool
        ↓
run migrations if the restored database is from an older (but supported) app version
        ↓
verify integrity once more post-migration
```

The contract this locks: **a restore either succeeds completely or leaves the existing installation untouched.** `delete current.db` followed by `copy backup` is explicitly rejected — a failure between those two steps would leave the user with no working database at all. The exact filesystem mechanics (temp file + atomic rename, which is what makes the replace step safe) are Round 7's job; the safety contract is Round 4's.

## 5. Tauri command surface

Commands in `commands/` mirror the use cases in §4 by name (`create_draft_invoice`, `update_draft_invoice`, `issue_invoice`, `duplicate_invoice`, `cancel_invoice`, `delete_draft_invoice`, `record_payment`, `update_payment`, `delete_payment`, `create_customer`/`update_customer`/`archive_customer`/`restore_customer`/`delete_customer` and the `*_product` equivalents, `get_dashboard_metrics`, `backup_database`, `restore_backup`). The binding rule isn't "exactly one use case call" as a rigid ceiling — a command could legitimately do a little orchestration later — it's the stronger, actually-enforceable boundary from Round 1: **Tauri commands contain no business rules, persistence logic, or calculation logic; they translate transport input/output and invoke application-layer behavior.**

## 6. Error handling

Two error types, not one, so infrastructure failure modes and user-facing application errors don't get conflated:

```rust
// application/ports/infrastructure_error.rs — what a repository/transaction call can fail with
pub enum InfrastructureError {
    Database(String),              // opaque wrapper over sqlx::Error — never leaks SQL details upward
    ConstraintViolation(String),    // e.g. a RESTRICT/UNIQUE violation
    Io(String),                     // backup/restore filesystem failures
    Transaction(String),            // begin/commit/rollback failure
}

// application/error.rs — what a use case can fail with, and what reaches Tauri
pub enum ApplicationError {
    NotFound { entity: &'static str, id: i64 },
    Validation(String),             // e.g. "customer required to issue"
    Conflict(String),               // e.g. delete attempted on a referenced row
    Infrastructure(InfrastructureError), // opaque fallback when a use case has no more specific translation
}
```

Flow: `sqlx::Error` → `InfrastructureError` (in `infrastructure/database/`) → `ApplicationError` (in the use case, which has the domain context to turn a `ConstraintViolation` into a specific `Conflict("customer has invoice history")` rather than a generic one) → a serializable `{ kind, message }` shape at the Tauri command boundary. `sqlx::Error` itself never crosses into `application/`; `InfrastructureError` never reaches the frontend directly. The exact JSON shape at the command boundary is a Round 5 (UI/UX) concern once there's a UI consuming it.

## 7. Transaction boundaries

Each of the following is one `Transaction` (§3a), not a sequence of independent writes — listed explicitly because a partial failure here would corrupt the invariants Round 2/3 locked. Full sequences for the first two are in §4c/§4d.

- `IssueInvoice`: number generation + snapshot + totals + status write — **including the counter increment**, so a failed issue never consumes a number. Explicitly: `InvoiceNumberSequencer::issue_next` must execute against the same transaction `IssueInvoice` uses for everything else — it must not open or commit an independent transaction/connection of its own. If any later step in `IssueInvoice` fails, the counter increment and every other write roll back together, as one unit. This is what makes `issue_invoice_rollback_does_not_consume_number` (§8, rows 1/9) a meaningful test rather than one that only happens to pass.
- `RecordPayment` / `UpdatePayment` / `DeletePayment`: the payment write + the invoice's `status` recalculation.
- `UpdateDraftInvoice` / `EditIssuedInvoice`: line items + recalculated totals written together, so a draft is never left with stale totals against new line items.
- `DeleteDraftInvoice`: the invoice delete + its cascaded line items (the DB's `ON DELETE CASCADE` already makes this atomic by construction, but the use case still runs it inside an explicit transaction rather than relying on that alone).
- `RestoreBackup`: see the full atomicity contract in §4e — the largest-blast-radius operation in the system, treated accordingly.

## 8. Verification guide

None of this is implemented yet (Round 7's job), so nothing here is unit-tested code — this is the checklist for confirming the design actually resolves the issue it was written for, plus the concrete test to write once Round 7 lands. Each row names the risk if the fix were only cosmetic.

| # | Fix | Verify now (design review) | Test to write in Round 7 | What it catches if the fix were fake |
|---|---|---|---|---|
| 1 | Transaction abstraction (§3a) | Every repository method listed in §7 takes `tx: &mut dyn Transaction`; none of them also expose a pool-based overload that a use case could reach for instead. | `issue_invoice_rollback_does_not_consume_number`: force a failure after `issue_next` but before commit, call `rollback`, then run `IssueInvoice` again and assert it gets the number the failed attempt would have used. | Without this, a failed issue silently burns an invoice number — contradicts the "never wasted" numbering guarantee from Round 2/3. |
| 2 | Explicit `InvoiceRepository` methods (§3b) | `save()` no longer appears anywhere in the doc; each use case in §4 names the one repository method it calls. | Per method: `issue()` asserts it wrote `invoice_number`/snapshots/`status`/`issued_at` and touched nothing else; `update_issued()` asserts the `payments` table's row count and sums are unchanged before/after. | A generic `save()` could silently overwrite fields (e.g. `created_at`, or worse, `payments`) a specific operation was never supposed to touch. |
| 3 | `InfrastructureError` vs `ApplicationError` (§6) | Every repository trait signature in §3 returns `InfrastructureError`; every use case signature in §4 returns `ApplicationError`. | Unit test with a fake `CustomerRepository` that returns `InfrastructureError::ConstraintViolation`; assert `DeleteCustomer` translates it into `ApplicationError::Conflict` with a specific, non-generic message. | If a raw `sqlx::Error` or SQL text ever reaches the frontend, it means a use case skipped translation — this test catches a missing `match` arm. |
| 4 | `NaiveDate` / `DateTime<Utc>` split (§2) | Every date/timestamp field in the domain type examples is one or the other, never a bare `DateTime`. | Round-trip test: construct an `Invoice` with a known `invoice_date`, persist via `SqliteInvoiceRepository`, read it back, assert exact equality — run once with the test process's local timezone set to something far from UTC (e.g. `TZ=Pacific/Kiritimati`) to catch accidental timezone drift on a value that should never have one. | A date silently promoted to a timestamp (or vice versa) can shift by a day depending on the machine's timezone — exactly the kind of bug that passes in one timezone and fails in another. |
| 5 | `calculate_invoice` contract (§4a) | `InvoiceCalculationResult` has a `LineItemResult` per input line, in the same order, with all five persisted `*_minor` fields from `database-schema.md` §13 present. | Table-driven unit tests once Round 6 fixes the arithmetic (e.g. "2 lines, 10% invoice discount, 18% GST" → hand-computed expected output); a property test that shuffles input line order and asserts output order tracks it. | A vague contract lets Round 6 quietly drop `taxable_amount_minor` or reorder lines relative to input, breaking the UI's line-by-line display. |
| 6 | `DashboardRepository` (§3c) | The port has one method per dashboard metric, each returning an aggregate value/small struct — no method returns `Vec<Invoice>` or anything shaped like "all rows." | Seed a fixture DB with known invoices/payments spanning several dates and statuses; call each method and assert against a hand-computed expected number; separately, `EXPLAIN QUERY PLAN` each implementation's SQL and assert it doesn't do a full table scan of `invoices` where an index/aggregate should apply. | Confirms the metric is computed by SQL (`SUM`/`COUNT`), not by pulling every invoice into Rust and reducing it there — a correctness pass alone wouldn't catch the performance regression. |
| 7 | `RestoreBackup` atomicity (§4e) | The sequence in §4e has a validation step before any destructive step, and the destructive step is described as "temp file + rename," not "delete then copy." | One integration test per failure-injection point (corrupt archive, unsupported `format_version`, `database.sqlite` failing `PRAGMA integrity_check`, a forced failure during the rename step) — after each, assert the original database file still exists, still opens, and still passes its own integrity check. | This is the one place a "looks atomic" design could still leave a user with zero working data if the implementation actually does delete-then-copy — the test has to inject failure *during* the replace step specifically, not just before or after it. |
| 8 | Master-data mutation invariant (§4b) | `sqlite_customer_repository.rs` and `sqlite_product_repository.rs` (once written) contain no SQL referencing `invoices` or `invoice_line_items` at all — not even a read. | A CI-level static check (grep for `invoice` inside those two files) as a cheap first line of defense, plus an integration test: snapshot every row of `invoices`/`invoice_line_items`, call `UpdateCustomer`/`UpdateProduct`, re-snapshot, assert byte-for-byte equality. | The whole point of Round 2/3's snapshot design is defeated if a "helpful" future edit makes `UpdateCustomer` cascade a name change into old invoices — this is the regression test that would catch that the moment it's introduced. |
| 9 | Numbering + issuance one transaction (§4c, §7) | Same row as #1 — restated here because it's also, separately, a numbering-correctness property, not just a transaction-mechanics one: `invoice_number_counters.last_value` must never be higher than the count of invoices that actually reached `Issued`. | A second assertion in the same rollback test from #1: after the rollback, query `invoice_number_counters` directly and assert `last_value` matches the count of successfully issued invoices, not "issued + 1." | Confirms the invariant isn't just "no crash on rollback" but specifically "the counter's value stays semantically correct." |

## Round 4 definition of done

Every use case named in the roadmap discussion (`CreateDraftInvoice` through `RestoreBackup`) has a home, a layer, and a stated pre/postcondition; every repository port needed to implement them is named with a representative signature and threaded through an explicit transaction where one is required; infrastructure and application errors are kept separate; date/timestamp types are fixed; the calculation engine has a concrete input/output contract; dashboard reads go through purpose-built aggregate queries; and restore's atomicity contract is stated — each of these nine points has a named test in §8 so "designed" and "verified" don't quietly drift apart once Round 7 starts writing code. Round 5 (UI/UX) designs screens against this command surface; Round 6 (calculation engine) fills in `calculate_invoice`'s arithmetic against the §4a contract; Round 7 (implementation) writes the actual Rust code following this module layout and the test list in §8.
