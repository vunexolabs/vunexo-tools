---
status: locked
round: 1
---

# Vunexo Expense Manager — V1 Product Spec

This is an AI context file. Before modifying product-facing code, read this document. Do not violate a locked decision below without explicitly proposing a change (an ADR under `.ai/decisions/`) first.

## Vision

Free, open-source, offline-first expense tracking for the same small-business user base as Vunexo Billing (retail shops, freelancers, service providers, contractors, traders, small agencies, independent professionals). A standalone companion tool, not an accounting suite.

**Product positioning: expense management for small businesses — not accounting.** This distinction is load-bearing: tax-deduction tracking, GST/ITC classification, and reporting can each gradually pull the product toward bookkeeping if the boundary isn't named explicitly. It is named here so every later round can check against it.

**Product boundary relative to Vunexo Billing**: Billing answers "what did I sell and who owes me?" Expense Manager answers "what did my business spend and what did I spend it on?" Two complementary tools, neither one an ERP.

## In scope — V1

- **Business profile**: business identity and optional tax-registration information required for expense records and reports (name, address, tax info). Independent from Billing's business profile, no shared table. Not a full tax-regime configuration surface — that remains Billing V2's territory.
- **Vendors/suppliers**: create/edit/delete, history.
- **Expense categories**: predefined starter set + custom. Each category carries a *default* tax-deductibility classification — not an authoritative tax rule — that can be overridden or explicitly recorded per expense. Whether V1's UI actually exposes that override is a user-flow-round (Round 2) decision, not decided here.
- **Expenses**: create/edit/delete; date, amount, category, vendor, payment method, tax amount, a tax/ITC classification, notes, optional receipt image attachment.
  - Tax paid and ITC-eligibility are separate facts, not one field — the exact data representation is a Round 3 (schema) decision, not designed here.
- **Reports**: totals by category, totals by period, deductible vs. non-deductible summary, tax/ITC summary, top vendors. Deliberately operational, not accounting statements.
- **Dashboard**: this period's spend, category breakdown, recent expenses.
- **Data**: backup, restore, CSV export, JSON export.

## Tax and receipt boundaries (locked)

- **Tax**: Expense Manager records tax information supplied by the user. It does not determine legal tax eligibility, compute statutory ITC entitlement, or provide tax advice.
- **Receipt attachments**: local-only supporting documents. V1 does not perform OCR, extraction, classification, or cloud upload. Storage mechanics (filesystem vs. in-DB, backup/restore handling, size/format limits) are a Round 3/4 decision, not designed here.

## Explicitly out of scope — V1

Full double-entry accounting/ledger, payroll, budgeting/forecasting, recurring expenses, bank statement import, receipt OCR/automatic data extraction, multi-currency, multi-user, cloud sync, invoicing (that remains Vunexo Billing's job), tax filing, tax return preparation, tax advice, and automatic determination of legal tax deductibility/ITC eligibility.

No P&L, balance sheet, accounts payable, or general ledger reporting — those would change the product's category from expense management to accounting.

## Architecture principles frozen into the spec

- **Money handling**: financial calculations must never use binary floating-point arithmetic — integer minor units, same discipline as Vunexo Billing. The exact representation is a Round 3/6 decision, not frozen here.
- **No auth, no cloud in V1**: local SQLite only. Cloud/sync is a possible future, optional extension — never a dependency of core functionality.
- **Independent product data boundary**: own SQLite database, own business profile, no coupling to Vunexo Billing's data. This does not mean zero shared code — shared monorepo tooling and non-domain infrastructure (e.g. build tooling, lint config, possibly a currency table or money type) may be reused where appropriate. That's a Round 3/4 decision, not made here.
- **Historical immutability** (broadened from Billing's invoice-snapshot principle): historical expenses must remain immutable in meaning even when referenced master data changes later. A renamed vendor must not retroactively rewrite what an old expense shows; a renamed category must not either; a category's deductibility flag changing later must not silently flip old expenses from deductible to non-deductible.

## License

MIT — reuses the repo-wide decision already recorded in `.ai/decisions/ADR-002-license.md`. No new ADR needed.

## V1 Definition of Done

V1 is complete only when a user can:

- Create a business profile.
- Create/manage vendors.
- Create/manage expense categories, including their default deductibility classification.
- Create, edit, and delete expenses, each with category, vendor, payment method, tax amount, and tax/ITC classification.
- Attach an optional receipt image to an expense.
- Record tax information and an ITC classification for expenses and view corresponding summaries; the application does not determine statutory eligibility.
- View dashboard metrics (this period's spend, category breakdown, recent expenses).
- Generate reports: category totals, period totals, deductible/non-deductible summary, tax/ITC summary, top vendors.
- Backup and restore all local data, including receipt attachments where the backup format supports it.
- Export supported data (CSV, JSON).
- Operate entirely offline.
- Install and run on supported desktop platforms (Windows, macOS, Linux).
- Trust that existing expense history remains unchanged in meaning when a vendor/category record is later edited.

## Roadmap

1. Spec + foundation (this document, app skeleton) — **done**
2. Complete user flows (business setup → vendor → category → expense entry → reports)
3. Database schema (tables, relationships, indexes)
4. Application architecture detail (Tauri/SQLite/IPC-command boundaries)
5. UI/UX structure (every screen, navigation)
6. Expense/tax calculation engine (category totals, deductible/ITC summaries, rounding, money representation)
7. Implementation
8. Testing
9. Release (docs, licensing, packaging, CI)
