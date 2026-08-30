---
status: locked
round: 2
---

# Vunexo Billing — V2 User Flows (Round 2)

This is an AI context file, same status as `docs/vunexo-billing/user-flows.md`. Before designing V2's database schema (Round 3) or screens (Round 5), read this document — it is the source of truth for every new screen and state transition V2 adds. See `.ai/product-v2.md` for the locked V2 scope this was designed against, and `docs/vunexo-billing/user-flows.md` for the V1 flows these extend (invoice creation, payment recording, PDF, dashboard, backup/restore are unchanged by V2 unless explicitly noted below).

Scope: UI-agnostic, same discipline as V1's Round 2 — no mockups (Round 5), no schema (Round 3), no calculation formulas (Round 6). This document pins down entry points, required/optional fields, state transitions, and edge cases for: the Quote lifecycle, Quote→Invoice conversion, customer statements, reports, payment reminders, tax-regime configuration, and UPI QR.

## 1. Tax regime configuration flow

**Why this exists as its own flow**: V2's foundation item (`.ai/product-v2.md`) is that a business can be configured for a tax regime other than India GST. Someone has to make that choice, once, per business.

**Entry point:** Settings → Business Profile, a new **Tax Regime** field alongside the existing country/currency selection. Not a separate onboarding step — V1's first-run flow (`user-flows.md` §1) still only requires a business name; tax regime defaults to **India GST** (V1's only regime, and the existing default) and can be changed later without re-running setup.

**Behavior:**
- Changing a business's tax regime is a **forward-looking** setting, same principle as changing the business address: it affects new Draft invoices/quotes created after the change, never retroactively recalculates an already-issued document (issued documents carry their own snapshot, same as V1's business-snapshot principle).
- Switching regimes changes which fields are relevant on the invoice/quote line-item and totals UI — e.g. India GST shows GSTIN/HSN-SAC/CGST-SGST-IGST; a non-GST regime shows whatever that regime's model defines instead (exact field set is a Round 4/6 decision, not this one). **This flow only commits to**: the regime is a single business-level setting, not a per-invoice or per-customer choice, and switching regimes must not corrupt or reinterpret line items already added to an in-progress Draft under the old regime (a Draft's line-item tax rates were already frozen at add-time, same as V1 — switching regime mid-draft is an edge case the UI should warn about, not silently reinterpret).
- Tax Rates management (Settings → Tax Rates, existing V1 screen) becomes regime-scoped: the rates a business configures are the rates for *its selected regime*, not a flat global list. What "a rate" even consists of under a non-GST regime (e.g. does it still need an interstate/intrastate split) is Round 4/6 territory.

**Edge case:** a business with existing issued invoices switches regime — those invoices are untouched (already-snapshotted). Draft invoices/quotes in progress at the moment of the switch are flagged for review (their line items keep their already-frozen tax rates, but the invoice-level tax presentation now follows the new regime) rather than silently discarded or silently reinterpreted.

## 2. Quote lifecycle flow

**Entry points:** Quotes section → "+ New Quote"; or "Convert to Quote" is explicitly **not** offered from an existing invoice (quotes precede invoices, never the reverse — see §3).

Mechanically this reuses V1's invoice creation flow almost entirely (`user-flows.md` §5): customer picker, line items with products/quantities/prices/discounts/tax, invoice-level discount, notes/terms, a live PDF-style preview. The differences are the status lifecycle and what "issuing" produces.

### Quote state machine

```
DRAFT ──issue──▶ ISSUED ──accept──▶ ACCEPTED ──convert──▶ CONVERTED
                    │
                    ├──decline──▶ DECLINED
                    │
                    └──expire───▶ EXPIRED   (valid_until passed, still ISSUED)

DRAFT, ISSUED, ACCEPTED ──cancel──▶ CANCELLED
```

Settled here, per the open questions `.ai/product-v2.md` flagged for this round:

