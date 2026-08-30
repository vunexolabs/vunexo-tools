---
status: draft
round: 1
---

# Vunexo Billing — V2 Scope Research (Round 1, draft)

This is a discovery document, not a locked spec. Nothing here is decided — it exists to give you something concrete to react to (cut, add, reorder) rather than starting from a blank page. Once you lock a scope, that becomes a new `.ai/product-v2.md` (or amendments to `product.md`), the same way Round 1 worked for V1.

## Ground rule carried over from V1

`.ai/product.md` explicitly locks a list of things V1 deliberately does *not* do: full accounting, payroll, inventory, POS, CRM, banking integration, payment gateway integration, cloud sync, AI features, mobile apps, multi-company SaaS, subscriptions, complex GST filing, e-commerce, multiple templates. That list exists on purpose — the whole positioning is "does invoicing extremely well, isn't trying to become an ERP." V2 should be judged against that same discipline: does a candidate feature deepen the invoicing core, or does it start dragging the product toward being something else? A few candidates below fail that test on purpose, flagged as such.

## Target user for V2 — needs your call

V1's target user (`.ai/product.md`) is broad: retail shops, freelancers, service providers, contractors, traders, distributors, home businesses, repair businesses, agencies, independent professionals — implicitly India-first (GST-aware from day one, ₹ as the working example currency throughout the docs, though display currency is already 60-country-wide).

V2's target user determines which candidates below actually matter:

- **Same user, same country, deeper workflow** — the existing India-based user doing more of their billing lifecycle in Vunexo Billing (quotes, recurring bills, reports) instead of switching to a spreadsheet for the parts we don't cover yet.
- **Same user, going international** — an existing user's business now has customers or a footprint in a second country, and the India-only tax model breaks for those invoices.
- **New user in a different country entirely** — someone in the US/UK/EU/wherever picking up Vunexo Billing for the first time, for whom the India-only tax model is a hard blocker on day one, not an edge case.

The last two both point at multi-country tax, but they're different bets: "deepen for existing users" doesn't need it at all, while "new user, different country" needs it as a *prerequisite*, not a feature alongside others.

## Candidate list, scored against four axes

