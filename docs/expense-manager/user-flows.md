---
status: locked
round: 2
---

# Vunexo Expense Manager — User Flows (Round 2)

Builds on `.ai/product-expense-manager.md` (Round 1, locked). Mirrors the shape of `docs/vunexo-billing/user-flows.md`.

## 1. First-run flow (business setup)

On first launch with no business profile: a setup form (name, address, tax info — all optional except name) must be completed before any other screen is usable. Same discipline as Billing.

## 2. Returning-user entry flow

Dashboard is the default landing screen, same as Billing.

## 3. Vendor creation / management flow

Create/edit/delete a vendor (name, contact, notes). Delete is blocked (soft-block, same as Billing's `has_invoices` pattern) if the vendor has any expenses recorded — `has_expenses` check instead. No history/balance concept (vendors aren't owed money the way customers are in Billing — expense manager tracks *outgoing* spend, not receivables).

## 4. Category management flow

Create/edit/delete a category (name, default tax-deductible flag). A starter set is seeded on first run (e.g. Rent, Utilities, Office Supplies, Travel, Professional Fees, Software/Subscriptions, Marketing, Miscellaneous) — exact list finalized in implementation, not frozen here. Delete is blocked if any expense references the category (`has_expenses` check, same pattern as vendor delete).

Editing a category's name or its default-deductible flag must never change the deductibility already recorded on existing expenses (per Round 1's historical-immutability principle) — an expense stores its own deductibility at creation time, the category's flag is only ever a *default* applied when the expense is first created.

## 5. The core flow: expense entry

1. User opens "New Expense".
2. Picks or quick-adds a vendor (searchable picker, same UX pattern as Billing's `SearchablePicker`).
3. Picks a category — its default deductibility classification pre-fills the expense's own deductibility field, editable per Round 1 (category flag is a default, not a rule).
4. Enters date, amount, payment method, tax amount, notes.
5. Optionally attaches a receipt image.
6. Saves. No draft/issued state machine — unlike an invoice, an expense is either recorded or it isn't. No multi-status lifecycle.

### Expense edit / delete

An expense can be edited or deleted at any time (no locked/issued state — expenses aren't shared with anyone outside the business, unlike invoices). Editing re-evaluates nothing retroactively; it simply updates the stored row. Deleting removes the row and its receipt attachment (if any).

## 6. Receipt attachment flow

Attach an image file (JPEG/PNG) to an expense at creation or via edit. View/replace/remove the attachment from the expense's own screen. No OCR, no auto-fill from the image (Round 1 boundary).

## 7. Reports flow

From the Reports section: pick a report (Category Summary, Period Summary, Deductible/Non-Deductible Summary, Tax/ITC Summary, Top Vendors), pick a date range, view the result, optionally export (CSV/JSON) via the same generic export mechanism Billing's Reports/Statement screens use (`write_export_file`-style: frontend renders the export text, backend just writes it to a chosen path).

## 8. Dashboard flow

Landing screen: this period's total spend, a category breakdown (chart or table), a recent-expenses list. Clicking a category row filters the Expenses list to that category (mirrors Billing's Overdue-card click-through pattern).

## 9. Backup / restore flow

Identical shape to Billing's: `.vbx`-style zip archive (or a new extension, e.g. `.vex`), `VACUUM INTO` snapshot, staged restore + app restart. Must additionally bundle receipt attachment files, since those live outside the database (exact packaging is a Round 3/4 decision — flagged here, not designed).

## Round 2 definition of done

- Every screen a user touches in V1 has a named flow above: business setup, vendor CRUD, category CRUD, expense CRUD, receipt attach/view/remove, reports (5 kinds), dashboard, backup/restore.
- No flow implies a multi-status state machine for expenses (explicitly ruled out — expenses are simpler than invoices).
- Historical-immutability principle from Round 1 is reflected concretely: category edits don't retroactively change existing expenses' stored deductibility.
