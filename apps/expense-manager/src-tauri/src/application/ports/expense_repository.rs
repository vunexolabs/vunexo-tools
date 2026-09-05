//! application-architecture.md's module layout. `ExpenseFilter` folds
//! "list_by_category"/"list_by_vendor"/"list_by_date_range" into one
//! combinable filter behind the single `list_expenses` Tauri command the
//! command-surface section actually names — the Expenses List screen (and a
//! dashboard category-row click-through) can then filter by any combination
//! at once rather than by exactly one axis.

use async_trait::async_trait;

use crate::domain::expense::{Expense, ExpenseFilter, ExpenseToSave};

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait ExpenseRepository: Send + Sync {
    async fn create(&self, fields: ExpenseToSave) -> Result<Expense, InfrastructureError>;
    async fn update(&self, id: i64, fields: ExpenseToSave) -> Result<Expense, InfrastructureError>;
    async fn delete(&self, id: i64) -> Result<(), InfrastructureError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Expense>, InfrastructureError>;
    async fn list(&self, filter: ExpenseFilter) -> Result<Vec<Expense>, InfrastructureError>;
    /// `AttachReceipt`/`ReplaceReceipt`/`RemoveReceipt` write only this
    /// column — never touches any other field, and never re-derives a
    /// snapshot (application-architecture.md's `UpdateExpense` rule doesn't
    /// even apply here, since this isn't `UpdateExpense`).
    async fn set_receipt_path(
        &self,
        id: i64,
        receipt_path: Option<String>,
    ) -> Result<(), InfrastructureError>;
}
