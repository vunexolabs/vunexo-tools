//! application-architecture-v2.md §2. Purpose-built read port, same rule as
//! `DashboardRepository`.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::report::{SalesGrouping, SalesSummaryResult, TaxSummaryResult};

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait ReportRepository: Send + Sync {
    async fn sales_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        group_by: SalesGrouping,
    ) -> Result<SalesSummaryResult, InfrastructureError>;

    /// Grouped by `tax_regime_snapshot`, per database-schema-v2.md §7's
    /// mixed-regime edge case.
    async fn tax_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxSummaryResult, InfrastructureError>;
}
