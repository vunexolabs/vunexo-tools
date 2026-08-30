//! Invoice PDF generation — the `InvoicePdfRenderer` port's implementation.
//! `printpdf` never escapes this module: the rest of the app knows only the
//! port and `domain::invoice_pdf`'s document model.

pub mod canvas;
pub mod fonts;
pub mod invoice_renderer;

pub use invoice_renderer::PrintpdfInvoiceRenderer;
