---
status: locked
round: 5
---

# Vunexo Billing — V2 UI/UX Structure Deltas (Round 5)

This is an AI context file, same status as `docs/vunexo-billing/ui-ux.md`. It is a **delta** document — everything in that file still applies unless overridden below. It designs every screen `user-flows-v2.md` implies against the command surface `application-architecture-v2.md` §5 locked. It does not add scope: every screen here exists because a locked V2 flow needs it. **It does not invent a second tax-regime model in the frontend** — the UI consumes what the backend already decided (Round 4 §4a: a closed enum dispatched server-side), it does not re-derive regime-specific behavior from `country_code` or any other client-side heuristic.

## 1. Screen inventory (additions)

| Screen | Locked flow it implements | Primary commands it calls |
|---|---|---|
| Quotes List | `user-flows-v2.md` §2 (entry point) | `list_quotes` |
| Quote Editor | §2, §3 (the core V2 flow, mirrors Invoice Editor) | `create_draft_quote`, `update_draft_quote`, `issue_quote`, `accept_quote`, `decline_quote`, `cancel_quote`, `convert_quote_to_invoice`, `duplicate_quote`, `delete_draft_quote`, plus the same customer/product inline-create commands the Invoice Editor already uses |
| Customer Statement (panel within Customer Detail, not a new route) | §4 | `generate_customer_statement` |
| Reports | §5 | `generate_sales_report`, `generate_tax_summary_report` |
| Payment Reminder (modal, opened from an invoice) | §6 | `generate_reminder_message` |
| Settings — Business Profile (extended) | §1 | `update_business` (now carries `tax_regime_code`) |
| Settings — Invoicing (extended) | §1, §6 | `update_settings` (now carries `quote_number_format`, `payment_reminder_template`) |

`features/reports/` — scaffolded empty in V1 (`ui-ux.md` §7, "left as a placeholder... since Round 2's own roadmap doc still lists it as a future area") — is the one V1 placeholder V2 actually fills. No other V1 placeholder is touched.

## 2. Navigation structure (revised)

```
App shell
├── Dashboard                (default route, unchanged)
├── Invoices                 (unchanged)
├── Quotes                   ← new
│   ├── Quotes List
│   └── Quote Editor          (new / edit / view — one component, see §4)
├── Customers
│   ├── Customers List
│   └── Customer Detail        (gains a Statement tab, see §5)
├── Products                 (unchanged)
├── Reports                  ← new
│   ├── Sales Summary
│   └── Tax Summary
└── Settings
    ├── Business Profile      (gains Tax Regime, see §6)
    ├── Tax Rates
    ├── Invoicing              (gains Quote numbering + reminder template)
    └── Data
```

Two additions to the sidebar, not one per V2 feature — **Statements deliberately do not get their own top-level section**, per the explicit call to keep them scoped to the customer they belong to rather than becoming a seventh sidebar destination for what is, functionally, a customer-detail sub-view. **Payment Reminder is not a sidebar destination at all** — it's a modal opened from an overdue invoice (§7), the same "inline action, not a new place to navigate to" pattern V1 already uses for Record Payment.

## 3. Cross-cutting UI patterns (additions)

