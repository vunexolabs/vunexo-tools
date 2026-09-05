---
status: locked
round: 5
---

# Vunexo Expense Manager — UI/UX Structure (Round 5)

Builds on Rounds 1–4. Mirrors `docs/vunexo-billing/ui-ux.md`'s conventions (screen inventory table, flat sidebar nav, cross-cutting patterns).

## 1. Screen inventory

| Screen | Locked flow it implements | Primary commands it calls |
|---|---|---|
| Business Setup (first-run) | `user-flows.md` §1 | `create_business`, `get_business` |
| Dashboard | §2, §8 | `get_dashboard_metrics` |
| Expenses List | §5 (entry point), filterable by category/vendor/date | `list_expenses` |
| Expense Editor | §5 (core flow), §6 (receipt attach) | `create_expense`, `update_expense`, `delete_expense`, `attach_receipt`, `replace_receipt`, `remove_receipt`, plus vendor/category inline-create commands |
| Vendors List | §3 | `list_vendors` |
| Vendor Detail | §3 | `create_vendor`, `update_vendor`, `delete_vendor` |
| Categories (single screen, inline-edit table — no separate list/detail, matches Billing's Tax Rates pattern) | §4 | `create_category`, `update_category`, `delete_category`, `list_categories` |
| Reports | §7 | `generate_category_summary`, `generate_period_summary`, `generate_deductible_summary`, `generate_tax_itc_summary`, `generate_top_vendors`, `write_export_file` |
| Settings — Business Profile | `.ai/product-expense-manager.md` business profile fields | `update_business` |
| Settings — Data | §9 (backup/restore) + export | `backup_data`, `restore_backup` |

## 2. Navigation structure

```
App shell
├── Dashboard              (default route)
├── Expenses
│   ├── Expenses List
│   └── Expense Editor       (new / edit — one component)
├── Vendors
│   ├── Vendors List
│   └── Vendor Detail
├── Categories               (single inline-edit table screen)
├── Reports
└── Settings
    ├── Business Profile
    └── Data
```

A persistent sidebar holds the five top-level sections (Dashboard, Expenses, Vendors, Categories, Reports, Settings) — flat, no nested menus, mirroring Billing's rationale that V1's entire scope fits without a nested menu. Business Setup is a full-screen gate shown instead of the shell when `get_business` returns nothing, matching Billing exactly.

## 3. Cross-cutting UI patterns

Reuse the same interaction patterns Billing already validated, rather than inventing new ones:

- `ConfirmDialog` for destructive actions (delete vendor/category/expense).
- `SearchablePicker` + quick-add modal for picking a vendor or category inline from the Expense Editor, same as Billing's customer/product picker in the Invoice Editor.
- Blocked-delete messaging: attempting to delete a vendor/category with expenses shows the same clear "can't delete, N expenses reference this" message Billing uses for `has_invoices`.

## 4. Expense Editor (the core screen)

Fields in entry order, mirroring `user-flows.md` §5: vendor (searchable picker, optional), category (searchable picker, required — selecting one pre-fills the deductible toggle from `categories.default_deductible`, editable), date, amount, tax amount, an ITC-eligible toggle (separate from deductible, per Round 1/3), payment method (fixed small set + free text), notes, receipt attachment (file picker + thumbnail/preview once attached, replace/remove actions). No draft/issued state, no "Issue" button — one "Save" action, per Round 2's explicit no-state-machine decision.

## 5. List screens (Expenses, Vendors)

Same shape as Billing's list screens: a filterable/sortable table, row actions (edit, delete), a primary "New" button. Expenses List additionally supports the category/vendor/date-range filters `user-flows.md` §7's reports also use, so a user can pivot from "view a report" to "see the underlying rows" in one click (mirrors Billing's Overdue-card → filtered-list click-through).

## 6. Categories screen

Single inline-edit table (name + default-deductible toggle per row + delete action), matching Billing's Tax Rates screen exactly — no separate create/detail screens needed for a small, flat list.

## 7. Reports screen

A report picker (5 kinds, §7 of user-flows) + date range + result table/summary + an export button (CSV/JSON via `write_export_file`), matching Billing's Reports screens (`features/reports/`) structure directly — same generic pattern, applied to this domain's report kinds instead of Sales/Tax Summary.

## 8. Frontend module mapping

```
src/features/
├── business/       (Business Setup, Settings → Business Profile)
├── dashboard/
├── expenses/        (ExpensesList, ExpenseEditor)
├── vendors/          (VendorsList, VendorDetail)
├── categories/         (Categories inline-edit table)
├── reports/              (ReportPicker + 5 report views)
└── settings/               (Data tab: backup/restore/export)
```

Same `hooks/useExpenses.ts`-style pattern as Billing's `hooks/useQuotes.ts` etc. — one hook per aggregate, wrapping the Tauri command bridge (`lib/tauri/types.ts`/`commands.ts`).

## Round 5 definition of done

- Every command named in Round 4 has a calling screen above.
- No screen implies a status/state-machine UI (no "Issue" button, no status badge) — consistent with Round 2/4's explicit decision.
- Reused Billing UI patterns are named, not redesigned from scratch.
