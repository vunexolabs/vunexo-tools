//! application-architecture.md §3b. `get` never returns `None` — the row is
//! seeded with schema defaults at DB init (infrastructure/database/mod.rs),
//! not created via a use case the way `Business` is.

use async_trait::async_trait;

use crate::domain::settings::{Settings, SettingsFields};

use super::infrastructure_error::InfrastructureError;
use super::transaction::Transaction;

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self) -> Result<Settings, InfrastructureError>;
    async fn update(
        &self,
        tx: &mut dyn Transaction,
        fields: SettingsFields,
    ) -> Result<Settings, InfrastructureError>;
}
