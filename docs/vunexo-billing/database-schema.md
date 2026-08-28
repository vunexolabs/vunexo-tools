---
status: locked
round: 3
---

# Vunexo Billing — Database Schema (Round 3)

This is an AI context file. It is the source of truth for the SQLite schema. Read it before writing any migration, repository, or domain code. It is designed against the locked flows in `docs/vunexo-billing/user-flows.md` and the locked principles in `.ai/product.md` — it does not relitigate them, only implements them.

Master data (current truth: `business`, `customers`, `products`, `tax_rates`, `settings`) is kept structurally separate from transactional/historical data (`invoices` and everything hanging off an invoice: line items, payments, snapshots). Nothing about a historical invoice is ever reconstructed by joining to current master rows — it's reconstructed from what's stored on the invoice itself.

## 1. Domain entities

- **Business** — the single business profile (V1 is single-business per install).
- **Settings** — app-level configuration (numbering format, defaults, locale) — kept separate from Business because it isn't part of what gets printed on an invoice or snapshotted.
- **TaxRate** — a small, user-maintained list of tax rates (e.g. "GST 18%"). Lightweight master data, not a big tax-configuration subsystem.
- **Customer**
- **Product**
- **Invoice** — the transactional root. Carries its own frozen customer/business snapshot, its own computed totals, and its lifecycle state.
- **InvoiceLineItem** — belongs to exactly one invoice; carries its own frozen product snapshot.
- **Payment** — belongs to exactly one invoice; an independent historical record (per Round 2).
- **InvoiceNumberCounter** — internal bookkeeping for numbering; not really a "domain entity" a user thinks about, but it has to live somewhere.

Deliberately **not** modeled as tables in V1: an audit log, and attachments. Round 2 doesn't require either — added only if a later round's flows actually need them.

## 2. Entity relationships

```
business (1 row)              settings (1 row)              tax_rates
                                                                  ▲
                                                                  │ (nullable, informational)
customers ──┐                                                    │
            │                                              products ──tax_rate_id
            │ (nullable while draft; required to issue)          │
            ▼                                                    │ (nullable, informational)
        invoices ──────────────────────────────────────┐          │
            │  1:N                                     │          │
            ▼                                          ▼          ▼
    invoice_line_items ◀──────────────────────── product_id (nullable ref)
            │
        (frozen fields live on the line item itself, not derived via product_id)

        invoices
            │ 1:N
            ▼
        payments  (independent of invoice edits — see Round 2)
```

`customer_id` and `product_id` on invoices/line items are **references for filtering, search, and "reorder this"-style UX** — never for reconstructing what an invoice said at the time. What an invoice said is stored on the invoice/line-item row itself (see §4).

## 3. Lifecycle / state rules

- `invoices.status ∈ {DRAFT, ISSUED, PARTIALLY_PAID, PAID, CANCELLED}` — a **stored** column, matching Round 2's state machine. `OVERDUE` is never stored; it's computed at query time (§8).
- `DRAFT → ISSUED`: only transition that sets `invoice_number`, `issued_at`, and the customer/business snapshot columns.
- `ISSUED/PARTIALLY_PAID/PAID → CANCELLED`: sets `cancelled_at` and optional `cancel_reason`. Terminal from there.
- Status is **recalculated and written** by the application layer whenever a payment is inserted/updated/deleted (per Round 2: "status is recalculated automatically"), not derived at every read — this keeps invoice list/dashboard queries simple (`WHERE status = 'PAID'`) instead of requiring a `SUM(payments)` join everywhere.
- `customers.status`, `products.status` ∈ `{ACTIVE, ARCHIVED}` — archive instead of delete once referenced by any invoice; hard `DELETE` stays available (and is what the app uses) when a row has zero references, enforced additionally by `ON DELETE RESTRICT` foreign keys as a backstop against application bugs.

**Invoice state invariants** — these are enforced by the service layer (Round 4), not by DB triggers (deliberately: trigger machinery is unnecessary weight for V1), but they're documented here as the contract that layer must uphold:

