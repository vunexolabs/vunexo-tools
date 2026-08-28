---
status: accepted
round: 1
---

# ADR-001: Desktop Application Stack

## Decision

**Tauri 2** (Rust backend + system WebView) with a **TypeScript + React** frontend and **SQLite via SQLx**, over Electron, Flutter, Wails, .NET MAUI, and Qt.

## Why

- Local-first fit: a Rust backend with direct, synchronous-feeling access to the filesystem and SQLite suits an offline-only application.
- Smaller runtime footprint than Electron specifically because Tauri uses the operating system's WebView instead of bundling Chromium — this is a comparative claim about the runtime, not a blanket "Tauri is lightweight" claim; the application's own dependencies and implementation still determine actual footprint and performance.
- Native filesystem access from Rust, useful for backup/restore and PDF export.
- Rust ecosystem and tooling (cargo, crates.io) fit a utility-software direction for VunexoLabs generally, not just Vunexo Billing.
- TypeScript + React frontend keeps the UI layer in familiar, widely-supported web technology, which also keeps the door open to reusing frontend code for a possible future web build.

## Alternatives considered

- **Electron**: mature and extremely well-documented, but bundles a full Chromium runtime per application — heavier than the system-WebView approach Tauri takes.
- **Flutter**: a strong cross-platform option, but introduces Dart and a separate UI ecosystem; unnecessary given a web-technology-based frontend strategy.
- **.NET MAUI**: mature Microsoft ecosystem, but introduces C#/.NET and gives less alignment with a Rust + webview architecture chosen for VunexoLabs' local-first utilities.
- **Qt**: highly capable native framework, but introduces Qt/C++/QML — a substantially different development ecosystem from the rest of the stack.
- **Wails**: the closest analog to Tauri (Go backend + system webview), but has a smaller ecosystem and less maturity than Tauri at the time of this decision.

## Consequences

- Business logic (domain/application layers) is written in Rust, not TypeScript/Node — no Electron main-process/IPC/preload split; instead, a `commands → application → domain → infrastructure` layering inside the Rust crate (see [docs/vunexo-billing/architecture.md](../../docs/vunexo-billing/architecture.md)).
- SQLite access uses SQLx (async, compile-time-checked queries, built-in migration runner) rather than Diesel or raw `rusqlite`.
- The frontend communicates with the backend exclusively through Tauri's `invoke()` command bridge — never direct filesystem or database access from React.

## Status

Accepted.
