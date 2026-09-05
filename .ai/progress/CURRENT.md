# Vunexo Tools — Current State

**Read this file first, every session — nothing else in this folder.** It's kept short on purpose: this file gets *overwritten* each session to reflect the latest state, it never grows. The daily files next to it (`YYYY-MM-DD.md`) are the append-only history — one file per day, immutable once the day is over — read one of those only if you need the *why* behind something listed here, not to reconstruct current state (this file already has that).

**Standing instruction (2026-08-30): update this file at the end of every session that changes code**, and write/append that day's `YYYY-MM-DD.md` with what happened. Don't let this file grow into a log — if you're tempted to append here, that content belongs in today's dated file instead. See `feedback_progress_log` in the agent's own memory for the full instruction.

Last updated: 2026-09-05 (Billing section; see this date's edits for both projects — Expense Manager and Billing sessions ran independently the same day).

This repo now holds two independent products — **Vunexo Billing** (`apps/vunexo-billing/`) and **Vunexo Expense Manager** (`apps/expense-manager/`), added 2026-09-05. They share no data and no runtime coupling; each has its own SQLite DB and business profile. Sections below are per-project.

---

# Vunexo Billing

## How this fits with the other `.ai/` files

- `.ai/product.md` — the locked V1 spec (what Vunexo Billing is, in/out of scope). Rarely changes.
- `.ai/product-v2.md` — the locked V2 spec (2026-08-30, session 12): a complete quote-to-payment billing workflow with a country-aware tax engine. `.ai/product-v2-scope.md` is the Round 1 research trail it was decided from — read for *why*, not *what*.
- `docs/vunexo-billing/user-flows-v2.md` — locked V2 Round 2 (2026-08-30, session 12): tax regime configuration, Quote lifecycle, Quote→Invoice conversion, customer statement, reports, payment reminder, UPI QR.
- `docs/vunexo-billing/database-schema-v2.md` — locked V2 Round 3 (2026-08-30, session 12): schema *deltas* only (read alongside `database-schema.md`, doesn't replace it) — Quotes/QuoteLineItems/QuoteNumberCounters, `tax_regime_snapshot` on invoices, `business.tax_regime_code`.
- `docs/vunexo-billing/application-architecture-v2.md` — locked V2 Round 4 (2026-08-30, session 12): deltas only. `QuoteRepository`/`QuoteNumberSequencer`/`StatementRepository`/`ReportRepository` ports, tax-regime dispatch (closed enum, matched in `calculate_invoice`), `ConvertQuoteToInvoice`'s atomic 2-table transaction, legacy-`NULL`-regime normalization pinned to `SqliteInvoiceRepository`'s row mapping only.
- `docs/vunexo-billing/ui-ux-v2.md` — locked V2 Round 5 (2026-08-30, session 12): deltas only. Quotes + Reports as new sidebar sections; Statement lives as a Customer Detail tab; Reminder is a modal, not a route; tax-regime-conditional field rendering pinned to one lookup (`useTaxRegimeFields`), never `country_code`-driven.
- `docs/vunexo-billing/calculation-engine-v2.md` — locked V2 Round 6 (2026-08-30, session 12), the last design round before implementation. Names `VAT_STANDARD` (deliberately narrow), finds it needs **zero changes** to `calculate_invoice`'s core algorithm — only its own presentation function (`present_vat`). Amends `database-schema-v2.md` §5/§9 and `application-architecture-v2.md` §4a's deliberately-deferred enum/CHECK widenings.
- `.ai/decisions/ADR-*.md` — architecture decision records. Append new ones, never edit old ones.
- `docs/vunexo-billing/*.md` — the locked Round 2–6 design docs (user-flows, database-schema, application-architecture, ui-ux, calculation-engine). Source of truth for *how* something should work — check before implementing, don't guess.
- `.ai/progress/` (this folder) — *as-built* state and history. Everything above is the plan; this is what's actually done.

## Current state

**V1 shipped as a real release: `app-v1.0.0`, published (not prerelease) at https://github.com/vunexolabs/vunexo-tools/releases/tag/app-v1.0.0** (2026-08-30, session 11). Every V1 Definition-of-Done item in `.ai/product.md` is implemented and tested. The only unconfirmed DoD line is Windows/Linux installers running on real hardware by a human — macOS is confirmed (session 11), Windows/Linux are not.

Backend: Rust/Tauri/SQLx, `apps/vunexo-billing/src-tauri/`. Frontend: React/TS/Tailwind, `apps/vunexo-billing/src/`. Version is `1.0.0` in `package.json`/`tauri.conf.json`/`Cargo.toml`.

Last commit is `1dedf6b`. Run `git status` before assuming the tree is clean.

| Slice | Backend | Frontend | Tests |
|---|---|---|---|
| Business, Customers, Products | ✅ full CRUD | ✅ CRUD screens, archive/delete via `has_invoices` | manually verified by user |
| Invoices (draft/issue/cancel/duplicate/list) | ✅ | ✅ Invoice Editor + list, including per-line discount type/value | integration tests, real SQLite |
| Payments | ✅ record/update/delete, status auto-recalc | ✅ `PaymentPanel`, wired into editor + list | 6 integration tests |
| Dashboard | ✅ `DashboardRepository`, all SQL-aggregated | ✅ default landing screen, recent-invoices + Overdue-card click-through | 1 integration test |
| Settings screen | ✅ | ✅ 3 tabs: Business Profile / Tax Rates / Invoicing | n/a |
| Tax Rates CRUD | ✅ create/update/list (no delete, per spec) | ✅ inline-edit table; wired into Product form + invoice line items | 3 integration tests |
| EditIssuedInvoice | ✅ `update_issued`, re-snapshots fresh at every save | ✅ Issued/PartiallyPaid/Paid fully editable, Save Changes/Duplicate/Cancel in editor | 3 integration tests |
| GST split (CGST/SGST vs IGST) | ✅ called server-side by `domain::invoice_pdf` | ✅ mirrored in `lib/tauri/types.ts::splitGst`, shown in editor totals | covered by PDF tests |
| **PDF generation** | ✅ `printpdf` template, `InvoicePdfRenderer` port, `FileWriter` port | ✅ preview modal (real PDF in an iframe), Preview / Issue & PDF / Print–Save PDF, list row action, logo file picker | 35 tests |
| **Backup / restore** | ✅ `.vbx` zip archive, `VACUUM INTO` snapshot, staged restore + app restart | ✅ Settings → Data, confirmation names the backup's date/version | 6 integration tests |
| **Export (CSV + JSON)** | ✅ `export_data`, RFC 4180 CSV, every table as domain shapes in JSON | ✅ four buttons in Settings → Data | 4 integration + 9 unit |
| UX audit fixes | — | ✅ `ConfirmDialog`, `SearchablePicker` + quick-add modals, live-updating totals, "Overdue" filter | — |
| Currency/country | — (pure display config) | ✅ `lib/currency.ts` (60 countries), `hooks/useCurrency.tsx` (app-wide context), every screen money-format-aware | — |
| Release readiness (license/CI/docs) | ✅ `LICENSE` (MIT), `ADR-002` dependency audit, `THIRD_PARTY_NOTICES.md`, `.github/workflows/ci.yml`, app `README.md` | n/a | CI workflow untested — no push to a remote has triggered it yet |
| **V2 — Quote lifecycle + tax regime (Round 7, session 12)** | ✅ full lifecycle incl. `convert_quote_to_invoice`'s atomic 2-table transaction; `business.tax_regime_code`, `VAT_STANDARD` presentation (`present_vat`); wired into `main.rs`, 11 `commands::*_quote` Tauri commands live; `quote_number_format` read-only-after-first-issue lock enforced | ✅ Quotes List + Editor (mirrors Invoice Editor, editable in Draft only per the locked design), Quotes nav section, converting hands off into the Invoices section | 12 backend integration/unit tests; **manually clicked through the running app 2026-09-05 (session 14) — Draft → Issue → Accepted → Convert to Invoice all verified live, no errors** |
| **V2 — Statements, reports, reminders (Round 7 slice 7C–7G, session 12–13)** | ✅ `StatementRepository`/`ReportRepository` (SQL-aggregated, same discipline as `DashboardRepository`), `GenerateCustomerStatement`/`GenerateSalesReport`/`GenerateTaxSummaryReport`/`GenerateReminderMessage`; wired, 4 Tauri commands live; plus one small session-13 addition, `FileExportUseCases`/`write_export_file` (generic "write already-rendered text to a path" — backs the frontend-built Statement/Report CSV/JSON, since those are parameterized read models `ExportEntity`'s fixed-shape design doesn't cover) | ✅ Statement tab (`features/customers/CustomerDetail.tsx`, new — see gap below), Reports (`features/reports/`: Sales Summary + Tax Summary, new "Reports" nav section), Payment Reminder modal (`features/reminders/ReminderModal.tsx`, wired into Invoices List + Invoice Editor) | 6 backend integration tests incl. the opening/closing-balance reconciliation property across 3 real periods; **Statement tab and both Reports screens manually verified live 2026-09-05 (session 14) against real data — see note below on the Reminder modal** |

Backend: 134 tests passing (was 116 pre-V2), `cargo fmt`/`clippy` clean modulo the warnings below. Frontend: `pnpm typecheck`/`lint`/`build` all clean.

### Session 13's one real gap: there was no "Customer Detail" screen to add the Statement tab to

`ui-ux-v2.md §5` describes the Statement tab as an addition to an existing Overview/Invoices/Payments Customer Detail screen — that screen was never built in V1 (`CustomersList.tsx` only ever grew an inline edit form). Session 13 built the minimum `CustomerDetail.tsx` the design actually presupposes: **Overview + Statement only**, not Invoices/Payments (those exist as separate filterable views elsewhere; per-customer-filtered versions weren't part of this slice). Reachable via a new "Statement" row action on `CustomersList`. Worth knowing if a future session goes looking for per-customer Invoices/Payments tabs and doesn't find them — they were never built, not removed.

Two smaller, disclosed gaps from the same session: the Payment Reminder's "Print / Save PDF" button is the OS print dialog (`window.print()`, scoped via `@media print`), not a generated `.pdf` file — there's no reminder PDF renderer in the backend, and the OS dialog's own "Save as PDF" already covers that half; and Reports/Statement CSV export goes through the new `write_export_file` generic command rather than `export_data`, since a parameterized report has no fixed `ExportEntity` shape to extend.

### Session 14 (2026-09-05): V2 frontend manually verified live — the one gap from sessions 12–13 is now closed

Quotes (full Draft → Issue → Accepted → Convert to Invoice lifecycle), the Customer Statement tab, and both Reports screens (Sales Summary, Tax Summary) were all driven in a real running `pnpm tauri dev` window (via macOS Accessibility/AppleScript UI scripting, since screenshot capture wasn't available in that environment) and confirmed working correctly against real existing data, with no console/runtime errors. **The Payment Reminder modal could not be triggered live**: no invoice in the current dataset has a past `due_date`, so `is_overdue` correctly evaluates false everywhere and the Remind button legitimately doesn't render anywhere right now — this was verified as *correct* behavior (checked directly against `reminders.rs`'s predicate and a specific invoice, `INV-2026-0008`), not a bug or an untested path. Manufacturing an overdue invoice to force the click-through was not attempted further: the date-picker's sub-widgets weren't reachable via accessibility scripting, and a direct SQL edit to backdate a real invoice was correctly declined as a destructive action on live data. **Net effect: Round 7 frontend is now considered fully verified**, with the Reminder modal's *rendering* still technically unconfirmed by eye (only its gating logic), pending either a naturally-occurring overdue invoice or a deliberate test one.

### Pre-existing harmless warnings (don't "fix" without a reason)

- `ApplicationError::Infrastructure` field never read — intentional, kept for API completeness.
- `hooks/useCurrency.tsx` triggers one ESLint `react-refresh/only-export-components` warning (exports both a component and a hook) — cosmetic, common context+hook pattern.
- `domain::calculation::present_vat`/`VatPresentation` never used — expected: nothing renders a `VAT_STANDARD` PDF/report yet (that's a later Round 7 slice), so there's no call site until then. Not the ~30-warning situation from earlier in session 12 — `main.rs` is now wired (see below) and that batch of warnings is gone.

### Things that will bite you if you don't know them

- **`printpdf` must keep its `text_layout` feature.** Font subsetting only exists on that feature (`prepare_fonts_for_serialization` has a `#[cfg(not(...))]` arm that embeds the full face). Removing it to slim the dependency tree turns a 32 KB invoice into a 743 KB one. The feature also changes `ParsedFont`'s API — see `infrastructure/pdf/fonts.rs`.
- **Restore closes the database pool and restarts the app**, and must: every repository holds that pool. `restore_backup` never returns on success. Validation happens *before* the pool closes, and extraction goes to a staging file first, so a rejected or corrupt archive leaves the running app untouched.
- **Backups must use `VACUUM INTO`, not a file copy.** WAL mode means the main `.db` file alone is missing committed data.
- **`business.logo_path` is stored relative (`assets/business-logo.png`) for any logo chosen since 2026-08-30 session 4, and absolute for anything chosen before that** — `domain::business::resolve_logo_path` is the one place that knows both. Every reader (`PdfUseCases`, `probe_business_logo`, backup's `assets_to_archive`) must resolve through it rather than opening `logo_path` directly, or a managed logo on a restored/different machine silently fails to open.
- **`domain/currency.rs` and `src/lib/currency.ts` are two copies of the same table.** Add a currency to both, or the screen and the PDF disagree.
- The embedded DejaVu Sans has no glyph for BDT's `৳` or SAR's `﷼`; those fall back to the ISO code by design (`Fonts::can_render`). Don't "fix" it without swapping the font.
- **An issued invoice prints its frozen business snapshot**, so changing the logo (or address, or bank details) in Settings does *not* change invoices already issued — by design (`.ai/product.md`'s locked snapshot principle). Editing one and saving re-snapshots it. This looks like a bug when reported ("my logo isn't showing"); check `business_snapshot_logo_path` on the actual invoice before hunting in the renderer.
- macOS screenshot filenames contain U+202F (narrow no-break space) before `AM`/`PM`. Retyping such a path with an ordinary space silently finds nothing — copy it, don't retype it.
- **Never use `Path::is_absolute()` on a `business.logo_path` string.** It means "absolute for the OS this binary is compiled for" — a legacy Unix path is not `is_absolute()` on Windows, and a Windows drive path is not `is_absolute()` on Unix. `domain::business::looks_absolute` is the portable, OS-independent check both `is_managed_logo_path`/`resolve_logo_path` use instead; CI's Windows runner caught this the first time it ran (see session 9).
- **A workflow needs an explicit `permissions: contents: write`** to have `tauri-action`/`softprops-action-gh-release`-style steps create a GitHub Release — the default `GITHUB_TOKEN` is read-only unless a workflow asks for more, even in a repo the pushing account owns. Silent failure mode is "the build succeeds, only the release-creation API call 404s."
- **`pnpm tauri icon <source>` generates iOS/Android/Windows-Store assets by default** (`icons/ios/`, `icons/android/`, `Square*.png`, `StoreLogo.png`) even though this project only targets desktop. Delete those after regenerating icons — they're pure clutter, nothing in `tauri.conf.json` references them. The brand logo (`src/assets/vunexo-billing-logo.png`) has the full "Vunexo Billing" wordmark baked in, which reads fine at 128px+ but is illegible mush at 32px (taskbar/window-icon size) — known, not yet revisited; a mark-only crop would fix it if it's ever worth the trouble.

## Known gaps (deliberate, not oversights)

- **Only India's GST tax model is implemented.** Currency display is dynamic per-country now, but tax regime logic is India-specific and locked as V1 scope. **User confirmed (2026-08-30): fine for now — multi-country tax support is an explicit future-version item. Don't build it speculatively, but don't design anything that'd make it harder later either.**
- ~~Line-level discounts: engine supports them, editor UI only exposes invoice-level discount.~~ Fixed 2026-08-30 (session 7) — per-line discount type/value now editable in the Invoice Editor's line-item table.
- ~~Dashboard metric cards aren't clickable-through to a filtered Invoices List.~~ Fixed 2026-08-30 (session 7), Overdue card only — see the daily file for why the other four cards deliberately stay non-clickable.
- The PDF prints one neutral `Tax` line outside India rather than a CGST/SGST/IGST split — same India-only constraint as above, and the Invoice Editor's on-screen totals now match it.
- Restore has full test coverage, now including a round-trip against a real-data-shaped copy (session 6), but still hasn't been clicked through in the running app — `app.restart()` specifically can only be confirmed by hand.

## History (agreed order)

1. ~~PDF generation~~ — done 2026-08-30 (session 2). See the daily file for the library choice, the layering, and the font trade-off.
2. ~~Backup/restore + export~~ — done 2026-08-30 (session 3).
3. ~~Make `business.logo_path` app-managed~~ — done 2026-08-30 (session 4).
4. ~~PDF generation audit against real data~~ — done 2026-08-30 (session 5), passed. ~~Restore audit against real data~~ — done 2026-08-30 (session 6), passed. Only `app.restart()` itself (the OS-level relaunch) remains unconfirmed — that needs a human click-through in the running app, see the note in "Known gaps" below.
5. ~~Fix the two actionable known gaps~~ — done 2026-08-30 (session 7): line-level discount UI, Dashboard Overdue-card click-through. The other two "known gaps" (multi-country tax, the PDF's neutral non-India tax line) are locked out-of-scope, not bugs — left as-is on purpose. **User manually confirmed both in the running app (session 9)** — line-discount UI and the Overdue-card click-through both work.
6. ~~Release readiness: license, CI, docs~~ — done 2026-08-30 (session 8), pushed 2026-08-30 (session 9). **First CI run caught a real cross-OS bug** (`domain::business::is_managed_logo_path` used `Path::is_absolute()`, which is platform-dependent — a legacy Unix logo path restored onto Windows was misjudged as a *managed* relative one). Fixed to a portable string check + a regression test covering Unix/Windows-drive/UNC forms; also fixed the CI workflow itself (`node-version: 20` → `22`, pnpm 11.7 requires ≥22.13). Second run: **fully green** on Ubuntu/Windows/macOS.
7. ~~Publish a real installer build~~ — done 2026-08-30 (session 9): `.github/workflows/release.yml` (manual `workflow_dispatch` or an `app-v*` tag), using `tauri-action` across a Windows/macOS/Ubuntu matrix. First attempt built successfully on all three but failed to publish ("Resource not accessible by integration" — the default `GITHUB_TOKEN` lacks `contents: write` unless a workflow explicitly requests it); fixed with an explicit `permissions:` block. Second attempt published a real public release: https://github.com/vunexolabs/vunexo-tools/releases (tag `app-v0.0.0-manual2`, marked prerelease — a manual dev-build tag, not a real version number). **Nobody has actually run these installers on a real Windows/Linux machine yet** — that's still the one thing only a human, on real hardware, can confirm.
8. ~~Product logo + branding~~ — done 2026-08-30 (session 10): app icons (all platforms), a web favicon, and both READMEs now carry the real Vunexo Billing logo, plus a "Download" section in `apps/vunexo-billing/README.md` pointing at the GitHub Releases page (with the SmartScreen/Gatekeeper unsigned-build warning spelled out, since users will hit it immediately).
9. ~~Confirm macOS build works on real hardware~~ — done 2026-08-30 (session 11): user downloaded and ran the `.dmg`, ad-hoc signing fix from session 10 confirmed working, no Gatekeeper "damaged" error.
10. ~~Cut a real v1.0.0 release~~ — done 2026-08-30 (session 11): version bumped to `1.0.0` across `package.json`/`tauri.conf.json`/`Cargo.toml`/`Cargo.lock`, tagged `app-v1.0.0`, release workflow fixed so real tags publish `prerelease: false` (previously always `true`, a leftover from the manual-dev-build era) — both the workflow and the already-published release were corrected. https://github.com/vunexolabs/vunexo-tools/releases/tag/app-v1.0.0

## Next up

- **Confirm Windows/Linux installers on real hardware** — the one remaining V1 DoD item. Not urgent (doesn't block using the app), but V1 isn't *fully* closed out until this happens.
- **V2 design (Rounds 1–6) fully locked** (session 12) — `.ai/product-v2.md` + the six `docs/vunexo-billing/*-v2.md` documents. See the 2026-08-30 daily file for the full history if needed; not repeated here since design is done and this file tracks current state, not history.
- **V2 Round 7 backend is entirely done, session 12** — Quote lifecycle (incl. `convert_quote_to_invoice`) and statements/reports/reminders (7C), all real, tested, and wired into `main.rs`/Tauri. 15 `commands::*` functions total across the two slices. 134 tests passing, `fmt`/`clippy` clean (3 warnings, all expected: `present_vat` has no caller yet since nothing renders a `VAT_STANDARD` PDF/report).
- **V2 frontend, session 12 — Quotes.** `features/quotes/{QuotesList,QuoteEditor}.tsx`, `hooks/useQuotes.ts`, full type/command bridge (`lib/tauri/types.ts`/`commands.ts`), `QuoteStatusBadge`, Quotes as a new nav section in `App.tsx`. **One known, disclosed gap**: a Converted quote shows a plain note instead of a working link to the invoice it produced — no backend reverse-lookup (`quote → its invoice`) was built, since nothing in the locked design asked for one beyond display.
- **V2 frontend, session 13 — Statement tab, Reports, Payment Reminder modal.** See the "Session 13's one real gap" note above for the new `CustomerDetail` screen's scope and the other two disclosed gaps. `typecheck`/`lint`/`build` all clean.
- **Round 7 is now feature-complete and manually verified, session 14 (2026-09-05).** Quotes lifecycle, Statement tab, and both Reports screens confirmed working live against real data — see the "Session 14" note above. Only remaining thread: the Reminder modal's on-screen rendering hasn't been eyeballed (its gating logic has, and is correct) since no invoice is currently overdue — pick this up opportunistically whenever a real invoice goes overdue, or deliberately create a test one if it matters sooner.
- Round 7 (Quotes + tax regime + statements/reports/reminders) can now be considered **done**, pending only that one cosmetic Reminder-modal-rendering confirmation. Next real decision point: what comes after V2 Round 7 — e.g. a fresh audit pass, multi-country tax work (still explicitly deferred), or something new the user wants prioritized.

## Verification commands (all of these, every slice)

```bash
# Backend, from apps/vunexo-billing/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/vunexo-billing/
pnpm typecheck && pnpm lint && pnpm build
```

The user's `pnpm tauri dev` session tends to stay running for an entire work session (backend file-watcher auto-rebuilds, Vite HMR picks up frontend changes) — **check `ps aux | grep "tauri dev"` before launching a new one**, a second instance collides on port 1420. If one's already running, verify with the commands above and let the user reload/test in their live window instead of starting another.

---

# Vunexo Expense Manager

Backend: Rust/Tauri/SQLx, `apps/expense-manager/src-tauri/`. Frontend: React/TS/Tailwind, `apps/expense-manager/src/`. Version `0.1.0` everywhere (`package.json`/`tauri.conf.json`/`Cargo.toml`) — not yet released, not yet manually verified.

**How this fits with the other `.ai`/`docs` files**: `.ai/product-expense-manager.md` is the locked V1 spec (status: locked, round 1). `docs/expense-manager/{user-flows,database-schema,application-architecture,ui-ux,calculation-engine}.md` are Rounds 2–6, all locked, written and implemented in one session (2026-09-05) — see that day's file for the full narrative, including the four scope corrections the user made before locking Round 1.

## Current state

| Slice | Backend | Frontend | Tests |
|---|---|---|---|
| Business profile | ✅ CRUD | ✅ first-run setup gate + Settings tab | covered by integration tests |
| Vendors | ✅ CRUD, `has_expenses` blocked-delete | ✅ Vendors List + Detail | ✅ |
| Categories | ✅ CRUD, `has_expenses` blocked-delete, starter set seeded once on first run | ✅ single inline-edit table (mirrors Billing's Tax Rates) | ✅ (incl. seed-not-resurrected-after-delete) |
| Expenses | ✅ CRUD, vendor/category name+deductibility snapshot on create only (never on update, except when re-picking a different vendor/category) | ✅ Expenses List + Editor | ✅ (incl. the snapshot-immutability rule, the one most likely to regress) |
| Receipts | ✅ attach/replace/remove, app-managed relative path (`receipts/<uuid>.ext`) | ✅ file picker + preview in Expense Editor | ✅ (incl. replace never leaving a dangling reference) |
| Dashboard | ✅ SQL-aggregated | ✅ this period's spend, category breakdown, recent expenses | ✅ |
| Reports (Category/Period/Deductible/Tax-ITC/Top-Vendors) | ✅ SQL-aggregated, all 5 Round 6 test vectors covered | ✅ Reports screen, CSV export via `write_export_file` | ✅ |
| Backup/restore | ✅ `.vex` archive incl. `receipts/` dir, `VACUUM INTO` | ✅ Settings → Data | ✅ (incl. receipt survives round-trip) |

Backend: 40 tests passing, `cargo fmt`/`clippy` clean (one expected warning, same class as Billing's `ApplicationError::Infrastructure` dead-code warning). Frontend: `typecheck`/`lint`/`build` all clean (one expected `react-refresh/only-export-components` warning on `useCurrency.tsx`, same as Billing's).

## Known gaps (disclosed, not oversights)

- **Not manually clicked through in a running window** — no UI-automation tool available this session. Same disclosed-gap shape as Billing's own V2 frontend rounds (sessions 12–13).
- Currency formatting assumes 2 decimal places always (`business.currency_symbol` has no decimals field) — a simplification, not a locked schema decision.
- JSON export isn't wired to a UI button on Reports yet, though `write_export_file` itself is format-agnostic — only CSV has a trigger point.
- No installer has ever been built; nobody has run this on real hardware.

## Release readiness

- `.github/workflows/ci.yml` has `frontend-expense-manager`/`backend-expense-manager` jobs (added 2026-09-05, alongside — not replacing — Billing's existing `frontend`/`backend` jobs).
- `.github/workflows/release-expense-manager.yml` exists (tag prefix `expense-v*`, plus `workflow_dispatch`) but has never been triggered — no tag pushed, no manual dispatch run.
- Nothing has been committed, pushed, tagged, or released yet — the 2026-09-05 session built and verified locally only.

## History (agreed order)

1. ~~Project started: spec through implementation, all in one session~~ — done 2026-09-05 (session 1). Full narrative, all nine rounds, and every disclosed deviation from the locked docs are in `2026-09-05.md`.

## Next up

- Human click-through of the running app (`pnpm tauri dev` from `apps/expense-manager/`) — the same first gap Billing had to close before anything else.
- Decide when to commit/push this work.
- Decide when (if) to actually run `release-expense-manager.yml` and cut a real build.
- Revisit whether any Rust domain primitives (money newtype, currency handling) should become genuinely shared code between the two apps — deliberately deferred at Round 1 ("independent data ≠ zero shared code"), not decided.

## Verification commands

```bash
# Backend, from apps/expense-manager/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/expense-manager/
pnpm typecheck && pnpm lint && pnpm build
```

---

## Daily files in this folder

- `2026-08-28.md` — Rounds 1–6 locked; Business/Customers/Products CRUD + calculation engine implemented.
- `2026-08-29.md` — Invoices vertical slice (draft/issue/cancel/duplicate/list).
- `2026-08-30.md` — thirteen sessions. Session 1: Payments, Dashboard, Settings, Tax Rates, EditIssuedInvoice, UX audit, currency/country support; the progress-tracking system itself was created. Session 2: PDF generation end to end. Session 3: backup/restore + CSV/JSON export (Settings → Data). Session 4: made `business.logo_path` app-managed so it survives a restore onto a different machine. Session 5: audited PDF generation against real invoice data — passed, no defects. Session 6: audited backup/restore against a real-data-shaped copy — passed, no defects. Session 7: fixed the two actionable known gaps (line-level discount UI, Dashboard Overdue click-through). Session 8: release readiness — license, CI, third-party notices, app README. Session 9: pushed to GitHub, fixed what CI's first real run caught (a cross-OS logo-path bug, plus a CI config bug), then built and published the first real installers via a new release workflow. Session 10: added the real Vunexo Billing logo — app icons, favicon, both READMEs, a public-facing Download section. Session 11: macOS build confirmed working on real hardware; cut and published the real `app-v1.0.0` release, fixing the release workflow so real tags publish as full (non-prerelease) releases. Session 12: all six V2 design rounds locked (product scope → user flows → schema → application architecture → UI/UX → calculation engine); Round 7 implementation done through Quotes lifecycle + `ConvertQuoteToInvoice`, `main.rs` wiring, 7C statement/report/reminder backend, and the Quotes frontend. Session 13: finished Round 7 — Statement tab, Reports screens, Payment Reminder modal, plus the small generic `write_export_file` backend addition their CSV/JSON export needed.
- `2026-09-05.md` — Two independent threads. Expense Manager (Project #2) started and built end to end in one session: all nine rounds (spec → user flows → schema → architecture → UI/UX → calculation engine → implementation → testing → release readiness), 40 backend tests passing, `typecheck`/`lint`/`build` clean, not yet manually clicked through/committed/released. Separately, Billing session 14: V2 Round 7 frontend (Quotes, Statement tab, Reports) manually verified live in the running app for the first time — see "Session 14" in the Billing section above.
