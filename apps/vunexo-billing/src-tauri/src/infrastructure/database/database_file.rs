//! `DatabaseFile` over the live SQLx pool.

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
    /// `-wal`/`-shm` sidecars to keep alongside it. Copying the main file
    /// while WAL mode is on would produce a database missing every committed
    /// change still living in the WAL.
    async fn snapshot_to(&self, destination: &Path) -> Result<(), InfrastructureError> {
        // VACUUM INTO refuses to overwrite, so clear any stale temp first.
        if destination.exists() {
            std::fs::remove_file(destination).map_err(|err| {
                InfrastructureError::Io(format!("could not clear {}: {err}", destination.display()))
            })?;
        }
        // The path is interpolated because SQLite does not accept a bound
        // parameter here; the single quotes are escaped so a data directory
        // containing one can't break out of the literal.
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
