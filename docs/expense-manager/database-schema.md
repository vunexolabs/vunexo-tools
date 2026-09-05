---
status: locked
round: 3
---

# Vunexo Expense Manager — Database Schema (Round 3)

Builds on Round 1 (`.ai/product-expense-manager.md`) and Round 2 (`user-flows.md`). Mirrors the conventions of `docs/vunexo-billing/database-schema.md` where applicable; simpler where the domain is simpler (no invoice-style state machine, no numbering sequence).

## 1. Domain entities

`business` (single row), `vendors`, `categories`, `expenses`. No separate receipt-attachment table — see §4.

## 2. Entity relationships

- `expenses.vendor_id` → `vendors.id`, nullable (a "misc/no vendor" expense is allowed — Round 2 says vendor is picked via a searchable picker, not stated as mandatory).
- `expenses.category_id` → `categories.id`, required (every expense must be categorized — reports are category-driven).

## 3. Delete rules

- `vendors`/`categories` deletion is blocked at the application layer if any `expenses` row references it (`has_expenses` check), same pattern as Billing's `has_invoices` check on customers/products. No DB-level `ON DELETE RESTRICT` trigger needed beyond the FK itself defaulting to `RESTRICT` — SQLite's default FK behavior already refuses the delete; the application check exists to give the user a clear message before attempting it.
- `expenses` rows are hard-deleted (Round 2: no draft/issued lifecycle, no reason to soft-delete). Deleting an expense also deletes its receipt file, if any (see §4).

## 4. Snapshot strategy

Per Round 1's historical-immutability principle, an expense must keep showing what it showed at creation even if the vendor or category is later renamed, or the category's default-deductible flag changes.

- `expenses.vendor_name_snapshot` and `expenses.category_name_snapshot` — denormalized `TEXT` columns, copied from `vendors.name`/`categories.name` at the moment the expense is saved. Same reasoning as Billing's invoice/customer snapshot: this is a 1:1 relationship with no independent lifecycle, so a side table adds a join with no normalization benefit.
- `expenses.deductible` is the expense's **own** boolean, seeded from `categories.default_deductible` at creation time (per Round 2) and editable thereafter. It is never re-read from the category after creation — that's the mechanism that satisfies "a category's flag changing later doesn't silently flip old expenses."
- `expenses.vendor_id`/`category_id` are kept (not dropped in favor of snapshot-only) so the UI can still navigate to the live vendor/category record and so reports can regroup by current category if the user wants — but every *displayed* historical name/deductibility reads from the snapshot columns, never a live join, matching Billing's rule that a snapshot is written once and never silently recomputed.

## 5. Money representation

Same rule as Billing, locked at Round 1: no binary floating point. All money columns are `INTEGER`, storing minor currency units (paise). `expenses.amount_minor`, `expenses.tax_amount_minor` are both `INTEGER`.

Multi-currency is out of scope for V1 (Round 1) — there is exactly one currency per install, configured once on `business.currency_symbol` (a plain display string, e.g. `"₹"`), not a per-expense field. No currency-conversion logic exists or is needed.

## 6. Tax / ITC representation

- `expenses.tax_amount_minor` — the tax portion of the expense amount, as entered by the user. Not derived from a rate table; V1 doesn't model tax rates as master data (unlike Billing's `tax_rates` table) since the product boundary (Round 1) is "records tax information supplied by the user," not "computes tax."
- `expenses.itc_eligible` — `BOOLEAN`, a separate fact from `deductible` per Round 1 ("tax paid and ITC-eligibility are separate facts, not one field"). Both are plain user-entered flags, not computed.
- No `itc_amount` column in V1 — Round 1 flagged partial-ITC amounts as a possible future refinement, not designed now. If itc_eligible is true, reports treat the full `tax_amount_minor` as the ITC total; V1 doesn't need a separate amount column for that.

## 7. Receipt attachment storage

Single optional receipt per expense: `expenses.receipt_path`, a nullable `TEXT` column storing an **app-managed relative path** (e.g. `receipts/<uuid>.jpg`), copied into the app's data directory on attach — same pattern as Billing's `business.logo_path` (`domain::business::resolve_logo_path`/`looks_absolute`). No separate table: it's a 1:1, no independent lifecycle, exactly the case Billing's snapshot-strategy reasoning already covers for why a side table isn't worth it here either.

Backup must copy the `receipts/` directory into the archive alongside the database snapshot, same shape as Billing's asset-bundling for `business.logo_path` (`assets_to_archive`). This is a Round 4 (application architecture) wiring detail, flagged here as a schema-level constraint: the file must live somewhere the backup step already knows to walk.

## 8. Payment method

`expenses.payment_method` — plain `TEXT`, not an enum table (Cash / Card / Bank Transfer / UPI / Other, or free text). No master-data table for this; a fixed small set doesn't need one, and adding a new method later is a non-breaking column-value change, not a schema migration.

## 9. Indexes & constraints

- `expenses(category_id)`, `expenses(vendor_id)`, `expenses(date)` — indexed, since reports filter/group by all three.
- `vendors.name`, `categories.name` — no uniqueness constraint forced (Billing doesn't force unique customer/product names either); duplicates are a user data-quality concern, not a DB one.

## 10. Migration strategy

Same as Billing: SQLx migrations under `src-tauri/migrations/`, applied on startup. First migration creates all four tables in one file (no legacy data to evolve from, unlike Billing's later migrations).

## 11. Final SQLite schema

```sql
CREATE TABLE business (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    address TEXT,
    tax_info TEXT,
    currency_symbol TEXT NOT NULL DEFAULT '₹',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE vendors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    contact TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    default_deductible INTEGER NOT NULL DEFAULT 0, -- boolean
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    tax_amount_minor INTEGER NOT NULL DEFAULT 0,
    itc_eligible INTEGER NOT NULL DEFAULT 0, -- boolean
    deductible INTEGER NOT NULL DEFAULT 0,   -- boolean, snapshot from category at creation
    payment_method TEXT NOT NULL,
    notes TEXT,
    receipt_path TEXT,
    vendor_id INTEGER REFERENCES vendors(id),
    vendor_name_snapshot TEXT,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    category_name_snapshot TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_expenses_category ON expenses(category_id);
CREATE INDEX idx_expenses_vendor ON expenses(vendor_id);
CREATE INDEX idx_expenses_date ON expenses(date);
```

## Round 3 definition of done

- Every entity in Round 2's flows has a table.
- Historical-immutability principle (Round 1) is concretely satisfied by `vendor_name_snapshot`/`category_name_snapshot`/`deductible`.
- Money columns are all `INTEGER` minor units, no floats.
- Receipt storage is specified enough for Round 4 to wire backup/restore against it.
