//! Pure business logic (invoice, tax, payment rules). Must have zero
//! dependencies on infrastructure or framework concerns — no Tauri, no SQLx,
//! no filesystem, no PDF libraries. See docs/vunexo-billing/architecture.md,
//! rule 3. Real domain types land in later rounds.