- **Quote status badges**, same fixed-color-per-status rule as invoices (`ui-ux.md` §3): `DRAFT` neutral/gray (shared styling with invoice Draft), `ISSUED` blue, `ACCEPTED` green, `DECLINED` red, `CONVERTED` a distinct violet/purple (visually different from `PAID` green — an accepted-then-converted quote is not "paid," it's "became a different document," and reusing green would read as a payment state it isn't), `CANCELLED` struck-through gray. `EXPIRED` renders as an additional badge alongside `ISSUED`, exactly like `OVERDUE` does for invoices (`database-schema-v2.md` §3) — never replaces the stored status badge.
- **Tax-regime-conditional fields, one switch point, not scattered `if`s.** Wherever the UI shows regime-specific fields (GSTIN, HSN/SAC, the CGST/SGST/IGST breakdown on totals, the interstate toggle), the component branches on **one** piece of data the backend already returned — the document's `tax_regime_snapshot` when viewing an issued Invoice/Quote, or `business.tax_regime_code` when editing a Draft (matching Round 4 §4d's "a Draft always reflects current business regime" rule) — never on `country_code` (a separate, currency-display-only setting per V1) and never a client-side re-derivation of "is this India" from any other field. Concretely: one `TaxRegimeFieldSet` lookup (a small frontend constant keyed by regime code, e.g. `{ IN_GST: ['gstin', 'hsn_sac', 'is_interstate'] }`) drives which fields render — adding the second regime later means adding one entry to this table, not hunting down conditional branches across the Invoice Editor, Quote Editor, and PDF preview independently.
- **Regime-switch confirmation copy, deliberately low-key.** Changing `business.tax_regime_code` in Settings shows a plain confirmation, not a migration wizard:
  > **Tax regime changed**
  > Existing issued documents won't be affected. Any Draft documents will use the new regime the next time you save them.

  No recalculation preview, no per-draft opt-in flow, no "N drafts will be affected" count — Round 4 §4d already established this is a non-event at the data layer (a Draft simply recalculates on its next save), so the UI shouldn't manufacture ceremony around it. This mirrors the "not required for archive/restore (reversible, low-stakes)" restraint `ui-ux.md` §3 already exercises for other low-risk mutations.
- **`ACCEPTED` quotes' cancel action asks for the same optional reason field** cancelling an Invoice or a `DRAFT`/`ISSUED` Quote already does (`ui-ux.md` §3's confirmation-dialog list) — no separate copy for the "customer backed out after accepting" case; it's the same dialog, same field, just reachable from one more state.

## 4. Quote Editor (mirrors the Invoice Editor almost exactly)

One component, modes driven by `quote status + editor mode` together, same principle as `ui-ux.md` §4: `CreateDraft` / `EditDraft` / `ViewIssued` (covers `Issued`/`Accepted`/`Declined` — all read-only content-wise per the locked "editable in Draft only" rule, differing only in which action buttons show) / `ViewConverted` / `ViewCancelled`.

```
┌──────────────────────────────────────────────────────┐
│ ← Back              [DRAFT]                            │
├──────────────────────────────────────────────────────┤
│ Customer        [ Search or select customer      ▾ ]  │
│                                                        │
│ Next quote number • automatic            Date  28 Aug │
│ QUO-2027-0012                        Valid until 12 Sep│
├──────────────────────────────────────────────────────┤
│ Item                    Qty   Rate      Tax    Total   │
│ ──────────────────────────────────────────────────    │
│ [product picker row — identical component to Invoices] │
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
│ [Save Draft]   [Preview]   [Issue]                       │
└──────────────────────────────────────────────────────┘
```

