//! Pure business logic (invoice, tax, payment rules). Must have zero
//! dependencies on infrastructure or framework concerns — no Tauri, no SQLx,
//! no filesystem, no PDF libraries. See docs/vunexo-billing/architecture.md,
//! rule 3.

pub mod business;
pub mod calculation;
pub mod customer;
pub mod invoice;
pub mod invoice_line_item;
pub mod money;
pub mod product;
pub mod settings;
