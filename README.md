<p align="center">
  <img src="apps/vunexo-billing/src/assets/vunexo-billing-logo.png" alt="Vunexo Billing" width="200">
</p>

# Vunexo Tools

Monorepo for VunexoLabs' free, open-source, offline-first desktop tools.

- **Project #1: [Vunexo Billing](apps/vunexo-billing/)** — invoicing software for small businesses. See [.ai/product.md](.ai/product.md) for the locked product spec.
- **Project #2: [Vunexo Expense Manager](apps/expense-manager/)** — expense tracking for small businesses (not accounting). See [.ai/product-expense-manager.md](.ai/product-expense-manager.md) for the locked product spec.

Each app is independent — its own SQLite database, own business profile, no coupling between them. See [.ai/decisions/](.ai/decisions/) for architecture decision records shared across the monorepo.

**[Download Vunexo Billing](https://github.com/vunexolabs/vunexo-tools/releases)** for Windows, macOS, or Linux. Vunexo Expense Manager hasn't been released yet — see its [README](apps/expense-manager/README.md) for current status.

Licensed under [MIT](LICENSE); see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the dependency license audit behind that choice.
