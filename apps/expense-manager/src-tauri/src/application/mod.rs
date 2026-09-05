//! Use-case orchestration. Depends on `crate::domain` and on ports it
//! defines itself; never reaches directly into `crate::infrastructure`
//! (dependency inversion — infrastructure implements the ports defined here).

pub mod backup;
pub mod business;
pub mod categories;
pub mod dashboard;
pub mod error;
pub mod expenses;
pub mod export;
pub mod ports;
pub mod reports;
pub mod vendors;

pub use error::ApplicationError;
