//! application-architecture.md §3b. `preview_next` is read-only and never
//! reserves a number (database-schema.md §7) — only `issue_next`, called
//! exclusively from `IssueInvoice`'s transaction, actually advances the
//! counter.

use async_trait::async_trait;
use chrono::NaiveDate;

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait InvoiceNumberSequencer: Send + Sync {
    async fn preview_next(
        &self,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError>;

    /// Only ever called as part of the same transaction as the rest of
    /// `IssueInvoice` — see calculation-engine.md's counterpart discussion
    /// in application-architecture.md §4c/§7: a failed issue must roll back
    /// the counter increment too, so a number is never burned by a failure.
    async fn issue_next(
        &self,
        tx: &mut dyn Transaction,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError>;
}
