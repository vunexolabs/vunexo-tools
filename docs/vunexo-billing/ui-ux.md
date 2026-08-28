---
status: locked
round: 5
---

# Vunexo Billing — UI/UX Structure (Round 5)

This is an AI context file. It designs every screen and navigation path implied by the locked flows (`user-flows.md`) against the locked command surface (`application-architecture.md` §5). It does not add scope: every screen here exists because a locked flow or a V1 feature in `.ai/product.md` needs it, and nothing here invents a feature those documents didn't already commit to.

## 1. Screen inventory

| Screen | Locked flow it implements | Primary commands it calls |
|---|---|---|
| Business Setup (first-run) | `user-flows.md` §1 | `create_business`, `get_business` |
| Dashboard | §2, §8 | `get_dashboard_metrics` |
| Invoices List | §5 (entry point) | `list_invoices` |
| Invoice Editor | §5 (the core flow) | `create_draft_invoice`, `update_draft_invoice`, `issue_invoice`, `cancel_invoice`, `delete_draft_invoice`, `duplicate_invoice`, plus customer/product inline-create commands |
| Invoice PDF Preview | §5 step 5, §7 | (reads the already-computed invoice; PDF generation itself is Round 6/7) |
| Customers List | §3 | `list_customers` |
| Customer Detail | §3 | `create_customer`, `update_customer`, `archive_customer`, `restore_customer`, `delete_customer` |
| Products List | §4 | `list_products` |
| Product Detail | §4 | `create_product`, `update_product`, `archive_product`, `restore_product`, `delete_product` |
| Record Payment (panel, not a full screen) | §6 | `record_payment`, `update_payment`, `delete_payment` |
| Settings — Business Profile | `.ai/product.md` business profile fields | `update_business` |
| Settings — Tax Rates | Round 3 `tax_rates` | `create_tax_rate`, `update_tax_rate`, `list_tax_rates` |
| Settings — Invoicing | numbering format (locked after first issue), default due days | `update_settings` |
| Settings — Data | §9 (backup/restore) + JSON/CSV export (`.ai/product.md`, not detailed in Round 2 — filled in below, §6) | `backup_database`, `restore_backup`, `export_data` (new — §6) |

No standalone "Reports" screen: `.ai/product.md` locks dashboard metrics as the only V1 reporting surface — a separate reports section was in the original vision doc but was never carried into the locked spec, so it isn't designed here.

## 2. Navigation structure

```
App shell
├── Dashboard                (default route)
├── Invoices
│   ├── Invoices List
│   └── Invoice Editor        (new / edit / view — one component, see §4)
├── Customers
│   ├── Customers List
│   └── Customer Detail
├── Products
│   ├── Products List
│   └── Product Detail
└── Settings
    ├── Business Profile
    ├── Tax Rates
    ├── Invoicing
    └── Data
```

A persistent sidebar (matching `src/app/` + `src/components/`) holds the five top-level sections (Dashboard, Invoices, Customers, Products, Settings) — flat, no nested menus, since V1's entire scope fits in five sections. Business Setup isn't a sidebar destination; it's a full-screen gate shown instead of the shell when `get_business` returns nothing (per `user-flows.md` §1), and is never reachable again once a business profile exists.

## 3. Cross-cutting UI patterns

- **Status badges**: one fixed color per `InvoiceStatus` plus the derived `OVERDUE` badge — `DRAFT` neutral/gray, `ISSUED` blue, `PARTIALLY_PAID` amber, `PAID` green, `CANCELLED` struck-through gray, `OVERDUE` red (rendered as an additional badge alongside whatever the stored status is, per the derived-badge rule in `database-schema.md` §8 — never replaces the stored status badge).
- **Archive/delete decided by `has_invoices`, not by trial and error**: `list_customers`/`list_products` return a `has_invoices` flag per row (`application-architecture.md` §3b — computed in SQL via `EXISTS`, never by loading invoices client-side), and the row menu is built directly from it:

  ```
  Active, has_invoices = false → Edit · Archive · Delete
  Active, has_invoices = true  → Edit · Archive · (Delete unavailable)
  Archived                     → Restore
  ```

  The UI never attempts a delete and catches the resulting `Conflict` as its primary mechanism — it already knows which action is valid before the user opens the menu. The rare race (a row becomes referenced between page load and the delete click) still falls back to the `Conflict` error rendering below, but that's the exception path, not the design.
