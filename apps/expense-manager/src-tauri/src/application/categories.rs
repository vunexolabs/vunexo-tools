//! Category use cases. application-architecture.md's module layout.
//! user-flows.md §4 — editing a category's name/default-deductible flag must
//! never change deductibility already recorded on existing expenses; that
//! rule is enforced by `Expense.deductible` being written once at creation
//! (see `application::expenses::ExpenseUseCases::create_expense`), never by
//! anything in this module re-touching existing rows.

use std::sync::Arc;

use crate::domain::category::{Category, CategoryFields, CategoryId, CategoryListItem};

use super::error::ApplicationError;
use super::ports::category_repository::CategoryRepository;

pub struct CategoryUseCases {
    repo: Arc<dyn CategoryRepository>,
}

impl CategoryUseCases {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_category(
        &self,
        fields: CategoryFields,
    ) -> Result<Category, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "category name is required".into(),
            ));
        }
        Ok(self.repo.create(fields).await?)
    }

    pub async fn update_category(
        &self,
        id: CategoryId,
        fields: CategoryFields,
    ) -> Result<Category, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "category name is required".into(),
            ));
        }
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "category",
                id,
            })?;
        Ok(self.repo.update(id, fields).await?)
    }

    /// user-flows.md §4 — same blocked-delete shape as vendor delete.
    pub async fn delete_category(&self, id: CategoryId) -> Result<(), ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "category",
                id,
            })?;
        if self.repo.has_expenses(id).await? {
            return Err(ApplicationError::Validation(
                "this category has expenses recorded and can't be deleted".into(),
            ));
        }
        self.repo.delete(id).await?;
        Ok(())
    }

    pub async fn list_categories(&self) -> Result<Vec<CategoryListItem>, ApplicationError> {
        Ok(self.repo.list().await?)
    }
}
