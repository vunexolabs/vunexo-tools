//! Reading and writing the `.vbx` container (database-schema.md §9).
//!
//! Split into "tell me what's in it" and "unpack it" so the caller can
//! *check* an archive — and refuse it, or ask the user to confirm — before
//! anything on disk is replaced.

use std::path::{Path, PathBuf};

use crate::domain::backup::BackupMetadata;

use super::infrastructure_error::InfrastructureError;

pub struct ArchiveContents<'a> {
    pub metadata: &'a BackupMetadata,
    /// The database snapshot to store as `database.sqlite`.
    pub database: &'a Path,
    /// Files to store under `assets/`, keyed by the name they take inside the
    /// archive.
    pub assets: &'a [(String, PathBuf)],
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

    /// Unpacks the database to `database_destination` and every `assets/`
    /// member into `assets_directory`, returning the asset paths written.
    fn extract(
        &self,
        source: &Path,
        database_destination: &Path,
        assets_directory: &Path,
    ) -> Result<Vec<PathBuf>, InfrastructureError>;
}
