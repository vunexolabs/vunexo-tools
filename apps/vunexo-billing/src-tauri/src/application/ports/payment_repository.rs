//! application-architecture.md §3b. Payments are independent of the
//! invoice edit lifecycle (database-schema.md §8) — this port only ever
//! touches `payments`, never `invoices` itself. Recalculating the parent
//! invoice's `status` afterward is `application/payments.rs`'s job, via
//! `InvoiceRepository::set_status`, in the same transaction.

use async_trait::async_trait;

use crate::domain::payment::{NewPayment, Payment, PaymentFields};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        payment: NewPayment,
    ) -> Result<Payment, InfrastructureError>;

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: PaymentFields,
    ) -> Result<Payment, InfrastructureError>;

    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;

    async fn get(&self, id: i64) -> Result<Option<Payment>, InfrastructureError>;

    async fn list_for_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, InfrastructureError>;

    /// Must run against the same transaction as the write it follows, so it
    /// observes the just-created/updated/deleted row before commit — the
    /// status recalculation this feeds (database-schema.md §3's worked
    /// example) has to reflect the write that triggered it, not the
    /// last-committed state.
    async fn sum_for_invoice(
        &self,
        tx: &mut dyn Transaction,
        invoice_id: i64,
    ) -> Result<i64, InfrastructureError>;
}
