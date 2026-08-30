//! application-architecture.md §3b. No `delete` — a handful of GST slabs,
//! deliberately not worth the "is it referenced" complexity a delete path
//! would need in V1.

use async_trait::async_trait;

use crate::domain::tax_rate::{TaxRate, TaxRateFields};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait TaxRateRepository: Send + Sync {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: TaxRateFields,
    ) -> Result<TaxRate, InfrastructureError>;

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: TaxRateFields,
    ) -> Result<TaxRate, InfrastructureError>;

    async fn list(&self) -> Result<Vec<TaxRate>, InfrastructureError>;
}
