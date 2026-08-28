//! application-architecture.md §3b. `get` returns `Option<Business>` —
//! `None` is the first-run signal (`user-flows.md` §1), so this can't be a
//! guaranteed-row singleton the way `Settings` is.

use async_trait::async_trait;

use crate::domain::business::Business;

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait BusinessRepository: Send + Sync {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        business: Business,
    ) -> Result<Business, InfrastructureError>;
    async fn get(&self) -> Result<Option<Business>, InfrastructureError>;
    async fn update(
        &self,
        tx: &mut dyn Transaction,
        business: Business,
    ) -> Result<Business, InfrastructureError>;
}
