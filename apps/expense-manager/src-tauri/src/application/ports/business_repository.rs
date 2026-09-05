//! application-architecture.md's module layout. `get` returns
//! `Option<Business>` — `None` is the first-run signal (user-flows.md §1).
//!
//! No `TransactionManager` here (application-architecture.md's explicit
//! decision — a single business row write is one statement, no multi-table
//! atomic write to protect), so repository methods run directly against the
//! pool.

use async_trait::async_trait;

use crate::domain::business::Business;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait BusinessRepository: Send + Sync {
    async fn create(&self, business: Business) -> Result<Business, InfrastructureError>;
    async fn get(&self) -> Result<Option<Business>, InfrastructureError>;
    async fn update(&self, business: Business) -> Result<Business, InfrastructureError>;
}
