//! The live SQLite file, as backup and restore need to see it.
//!
//! Every other port speaks in domain types and never admits that a file is
//! involved. This one has to: a backup copies the database *as a file*, and a
//! restore replaces it. Keeping that behind a port is still worth it — the
//! use cases stay free of SQLx and of `std::fs`, and a future storage change
//! has one implementation to rewrite.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait DatabaseFile: Send + Sync {
    /// A consistent point-in-time copy, taken while the app is still running
    /// and possibly mid-transaction. Must **not** be a plain file copy: the
    /// live database has WAL and shared-memory sidecars, and copying just the
    /// main file can capture a torn state.
    async fn snapshot_to(&self, destination: &Path) -> Result<(), InfrastructureError>;

    /// Where the live database lives — what a restore overwrites.
    fn path(&self) -> PathBuf;

    /// Closes every pooled connection and flushes the WAL, so the file can be
    /// replaced. Nothing may touch the database afterwards; the app restarts.
    async fn close(&self);
}
