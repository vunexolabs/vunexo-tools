//! application-architecture.md's module layout. Purpose-built read port,
//! same rule as `DashboardRepository` — aggregation happens in SQL.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::report::{
    CategorySummaryResult, DeductibleSummaryResult, PeriodSummaryResult, TaxItcSummaryResult,
    TopVendorsResult,
};

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait ReportRepository: Send + Sync {
    async fn category_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<CategorySummaryResult, InfrastructureError>;

    async fn period_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<PeriodSummaryResult, InfrastructureError>;

    async fn deductible_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<DeductibleSummaryResult, InfrastructureError>;

    async fn tax_itc_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxItcSummaryResult, InfrastructureError>;

    async fn top_vendors(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        limit: i64,
    ) -> Result<TopVendorsResult, InfrastructureError>;
}