- **`ACCEPTED` is a real, stored state, not just an action.** A business needs to know "the customer said yes but I haven't billed it yet" as a distinct, visible state — that's real information (e.g. work can start), separate from "already invoiced."
- **A Quote is editable in `DRAFT` only.** Once `ISSUED`, it is immutable content-wise (same rationale as an issued invoice needing deliberate re-snapshotting, but simpler here — a Quote has no payment history to protect, so V2 does not extend V1's "edit an issued document" allowance to Quotes: if the numbers were wrong, cancel and duplicate into a new Draft, mirroring how V1 already treats a `CANCELLED` invoice).
- **`EXPIRED` is derived, not a stored transition the user triggers** — same pattern as V1's `OVERDUE` badge (`user-flows.md` §6): `is_expired = valid_until < today AND status == ISSUED`. It's a display badge, not something reachable via a "mark expired" button.
- **`CONVERTED` is terminal.** A Quote converts to an invoice **exactly once** — after conversion it cannot be converted again, edited, or reissued. This answers the "can one quote produce multiple invoices" question: no. (A business that genuinely wants to bill the same accepted work twice duplicates the *resulting invoice*, the normal V1 duplicate flow — not the quote.)
- **`ACCEPTED` quotes may also be cancelled, not only `DRAFT`/`ISSUED`.** Explicit invariant, not left for Round 3 to infer: acceptance means the customer agreed to the price, not that the job is guaranteed to happen — a customer backing out after accepting is a real scenario, and a business needs to be able to record "this isn't proceeding" rather than being stuck with a quote permanently stranded in `ACCEPTED` with no legal next state. Cancelling an `ACCEPTED` quote requires the same optional reason field V1 already uses for cancelling an invoice.
- **`CANCELLED` is terminal**, same as V1 invoices: no edits, no conversion, duplicate-only into a new Draft Quote.
- **Conversion produces an independent snapshot.** Converting a Quote copies its line items (products, quantities, prices, per-line discounts/tax), customer, quote-level discount, and notes/terms into a new `DRAFT` Invoice — exactly like V1's invoice-duplicate flow (`user-flows.md` §5's "Duplication, precisely" paragraph). The new Draft Invoice has no ongoing link back to the Quote's own line items: editing the (already-terminal, `CONVERTED`) Quote is impossible, so there's nothing to keep in sync, but the principle is stated explicitly here because it's the same snapshot discipline `.ai/product.md` locks for invoices, now applied one layer earlier.

**Numbering:** Quotes get their own sequential numbering series, independent of the Invoice series (e.g. `QUO-{year}-{seq}` vs `INV-{year}-{seq}`), same per-business/format-configured-once/never-reused rules as V1 invoice numbering (`user-flows.md` §"Numbering"). A converted Quote keeps its own `QUO-` number permanently (for traceability back from the resulting invoice) — the resulting Invoice gets a normal, newly-generated `INV-` number at conversion time, the same way a duplicated invoice gets its own number at its own eventual issue time.

**Fields specific to a Quote vs. an Invoice:** a **`valid_until`** date (defaults to today + N days, N configurable in Settings, mirroring how due-date defaulting already works) replaces "due date" conceptually — a Quote doesn't have a payment due date, it has a window during which the price is honored. Naming this precisely matters: Round 3 should model `quotes.valid_until`, never reuse or alias `invoices.due_date`'s name for a conceptually different field.

## 3. Quote → Invoice conversion flow

**Entry point:** an `ACCEPTED` Quote's detail view — a single "Convert to Invoice" action. Not available from any other Quote status (see state machine above).

1. User clicks "Convert to Invoice."
2. A new `DRAFT` Invoice is created, pre-filled with the Quote's customer, line items, quote-level discount, and notes/terms (copied, not referenced — see snapshot note above).
3. The Quote transitions to `CONVERTED` (terminal).
4. User lands on the new Draft Invoice, in the normal V1 invoice-editing flow — they can adjust line items before issuing (a Quote's line items becoming a Draft's line items are just a starting point, same editability as any other Draft) — then Save Draft or Issue exactly as V1 already works.

**Traceability:** the resulting Invoice stores a reference back to its source Quote (for "this invoice came from Quote QUO-2027-0012" display on both documents). This is a display/navigation link, not a live data dependency — editing or even hypothetically un-converting is not possible (Quote stays `CONVERTED` regardless of what happens to the resulting invoice afterward, including if that invoice is later cancelled).

**Edge case — cancelled downstream invoice:** if the resulting Invoice is later cancelled, the source Quote remains `CONVERTED` (not reverted to `ACCEPTED`) — conversion is a one-way, permanent action, matching the "exactly once" rule above. The business's recourse is the same as any cancelled invoice: duplicate it into a new Draft.

## 4. Customer statement flow

**Entry point:** a customer's detail view (Customers section) → "Generate Statement," or a date-range picker reachable from the same place.

**Fields:** date range (defaults to a sensible recent window, e.g. this quarter — exact default is a Round 5 UI decision), customer (pre-selected from the entry point).

**Content**, per `.ai/product-v2.md`'s "deliberately simple" constraint:

