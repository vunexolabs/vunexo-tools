---
status: locked
round: 1
---

# Vunexo Billing — V1 Product Spec

This is an AI context file. Before modifying product-facing code, read this document. Do not violate a locked decision below without explicitly proposing a change (an ADR under `.ai/decisions/`) first.

## Vision

Free, open-source, offline-first billing/invoicing software for small businesses (retail shops, freelancers, service providers, contractors, traders, small distributors, home businesses, repair businesses, small agencies, independent professionals). No forced account, no subscription, no ads, no cloud dependency. The user's data stays on their machine.

## UX objective (not a hard requirement)

First invoice created in under 3 minutes from install; subsequent invoices in under 30 seconds. This is a design target to validate via usability testing later, not a V1 acceptance gate.

## In scope — V1

- **Business profile**: name, logo, address, phone, email, tax/GST info, bank details, UPI ID.
- **Customers**: create/edit/delete, history, balance.
- **Products/services**: name, SKU, description, price, tax, unit.
- **Invoices**: create/edit/duplicate/draft, invoice numbering, invoice date, due date, line items, discount, tax, notes, terms.
- **Invoice statuses**: `DRAFT`, `ISSUED`, `PARTIALLY_PAID`, `PAID`, `OVERDUE`, `CANCELLED`. No more than these six.
- **Payments**: paid / partially paid / unpaid, method, date, reference.
- **PDF**: one professional invoice template, print, save as PDF. Exactly one template in V1 — no template builder, no theme engine.
- **Dashboard**: today's sales, this month, outstanding, paid, overdue, invoice list.
- **Data**: backup, restore, JSON export, CSV export.

## Tax

GST-aware from the start (GSTIN, HSN/SAC, CGST/SGST/IGST, tax-inclusive vs. tax-exclusive pricing), but the tax model is generalized so non-GST regimes can be added later without a rewrite. The concrete calculation engine (rounding rules, line-level vs. invoice-level rounding) is a Round 6 decision, not frozen here.

## Explicitly out of scope — V1

Full accounting, payroll, inventory management, POS, CRM, banking integration, WhatsApp API integration, payment gateway integration, cloud sync, AI features, mobile apps, multi-company SaaS, subscription system, complex GST filing, e-commerce integration, multiple invoice templates.

## Architecture principles frozen into the spec

- **Invoice snapshotting**: an invoice stores a snapshot of the customer and product/line-item data as they were at creation time, not just foreign keys. Editing a customer or product later must never change historical invoices.
- **Money handling**: financial calculations must never use binary floating-point arithmetic. The exact representation (integer minor units vs. an exact decimal type) is a Round 6 (calculation engine) decision — not locked here.
- **No auth, no cloud in V1**: local SQLite only. Cloud/sync is a possible future, optional extension — never a dependency of core functionality.

## License direction

MIT recommended. Final decision deferred until third-party dependencies/templates used are reviewed.

## V1 Definition of Done

V1 is complete only when a user can:

- Create a business profile.
- Create/manage customers.
- Create/manage products/services.
- Create, edit, duplicate, and issue invoices.
- Record full/partial payments.
- Have configured taxes calculated correctly.
- Generate/print the invoice PDF.
- View dashboard metrics.
- Backup and restore all local data.
- Export supported data.
- Operate entirely offline.
- Install and run on supported desktop platforms (Windows, macOS, Linux).
- Trust that existing invoice history remains unchanged when a customer/product record is later edited.

## Roadmap

1. Spec + foundation (this document, ADR-001, repo/app skeleton) — **done**
2. Complete user flows (business setup → customer → product → invoice → payment → PDF)
3. Database schema (tables, relationships, indexes)
4. Application architecture detail (Electron/Tauri/SQLite/IPC-command boundaries)
5. UI/UX structure (every screen, navigation)
6. Invoice calculation engine (discount, GST/CGST/SGST/IGST, rounding, payments, money representation)
7. Implementation
8. Testing
9. Release (docs, licensing, packaging, CI)
