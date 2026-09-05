//! Writes already-rendered text to a path the user picked. Backs the
//! frontend-built CSV/JSON for Reports (user-flows.md §7) — those are
//! parameterized read models (a report kind, a date range), not a fixed
//! shape an `ExportEntity` enum could enumerate, so there's no per-entity
//! export command to extend. The content itself is built by the caller,
//! which already holds the typed report result the backend returned — this
//! use case only owns the "write bytes to a path" side, same as
//! `vunexo-billing`'s `FileExportUseCases`/`write_export_file` (a session-13
//! addition there, applied here from V1 rather than bolted on later).

use std::path::Path;
use std::sync::Arc;

use super::error::ApplicationError;
use super::ports::file_writer::FileWriter;

pub struct ExportUseCases {
    file_writer: Arc<dyn FileWriter>,
}

impl ExportUseCases {
    pub fn new(file_writer: Arc<dyn FileWriter>) -> Self {
        Self { file_writer }
    }

    pub async fn write_text_file(
        &self,
        path: &Path,
        contents: &str,
    ) -> Result<(), ApplicationError> {
        self.file_writer.write(path, contents.as_bytes())?;
        Ok(())
    }
}