```
Opening balance (as of range start)
+ Invoices issued in range (date, number, amount)
− Payments recorded in range (date, invoice reference, amount)
──────────────────────────────────────────────────
Closing balance (running balance as of range end)
```

No journal entries, no reconciliation, no separate persisted "statement" record — this is a read-only, generate-on-demand view over existing `Invoices`/`Payments` data (consistent with the Round 3 note in `.ai/product-v2.md` that statements are read models, not domain entities).

**Output:** on-screen view with Export (PDF and/or CSV, reusing the existing PDF-rendering and CSV-export infrastructure) — not a new delivery mechanism. Printing/saving follows the same OS-native flow as V1's invoice PDF (`user-flows.md` §7).

**Edge case — cancelled invoices:** a `CANCELLED` invoice does not contribute to the balance (it was never actually owed), but may optionally be shown greyed-out in the activity list for context — exact display treatment is Round 5.

## 5. Sales & tax summary reports flow

**Entry point:** a new Reports section (or a Dashboard sub-section — exact placement is Round 5), offering a small, named set of reports — not a generic report-builder (per `.ai/product-v2.md`'s "answers what happened, not what to file" boundary):

- **Sales summary** — totals by period (day/week/month/custom range), optionally grouped by product or customer.
- **Tax summary** — tax collected by period and by rate/regime bucket (e.g. CGST/SGST/IGST totals under India GST; the equivalent breakdown under whatever regime is active), for handing to an accountant.

**Fields:** report type, date range, optional grouping dimension.

**Output:** on-screen table + Export (CSV/JSON, reusing V1's existing export pipeline — `Settings → Data`'s CSV/JSON infrastructure, not a new format). No PDF requirement for reports (unlike statements, which are customer-facing documents) — a report is an internal working document.

**Edge case — regime change mid-range:** a report spanning a date range during which the business switched tax regime shows each invoice under the regime that was actually snapshotted onto it at issue time (never retroactively reinterpreted) — the tax-summary grouping should surface the regime alongside the numbers when a range mixes more than one, rather than silently summing incompatible tax buckets together.

## 6. Payment reminder flow

**Entry point:** an overdue invoice's detail view, or a quick action on overdue rows in the Invoices list (same rows V1's dashboard Overdue card already links to) — "Generate Reminder."

**Behavior:**
1. The app composes a reminder message from a template, pre-filled with invoice number, amount due (`total − amount_paid`, matching the same overpayment-safe math already used elsewhere), due date, and customer name.
2. The message is shown in an editable text area — the user can adjust wording before doing anything with it.
3. Actions: **Copy to clipboard**, **Print/Save as text or PDF**. No in-app send — no email, no WhatsApp, no SMS integration (locked: `.ai/product-v2.md` explicitly rules out email-sending infrastructure and WhatsApp API for V2). The user pastes/attaches it into whatever channel they already use, the same "OS handles the rest" pattern V1 already uses for PDF sharing (`user-flows.md` §7).

**Template configuration:** a single, business-level editable template (Settings → Invoicing, alongside existing invoicing defaults) with placeholders (`{invoice_number}`, `{amount_due}`, `{due_date}`, `{customer_name}`, `{business_name}`) — not a template *builder* (no rich theming, consistent with V1's "no template builder" lock for PDFs). One template is enough; this is a reminder message, not a marketing system.

**Edge case:** generating a reminder does not change the invoice's status, does not create any stored record of "a reminder was sent" (V2 has no delivery channel to confirm against — logging a reminder as "sent" when the app doesn't know if the user actually shared it would be misleading state). If per-invoice reminder history ever becomes valuable, that's a future decision, not implied by this flow.

## 7. UPI QR code (PDF acceptance criterion, not a standalone flow)

No new screen. When `business.upi_id` is set (existing V1 field, Settings → Business Profile), the invoice PDF template renders a small UPI deep-link QR code in the payment-details area, alongside the existing bank-details text. When `upi_id` is unset, the QR is simply absent — same conditional-rendering pattern V1 already uses for optional business fields on the PDF. No user-facing configuration beyond the field that already exists.

## Round 2 definition of done

Every new screen/state transition V2 adds — tax regime configuration, the Quote lifecycle and its conversion into an Invoice, customer statements, reports, and payment reminders — is named above with entry points, fields, and edge cases, extending (never contradicting) the V1 flows in `user-flows.md`. Round 3 (schema deltas) and Round 5 (UI/UX) are designed against this document; Round 4 (application architecture) and Round 6 (calculation engine) resolve the specific open questions this round deliberately deferred (exact tax-regime field sets, per-regime calculation rules).
