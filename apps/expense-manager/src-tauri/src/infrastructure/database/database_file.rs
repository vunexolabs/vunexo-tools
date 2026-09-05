//! `DatabaseFile` over the live SQLx pool. Mirrors `vunexo-billing`'s
//! `SqliteDatabaseFile` exactly.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::application::ports::database_file::DatabaseFile;
use crate::application::ports::infrastructure_error::InfrastructureError;

pub struct SqliteDatabaseFile {
    pool: SqlitePool,
    path: PathBuf,
}

impl SqliteDatabaseFile {
    pub fn new(pool: SqlitePool, path: PathBuf) -> Self {
        Self { pool, path }
    }
}

#[async_trait]
impl DatabaseFile for SqliteDatabaseFile {
    /// `VACUUM INTO` rather than a file copy: it takes its own read
    /// transaction, so the result is a consistent snapshot even if something
    /// writes mid-backup, and it writes a single self-contained file with no
    /// `-wal`/`-shm` sidecars to keep alongside it.
    async fn snapshot_to(&self, destination: &Path) -> Result<(), InfrastructureError> {
        if destination.exists() {
            std::fs::remove_file(destination).map_err(|err| {
                InfrastructureError::Io(format!("could not clear {}: {err}", destination.display()))
            })?;
        }
        let escaped = destination.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{escaped}'"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn close(&self) {
        self.pool.close().await;
    }
}
