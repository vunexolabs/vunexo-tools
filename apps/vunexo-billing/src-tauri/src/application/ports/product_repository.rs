//! Mirrors application/ports/customer_repository.rs exactly.

use async_trait::async_trait;

use crate::domain::product::{Product, ProductFields, ProductFilter, ProductListItem};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: ProductFields,
    ) -> Result<Product, InfrastructureError>;
    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: ProductFields,
    ) -> Result<Product, InfrastructureError>;
    async fn archive(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn restore(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Product>, InfrastructureError>;
    async fn list(
        &self,
        filter: ProductFilter,
    ) -> Result<Vec<ProductListItem>, InfrastructureError>;
}
