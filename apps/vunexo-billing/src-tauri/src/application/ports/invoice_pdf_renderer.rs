//! The port `infrastructure::pdf` implements. Deliberately narrow: it takes
//! an already-fully-formatted document (`domain::invoice_pdf`) and returns
//! PDF bytes. Nothing about invoices, currencies, or tax reaches the
//! implementation — swapping the PDF library later touches only the
//! implementation behind this trait (architecture.md rule 3).
//!
//! Not `async`: rendering is pure CPU work with no I/O to await except
//! reading the logo file, which the implementation does synchronously.

use std::path::Path;

use crate::domain::invoice_pdf::{InvoicePdfDocument, LogoProbe};

use super::infrastructure_error::InfrastructureError;

pub trait InvoicePdfRenderer: Send + Sync {
    fn render(&self, document: &InvoicePdfDocument) -> Result<Vec<u8>, InfrastructureError>;

    /// Whether `path` is an image this renderer could actually place on a
    /// page — the same read-and-decode the letterhead does, asked ahead of
    /// time so Settings can tell the user before an invoice silently prints
    /// without their logo.
    fn probe_logo(&self, path: &Path) -> LogoProbe;
}
