//! Pure business logic (business profile, vendor, category, expense,
//! dashboard, report, backup rules). Must have zero dependencies on
//! infrastructure or framework concerns — no Tauri, no SQLx, no filesystem.
//! See application-architecture.md's module layout.

pub mod backup;
pub mod business;
pub mod category;
pub mod dashboard;
pub mod expense;
pub mod money;
pub mod receipt;
pub mod report;
pub mod vendor;
