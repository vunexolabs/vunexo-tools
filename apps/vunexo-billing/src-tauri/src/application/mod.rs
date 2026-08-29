//! Use-case orchestration. Depends on `crate::domain` and on ports it
//! defines itself; never reaches directly into `crate::infrastructure`
//! (dependency inversion — infrastructure implements the ports defined here).

pub mod business;
pub mod customers;
pub mod error;
pub mod invoices;
pub mod ports;
pub mod products;
pub mod settings;

pub use error::ApplicationError;
