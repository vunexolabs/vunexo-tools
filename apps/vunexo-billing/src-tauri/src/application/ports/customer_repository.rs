//! application-architecture.md §3b.

use async_trait::async_trait;

use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: CustomerFields,
    ) -> Result<Customer, InfrastructureError>;
    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: CustomerFields,
    ) -> Result<Customer, InfrastructureError>;
    async fn archive(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn restore(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    /// Surfaces `InfrastructureError::ConstraintViolation` if the customer is
    /// referenced by any invoice — the caller (the `DeleteCustomer` use case)
    /// translates that into `ApplicationError::Conflict`.
    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Customer>, InfrastructureError>;
    async fn list(
        &self,
        filter: CustomerFilter,
    ) -> Result<Vec<CustomerListItem>, InfrastructureError>;
}
