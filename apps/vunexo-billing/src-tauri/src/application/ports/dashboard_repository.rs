//! application-architecture.md §3c. One method per dashboard metric, each
//! returning an aggregate value/small struct — never `Vec<Invoice>` or
//! anything shaped like "all rows"; implementations must answer these in
//! SQL (`SUM`/`COUNT`), not by pulling every invoice into Rust and reducing
//! it there.
//!
//! Exact metric definitions (application-architecture.md §3c table):
//! - `today_sales` / `month_sales`: `SUM(total_minor)` for invoices issued
//!   on that date/within that month, `status NOT IN (Draft, Cancelled)`.
//! - `outstanding_total`: `SUM(total_minor - amount_paid)` for
//!   `status IN (Issued, PartiallyPaid)`.
//! - `paid_total`: `SUM(total_minor)` for `status = Paid` **and** issued
//!   within the given month.
//! - `overdue_summary`: count + `SUM(total_minor - amount_paid)` matching
//!   the exact `is_overdue` predicate from database-schema.md §8.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::dashboard::OverdueSummary;
use crate::domain::invoice::InvoiceSummary;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait DashboardRepository: Send + Sync {
    async fn today_sales(&self, today: NaiveDate) -> Result<i64, InfrastructureError>;
    async fn month_sales(&self, year: i32, month: u32) -> Result<i64, InfrastructureError>;
    async fn outstanding_total(&self) -> Result<i64, InfrastructureError>;
    async fn paid_total(&self, year: i32, month: u32) -> Result<i64, InfrastructureError>;
    async fn overdue_summary(
        &self,
        today: NaiveDate,
    ) -> Result<OverdueSummary, InfrastructureError>;
    async fn recent_invoices(&self, limit: i64)
        -> Result<Vec<InvoiceSummary>, InfrastructureError>;
}
