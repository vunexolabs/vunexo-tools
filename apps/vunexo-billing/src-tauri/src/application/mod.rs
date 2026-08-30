//! Use-case orchestration. Depends on `crate::domain` and on ports it
//! defines itself; never reaches directly into `crate::infrastructure`
//! (dependency inversion — infrastructure implements the ports defined here).

pub mod backup;
pub mod business;
pub mod customers;
pub mod dashboard;
pub mod error;
pub mod export;
pub mod file_export;
pub mod invoices;
pub mod payments;
pub mod pdf;
pub mod ports;
pub mod products;
pub mod quotes;
pub mod reminders;
pub mod reports;
pub mod settings;
pub mod statements;
pub mod tax_rates;

pub use error::ApplicationError;
