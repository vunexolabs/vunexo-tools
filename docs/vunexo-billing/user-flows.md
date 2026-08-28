---
status: locked
round: 2
---

# Vunexo Billing — User Flows (Round 2)

This is an AI context file. Before designing the database schema (Round 3) or screens (Round 5), read this document — it is the source of truth for every screen a user passes through and every state transition. See `.ai/product.md` for the locked V1 scope and `docs/vunexo-billing/architecture.md` for the layering rules these flows must be implemented behind.

Scope: this document is UI-agnostic (no mockups — that's Round 5). It pins down entry points, required/optional fields, state transitions, and edge cases.

## 1. First-run flow (business setup)

**Trigger:** app launched, no business profile row exists yet.

1. App opens directly to a "Set up your business" screen — no splash, no login, no account creation (locked: no auth in V1).
2. Only **business name** is required to proceed. Logo, address, phone, email, GSTIN, bank details, UPI ID are optional and editable later from Settings.
3. Saving creates the single business profile row. V1 is single-business per install (multi-company is out of scope).
4. On save → Dashboard.

**Edge case:** app closed mid-setup without saving → next launch resumes the setup screen (no partial-save mechanic needed for a single form).

## 2. Returning-user entry flow

**Trigger:** app launched, business profile exists.
→ Opens directly to the Dashboard. No login step, ever.

## 3. Customer creation / management flow

**Entry points** (same underlying flow both times):
- Standalone: Customers section → "+ New Customer".
- Inline: from the invoice line-item customer picker → "+ Create new customer" (so creating a first invoice never forces a detour to a different section).

**Fields:** name (required); phone, email, address, GSTIN (all optional — GSTIN only matters for B2B GST invoices).

Saving returns the user to wherever they came from: the Customers list, or back into the in-progress invoice draft with the new customer already selected.

**Editing:** updates the customer's master record only. Past invoices are unaffected — see the snapshot principle in `.ai/product.md`.

**State:** a customer is `ACTIVE` or `ARCHIVED`.
- A customer with **zero invoices** can be hard-deleted, or archived.
- A customer with **any invoice history** cannot be hard-deleted (doing so would break the customer's history/balance view and contradict the Definition of Done's "existing invoice history remains unchanged"). Instead it is **archived**: hidden from the picker for new invoices, but still visible in reports and in its own history/balance view.
- **Archived customers can be restored** to `ACTIVE` at any time, reappearing in the picker.

## 4. Product/service creation / management flow

Same dual entry-point pattern as customers (standalone via Products section, or inline from the invoice line-item picker).

**Fields:** name (required), unit (required — e.g. pcs/hr/kg/day; small preset list + free text), price (required), tax rate (required, defaults to a business-level default tax rate set in Settings). SKU and description are optional.

**State and deleting:** identical rule to customers — `ACTIVE`/`ARCHIVED`, snapshot protects past invoices, hard-delete only if never used on an invoice, otherwise archived, and archived products can be restored to `ACTIVE`.

## 5. The core flow: invoice creation

This flow is what the product's UX target (first invoice < 3 min, subsequent < 30 s) is built around.

**Entry points:** Dashboard primary action, Invoices list "+ New Invoice", or "Duplicate" from an existing invoice.

1. **Customer** — pick from a searchable list, or create inline. Not required to *start* a draft or add items, but required to **issue**.
2. **Invoice metadata** — invoice number (auto-generated, sequential, normally read-only — see numbering rule below), invoice date (defaults to today), due date (defaults to today + N days, N configurable in Settings).
3. **Line items** — add a product/service via a searchable picker or create one inline; quantity, unit price (pre-filled from the product **at the moment it's added to this invoice**, editable per line thereafter), optional per-line discount, tax rate (pre-filled, editable per line for edge cases). Subtotal/tax/total recalculate live as items change.
4. **Invoice-level discount and notes/terms** — optional fields below the line items. Discount is either a flat **amount** or a **percentage** (user picks per invoice, not a global setting) — see discount calculation model below.
5. **Preview** — a live, read-only rendering of the eventual PDF, reachable at any point without leaving the draft. Not a separate saved state.
6. **Save options:**
   - **Save Draft** — status `DRAFT`. Fully editable, does not require a customer or any line items.
   - **Issue** — status `ISSUED`. Requires a customer and ≥1 line item. This is the moment the invoice number is finalized (next in the business's sequence, immutable after this) and the **customer/product/line-item snapshot is taken** (locked architecture principle in `.ai/product.md`).
   - **Save & PDF** — same as Issue, then immediately opens the PDF preview/save/print dialog.
7. **After issuing:** Duplicate (new `DRAFT`, copies line items, not payment/status history), Edit (see rule below), Cancel, Print/Save PDF, Record Payment.

**Duplication, precisely:** customer, line items (products, quantities, prices, per-line discounts/tax), invoice-level discount, notes, and terms are copied as-is into a new `DRAFT`. Payments and status history are **never** copied — the duplicate starts at `DRAFT` with zero payments. It gets its own invoice number when it is eventually issued, generated the normal way (next in sequence at that later point in time), not reserved at duplication time.

**Draft vs. issue — what gets snapshotted, and when:** a per-line item's price/tax is already frozen the moment it's added to the invoice (draft or not) — changing a product's master price afterward never silently changes an already-added line, draft or issued. What's specific to **issuing** is the broader **customer/business snapshot** (name, address, GSTIN, bank details as they stood at that moment) — that copy is taken only at Issue, not while still a Draft. If the user changes the customer's address while an invoice is still a Draft, the eventual issued invoice reflects the address at issue time, not at draft-creation time — there is nothing to "protect" yet before issuing.

**Editing an issued invoice:** allowed — small businesses correct real mistakes (wrong quantity, a typo). Editing an `ISSUED`/`PARTIALLY_PAID`/`PAID` invoice re-snapshots at save time (an invoice's snapshot always reflects that invoice's own last save, not live customer/product data — editing the invoice is an explicit, intentional action, unlike a customer record silently changing elsewhere). **Payments are independent historical records and are never automatically modified when an invoice is edited.** If an edit drops the total below the amount already paid, the invoice stays `PAID` and the UI surfaces the difference as a visible **overpayment** (`amount_paid − total`), not a silent adjustment to any payment record.

**Deleting vs. cancelling:**
- `DRAFT` — can be deleted outright (nothing has been issued; there's no history to protect).
- `ISSUED`, `PARTIALLY_PAID`, `PAID` — cannot be deleted. They can be **cancelled** instead.
- `CANCELLED` — immutable: cannot be edited or deleted, can only be duplicated into a new `DRAFT`.

**Cancelling** requires an optional reason (free text) and records `cancelled_at` alongside it, for reporting/audit purposes.

**Numbering:** sequential per business, format configured once in Settings (e.g. `INV-{year}-{seq}`), and **normally auto-generated and read-only** — not something the user edits invoice-by-invoice. An advanced "custom invoice number" override exists for edge cases (imports, migrating from another tool) but is opt-in and out of the way of the default flow. A number is never reused, even if its invoice is later cancelled — cancelling does not free the number for reassignment. Once issued, the number is immutable regardless of how it was generated.

**Invoice-level discount calculation model** (full formulas are finalized in Round 6 — calculation engine; this fixes the shape of the model only): the invoice-level discount (amount or percentage) is applied **before tax**, allocated proportionally across taxable line items — not a flat subtraction after tax totals are computed. No cross-line allocation logic beyond simple proportional split is in scope for V1.

### Invoice state machine

```
DRAFT ──issue──▶ ISSUED ──payment──▶ PARTIALLY_PAID ──full payment──▶ PAID

ISSUED, PARTIALLY_PAID, PAID ──cancel──▶ CANCELLED
```

- `DRAFT` is the only state a user can delete from.
- `CANCELLED` is terminal: no edits, no further payments, duplicate-only.
- `OVERDUE` is **not a state in this machine** — see Section 6.

## 6. Payment recording flow

**Entry points:** an invoice's detail view ("Record Payment"), or a quick action on unpaid/overdue rows in the Invoices list.

**Fields:** amount (defaults to remaining balance), method (cash/bank transfer/UPI/cheque/other), date (defaults to today), reference/note (optional).

Multiple payments per invoice are allowed (installments). After each payment, status is recalculated automatically:
- `amount_paid == 0` → stays `ISSUED` (or `OVERDUE`, see below)
- `0 < amount_paid < total` → `PARTIALLY_PAID`
- `amount_paid >= total` → `PAID`

Overpayment is allowed to record (some businesses treat a small overpayment as credit) but is visually flagged, never silently clamped to the invoice total.

**Editing and deleting payments:** a recorded payment (amount, method, date, reference) can be edited or deleted after the fact — people mis-key amounts. Either action recalculates the parent invoice's `amount_paid` and status immediately. This never modifies the invoice's own fields (total, discount, tax, line items) — the two are edited independently of each other.

`OVERDUE` is **not stored anywhere** — it's a derived display badge, computed at read time as:

```
is_overdue = due_date < today
             AND amount_paid < total
             AND status NOT IN (DRAFT, CANCELLED)
```

It is never a status the user sets directly or a stored transition they trigger — see the state machine above.

## 7. PDF / share flow

"Save & PDF", or the PDF action on an existing invoice, renders the single V1 template and opens the OS-native save/print dialog — from which the user can already reach email, AirDrop, WhatsApp Web, etc. via their platform. No in-app "Share to WhatsApp" button in V1 (explicitly out of scope per the locked spec).

## 8. Dashboard flow

Landing screen after setup or on every subsequent launch. Shows today/this-month/outstanding/paid/overdue metrics and a recent-invoices list. Every metric and every recent-invoice row is clickable through to the relevant filtered list or invoice detail — the dashboard is a set of entry points, not a dead end.

## 9. Backup / restore flow

- **Backup** (Settings → Data → Backup): one click, writes a single file (`vunexo-billing-backup-YYYY-MM-DD.vbx`) containing the SQLite database, settings, and a metadata block (`format_version`, `app_version`, `created_at`, `platform`); the user picks the save location via the OS file dialog. The format is versioned from V1 onward so a future app version can detect and migrate an older backup on restore, rather than assuming the format never changes.
- **Restore**: the user picks a `.vbx` file via the OS file dialog; the app requires an explicit confirmation ("this replaces all current data") before applying, since restoring overwrites whatever is currently in the local database.

## Round 2 definition of done

Every screen a user passes through, from first launch to a paid, PDF'd invoice, is named above with its entry points, required/optional fields, and edge cases. Round 3 (database schema) is designed against these flows; Round 5 (UI/UX) designs the actual screens implementing them.
