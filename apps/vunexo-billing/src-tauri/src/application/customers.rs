//! Customer use cases. application-architecture.md §4.

use std::sync::Arc;

use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};

use super::error::ApplicationError;
use super::ports::customer_repository::CustomerRepository;
use super::ports::infrastructure_error::InfrastructureError;
use super::ports::transaction::TransactionManager;

pub struct CustomerUseCases {
    repo: Arc<dyn CustomerRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl CustomerUseCases {
    pub fn new(repo: Arc<dyn CustomerRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    pub async fn create_customer(
        &self,
        fields: CustomerFields,
    ) -> Result<Customer, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "customer name is required".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, fields).await {
            Ok(customer) => {
                tx.commit().await?;
                Ok(customer)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn update_customer(
        &self,
        id: i64,
        fields: CustomerFields,
    ) -> Result<Customer, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "customer name is required".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, id, fields).await {
            Ok(customer) => {
                tx.commit().await?;
                Ok(customer)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn archive_customer(&self, id: i64) -> Result<(), ApplicationError> {
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

    pub async fn restore_customer(&self, id: i64) -> Result<(), ApplicationError> {
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

    /// Attempts a hard delete; a referenced customer surfaces as
    /// `ApplicationError::Conflict` (application-architecture.md §4)
    /// rather than a raw database error reaching the UI.
    pub async fn delete_customer(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.delete(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(InfrastructureError::ConstraintViolation(_)) => {
                let _ = tx.rollback().await;
                Err(ApplicationError::Conflict(
                    "this customer has invoice history and can't be deleted — archive it instead"
                        .into(),
                ))
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_customer(&self, id: i64) -> Result<Customer, ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "customer",
                id,
            })
    }

    pub async fn list_customers(
        &self,
        filter: CustomerFilter,
    ) -> Result<Vec<CustomerListItem>, ApplicationError> {
        Ok(self.repo.list(filter).await?)
    }
}
