# Vunexo Billing — Architecture (Round 1)

This is an AI context file. Before modifying code, read this document. Do not violate a locked decision below without explicitly proposing a change (an ADR under `.ai/decisions/`) first. See also [ADR-001](../../.ai/decisions/ADR-001-desktop-stack.md) for why this stack was chosen.

## Dependency direction

```
React UI
   ↓
Tauri API adapter        (src/lib/tauri/*)
   ↓
Tauri commands            (src-tauri/src/commands)
   ↓
Application layer         (src-tauri/src/application — use-case orchestration)
   ↓
Domain                    (src-tauri/src/domain — pure business logic)
   ↓
Infrastructure            (src-tauri/src/infrastructure — SQLite/SQLx, filesystem, PDF)
```

Application depends on domain plus the ports/interfaces it defines itself; infrastructure provides the concrete implementations of those ports (dependency inversion):

```
application/  →  Ports / Interfaces  ←  infrastructure/
                  (e.g. InvoiceRepository)   (e.g. SqliteInvoiceRepository
                                               implements InvoiceRepository)
```

## Rules (locked)

1. **Never `React → SQLite` directly.** All persistence goes through a Tauri command.
2. **Never `Domain → Tauri`.** The domain layer must not know Tauri exists, so it stays portable behind a future HTTP API for a possible web version.
3. **Infrastructure dependencies must not leak into the domain layer.** `domain/` must have zero dependencies on infrastructure or framework concerns — including Tauri, SQLx, filesystem APIs, PDF-generation libraries, and UI/runtime frameworks. This is a rule about the *category* of dependency, not today's specific crates: swapping SQLx for a different persistence library later does not make a domain-layer dependency on it acceptable.
4. **`application/` does not reach directly into `infrastructure/`.** It depends on ports/interfaces it owns; `infrastructure/` implements them.
5. **`src/lib/tauri/client.ts` is the only frontend file that imports `@tauri-apps/api`.** Everything else in `src/` calls through `src/lib/tauri/commands.ts`.

## Layer responsibilities

| Layer | Location | Responsibility |
|---|---|---|
| Commands | `src-tauri/src/commands/` | Thin `#[tauri::command]` handlers: deserialize input, call into `application/`, serialize output. No business logic. |
| Application | `src-tauri/src/application/` | Use-case orchestration (e.g. "create invoice"). Depends on `domain/` and on ports it defines. |
| Domain | `src-tauri/src/domain/` | Pure business types and logic (invoice, tax, payment rules). No framework or infrastructure imports. |
| Infrastructure | `src-tauri/src/infrastructure/{database,filesystem,pdf}/` | Concrete implementations: SQLx-backed repositories, file I/O for backup/export, PDF rendering. |

## Frontend structure

```
src/
├── app/            # app shell, routing
├── features/        # domain-boundary UI modules: dashboard, customers, products, invoices, payments, reports, settings
├── components/       # shared/reusable UI components
├── hooks/
├── stores/
└── lib/tauri/         # invoke() wrapper (client.ts) + typed command signatures (commands.ts)
```

## Status of this document

Round 1 establishes the layering and the rules above; the concrete tables, use cases, and screens are designed in later rounds (see `.ai/product.md` roadmap) and must conform to this document once written.
