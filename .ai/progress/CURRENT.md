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

Last commit is `147ac93`. Run `git status` before assuming the tree is clean.

| Slice | Backend | Frontend | Tests |
|---|---|---|---|
| Business, Customers, Products | ✅ full CRUD | ✅ CRUD screens, archive/delete via `has_invoices` | manually verified by user |
| Invoices (draft/issue/cancel/duplicate/list) | ✅ | ✅ Invoice Editor + list | integration tests, real SQLite |
| Payments | ✅ record/update/delete, status auto-recalc | ✅ `PaymentPanel`, wired into editor + list | 6 integration tests |
| Dashboard | ✅ `DashboardRepository`, all SQL-aggregated | ✅ default landing screen, recent-invoices click-through | 1 integration test |
| Settings screen | ✅ | ✅ 3 tabs: Business Profile / Tax Rates / Invoicing | n/a |
| Tax Rates CRUD | ✅ create/update/list (no delete, per spec) | ✅ inline-edit table; wired into Product form + invoice line items | 3 integration tests |
| EditIssuedInvoice | ✅ `update_issued`, re-snapshots fresh at every save | ✅ Issued/PartiallyPaid/Paid fully editable, Save Changes/Duplicate/Cancel in editor | 3 integration tests |
| GST split (CGST/SGST vs IGST) | ✅ called server-side by `domain::invoice_pdf` | ✅ mirrored in `lib/tauri/types.ts::splitGst`, shown in editor totals | covered by PDF tests |
| **PDF generation** | ✅ `printpdf` template, `InvoicePdfRenderer` port, `FileWriter` port | ✅ preview modal (real PDF in an iframe), Preview / Issue & PDF / Print–Save PDF, list row action, logo file picker | 35 tests |
| **Backup / restore** | ✅ `.vbx` zip archive, `VACUUM INTO` snapshot, staged restore + app restart | ✅ Settings → Data, confirmation names the backup's date/version | 6 integration tests |
| **Export (CSV + JSON)** | ✅ `export_data`, RFC 4180 CSV, every table as domain shapes in JSON | ✅ four buttons in Settings → Data | 4 integration + 9 unit |
| UX audit fixes | — | ✅ `ConfirmDialog`, `SearchablePicker` + quick-add modals, live-updating totals, "Overdue" filter | — |
| Currency/country | — (pure display config) | ✅ `lib/currency.ts` (60 countries), `hooks/useCurrency.tsx` (app-wide context), every screen money-format-aware | — |

Backend: 114 tests passing, `cargo fmt`/`clippy` clean (1 harmless warning — see below). Frontend: `pnpm typecheck`/`lint`/`build` all clean.

### Pre-existing harmless warnings (don't "fix" without a reason)

- `ApplicationError::Infrastructure` field never read — intentional, kept for API completeness. This is now the *only* backend warning: `InfrastructureError::Io` (constructed by `StdFileWriter`) and `GstSplit`/`split_gst` (called by `domain::invoice_pdf`) both stopped being dead code when PDF generation landed.
- `hooks/useCurrency.tsx` triggers one ESLint `react-refresh/only-export-components` warning (exports both a component and a hook) — cosmetic, common context+hook pattern.

### Things that will bite you if you don't know them

- **`printpdf` must keep its `text_layout` feature.** Font subsetting only exists on that feature (`prepare_fonts_for_serialization` has a `#[cfg(not(...))]` arm that embeds the full face). Removing it to slim the dependency tree turns a 32 KB invoice into a 743 KB one. The feature also changes `ParsedFont`'s API — see `infrastructure/pdf/fonts.rs`.
- **Restore closes the database pool and restarts the app**, and must: every repository holds that pool. `restore_backup` never returns on success. Validation happens *before* the pool closes, and extraction goes to a staging file first, so a rejected or corrupt archive leaves the running app untouched.
- **Backups must use `VACUUM INTO`, not a file copy.** WAL mode means the main `.db` file alone is missing committed data.
- **`business.logo_path` is stored relative (`assets/business-logo.png`) for any logo chosen since 2026-08-30 session 4, and absolute for anything chosen before that** — `domain::business::resolve_logo_path` is the one place that knows both. Every reader (`PdfUseCases`, `probe_business_logo`, backup's `assets_to_archive`) must resolve through it rather than opening `logo_path` directly, or a managed logo on a restored/different machine silently fails to open.
- **`domain/currency.rs` and `src/lib/currency.ts` are two copies of the same table.** Add a currency to both, or the screen and the PDF disagree.
- The embedded DejaVu Sans has no glyph for BDT's `৳` or SAR's `﷼`; those fall back to the ISO code by design (`Fonts::can_render`). Don't "fix" it without swapping the font.
- **An issued invoice prints its frozen business snapshot**, so changing the logo (or address, or bank details) in Settings does *not* change invoices already issued — by design (`.ai/product.md`'s locked snapshot principle). Editing one and saving re-snapshots it. This looks like a bug when reported ("my logo isn't showing"); check `business_snapshot_logo_path` on the actual invoice before hunting in the renderer.
- macOS screenshot filenames contain U+202F (narrow no-break space) before `AM`/`PM`. Retyping such a path with an ordinary space silently finds nothing — copy it, don't retype it.

## Known gaps (deliberate, not oversights)

- **Only India's GST tax model is implemented.** Currency display is dynamic per-country now, but tax regime logic is India-specific and locked as V1 scope. **User confirmed (2026-08-30): fine for now — multi-country tax support is an explicit future-version item. Don't build it speculatively, but don't design anything that'd make it harder later either.**
- Line-level discounts: engine supports them, editor UI only exposes invoice-level discount.
- Dashboard metric cards aren't clickable-through to a filtered Invoices List (recent-invoices rows are). Needs `InvoiceFilter` to support a derived `OVERDUE` pseudo-status plus lifting filter state out of `InvoicesList`.
- The PDF prints one neutral `Tax` line outside India rather than a CGST/SGST/IGST split — same India-only constraint as above, and the Invoice Editor's on-screen totals now match it.
- Restore has full test coverage but hasn't been clicked through in the running app — `app.restart()` specifically can only be confirmed by hand.

## Next up (agreed order)

1. ~~PDF generation~~ — done 2026-08-30 (session 2). See the daily file for the library choice, the layering, and the font trade-off.
2. ~~Backup/restore + export~~ — done 2026-08-30 (session 3).
3. ~~Make `business.logo_path` app-managed~~ — done 2026-08-30 (session 4).
4. Another audit pass, same method as 2026-08-30's — the PDF template and the restore flow have only been checked against tests and generated samples, not against a real business's data on a second machine.

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
- `2026-08-30.md` — two sessions. Session 1: Payments, Dashboard, Settings, Tax Rates, EditIssuedInvoice, UX audit, currency/country support; the progress-tracking system itself was created. Session 2: PDF generation end to end. Session 3: backup/restore + CSV/JSON export (Settings → Data). Session 4: made `business.logo_path` app-managed so it survives a restore onto a different machine.
