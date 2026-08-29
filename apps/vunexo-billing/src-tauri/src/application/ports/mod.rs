//! Repository and transaction traits — the "Ports / Interfaces" from
//! docs/vunexo-billing/architecture.md. `application` owns these traits;
//! `infrastructure` implements them.

pub mod business_repository;
pub mod customer_repository;
pub mod infrastructure_error;
pub mod invoice_number_sequencer;
pub mod invoice_repository;
pub mod product_repository;
pub mod settings_repository;
pub mod transaction;
