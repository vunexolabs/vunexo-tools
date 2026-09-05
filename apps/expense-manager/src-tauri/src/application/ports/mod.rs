//! Ports the `application` layer depends on; `infrastructure` implements
//! them. See application-architecture.md's module layout.

pub mod backup_archive;
pub mod business_repository;
pub mod category_repository;
pub mod dashboard_repository;
pub mod database_file;
pub mod expense_repository;
pub mod file_writer;
pub mod infrastructure_error;
pub mod receipt_store;
pub mod report_repository;
pub mod vendor_repository;
