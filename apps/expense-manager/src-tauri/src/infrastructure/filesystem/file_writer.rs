//! `FileWriter` over `std::fs`. Mirrors `vunexo-billing`'s `StdFileWriter`.

use std::path::Path;

use crate::application::ports::file_writer::FileWriter;
use crate::application::ports::infrastructure_error::InfrastructureError;

pub struct StdFileWriter;

impl StdFileWriter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdFileWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWriter for StdFileWriter {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), InfrastructureError> {
        // The parent directory is whatever the OS save dialog handed back, so
        // it exists — but a removable volume can vanish between the dialog
        // closing and the write, and that must surface as an error rather
        // than a silent no-op.
        std::fs::write(path, bytes).map_err(|err| {
            InfrastructureError::Io(format!("could not write {}: {err}", path.display()))
        })
    }
}
