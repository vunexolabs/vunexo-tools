//! Business use cases. application-architecture.md §4 ("Business" bullet).

use std::sync::Arc;

use crate::domain::business::Business;

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::transaction::TransactionManager;

pub struct BusinessUseCases {
    repo: Arc<dyn BusinessRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl BusinessUseCases {
    pub fn new(repo: Arc<dyn BusinessRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    /// Precondition: no business row exists yet (user-flows.md §1) — enforced
    /// here since "exactly once" is the entire point of this use case.
    pub async fn create_business(&self, business: Business) -> Result<Business, ApplicationError> {
        if self.repo.get().await?.is_some() {
            return Err(ApplicationError::Conflict(
                "a business profile already exists".into(),
            ));
        }
        if business.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "business name is required".into(),
            ));
        }

        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, business).await {
            Ok(created) => {
                tx.commit().await?;
                Ok(created)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_business(&self) -> Result<Option<Business>, ApplicationError> {
        Ok(self.repo.get().await?)
    }

    pub async fn update_business(&self, business: Business) -> Result<Business, ApplicationError> {
        if business.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "business name is required".into(),
            ));
        }

        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, business).await {
            Ok(updated) => {
                tx.commit().await?;
                Ok(updated)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }
}
