# Third-Party Notices

Vunexo Billing is MIT-licensed (see [`LICENSE`](LICENSE)) and built on open-source dependencies under their own licenses. See [`.ai/decisions/ADR-002-license.md`](.ai/decisions/ADR-002-license.md) for the audit behind this file.

## Summary

As of this writing, the Rust dependency tree (`apps/vunexo-billing/src-tauri`, 595 crates including transitive dependencies) is:

- No GPL, AGPL, or other strong-copyleft licenses.
- The large majority are MIT, `MIT OR Apache-2.0`, BSD-2/3-Clause, Zlib, ISC, or Unlicense.
- Six crates are MPL-2.0 (`azul-simplecss`, `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`), pulled in transitively via `printpdf`'s `text_layout` feature. MPL-2.0 is file-level weak copyleft and imposes no obligation on this application's own license.

The frontend dependency tree (`apps/vunexo-billing/package.json`) is entirely MIT/Apache-2.0/BSD-licensed (React, Tauri's JS bindings, Vite, TypeScript, ESLint, Tailwind CSS, etc.).

## Regenerating the full list

This file intentionally does not enumerate every crate by name — the list changes on every `cargo update`/`pnpm update` and would drift immediately. To regenerate the exact current list before a release:

```bash
# Rust dependencies, from apps/vunexo-billing/src-tauri/
cargo metadata --format-version 1 | python3 -c "
import json, sys
d = json.load(sys.stdin)
for pkg in sorted(d['packages'], key=lambda p: p['name']):
    print(f\"{pkg['name']} {pkg['version']}: {pkg.get('license') or 'UNKNOWN'}\")
"

# Frontend dependencies, from apps/vunexo-billing/
pnpm licenses list
```
