//! application-architecture.md's module layout. SQL-aggregated, same
//! discipline as Billing's `DashboardRepository`.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::dashboard::CategoryBreakdownRow;
use crate::domain::expense::Expense;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait DashboardRepository: Send + Sync {
    /// Sum of `amount_minor` for expenses dated within
    /// `[period_start, period_end)`.
    async fn period_total(
        &self,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<i64, InfrastructureError>;

    async fn category_breakdown(
        &self,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Vec<CategoryBreakdownRow>, InfrastructureError>;

    async fn recent_expenses(&self, limit: i64) -> Result<Vec<Expense>, InfrastructureError>;
}
