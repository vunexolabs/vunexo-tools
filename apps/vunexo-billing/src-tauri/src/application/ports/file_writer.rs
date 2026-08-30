//! Writing a produced artefact (an invoice PDF today; a `.vbx` backup and
//! the CSV/JSON exports next) to a path the user picked in the OS file
//! dialog. Behind a port so use cases stay free of `std::fs`
//! (architecture.md rule 3 — the rule is about the *category* of dependency,
//! and filesystem I/O is named in it explicitly).

use std::path::Path;

use super::infrastructure_error::InfrastructureError;

pub trait FileWriter: Send + Sync {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), InfrastructureError>;
}
