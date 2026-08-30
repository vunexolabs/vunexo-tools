//! application-architecture.md §3b. Explicit per-use-case persistence
//! operations, not a generic `save()` — the repository persists what the
//! use case already decided, it doesn't infer intent from a blob.

use async_trait::async_trait;

use crate::domain::invoice::{
    DraftInvoiceToSave, EditIssuedInvoiceData, InvoiceFilter, InvoiceStatus, InvoiceSummary,
    InvoiceWithLineItems, IssueInvoiceData,
};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn create_draft(
        &self,
        tx: &mut dyn Transaction,
        draft: DraftInvoiceToSave,
    ) -> Result<InvoiceWithLineItems, InfrastructureError>;

    /// `ConvertQuoteToInvoice`'s half of the atomic two-table transaction
    /// (application-architecture-v2.md §4c) — identical to `create_draft`
    /// except the new row's `source_quote_id` is set. Kept as its own method
    /// rather than adding an optional field to `create_draft`'s signature, so
    /// the ordinary create-draft path is untouched by the V2 addition.
    async fn create_draft_from_quote(
        &self,
        tx: &mut dyn Transaction,
        source_quote_id: i64,
        draft: DraftInvoiceToSave,
    ) -> Result<InvoiceWithLineItems, InfrastructureError>;

    /// Preconditions (status = Draft) are the use case's job, not the
    /// repository's — see application-architecture.md §4.
    async fn update_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        draft: DraftInvoiceToSave,
    ) -> Result<InvoiceWithLineItems, InfrastructureError>;

    async fn issue(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        data: IssueInvoiceData,
    ) -> Result<InvoiceWithLineItems, InfrastructureError>;

    /// Preconditions (status ∈ {Issued, PartiallyPaid, Paid}) are the use
    /// case's job — see application-architecture.md §4. Never touches
    /// `payments`, `status`, or the invoice number.
    async fn update_issued(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        data: EditIssuedInvoiceData,
    ) -> Result<InvoiceWithLineItems, InfrastructureError>;

    async fn cancel(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), InfrastructureError>;

    /// Used only by the payment recalculation step — application/payments.rs.
    async fn set_status(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        status: InvoiceStatus,
    ) -> Result<(), InfrastructureError>;

    /// Repository trusts the caller already checked status = Draft.
    async fn delete_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
    ) -> Result<(), InfrastructureError>;

    async fn get(&self, id: i64) -> Result<Option<InvoiceWithLineItems>, InfrastructureError>;
    async fn list(&self, filter: InvoiceFilter)
        -> Result<Vec<InvoiceSummary>, InfrastructureError>;

    /// Backs the "`invoice_number_format` locked after the first issued
    /// invoice" rule (database-schema.md §7) — equivalent to "any invoice
    /// with `issued_at IS NOT NULL`" since every non-Draft status passed
    /// through Issued at some point, including Cancelled.
    async fn has_any_issued(&self) -> Result<bool, InfrastructureError>;
}
