//! Copying a chosen receipt image into the app's managed `receipts/`
//! directory, and removing one already living there. Behind a port so
//! `application::expenses` stays free of `std::fs` and of UUID generation —
//! application-architecture.md names `infrastructure/filesystem/receipts.rs`
//! as the module that implements this ("copy-in-on-attach, delete-on-remove,
//! same pattern as logo management").

use std::path::{Path, PathBuf};

use super::infrastructure_error::InfrastructureError;

pub trait ReceiptStore: Send + Sync {
    /// Copies `source` into `receipts/<uuid>.<ext>` under the data
    /// directory, returning the resulting **relative** path to store as
    /// `expenses.receipt_path` (database-schema.md §7).
    fn attach(&self, source: &Path) -> Result<String, InfrastructureError>;

    /// Deletes the file at `relative_path` (as stored in `receipt_path`) if
    /// it exists. Missing-file is not an error — a receipt that has gone
    /// missing on disk must never block a DB operation
    /// (application-architecture.md's `DeleteExpense` note).
    fn remove(&self, relative_path: &str) -> Result<(), InfrastructureError>;

    /// Every file currently living in the managed `receipts/` directory,
    /// keyed by its file name — what `BackupUseCases` walks to bundle every
    /// receipt into a `.vex` archive (database-schema.md §7's "the file must
    /// live somewhere the backup step already knows to walk").
    fn list_all(&self) -> Result<Vec<(String, PathBuf)>, InfrastructureError>;
}
