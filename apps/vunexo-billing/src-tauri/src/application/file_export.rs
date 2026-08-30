//! Writes already-rendered text to a path the user picked. Backs the
//! frontend-built CSV/JSON for Statements and Reports (ui-ux-v2.md §5/§6) —
//! those are parameterized read models (a date range, a group-by), not a
//! fixed shape `domain::export::ExportEntity` enumerates, so there's no
//! existing command to extend. The content itself is built by the caller,
//! which already holds the typed `StatementResult`/`SalesSummaryResult`/
//! `TaxSummaryResult` the backend returned — this use case only owns the
//! "write bytes to a path" side, same as every other exporter in the app.

use std::path::Path;
use std::sync::Arc;

use super::error::ApplicationError;
use super::ports::file_writer::FileWriter;

pub struct FileExportUseCases {
    file_writer: Arc<dyn FileWriter>,
}

impl FileExportUseCases {
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
