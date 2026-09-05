---
status: locked
round: 4
---

# Vunexo Expense Manager — Application Architecture (Round 4)

Builds on Rounds 1–3. Mirrors `docs/vunexo-billing/application-architecture.md`'s layering (`domain` / `application` / `infrastructure`, dependency-inversion via ports), scaled down to this domain's simpler shape — no invoice-style state machine, no numbering sequencer, no multi-table atomic conversions.

## Module layout

```
domain/
├── mod.rs
├── money.rs           (MinorUnits newtype)
├── business.rs         (Business, same resolve_logo_path/looks_absolute-style helper for receipt paths — see receipt.rs)
├── vendor.rs            (Vendor)
├── category.rs            (Category)
├── expense.rs               (Expense — incl. vendor_name_snapshot/category_name_snapshot/deductible/itc_eligible)
└── receipt.rs                (resolve_receipt_path/looks_absolute, mirrors business::resolve_logo_path)

application/
├── mod.rs
├── ports/
│   ├── mod.rs
│   ├── infrastructure_error.rs
│   ├── business_repository.rs
│   ├── vendor_repository.rs
│   ├── category_repository.rs
│   ├── expense_repository.rs
│   ├── dashboard_repository.rs
│   └── report_repository.rs
├── business.rs        (business use cases)
├── vendors.rs          (vendor use cases, incl. has_expenses check before delete)
├── categories.rs        (category use cases, incl. has_expenses check before delete)
├── expenses.rs             (expense use cases, incl. receipt attach/replace/remove)
├── dashboard.rs              (GetDashboardMetrics)
├── reports.rs                 (GenerateCategorySummary/PeriodSummary/DeductibleSummary/TaxItcSummary/TopVendors)
├── backup.rs                    (BackupData, RestoreBackup — bundles receipts/ dir, mirrors Billing's asset bundling)
├── export.rs                     (WriteExportFile — same generic "write already-rendered text to a path" as Billing's write_export_file)
└── error.rs                       (ApplicationError)

infrastructure/
├── database/
│   ├── mod.rs
│   ├── sqlite_business_repository.rs
│   ├── sqlite_vendor_repository.rs
│   ├── sqlite_category_repository.rs
│   ├── sqlite_expense_repository.rs
│   ├── sqlite_dashboard_repository.rs
│   └── sqlite_report_repository.rs
└── filesystem/
    ├── receipts.rs        (copy-in-on-attach, delete-on-remove, same pattern as logo management)
    └── backup.rs           (VACUUM INTO + zip archive incl. receipts/, staged restore + app restart)
```

No `pdf/` module — V1 has no PDF generation (no invoices to render). No `invoice_number_sequencer`-equivalent — expenses aren't numbered.

## Domain types

```rust
// domain/money.rs
pub struct MinorUnits(pub i64);

// domain/expense.rs
pub struct Expense {
    pub id: ExpenseId,
    pub date: NaiveDate,
    pub amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub itc_eligible: bool,
    pub deductible: bool,
    pub payment_method: String,
    pub notes: Option<String>,
    pub receipt_path: Option<String>,
    pub vendor_id: Option<VendorId>,
    pub vendor_name_snapshot: Option<String>,
    pub category_id: CategoryId,
    pub category_name_snapshot: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`Vendor`, `Category`, `Business` follow the schema 1:1 (§11 of `database-schema.md` is the source of truth for fields). No status enums anywhere in this domain — every entity is either present or deleted, matching Round 2's "no multi-status lifecycle" decision.

## Repository ports

Standard CRUD ports for `vendor_repository`/`category_repository` (`create`/`update`/`delete`/`get`/`list`, plus a `has_expenses(id)` check each use case calls before allowing delete — same shape as Billing's `has_invoices`).

`expense_repository` adds `list_by_category`/`list_by_vendor`/`list_by_date_range` (backing both the Expenses list screen's filters and the report queries that don't go through `report_repository`'s own SQL aggregation).

`dashboard_repository` and `report_repository` are SQL-aggregated, same discipline as Billing's `DashboardRepository`/`StatementRepository`/`ReportRepository` — aggregation happens in the SQL, not by loading every row into Rust and summing in a loop.

No `TransactionManager` port is introduced. Billing needed one because `ConvertQuoteToInvoice` writes two tables atomically; nothing in this domain does a multi-table write that must succeed-or-fail together — a single expense insert/update/delete is one statement against one table (plus, for delete, one filesystem removal that's naturally idempotent-safe to retry). If a future round introduces a genuine multi-table atomic write, add the port then.

## Use cases (application layer), spelled out where non-obvious

- **`CreateExpense`**: resolves `vendor_name_snapshot`/`category_name_snapshot` by reading the live vendor/category row once at creation time and copying the name in — never re-read afterward (Round 3 §4).
- **`UpdateExpense`**: does **not** re-snapshot the vendor/category name from the current live record — only a brand-new `CreateExpense` writes a snapshot. Editing an expense's own fields (amount, notes, deductible, etc.) doesn't touch the snapshot columns unless the user explicitly re-picks a different vendor/category, in which case it snapshots the *newly picked* one's current name, same as picking it fresh.
- **`DeleteExpense`**: deletes the DB row, then deletes the receipt file at `receipt_path` if set (best-effort — a missing file on disk shouldn't block the DB delete, mirroring the "restore closes the pool" caution in Billing but scaled to a much lower-stakes single-file op).
- **`AttachReceipt`/`ReplaceReceipt`/`RemoveReceipt`**: copies the chosen file into the app's data directory under `receipts/<uuid>.<ext>`, writes the relative path to `expenses.receipt_path`. Replace deletes the old file after the new one is confirmed written (never leaves the row pointing at a deleted file mid-operation).
- **`DeleteVendor`/`DeleteCategory`**: call `has_expenses` first; refuse with a clear `ApplicationError::Validation` if any exist, same UX as Billing's blocked-delete message.
- **`BackupData`/`RestoreBackup`**: same shape as Billing's, with one addition — the archive must also walk `receipts/` and include every file `expenses.receipt_path` points at (plus orphan-tolerant: an archive missing a referenced file restores the row with a null-safe "receipt missing" state in the UI rather than failing the whole restore).

## Tauri command surface

One command per use case, `snake_case`, matching Billing's naming convention: `create_business`, `update_business`, `get_business`, `create_vendor`, `update_vendor`, `delete_vendor`, `list_vendors`, `create_category`, `update_category`, `delete_category`, `list_categories`, `create_expense`, `update_expense`, `delete_expense`, `list_expenses`, `attach_receipt`, `replace_receipt`, `remove_receipt`, `get_dashboard_metrics`, `generate_category_summary`, `generate_period_summary`, `generate_deductible_summary`, `generate_tax_itc_summary`, `generate_top_vendors`, `write_export_file`, `backup_data`, `restore_backup`.

## Error handling

Same `ApplicationError` shape as Billing: `Validation(String)` / `NotFound` / `Infrastructure(InfrastructureError)`, mapped to a string at the Tauri command boundary (`Result<T, String>`), same as every existing Billing command.

## Verification guide

```bash
# Backend, from apps/expense-manager/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/expense-manager/
pnpm typecheck && pnpm lint && pnpm build
```

## Round 4 definition of done

- Every Round 2 flow has a named use case and Tauri command above.
- Snapshot-writing is pinned to `CreateExpense` only, never `UpdateExpense` — the one rule most likely to be gotten wrong by a future session, called out explicitly.
- No `TransactionManager` port added without a genuine multi-table atomic write driving it.
