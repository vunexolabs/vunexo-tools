---
status: locked
round: 1
supersedes_research: .ai/product-v2-scope.md
---

# Vunexo Billing — V2 Product Spec

This is an AI context file, same status as `product.md`. Before modifying V2-scoped product-facing code, read this document. Do not violate a locked decision below without explicitly proposing a change (an ADR under `.ai/decisions/`) first. `.ai/product-v2-scope.md` is the Round 1 research trail this was decided from — read it for the *why*, not the *what*; this file is the *what*.

## Vision

V1 was "simple, free, offline-first invoicing." V2 is **a complete, flexible billing workflow — quote to payment — with a country-aware tax engine**, not "V1 plus international tax" as an afterthought.

## Target user

**Primary: existing V1 small-business users who need a more complete billing lifecycle**, not just invoice creation — quoting a job before billing it, sending a customer their running balance, pulling a sales/tax summary for their accountant, following up on an overdue payment. The tax engine becomes country-aware as part of this work (it's a real limitation the current architecture has, and touching the calculation engine for the workflow items above is the natural moment to fix it), but internationalization is an *enabler*, not the headline. V2 does not require abandoning the India-first market to become useful to it.

## In scope — V2

- **Multi-country tax architecture (foundation)**: generalize the tax engine beyond India-only GST so a business can be configured for a non-GST tax regime, without making Vunexo Billing an accounting or tax-filing product. This is architectural — it underpins every other V2 item that touches tax, but ships as infrastructure, not a standalone user-facing feature.
- **Quotes/Estimates → Invoice**: create/edit a Quote (reuses the existing line-item/discount/tax/snapshot architecture), issue it, and convert an accepted Quote into a Draft Invoice.
- **Customer statements**: a printable/exportable running-balance view per customer, reusing data already tracked (`Customers` dashboard, invoice/payment history).
- **Sales & tax summary reports**: SQL-aggregated reports (sales by period/product/customer, a GST/tax summary) exported via the existing CSV/JSON pipeline. A report to hand an accountant — not filing, not a compliance product.
- **Payment reminders / follow-up**: overdue detection already exists (V1 dashboard); V2 adds a "generate reminder" action producing a copyable/printable/shareable reminder message. No WhatsApp API integration, no email sending infrastructure — local generation only, matching the no-cloud-dependency principle.
- **UPI QR on invoice PDF**: small enhancement, not a scope driver. `business.upi_id` already exists as a stored field; render it as a static UPI deep-link QR on the PDF. Purely offline (encodes a link, doesn't process anything) — doesn't touch the "no payment gateway integration" lock.

## Explicitly deferred — own initiative, not V2 core

- **Recurring invoices**: real value, but the open question ("what triggers generation in an offline desktop app with no background server") needs its own design round before it has an estimate. Explicitly **not** to be solved by introducing a cloud service for scheduling — that would break the no-account/no-cloud-dependency principle this product is built on.
- **Spreadsheet/Excel import**: valuable as a growth/adoption lever for users switching off Excel, but doesn't deepen the workflow for existing users. Tracked as a separate growth/migration initiative, not part of V2's product scope.

## Explicitly out of scope — still locked from V1

Unchanged from `product.md`: full accounting, payroll, inventory management, POS, CRM, banking integration, WhatsApp API integration, payment gateway integration, cloud sync, AI features, mobile apps, multi-company SaaS, subscription system, complex GST filing, e-commerce integration, multiple invoice templates. **None of these are reopened by V2.** Reopening any one of them requires a deliberate ADR, never a quiet inclusion inside a larger batch of work.

## Architecture principles carried over from V1

All of `product.md`'s frozen principles still apply: invoice snapshotting, no binary floating-point money math, no auth/no cloud dependency for core functionality. The multi-country tax work must not compromise any of these — a non-India tax config is still computed locally, still snapshotted onto issued documents the same way GST is today.

## V2 Definition of Done

V2 is complete only when, in addition to everything V1's DoD already requires:

- A business can be configured for a tax regime other than India GST, and invoices/quotes calculate and display correctly under it.
- A user can create a Quote, issue it, and convert it into an Invoice without re-entering line items.
- A user can generate and export a customer statement.
- A user can generate a sales/tax summary report and export it.
- A user can generate a payment reminder for an overdue invoice.
- A UPI QR code renders correctly on the PDF when `business.upi_id` is set, and is absent when it isn't.

## Roadmap

1. V2 scope research + lock (this document, `.ai/product-v2-scope.md`) — **done**
2. User flows: Quote lifecycle, Quote→Invoice conversion, customer statement, reports, payment reminder
3. Database schema deltas (Quotes table + numbering series, tax-regime configuration, whatever a statement/report view needs)
4. Application architecture deltas (new IPC commands/repositories, tax-regime abstraction boundary)
5. UI/UX structure for the new screens/actions
6. Calculation engine changes — this is the one round that touches the money-math core (generalizing tax beyond the GST-specific `is_interstate`/CGST-SGST-IGST split in `calculation-engine.md` §5); everything else in V2 is additive on top of the existing engine
7. Implementation
8. Testing
9. Release
