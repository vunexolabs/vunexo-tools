<p align="center">
  <img src="src/assets/vunexo-billing-logo.png" alt="Vunexo Billing" width="220">
</p>

# Vunexo Billing

Free, open-source, offline-first invoicing for small businesses — retail shops, freelancers, service providers, contractors, traders, small distributors, home businesses, repair shops, small agencies, and independent professionals.

No account, no subscription, no ads, no cloud dependency. All data stays in a local SQLite database on your machine.

## Download

**[Download the latest build](https://github.com/vunexolabs/vunexo-tools/releases)** — pick the installer for your platform from the most recent release's Assets:

| Platform | File |
|---|---|
| Windows | `Vunexo_Billing_*_x64-setup.exe` or `Vunexo_Billing_*_x64_en-US.msi` |
| macOS | `Vunexo_Billing_*_aarch64.dmg` (or the matching Intel build if offered) |
| Linux | `Vunexo_Billing_*_amd64.deb`, `Vunexo_Billing_*-1.x86_64.rpm`, or the `.AppImage` |

These are unsigned development builds, not notarized/certified releases — Windows SmartScreen and macOS Gatekeeper will both warn on first launch:

- **Windows**: click "More info" → "Run anyway."
- **macOS**: right-click the app → "Open" (only needed the first time), since double-clicking an unsigned app is blocked by default. If macOS instead says the app **"is damaged and can't be opened"** — this happens on Apple Silicon for a build made before ad-hoc signing was added, or if Gatekeeper still won't budge — it's not actually corrupt; clear the quarantine flag it can't verify:
  ```bash
  xattr -cr "/Applications/Vunexo Billing.app"
  ```
  then open it normally.

## Status

In active development (V1). Business profile, customers, products, invoices, payments, PDF generation, the dashboard, and backup/restore/export are implemented and tested — see [`.ai/progress/CURRENT.md`](../../.ai/progress/CURRENT.md) for the exact as-built state.

## Features (V1 scope)

- Business profile: name, logo, address, phone, email, tax/GST info, bank details, UPI ID.
- Customers and products/services, with full history.
- Invoices: create, edit, duplicate, draft/issue, line items, discounts (invoice- and line-level), tax, notes, terms.
- Payments: full/partial, with method, date, reference — status (`Draft`/`Issued`/`Partially Paid`/`Paid`/`Overdue`/`Cancelled`) tracked automatically.
- One professional PDF invoice template — preview, print, save.
- Dashboard: today's/this month's sales, outstanding, paid, overdue.
- Backup, restore, CSV export, JSON export — all local, no cloud.
- India GST (CGST/SGST/IGST) is the only tax regime implemented in V1; other tax models are a planned future addition (see [`.ai/product.md`](../../.ai/product.md)).

## Running from source

Requires [Rust](https://rustup.rs/) (stable) and [pnpm](https://pnpm.io/).

```bash
# from the repo root
pnpm install

# from apps/vunexo-billing/
pnpm tauri dev
```

This starts the Rust/Tauri backend and the Vite dev server together, with hot reload on both sides.

### Building

```bash
# from apps/vunexo-billing/
pnpm tauri build
```

Produces a native installer/bundle for whichever platform you build it on (macOS/Windows/Linux) under `src-tauri/target/release/bundle/`.

### Verification

```bash
# Backend, from apps/vunexo-billing/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/vunexo-billing/
pnpm typecheck && pnpm lint && pnpm build
```

CI (`.github/workflows/ci.yml`) runs the same checks on every push/PR, across Ubuntu, Windows, and macOS.

## Architecture

Rust/Tauri/SQLx backend (`src-tauri/`, layered `commands → application → domain → infrastructure`), React/TypeScript/Tailwind frontend (`src/`). See [`docs/vunexo-billing/`](../../docs/vunexo-billing/) for the locked design docs (user flows, database schema, application architecture, UI/UX, calculation engine) and [`.ai/decisions/`](../../.ai/decisions/) for architecture decision records.

## License

MIT — see [`LICENSE`](../../LICENSE) and [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
