//! application-architecture-v2.md §2. Explicit per-use-case persistence
//! operations, mirroring `InvoiceRepository` — not a generic `save()`.

use async_trait::async_trait;

use crate::domain::quote::{
    DraftQuoteToSave, IssueQuoteData, QuoteFilter, QuoteSummary, QuoteWithLineItems,
};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait QuoteRepository: Send + Sync {
    async fn create_draft(
        &self,
        tx: &mut dyn Transaction,
        draft: DraftQuoteToSave,
    ) -> Result<QuoteWithLineItems, InfrastructureError>;

    /// Preconditions (status = Draft) are the use case's job — quotes are
    /// editable in Draft only (user-flows-v2.md §2), unlike invoices.
    async fn update_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        draft: DraftQuoteToSave,
    ) -> Result<QuoteWithLineItems, InfrastructureError>;

    async fn issue(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        data: IssueQuoteData,
    ) -> Result<QuoteWithLineItems, InfrastructureError>;

    async fn accept(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn decline(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;

    /// Sets `converted_at` + `status = Converted` only — never touches
    /// `invoices` (application-architecture-v2.md §2: each repository stays
    /// scoped to its own table).
    async fn mark_converted(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
    ) -> Result<(), InfrastructureError>;

    async fn cancel(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), InfrastructureError>;

    async fn delete_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
    ) -> Result<(), InfrastructureError>;

    async fn get(&self, id: i64) -> Result<Option<QuoteWithLineItems>, InfrastructureError>;
    async fn list(&self, filter: QuoteFilter) -> Result<Vec<QuoteSummary>, InfrastructureError>;

    /// Backs the "`quote_number_format` locked after the first issued quote"
    /// rule — same shape as `InvoiceRepository::has_any_issued`, an
    /// independent lock from the invoice one (database-schema-v2.md §6).
    async fn has_any_issued(&self) -> Result<bool, InfrastructureError>;
}
