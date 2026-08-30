//! Pure business logic (invoice, tax, payment rules). Must have zero
//! dependencies on infrastructure or framework concerns — no Tauri, no SQLx,
//! no filesystem, no PDF libraries. See docs/vunexo-billing/architecture.md,
//! rule 3.

pub mod backup;
pub mod business;
pub mod calculation;
pub mod currency;
pub mod customer;
pub mod dashboard;
pub mod export;
pub mod invoice;
pub mod invoice_line_item;
pub mod invoice_pdf;
pub mod money;
pub mod payment;
pub mod product;
pub mod settings;
pub mod tax_rate;
