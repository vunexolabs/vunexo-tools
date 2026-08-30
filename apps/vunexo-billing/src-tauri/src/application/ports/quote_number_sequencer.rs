//! application-architecture-v2.md §2. Mirrors `InvoiceNumberSequencer`
//! exactly, on its own counter table (`quote_number_counters`) — kept
//! parallel rather than merged into a generalized "document counter";
//! database-schema-v2.md §1 explains why.

use async_trait::async_trait;
use chrono::NaiveDate;

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait QuoteNumberSequencer: Send + Sync {
    async fn preview_next(
        &self,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError>;

    /// Only ever called as part of the same transaction as the rest of
    /// `IssueQuote` — same never-burn-a-number-on-rollback guarantee as
    /// `InvoiceNumberSequencer::issue_next`.
    async fn issue_next(
        &self,
        tx: &mut dyn Transaction,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError>;
}
