//! The transaction boundary port. application-architecture.md §3a.
//!
//! V1 has exactly one infrastructure implementation (SQLite), so this stays
//! an honest abstraction rather than a fully backend-agnostic one (§3a
//! explicitly allows this): `Transaction` exposes itself as `Any` so the
//! SQLite repository implementations can downcast back to the concrete
//! `sqlx::Transaction` they need to actually run a query, while the
//! `application` layer and its use cases only ever see the trait object.

use std::any::Any;

use async_trait::async_trait;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn Transaction>, InfrastructureError>;
}

#[async_trait]
pub trait Transaction: Send {
    async fn commit(self: Box<Self>) -> Result<(), InfrastructureError>;
    async fn rollback(self: Box<Self>) -> Result<(), InfrastructureError>;

    /// Escape hatch for infrastructure implementations only — see the module
    /// doc comment. Never called from `application` or `domain`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
