//! Dashboard aggregate types. application-architecture.md §3c — one small
//! struct per aggregate query the repository answers in SQL, never a
//! `Vec<Invoice>` pulled into Rust and reduced there.

use super::invoice::InvoiceSummary;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct OverdueSummary {
    pub count: i64,
    pub total_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardMetrics {
    pub today_sales_minor: i64,
    pub month_sales_minor: i64,
    pub outstanding_total_minor: i64,
    pub paid_total_minor: i64,
    pub overdue: OverdueSummary,
    pub recent_invoices: Vec<InvoiceSummary>,
}