| State | Invariant |
|---|---|
| `DRAFT` | `invoice_number`, `issued_at`, and every snapshot column are `NULL`. |
| `ISSUED` / `PARTIALLY_PAID` / `PAID` | `invoice_number`, `issued_at`, `customer_id`, and every snapshot column are `NOT NULL`. |
| `CANCELLED` | `cancelled_at` is `NOT NULL`; no further edits, no further payments accepted. |

**Payment changes → status recalculation, worked example:** an invoice with `total_minor = 1,000,000` (₹10,000) and one ₹10,000 payment is `PAID`. Delete that payment → `SUM(payments) = 0` → status recalculates to `ISSUED` (or `OVERDUE` as a derived badge, if past due). Edit it down to ₹5,000 instead → `PARTIALLY_PAID`. Every payment write is followed by this same recalculation, per Round 2.

## 4. Snapshot strategy

Both snapshots the flows doc requires (customer, business) are stored as **denormalized nullable columns directly on `invoices`**, not a separate 1:1 snapshot table. It's a genuine 1:1 relationship with no independent lifecycle of its own, so a side table would only add a join with no normalization benefit. Columns are `NULL` until `Issue` (per Round 2: nothing needs protecting before that point) and frozen from then on.

Line-item snapshotting works the same way: `invoice_line_items` stores its own `description`, `unit`, `unit_price_minor`, and `tax_rate_basis_points` columns, copied at the moment the item is added — not read through `product_id` at render time.

**The line item also snapshots enough of the discount math to explain itself without rerunning the calculation engine.** Round 2 locked invoice-level discount as pre-tax and proportionally allocated across taxable lines — so a line needs to record its own resolved discount, its allocated share of the invoice-level discount, and the resulting taxable base, not just the type/value inputs:

```
line_subtotal_minor              (quantity × unit_price, already present)
− line_discount_amount_minor     (this line's own discount, resolved from line_discount_type/value)
− invoice_discount_amount_minor  (this line's allocated share of the invoice-level discount)
= taxable_amount_minor
+ line_tax_minor                 (computed from taxable_amount_minor × tax_rate_basis_points)
= line_total_minor
```

Example: a ₹10,000 line with a ₹500 line discount and a ₹950 allocated share of the invoice discount has a ₹8,550 taxable amount; at 18% GST that's ₹1,539 tax, for a ₹10,089 line total. `line_discount_amount_minor`, `invoice_discount_amount_minor`, and `taxable_amount_minor` are added to `invoice_line_items` in §13 alongside the existing `line_subtotal_minor`/`line_tax_minor`/`line_total_minor`.

**Computed totals are persisted, not re-derived, once written.** `invoices.subtotal_minor` / `discount_amount_minor` / `tax_amount_minor` / `total_minor`, and each line item's `line_subtotal_minor` / `line_tax_minor` / `line_total_minor`, are written by the calculation engine (Round 6) at save time and stored as facts. This matters beyond styling: if a future app version changes rounding rules, a historical invoice must keep showing the numbers it showed the day it was issued — it must never be silently recomputed with new logic. Draft invoices get the same treatment on every "Save Draft" (simplest consistent rule — no separate "live, never-persisted" state).

## 5. Money representation

Round 6 (calculation engine) still owns the *in-memory arithmetic* decision (integer minor-unit math vs. `rust_decimal` inside Rust) — that's explicitly not locked yet. But the **on-disk column type** has to be picked now, and it constrains Round 6 either way:

- All money columns (`price_minor`, `unit_price_minor`, `*_amount_minor`, `*_total_minor`, `amount_minor`) are **`INTEGER`, storing minor currency units** (paise for INR). SQLite integers are exact and arbitrary-precision-safe within 64 bits — no floating point, satisfying the locked "never binary floating point for money" rule regardless of which Round 6 arithmetic strategy wins.
- **Quantity** feeds directly into money math (`quantity × unit_price`), so it gets the same treatment rather than being treated as "just a display number": stored as `quantity_thousandths INTEGER` (quantity × 1000 → 3 decimal places, e.g. `2.5` → `2500`). A `REAL` column was considered and rejected — plenty of ordinary decimal quantities (`1.1` hours, `0.3` kg) aren't exactly representable in binary floating point, which is exactly the class of bug the money rule exists to avoid. The 3-decimal-place scale is a provisional choice; Round 6 can widen it if some unit genuinely needs more precision.
- **Tax rate and discount percentages** are stored as `INTEGER` **basis points** (1 basis point = 0.01%, so `18%` → `1800`, `5.5%` → `550`), giving 2 decimal places of rate precision without floating point.