Axes: **frequency** (how often would a user touch this), **reach** (how many of V1's target user types benefit), **complexity** (rough implementation weight given the current architecture), **fit** (does it deepen invoicing, or drift toward the explicitly-out-of-scope ERP territory).

| Candidate | Frequency | Reach | Complexity | Fit | Notes |
|---|---|---|---|---|---|
| **Multi-country tax support** | N/A — foundational | High if targeting non-India users at all | High | Deepens core | Not really a "feature" — it's the tax engine's boundary. `calculation-engine.md` §5's GST split and `is_interstate` are India-specific all the way into the schema (`database-schema.md`), not just a display layer. Currency is already solved (`lib/currency.ts`, 60 countries) — tax is not. |
| **Quotes/Estimates → convert to Invoice** | High | High (contractors, agencies, service providers all quote before billing) | Medium | Deepens core | Very close to what's already built — same snapshot architecture, same line-item/discount/tax engine, a new status lane and a numbering series, plus a "convert" action that copies a Quote's lines into a new Draft Invoice. |
| **Customer statements** (running balance, print/export) | Medium | Medium–high | Low–medium | Deepens core | Customer balance is already tracked (`Customers` dashboard). This is mostly a report + PDF template reusing data that already exists — small surface area for real trust-building value ("here's everything you owe me," handed to a customer). |
| **Sales / tax summary reports** | Medium | High | Low–medium | Deepens core | SQL aggregation over existing tables (similar to `DashboardRepository`), exported via the CSV/JSON pipe that already exists. A GST summary report (not filing — a report to hand an accountant) is high-value and stays inside the "no complex GST filing" boundary already locked in V1. |
| **UPI/payment QR on invoice PDF** | Low (one-time setup, then automatic) | Medium (India-specific) | Low | Deepens core | `business.upi_id` is already a stored field with nothing done with it. Generating a static UPI deep-link QR code on the PDF is offline (no gateway, no cloud call) and small — doesn't violate the "no payment gateway integration" lock, since nothing processes the payment, it just encodes a link the customer's own UPI app reads. |
| **Recurring invoices** | High for the subset of users who bill retainers/subscriptions | Medium (not every user type bills recurringly) | Medium–high | Deepens core, but has a real design question | The complexity isn't the invoice generation — it's *what triggers it* in an offline-first desktop app with no background server. Realistic V2 shape: "due recurring invoices" surfaced on next app launch for one-click generation, not a true unattended cron. Needs its own design round before estimating further. |
| **Import from spreadsheet/other tool** | Low (one-time, at onboarding) | Medium–high (adoption lever, not retention) | Medium | Adjacent, not core | Real value for *getting* new users off Excel, but doesn't deepen anything for existing users. Better framed as a growth/adoption project than a V2 feature round. |
| **Expense tracking** | High if included | High | Medium–high | **Drifts toward accounting** | This is the first candidate that starts to cross into "full accounting," which V1 locks out explicitly. Possible later, but needs its own explicit scope decision, not a default V2 inclusion. |
| **Purchase/vendor management, inventory/stock** | High if included | Medium (mainly retail/traders) | High | **Explicitly locked out in V1** | Same list V1's spec calls out by name. Including this needs a deliberate reversal of that lock (an ADR), not a quiet scope-creep. |
| **Multiple invoice templates** | Low | Low–medium | Medium (needs a template abstraction the current single-template renderer doesn't have) | Cosmetic | Explicitly locked out in V1 as "no template builder, no theme engine." Lower leverage than any of the above — doesn't unlock new users or deepen workflow, just changes how existing output looks. |
| **Payment gateway integration** | — | — | — | **Explicitly locked out in V1** | Violates "no cloud dependency ... never a dependency of core functionality." Would need a deliberate architecture decision to introduce an optional, non-core integration — not a default V2 candidate. |

## What this suggests (a starting point, not a decision)

If the goal is "V2 = the natural next slice that keeps the product's identity intact and doesn't need a new architectural bet," the shape that falls out of the table above is:

**Anchor:** Multi-country tax support — because it's the one item already flagged, and because two of the three "target user" framings above (international existing user, new non-India user) are flatly blocked without it today.

**Paired with it**, because they're low-to-medium complexity, reuse the existing snapshot/calculation architecture almost directly, and each closes a real gap a user hits today:
- Quotes/Estimates → Invoice conversion
- Customer statements
- Sales/tax summary reports
- UPI QR on the PDF (small, but genuinely free given the field already exists)

**Deliberately deferred, not rejected:**
- Recurring invoices — real value, but needs its own design round for the "what triggers generation offline" question before it belongs in an estimate.
- Import/migration — a growth lever, arguably a separate initiative rather than a V2 feature.
- Expense tracking, inventory/purchase, payment gateway, multiple templates — each requires *reopening* an explicit V1 lock, which should be a conscious ADR-level decision, not something that rides in quietly as part of a bigger V2 batch.

## Open questions for you

1. **Target user**: is V2 primarily about unblocking non-India users, or about deepening the workflow for existing India-based users (or both, with multi-country tax as the technical prerequisite either way)?
2. **Scope**: does the anchor + paired list above look right, or do you want to pull in / cut specific items (e.g., recurring invoices matters enough to design now, or the report/statement items are lower priority than they look here)?
3. **Any of the explicitly-locked-out items** (inventory, expense tracking, templates, payment gateway) — worth reopening the lock for any of these in V2, or leave every one of them for a hypothetical V3+?
4. Once scope is picked, the next round is the same shape V1 used: user flows → schema/architecture deltas → calculation-engine changes (multi-country tax is the one item here that touches the money-math core, everything else is additive) → implementation.
