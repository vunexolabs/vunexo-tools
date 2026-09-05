//! Writing a produced artefact (a CSV/JSON export, a `.vex` backup) to a path
//! the user picked in the OS file dialog. Behind a port so use cases stay
//! free of `std::fs`. Mirrors `vunexo-billing`'s `FileWriter` port.

use std::path::Path;

use super::infrastructure_error::InfrastructureError;

pub trait FileWriter: Send + Sync {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), InfrastructureError>;
}
