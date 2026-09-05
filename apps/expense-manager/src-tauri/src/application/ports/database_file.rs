//! The live SQLite file, as backup and restore need to see it. Mirrors
//! `vunexo-billing`'s `DatabaseFile` port exactly.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait DatabaseFile: Send + Sync {
    /// A consistent point-in-time copy (`VACUUM INTO`, not a plain file
    /// copy — see the infrastructure implementation for why).
    async fn snapshot_to(&self, destination: &Path) -> Result<(), InfrastructureError>;

    /// Where the live database lives — what a restore overwrites.
    fn path(&self) -> PathBuf;

    /// Closes every pooled connection so the file can be replaced. Nothing
    /// may touch the database afterwards; the app restarts.
    async fn close(&self);
}
