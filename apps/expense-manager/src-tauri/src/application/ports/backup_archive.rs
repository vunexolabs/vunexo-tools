//! Reading and writing the `.vex` container (database-schema.md §7/§9).
//! Split into "tell me what's in it" and "unpack it" so the caller can
//! *check* an archive before anything on disk is replaced — same shape as
//! `vunexo-billing`'s `BackupArchive`, generalized from one named asset (a
//! logo) to a whole directory of receipt files.

use std::path::{Path, PathBuf};

use crate::domain::backup::BackupMetadata;

use super::infrastructure_error::InfrastructureError;

pub struct ArchiveContents<'a> {
    pub metadata: &'a BackupMetadata,
    /// The database snapshot to store as `database.sqlite`.
    pub database: &'a Path,
    /// Every receipt file to store under `receipts/`, keyed by the name it
    /// takes inside the archive (its file name under the data directory's
    /// own `receipts/` folder, so restoring needs no renaming).
    pub receipts: &'a [(String, PathBuf)],
}

pub trait BackupArchive: Send + Sync {
    fn write(
        &self,
        destination: &Path,
        contents: ArchiveContents<'_>,
    ) -> Result<(), InfrastructureError>;

    /// Reads just the metadata member. Cheap, and it is what makes
    /// "is this restorable?" answerable without unpacking anything.
    fn read_metadata(&self, source: &Path) -> Result<BackupMetadata, InfrastructureError>;

    /// Unpacks the database to `database_destination` and every `receipts/`
    /// member into `receipts_directory`, returning the receipt paths
    /// written. An archive missing a referenced receipt file simply writes
    /// fewer files than `expenses.receipt_path` names — the restore step
    /// treats that as orphan-tolerant, not a failure
    /// (application-architecture.md's `RestoreBackup` note).
    fn extract(
        &self,
        source: &Path,
        database_destination: &Path,
        receipts_directory: &Path,
    ) -> Result<Vec<PathBuf>, InfrastructureError>;
}
