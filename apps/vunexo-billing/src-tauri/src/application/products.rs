//! Product use cases. Mirrors application/customers.rs exactly.

use std::sync::Arc;

use crate::domain::product::{Product, ProductFields, ProductFilter, ProductListItem};

use super::error::ApplicationError;
use super::ports::infrastructure_error::InfrastructureError;
use super::ports::product_repository::ProductRepository;
use super::ports::transaction::TransactionManager;

pub struct ProductUseCases {
    repo: Arc<dyn ProductRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl ProductUseCases {
    pub fn new(repo: Arc<dyn ProductRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    pub async fn create_product(&self, fields: ProductFields) -> Result<Product, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "product name is required".into(),
            ));
        }
        if fields.unit.trim().is_empty() {
            return Err(ApplicationError::Validation("unit is required".into()));
        }
        if fields.price_minor < 0 {
            return Err(ApplicationError::Validation(
                "price cannot be negative".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, fields).await {
            Ok(product) => {
                tx.commit().await?;
                Ok(product)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn update_product(
        &self,
        id: i64,
        fields: ProductFields,
    ) -> Result<Product, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "product name is required".into(),
            ));
        }
        if fields.price_minor < 0 {
            return Err(ApplicationError::Validation(
                "price cannot be negative".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, id, fields).await {
            Ok(product) => {
                tx.commit().await?;
                Ok(product)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn archive_product(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.archive(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn restore_product(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.restore(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn delete_product(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.delete(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(InfrastructureError::ConstraintViolation(_)) => {
                let _ = tx.rollback().await;
                Err(ApplicationError::Conflict(
                    "this product has invoice history and can't be deleted — archive it instead"
                        .into(),
                ))
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_product(&self, id: i64) -> Result<Product, ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "product",
                id,
            })
    }

    pub async fn list_products(
        &self,
        filter: ProductFilter,
    ) -> Result<Vec<ProductListItem>, ApplicationError> {
        Ok(self.repo.list(filter).await?)
    }
}
