//! application-architecture.md's module layout.

use async_trait::async_trait;

use crate::domain::category::{Category, CategoryFields, CategoryId, CategoryListItem};

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn create(&self, fields: CategoryFields) -> Result<Category, InfrastructureError>;
    async fn update(
        &self,
        id: CategoryId,
        fields: CategoryFields,
    ) -> Result<Category, InfrastructureError>;
    async fn delete(&self, id: CategoryId) -> Result<(), InfrastructureError>;
    async fn find_by_id(&self, id: CategoryId) -> Result<Option<Category>, InfrastructureError>;
    async fn list(&self) -> Result<Vec<CategoryListItem>, InfrastructureError>;
    /// user-flows.md §4 — the check `DeleteCategory` calls before allowing a
    /// delete, same pattern as vendor delete.
    async fn has_expenses(&self, id: CategoryId) -> Result<bool, InfrastructureError>;
}