## 6. Tax representation

- `tax_rates` is small, user-maintained master data: a name (`"GST 18%"`) and a `rate_basis_points`. Products reference one as their default; nothing stops a line item from using a different one. The business-level default lives on `settings.default_tax_rate_id` — `tax_rates` itself carries no `is_default` flag, so there's exactly one place that concept can be set, not two that could disagree.
- GST's CGST/SGST vs. IGST split is **not** a property of the tax rate — the same 18% applies whether a sale is intra-state or inter-state; only the *display/reporting split* changes. That's modeled as `invoices.is_interstate BOOLEAN`, decided per invoice. `cgst`/`sgst`/`igst` amounts are **derived at render/report time** from `tax_amount_minor` + `is_interstate` (half/half vs. whole) rather than stored as separate redundant columns that could drift out of sync with the total.
- `products.hsn_sac_code` and `business.gstin` / `customers.gstin` are plain nullable text fields — GST filing itself stays out of scope per the locked spec.

## 7. Invoice numbering

- `invoice_number_counters(scope_key TEXT PRIMARY KEY, last_value INTEGER NOT NULL DEFAULT 0)`. `scope_key` is derived from whatever period the configured format resets on (e.g. `"2026"` for a `{year}`-based format) — not hardcoded to "year" in the schema, since that's a `settings.invoice_number_format` concern.
- A fresh `DRAFT` shows a **preview** of the next number (read the counter without incrementing it) — `invoices.invoice_number` stays `NULL` while drafting. The counter is only incremented, and the number written, atomically at **Issue**. This avoids burning numbers on abandoned/deleted drafts. (V1 is single-user/single-device, so the theoretical race between two drafts previewing the same number is not a real-world concern.)
- The advanced manual-override path (Round 2, §5) writes directly to `invoice_number` without touching the counter, and sets `invoice_number_is_custom = TRUE` so reporting can distinguish generated from imported numbers.
- `invoice_number` is `UNIQUE` whenever it's set, via a partial unique index (allows unlimited `NULL`s for drafts): `CREATE UNIQUE INDEX ... ON invoices(invoice_number) WHERE invoice_number IS NOT NULL`.
- **`settings.invoice_number_format` becomes read-only once the first invoice has been issued** (application-enforced, not a DB constraint — checked as "does any invoice with `issued_at IS NOT NULL` exist?"). Changing the format mid-sequence raises real ambiguity (what happens to `scope_key` continuity, whether old and new formats' sequences should share a counter) that V1 sidesteps entirely by locking the format after first use rather than trying to design a format-migration story now. A more sophisticated numbering-migration path is a future-version concern, not V1's.

## 8. Payment / balance model

- `payments` rows are independent of the invoice — inserting, editing, or deleting one never touches `invoices`' own fields (total, discount, tax, line items), per Round 2.
- **`amount_paid` is not denormalized** onto `invoices`. At V1's data volume (one small business's invoices — hundreds to low thousands of rows over years), `SUM(payments.amount_minor) WHERE invoice_id = ?` is trivially fast in SQLite, and skipping the cached column entirely sidesteps a whole class of cache-consistency bug for no real performance benefit.
- What *is* stored is `invoices.status`, written by the application whenever a payment changes (§3) — that's the one thing worth caching, since list/dashboard queries filter and count by status constantly, and status carries application decisions (e.g. "stays at PAID even if a later edit drops the total below what was already paid" — the overpayment case from Round 2) that a pure `SUM` comparison wouldn't capture on its own.
- Blocking payments against a `CANCELLED` invoice is an **application-layer invariant**, not a DB trigger — a `CHECK`/trigger was considered and rejected as unnecessary machinery for V1; the service layer (Round 4) enforces it.

## 9. Backup model

No new tables. But the container needs one correction: `business.logo_path` (and any future filesystem-backed asset) is a path into the app's local data directory, not a value stored inside SQLite — a `.vbx` that only copies the database file would restore onto a machine with a broken/missing logo reference. So `.vbx` is an **archive**, not a bare database copy:

