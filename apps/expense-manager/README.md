# Vunexo Expense Manager

Free, open-source, offline-first expense tracking for small businesses — retail shops, freelancers, service providers, contractors, traders, small agencies, and independent professionals.

No account, no subscription, no ads, no cloud dependency. All data stays in a local SQLite database on your machine.

**Expense management, not accounting.** Vunexo Billing (the companion app in this monorepo) answers "what did I sell and who owes me?" Expense Manager answers "what did my business spend and what did I spend it on?" Neither one is an ERP — see [`.ai/product-expense-manager.md`](../../.ai/product-expense-manager.md) for the locked scope, including what's deliberately out (no tax filing, no tax advice, no automatic ITC/deductibility determination — the app records what you tell it, it doesn't decide what's legally deductible).

## Status

V1 implemented (backend + frontend, tested via `cargo test` and `typecheck`/`lint`/`build`) but **not yet manually clicked through in a running window, and not yet released** — no installers have been built or published. See [`.ai/progress/CURRENT.md`](../../.ai/progress/CURRENT.md) for the exact as-built state.

## Features (V1 scope)

- Business profile: name, address, tax info.
- Vendors: create/edit/delete, with expense history.
- Expense categories: starter set + custom, each with a default tax-deductibility classification.
- Expenses: date, amount, category, vendor, payment method, tax amount, a separate ITC-eligibility flag, notes, optional receipt image attachment.
- Reports: category totals, period totals, deductible/non-deductible summary, tax/ITC summary, top vendors.
- Dashboard: this period's spend, category breakdown, recent expenses.
- Backup, restore, CSV/JSON export — all local, no cloud.

## Running from source

Requires [Rust](https://rustup.rs/) (stable) and [pnpm](https://pnpm.io/).

```bash
# from the repo root
pnpm install

# from apps/expense-manager/
pnpm tauri dev
```

This starts the Rust/Tauri backend and the Vite dev server together, with hot reload on both sides.

### Building

```bash
# from apps/expense-manager/
pnpm tauri build
```

Produces a native installer/bundle for whichever platform you build it on (macOS/Windows/Linux) under `src-tauri/target/release/bundle/`.

### Verification

```bash
# Backend, from apps/expense-manager/src-tauri/
cargo build && cargo test --quiet && cargo fmt --check && cargo clippy --all-targets --quiet

# Frontend, from apps/expense-manager/
pnpm typecheck && pnpm lint && pnpm build
```

CI (`.github/workflows/ci.yml`) runs the same checks on every push/PR, across Ubuntu, Windows, and macOS.

## Architecture

Rust/Tauri/SQLx backend (`src-tauri/`, layered `commands → application → domain → infrastructure`), React/TypeScript/Tailwind frontend (`src/`). See [`docs/expense-manager/`](../../docs/expense-manager/) for the locked design docs (user flows, database schema, application architecture, UI/UX, calculation engine).

## License

MIT — see [`LICENSE`](../../LICENSE) and [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
