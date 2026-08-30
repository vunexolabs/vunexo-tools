# Vunexo Billing — Current State

**Read this file first, every session — nothing else in this folder.** It's kept short on purpose: this file gets *overwritten* each session to reflect the latest state, it never grows. The daily files next to it (`YYYY-MM-DD.md`) are the append-only history — one file per day, immutable once the day is over — read one of those only if you need the *why* behind something listed here, not to reconstruct current state (this file already has that).

**Standing instruction (2026-08-30): update this file at the end of every session that changes code**, and write/append that day's `YYYY-MM-DD.md` with what happened. Don't let this file grow into a log — if you're tempted to append here, that content belongs in today's dated file instead. See `feedback_progress_log` in the agent's own memory for the full instruction.

Last updated: 2026-08-30.

## How this fits with the other `.ai/` files

- `.ai/product.md` — the locked V1 spec (what Vunexo Billing is, in/out of scope). Rarely changes.
- `.ai/decisions/ADR-*.md` — architecture decision records. Append new ones, never edit old ones.
- `docs/vunexo-billing/*.md` — the locked Round 2–6 design docs (user-flows, database-schema, application-architecture, ui-ux, calculation-engine). Source of truth for *how* something should work — check before implementing, don't guess.
- `.ai/progress/` (this folder) — *as-built* state and history. Everything above is the plan; this is what's actually done.

## Current state

Round 7 (implementation) in progress. Backend: Rust/Tauri/SQLx, `apps/vunexo-billing/src-tauri/`. Frontend: React/TS/Tailwind, `apps/vunexo-billing/src/`.

**⚠️ Uncommitted work-in-progress.** Last commit is `3e480f4` ("Round 7: Invoices vertical slice", 2026-08-29). Everything from 2026-08-30 onward (Payments, Dashboard, Settings, Tax Rates, EditIssuedInvoice, UX-audit fixes, currency/country support) exists only in the working tree. Run `git status` before assuming otherwise.

| Slice | Backend | Frontend | Tests |
|---|---|---|---|
| Business, Customers, Products | ✅ full CRUD | ✅ CRUD screens, archive/delete via `has_invoices` | manually verified by user |
| Invoices (draft/issue/cancel/duplicate/list) | ✅ | ✅ Invoice Editor + list | integration tests, real SQLite |
| Payments | ✅ record/update/delete, status auto-recalc | ✅ `PaymentPanel`, wired into editor + list | 6 integration tests |
| Dashboard | ✅ `DashboardRepository`, all SQL-aggregated | ✅ default landing screen, recent-invoices click-through | 1 integration test |
| Settings screen | ✅ | ✅ 3 tabs: Business Profile / Tax Rates / Invoicing | n/a |
| Tax Rates CRUD | ✅ create/update/list (no delete, per spec) | ✅ inline-edit table; wired into Product form + invoice line items | 3 integration tests |
| EditIssuedInvoice | ✅ `update_issued`, re-snapshots fresh at every save | ✅ Issued/PartiallyPaid/Paid fully editable, Save Changes/Duplicate/Cancel in editor | 3 integration tests |
| GST split (CGST/SGST vs IGST) | `split_gst` exists, not yet called server-side | ✅ mirrored in `lib/tauri/types.ts::splitGst`, shown in editor totals | see gaps |
| UX audit fixes | — | ✅ `ConfirmDialog`, `SearchablePicker` + quick-add modals, live-updating totals, "Overdue" filter | — |
| Currency/country | — (pure display config) | ✅ `lib/currency.ts` (60 countries), `hooks/useCurrency.tsx` (app-wide context), every screen money-format-aware | — |

Backend: 38 integration tests passing, `cargo fmt`/`clippy` clean (4 pre-existing harmless warnings — see below). Frontend: `pnpm typecheck`/`lint`/`build` all clean.

### Pre-existing harmless warnings (don't "fix" without a reason)

- `ApplicationError::Infrastructure` field never read, `InfrastructureError::Io` never constructed — intentional, kept for API completeness.
- `domain::calculation::GstSplit`/`split_gst` — dead code *for now*, stops being dead once PDF generation calls it server-side.
- `hooks/useCurrency.tsx` triggers one ESLint `react-refresh/only-export-components` warning (exports both a component and a hook) — cosmetic, common context+hook pattern.

## Known gaps (deliberate, not oversights)

- **Only India's GST tax model is implemented.** Currency display is dynamic per-country now, but tax regime logic is India-specific and locked as V1 scope. **User confirmed (2026-08-30): fine for now — multi-country tax support is an explicit future-version item. Don't build it speculatively, but don't design anything that'd make it harder later either.**
- Line-level discounts: engine supports them, editor UI only exposes invoice-level discount.
- Dashboard metric cards aren't clickable-through to a filtered Invoices List (recent-invoices rows are). Needs `InvoiceFilter` to support a derived `OVERDUE` pseudo-status plus lifting filter state out of `InvoicesList`.
- PDF generation — not started (`infrastructure/pdf/` still the Round 1 stub).
- Backup/restore/export — not started (`infrastructure/filesystem/` still the Round 1 stub).

## Next up (agreed order, 2026-08-30)

1. **PDF generation** — dedicated Rust PDF crate composing the invoice directly (user wants real layout control, not "print the webview"). Also where `split_gst` gets its first real caller, and where `business.logo_path` likely needs an actual file picker (not yet exposed in `BusinessProfileTab`).
2. Backup/restore + export — `.vbx` format already spec'd in `database-schema.md` §9 / `user-flows.md` §9, implement against that.
3. Another audit pass after PDF/backup land, same method as 2026-08-30's.

## Verification commands (all of these, every slice)

```bash
# Backend, from apps/vunexo-billing/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/vunexo-billing/
pnpm typecheck && pnpm lint && pnpm build
```

The user's `pnpm tauri dev` session tends to stay running for an entire work session (backend file-watcher auto-rebuilds, Vite HMR picks up frontend changes) — **check `ps aux | grep "tauri dev"` before launching a new one**, a second instance collides on port 1420. If one's already running, verify with the commands above and let the user reload/test in their live window instead of starting another.

## Daily files in this folder

- `2026-08-28.md` — Rounds 1–6 locked; Business/Customers/Products CRUD + calculation engine implemented.
- `2026-08-29.md` — Invoices vertical slice (draft/issue/cancel/duplicate/list).
- `2026-08-30.md` — Payments, Dashboard, Settings, Tax Rates, EditIssuedInvoice, UX audit, currency/country support. This progress-tracking system itself was created today.