Deliberately **no "Issue & PDF" combined action** here, unlike the Invoice Editor — a Quote isn't necessarily a document a business prints/sends as a PDF the same way an invoice is (Round 2 didn't lock a Quote PDF as a requirement); Preview + Issue stay separate actions. (If a Quote PDF turns out to matter in practice, that's a small addition to this screen later, not a reason to hold up this round.)

Behavior by mode:
- **`CreateDraft` / `EditDraft`** (`status = Draft`): every field editable; footer `Save Draft` / `Preview` / `Issue`. Same "next number • automatic," same non-reserving preview caveat as invoices (`ui-ux.md` §4).
- **`ViewIssued`** (`status ∈ {Issued, Accepted, Declined}`): entire form read-only (per the locked "Quotes are editable in Draft only" rule — no `EditIssued`-style exception the way invoices get one). Footer varies by exact status:
  - `Issued`: `Accept` · `Decline` · `Cancel` · `Duplicate`.
  - `Accepted`: `Convert to Invoice` · `Cancel` · `Duplicate` — this is the one screen `convert_quote_to_invoice` is reachable from, matching `user-flows-v2.md` §3's single named entry point.
  - `Declined`: `Cancel` · `Duplicate` only — no path back to `Accepted` (the state machine has no `Declined → Accepted` edge; a business that changes its mind duplicates into a fresh Draft).
  - An `EXPIRED` badge (when applicable) sits next to the status badge on `Issued`, same visual treatment as `OVERDUE`; it does not change which buttons show — accepting an expired quote is still allowed (Round 4 §3).
- **`ViewConverted`** (`status = Converted`): read-only; footer shows only a link — **"View Invoice INV-2027-0031 →"** — no action buttons at all, since a `Converted` quote is fully terminal (no cancel, no duplicate-from-here; duplicating a converted quote's terms is the resulting invoice's job via its own `Duplicate`, not the quote's).
- **`ViewCancelled`** (`status = Cancelled`): same treatment as a cancelled invoice — read-only, persistent banner with the cancel reason if given, footer only `Duplicate`.

## 5. Customer Detail — Statement tab (addition)

```
Customer
├── Overview        (existing — contact info, edit/archive)
├── Invoices         (existing — this customer's invoice history)
├── Payments          (existing)
└── Statement          ← new
```

The Statement tab is a date-range picker (defaulting to the current quarter) plus the on-screen rendering specified in `user-flows-v2.md` §4: opening balance, chronological invoice/payment entries, closing balance. **Export** (PDF/CSV) sits directly on this tab, not a separate screen — one `generate_customer_statement` call renders the on-screen view, and the same result feeds the export, so there's exactly one code path producing "what a statement says," never a second implementation for the exported version.

## 6. Reports screens

Two reports, matching `user-flows-v2.md` §5 exactly — **not a report picker with configurable dimensions**, per the locked "small, named set of reports" constraint:

```
Reports
├── Sales Summary    — date range + optional group-by (product / customer)
└── Tax Summary       — date range; shows a regime column whenever the range spans more than one tax_regime_snapshot value, per the mixed-regime edge case
```

Each is: a filter bar (date range, and for Sales Summary the group-by toggle), a table, and an **Export (CSV/JSON)** button reusing V1's existing export pipeline (`ui-ux.md` §6) — no PDF for reports, matching the locked "internal working document" framing.

## 7. Payment Reminder modal

Opened from: the Invoices List's row action on an overdue row (existing quick-action slot next to Record Payment, per `ui-ux.md` §5), or the `EditIssued` Invoice Editor footer (an addition to the button row in `ui-ux.md` §4 — `Record Payment`, `Duplicate`, `Cancel`, `Print/Save PDF`, **`Remind`** — shown only when the invoice is currently overdue).

```
┌───────────────────────────────────────────┐
│ Payment Reminder — INV-2026-0042             │
├───────────────────────────────────────────┤
│ [ editable text area, pre-filled from        │
│   the template + this invoice's data ]       │
│                                               │
├───────────────────────────────────────────┤
│ [Copy to Clipboard]   [Print / Save PDF]      │
└───────────────────────────────────────────┘
```

One `generate_reminder_message` call on open, editable before either action, no send button (per the locked no-delivery-mechanism decision) — closing the modal discards any edits, nothing is persisted (`application-architecture-v2.md` §3 — the use case itself never writes anywhere).

## 8. Frontend module mapping (additions)

```
features/
├── quotes/       → Quotes List + Quote Editor (§4)
├── reports/      → Sales Summary + Tax Summary (§6) — the V1 placeholder, now filled
├── reminders/    → the Payment Reminder modal (§7) — not a route, opened from invoices/
└── customers/    → gains the Statement tab (§5); no new top-level directory needed
```

`components/` gains one addition: the regime-conditional field-set lookup from §3 lives as a small shared constant/hook (e.g. `useTaxRegimeFields(regimeCode)`), imported by both `invoices/` and `quotes/` rather than each feature re-implementing its own version — this is the concrete mechanism behind §3's "one switch point" rule.

## Round 5 (V2) definition of done

Every screen implied by the locked V2 flows has a place in the navigation (or an explicit reason it isn't one — Statements and Reminders), a specification of what it shows and which commands it calls, and a home in `src/features/*`. The tax-regime-conditional rendering pattern is fixed at one lookup point so a future second regime is a data addition, not a UI refactor. Round 6 (calculation engine) fills in the exact arithmetic the Quote Editor's live totals depend on (same "frontend never calculates financial totals" rule as invoices) and names the second regime this round's field-set lookup will eventually gain an entry for; Round 7 (implementation) builds these screens against this spec.
