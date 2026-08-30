//! application-architecture-v2.md §2. Purpose-built read port, same rule as
//! `DashboardRepository`: SQL aggregates, never a Rust reduction over pulled
//! rows.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::statement::StatementResult;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait StatementRepository: Send + Sync {
    /// `[range_start, range_end)` — half-open, matching
    /// `database-schema-v2.md` §7's query shape exactly.
    async fn customer_statement(
        &self,
        customer_id: i64,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<StatementResult, InfrastructureError>;
}