```
vunexo-billing-backup-2026-08-28.vbx
├── metadata.json      (format_version, app_version, created_at, platform)
├── database.sqlite
└── assets/
    └── business-logo.*
```

Restoring re-extracts `assets/` back into the app's data directory alongside restoring the database, so `logo_path` keeps resolving. This is `infrastructure/filesystem/` territory (stubbed in Round 1), implemented in Round 7, and the archive shape leaves room for future attachments without another backup-format redesign. Migration history is already tracked by SQLx's own `_sqlx_migrations` table inside `database.sqlite`; no hand-rolled schema-version table is needed on top of it.

## 10. Indexes & constraints

- `customers`: index on `name` (search/picker), index on `status`.
- `products`: index on `name`, index on `status`.
- `invoices`: index on `customer_id`, index on `status`, index on `invoice_date`, index on `due_date` (drives the `is_overdue` query), partial unique index on `invoice_number`.
- `invoice_line_items`: index on `invoice_id`, index on `product_id`.
- `payments`: index on `invoice_id`, index on `paid_on`.
- Foreign keys: `invoices.customer_id → customers.id` **RESTRICT**; `invoice_line_items.product_id → products.id` **RESTRICT** (mirrors the archive-not-delete rule at the DB level, not just in application code); `invoice_line_items.invoice_id → invoices.id` **CASCADE** (line items are owned by their invoice); `payments.invoice_id → invoices.id` **CASCADE** (payments are likewise owned by their invoice, even though in practice an issued invoice is never deleted so this rarely fires); `products.tax_rate_id → tax_rates.id` **SET NULL** (a deprecated tax rate shouldn't block deleting it — the product just loses its default and the user picks a new one); `invoice_line_items.tax_rate_id → tax_rates.id` **SET NULL** (purely informational once frozen — `tax_rate_basis_points` already holds the number that matters).
- `CHECK` constraints: `status` enums on invoices/customers/products, `discount_type IN ('AMOUNT','PERCENTAGE')`, `payments.method IN ('CASH','BANK_TRANSFER','UPI','CHEQUE','OTHER')`, `payments.amount_minor > 0`, `products.price_minor >= 0`, `invoice_line_items.quantity_thousandths > 0`.

## 11. Delete / archive behavior

| Entity | Zero references | Referenced |
|---|---|---|
| Customer | hard delete or archive | archive only (never delete) |
| Product | hard delete or archive | archive only (never delete) |
| Invoice (`DRAFT`) | hard delete (cascades to its line items) | — (a draft can't be "referenced" by anything else) |
| Invoice (`ISSUED`/`PARTIALLY_PAID`/`PAID`) | never deleted | cancel instead |
| Invoice (`CANCELLED`) | never deleted | terminal |
| Payment | editable/deletable regardless of invoice status, recalculates parent `status` | — |

## 12. Migration strategy

Round 1 shipped `migrations/0001_init.sql` as a deliberately empty placeholder to prove the SQLx migration workflow, with no business tables. Since nothing has shipped externally yet, Round 3 replaces that placeholder's content directly with the real schema below rather than stacking an `0002_...` on top of an empty no-op — there's no external database that already ran `0001` and needs preserving. Once V1 ships, this "replace the last migration" move stops being available and all further schema changes become additive migration files.

## 13. Final SQLite schema

```sql
CREATE TABLE business (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    logo_path TEXT,
    address TEXT,
    phone TEXT,
    email TEXT,
    gstin TEXT,
    bank_details TEXT,
    upi_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tax_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    rate_basis_points INTEGER NOT NULL CHECK (rate_basis_points >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    country_code TEXT NOT NULL DEFAULT 'IN',
    currency_code TEXT NOT NULL DEFAULT 'INR',
    date_format TEXT NOT NULL DEFAULT 'DD/MM/YYYY',
    invoice_number_format TEXT NOT NULL DEFAULT 'INV-{year}-{seq:04d}',
    default_due_days INTEGER NOT NULL DEFAULT 15,
    default_tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    address TEXT,
    gstin TEXT,
    status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_customers_name ON customers(name);
CREATE INDEX idx_customers_status ON customers(status);

CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    sku TEXT,
    description TEXT,
    unit TEXT NOT NULL,
    price_minor INTEGER NOT NULL CHECK (price_minor >= 0),
    tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    hsn_sac_code TEXT,
    status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_products_name ON products(name);
CREATE INDEX idx_products_status ON products(status);

CREATE TABLE invoice_number_counters (
    scope_key TEXT PRIMARY KEY,
    last_value INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    invoice_number TEXT,
    invoice_number_is_custom INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'DRAFT'
        CHECK (status IN ('DRAFT', 'ISSUED', 'PARTIALLY_PAID', 'PAID', 'CANCELLED')),

    customer_id INTEGER REFERENCES customers(id) ON DELETE RESTRICT,

    -- Customer snapshot, frozen at Issue (NULL while DRAFT)
    customer_snapshot_name TEXT,
    customer_snapshot_phone TEXT,
    customer_snapshot_email TEXT,
    customer_snapshot_address TEXT,
    customer_snapshot_gstin TEXT,

    -- Business snapshot, frozen at Issue (NULL while DRAFT)
    business_snapshot_name TEXT,
    business_snapshot_address TEXT,
    business_snapshot_gstin TEXT,
    business_snapshot_phone TEXT,
    business_snapshot_email TEXT,
    business_snapshot_bank_details TEXT,
    business_snapshot_upi_id TEXT,
    business_snapshot_logo_path TEXT,

    is_interstate INTEGER NOT NULL DEFAULT 0,

    invoice_date TEXT NOT NULL DEFAULT (date('now')),
    due_date TEXT,

    notes TEXT,
    terms TEXT,

    discount_type TEXT CHECK (discount_type IN ('AMOUNT', 'PERCENTAGE')),
    discount_value INTEGER,

    subtotal_minor INTEGER NOT NULL DEFAULT 0,
    discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    tax_amount_minor INTEGER NOT NULL DEFAULT 0,
    total_minor INTEGER NOT NULL DEFAULT 0,

    issued_at TEXT,
    cancelled_at TEXT,
    cancel_reason TEXT,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX idx_invoices_number ON invoices(invoice_number) WHERE invoice_number IS NOT NULL;
CREATE INDEX idx_invoices_customer ON invoices(customer_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_invoice_date ON invoices(invoice_date);
CREATE INDEX idx_invoices_due_date ON invoices(due_date);

CREATE TABLE invoice_line_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id) ON DELETE RESTRICT,

    -- Frozen at the moment this item is added to the invoice
    description TEXT NOT NULL,
    unit TEXT NOT NULL,
    quantity_thousandths INTEGER NOT NULL CHECK (quantity_thousandths > 0),
    unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),

    line_discount_type TEXT CHECK (line_discount_type IN ('AMOUNT', 'PERCENTAGE')),
    line_discount_value INTEGER,

    tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    tax_rate_basis_points INTEGER NOT NULL DEFAULT 0,

    -- Computed and persisted at save time — see §4 (snapshot strategy)
    line_subtotal_minor INTEGER NOT NULL DEFAULT 0,
    line_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    invoice_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    taxable_amount_minor INTEGER NOT NULL DEFAULT 0,
    line_tax_minor INTEGER NOT NULL DEFAULT 0,
    line_total_minor INTEGER NOT NULL DEFAULT 0,

    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_invoice_line_items_invoice ON invoice_line_items(invoice_id);
CREATE INDEX idx_invoice_line_items_product ON invoice_line_items(product_id);

CREATE TABLE payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    method TEXT NOT NULL CHECK (method IN ('CASH', 'BANK_TRANSFER', 'UPI', 'CHEQUE', 'OTHER')),
    paid_on TEXT NOT NULL,
    reference TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_payments_invoice ON payments(invoice_id);
CREATE INDEX idx_payments_paid_on ON payments(paid_on);
```

## Round 3 definition of done

Every table, column, relationship, index, and delete/archive rule needed by the locked Round 2 flows is specified above, with money/quantity/tax kept as exact integers end to end. Round 4 (application architecture detail) designs the repository/service layer against this schema; Round 6 (calculation engine) fills in the exact arithmetic that computes the `*_minor` columns this schema already has a place for.