- **Confirmation dialogs**, required before: `CancelInvoice` (reason field, optional, per `user-flows.md` §5), `DeleteDraftInvoice`, `restore_backup` (the "this replaces all current data" confirmation locked in §9). Not required for archive/restore (reversible, low-stakes) or for ordinary saves.
- **Error mapping**: `ApplicationError` (Round 4 §6) renders as: `Validation` → inline field-level message near the offending input; `Conflict` → an inline banner with the specific message the use case already composed (e.g. "archive instead?"); `NotFound` → redirect to the relevant list with a toast; `Infrastructure` → a generic "something went wrong, your data is safe" toast (never shows the wrapped message verbatim, since it may contain SQL/path fragments even after the `InfrastructureError` → `ApplicationError` translation's best effort).
- **Searchable pickers**: customer and product selection (in the Invoice Editor and anywhere else they're referenced) share one component — type-to-filter over `list_customers`/`list_products` (active only), with a persistent "+ Create new…" row at the bottom that opens the same Customer/Product Detail form inline, per `user-flows.md` §3/§4's dual entry-point rule.
- **The frontend never calculates financial totals** (`application-architecture.md` §4a) — every subtotal/discount/tax/total shown anywhere (Invoice Editor, Invoices List, Dashboard) is a value the backend returned, never a value React computed from raw line items. A line-item edit round-trips through `update_draft_invoice` and re-renders whatever `InvoiceCalculationResult` comes back, rather than showing an optimistic client-side estimate first.
- **Mutations that affect an invoice's `status` refetch that invoice wherever it's currently shown.** Recording, editing, or deleting a payment (§6 of `user-flows.md`) changes `status` as a side effect the payment panel itself doesn't return inline — so a successful payment mutation invalidates/refetches the parent invoice (if the Invoice Editor has it open) and the Invoices List / Dashboard queries (if either is mounted), rather than requiring a manual refresh. This is a data-fetching-hook concern, not a reason to introduce global state (§7).

## 4. Invoice Editor (the core screen)

One component, four modes — `CreateDraft` / `EditDraft` / `EditIssued` / `ViewCancelled` — driven by `invoice status + editor mode` together, not status alone: `Issued`/`PartiallyPaid`/`Paid` all share the `EditIssued` mode (same fields, same footer), so the mode set is smaller than the status set, but it's still the mode, not the raw status string, that the component branches on. Fields and layout barely change across modes, which is why this stays one component rather than three-plus screens:

```
┌──────────────────────────────────────────────────────┐
│ ← Back              [DRAFT]                            │
├──────────────────────────────────────────────────────┤
│ Customer        [ Search or select customer      ▾ ]  │
│                                                        │
│ Next invoice number • automatic          Date  28 Aug │
│ INV-2026-0007                             Due   12 Sep │
│ Use a custom number instead →                          │
├──────────────────────────────────────────────────────┤
│ Item                    Qty   Rate      Tax    Total   │
│ ──────────────────────────────────────────────────    │
│ [product picker row — same pattern as customer]        │
│ + Add item                                             │
├──────────────────────────────────────────────────────┤
│ Discount  [ Amount ▾ / Percentage ▾ ]  [    ]           │
│ Notes     [                                    ]        │
│ Terms     [                                    ]        │
├──────────────────────────────────────────────────────┤
│ Subtotal            ₹52,000                            │
│ Discount            -₹2,000                             │
│ Tax                  +₹9,000                             │
│ TOTAL                ₹59,000                              │
├──────────────────────────────────────────────────────┤
│ [Save Draft]   [Preview]   [Issue]   [Issue & PDF]       │
└──────────────────────────────────────────────────────┘
```

Behavior by mode (all locked in `user-flows.md` §5):
- **`CreateDraft` / `EditDraft`** (invoice `status = Draft`): every field editable; footer shows `Save Draft` / `Preview` / `Issue` / `Issue & PDF`.
  - The invoice number is labeled **"Next invoice number • automatic"**, not presented as if the draft already owns that number — because it doesn't. `preview_next` (`application-architecture.md` §3b) is read-only and non-reserving, so a second open draft could preview the same number, and whichever one is issued first actually gets it. The label exists specifically to avoid the confusion of a draft appearing to "lose" the number it displayed if another invoice is issued first.
  - **Custom number toggle**: default is automatic (the label/preview above, no input shown). "Use a custom number instead" reveals a plain text field (`OLD-INV-1042`-style), deliberately one click away rather than a default-visible input, per the "opt-in, not in the way of the default flow" rule (`database-schema.md` §7). Once issued, whichever number was used — generated or custom — displays plainly with no further affordance to change it.
- **`EditIssued`** (invoice `status ∈ {Issued, PartiallyPaid, Paid}`): all fields still editable (per the locked "editing issued invoices" rule); invoice number is now the real, immutable number (no preview label, no custom-number toggle — both were `Draft`-only concepts); footer replaces `Issue`/`Issue & PDF` with `Save Changes`, and adds `Record Payment`, `Duplicate`, `Cancel`, `Print/Save PDF`. Editing a `Paid` invoice whose new total is below the amount already paid shows the overpayment banner inline (per `user-flows.md`'s "Editing an issued invoice" rule) rather than blocking the save.
- **`ViewCancelled`** (invoice `status = Cancelled`): entire form read-only; footer only shows `Duplicate`. A persistent banner states it's cancelled, plus the reason if one was given.

The **Preview** action (available at any status) opens the read-only PDF-shaped rendering in a side panel or modal without navigating away from the editor — per `user-flows.md`'s "reachable at any point without leaving the draft."

## 5. List screens (Invoices, Customers, Products)

All three share one layout: a filter/search bar, a table, and a primary "+ New" action.

- **Invoices List**: filters by status (including the derived `OVERDUE` badge as a filterable pseudo-status, per `database-schema.md` §8's `is_overdue` query) and a date range; each row shows number, customer, date, total, status badge; row actions include the quick "Record Payment" action from `user-flows.md` §6 for unpaid/overdue rows without opening the full editor.
- **Customers List** / **Products List**: filter by Active/Archived (Archived hidden by default, per the archive-hides-from-picker rule); row actions are Edit, Archive/Restore, and Delete (only enabled when unreferenced, per §3's pattern above).

**Tax Rates**, referenced in §1, is a simple list-and-inline-edit table under Settings (name + rate%), not a dedicated master-detail flow — it's small, low-cardinality master data (a handful of GST slabs), so a full list/detail screen pair would be more scaffolding than the data warrants. Its `create_tax_rate`/`update_tax_rate`/`list_tax_rates` commands (`application-architecture.md` §3b/§4, added there during this round's review) follow the exact same thin-command/use-case pattern as everything else.

## 6. Settings — Data (backup, restore, export)

`user-flows.md` §9 fully specifies backup/restore; JSON/CSV export is locked V1 scope (`.ai/product.md`) but wasn't separately walked through in Round 2. Filling that gap here, minimally: three buttons — **Export Customers (CSV)**, **Export Products (CSV)**, **Export Invoices (CSV)** — each triggers a native "save file" dialog, no options/filters/scheduling. This needs one more command not named in Round 4 (`export_data(entity, format)`), following the same thin-command pattern. Nothing about this needs its own screen — it lives as a row of buttons directly under the existing Backup/Restore section from `user-flows.md` §9.

A fourth button, **Export All Data (JSON)**, is a distinct operation, not just "CSV but JSON-shaped" — precisely defined as: every table in `database-schema.md` §13 (`business`, `settings`, `tax_rates`, `customers`, `products`, `invoices`, `invoice_line_items`, `payments`), serialized as structured JSON, not a blind SQLite table dump (column names/types are the domain shapes from `application-architecture.md` §2, not raw DB column names). Three properties carry over from the backup contract in `user-flows.md` §9 since this is the same kind of operation at heart: **read-only** (touches nothing in the database), uses the same native save dialog as backup/CSV, and a failed export never affects application data — it either produces a complete file or nothing.

## 7. Frontend module mapping

Maps directly onto the `src/features/*` directories already scaffolded in Round 1:

```
features/
├── dashboard/    → Dashboard screen (§1)
├── customers/    → Customers List + Customer Detail (§5)
├── products/     → Products List + Product Detail (§5)
├── invoices/     → Invoices List + Invoice Editor + PDF Preview (§4, §5)
├── payments/     → the Record Payment panel (§1) — not a route, opened from invoices/
├── reports/      → intentionally empty in V1 (§1's "no standalone Reports screen" note) — left as a placeholder directory per Round 1's original scaffold, not deleted, since Round 2's own roadmap doc still lists it as a future area
└── settings/     → Business Profile, Tax Rates, Invoicing, Data (§1, §6)
```

`components/` holds the cross-cutting pieces from §3 (status badge, searchable picker, confirmation dialog, error banner/toast) shared across features. `hooks/` wraps each Tauri command from `src/lib/tauri/commands.ts` (Round 1) in a small data-fetching hook per screen's needs — no global client-state store is needed for V1: every screen's data is either a fresh command call on mount or the in-progress Invoice Editor's own local component state (an invoice draft has exactly one active editor at a time, single-user desktop app, nothing to share across components). `stores/` (scaffolded empty in Round 1) stays empty for V1 rather than adopting a state-management library to fill it — consistent with the earlier decision not to add Zustand or similar without a concrete need.

## Round 5 definition of done

Every screen implied by the locked flows and locked V1 scope has a place in the navigation, a specification of what it shows and which commands it calls, and a home in the existing `src/features/*` module layout. Designing this round surfaced real gaps in Round 4 — missing `Business`/`Settings`/`TaxRate` use cases, unnamed query use cases, and an under-specified `Customer`/`Product` list projection — all fixed directly in `application-architecture.md` (still `status: locked`, amended rather than reopened) rather than worked around here. The `export_data` command gap is named here since it's UI-scope, not application-architecture scope. Round 6 (calculation engine) fills in the exact arithmetic the Invoice Editor's live totals depend on; Round 7 (implementation) builds these screens against this spec.
