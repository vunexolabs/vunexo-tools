---
status: accepted
round: 7
---

# ADR-002: Project License

## Decision

**MIT**, as `.ai/product.md` recommended, with the final decision deferred there "until third-party dependencies/templates used are reviewed." That review is what this ADR records.

## Dependency license audit

Ran `cargo metadata` over `apps/vunexo-billing/src-tauri` (595 packages reachable from the `vunexo-billing` binary, including transitive dependencies) and inspected `package.json`'s direct + dev dependencies. Findings:

- No GPL, AGPL, or other strong-copyleft license anywhere in the tree.
- The overwhelming majority (over 500 of 595 Rust crates) are MIT, `MIT OR Apache-2.0` (or an equivalent permissive dual/triple license), BSD-2/3-Clause, Zlib, ISC, or Unlicense — all straightforwardly compatible with an MIT top-level license.
- Six crates are MPL-2.0 (`azul-simplecss`, `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors` — pulled in transitively by `printpdf`'s `text_layout` feature, per `apps/vunexo-billing/src-tauri/Cargo.toml`'s comment on that feature). MPL-2.0 is file-level weak copyleft: it only requires that *modifications to MPL-covered files themselves* stay under MPL if distributed in source form. It does not require the combining work (this application) to be MPL-licensed, and imposes no additional obligation on a compiled binary distribution beyond the license text itself — not a blocker for an MIT-licensed application.
- `r-efi` offers `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; the permissive option is what applies here since nothing requires picking the LGPL leg.
- Frontend `dependencies`/`devDependencies` (React, Tauri's JS bindings, Vite, TypeScript, ESLint, Tailwind, etc.) are all standard MIT/Apache-2.0/BSD-licensed packages — no concerns.

## Consequences

- `LICENSE` (MIT, root of the repo) is the canonical license text.
- `license = "MIT"` is set on `apps/vunexo-billing/src-tauri/Cargo.toml`, and `"license": "MIT"` on both `package.json` files, so tooling (crates.io metadata, `cargo build`'s manifest checks, npm registries if ever published) agrees with the repo-level `LICENSE`.
- `THIRD_PARTY_NOTICES.md` documents the dependency license landscape summarized above and how to regenerate the full per-crate list, rather than hand-maintaining an enumeration of ~600 crates that will drift on every `cargo update`.

## Status

Accepted.
