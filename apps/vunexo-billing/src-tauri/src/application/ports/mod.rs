//! Repository and transaction traits — the "Ports / Interfaces" from
//! docs/vunexo-billing/architecture.md. `application` owns these traits;
//! `infrastructure` implements them.

pub mod backup_archive;
pub mod business_repository;
pub mod customer_repository;
pub mod dashboard_repository;
pub mod database_file;
pub mod file_writer;
pub mod infrastructure_error;
pub mod invoice_number_sequencer;
pub mod invoice_pdf_renderer;
pub mod invoice_repository;
pub mod payment_repository;
pub mod product_repository;
pub mod quote_number_sequencer;
pub mod quote_repository;
pub mod report_repository;
pub mod settings_repository;
pub mod statement_repository;
pub mod tax_rate_repository;
pub mod transaction;
