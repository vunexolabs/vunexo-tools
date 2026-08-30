//! Invoice PDF use cases (user-flows.md §7 — "renders the single V1 template
//! and opens the OS-native save/print dialog").
//!
//! The orchestration is deliberately thin: gather everything the document
//! needs, hand it to `domain::invoice_pdf` to turn into finished text, hand
//! *that* to the renderer port. No formatting, no layout, no `printpdf`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::business::resolve_logo_path;
use crate::domain::invoice::{Invoice, InvoiceStatus};
use crate::domain::invoice_pdf::{build_invoice_pdf_document, InvoicePdfInput, LogoProbe};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::customer_repository::CustomerRepository;
use super::ports::file_writer::FileWriter;
use super::ports::invoice_pdf_renderer::InvoicePdfRenderer;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::payment_repository::PaymentRepository;
use super::ports::settings_repository::SettingsRepository;

/// A rendered invoice, plus the name it should be offered under in the save
/// dialog. Both come from one call because the caller needs them together and
/// the file name depends on invoice data the frontend would otherwise have to
/// re-fetch.
pub struct RenderedInvoicePdf {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

pub struct PdfUseCases {
    invoice_repo: Arc<dyn InvoiceRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
    business_repo: Arc<dyn BusinessRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
    payment_repo: Arc<dyn PaymentRepository>,
    renderer: Arc<dyn InvoicePdfRenderer>,
    file_writer: Arc<dyn FileWriter>,
    /// Needed only to resolve a *managed* logo path (`business::MANAGED_LOGO_DIRECTORY`,
    /// stored relative so it survives a backup/restore onto a different
    /// machine — see `application::business::import_logo_if_chosen`). A
    /// legacy absolute path is untouched by `resolve_logo_path` regardless.
    data_directory: PathBuf,
}

impl PdfUseCases {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        invoice_repo: Arc<dyn InvoiceRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
        business_repo: Arc<dyn BusinessRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
        payment_repo: Arc<dyn PaymentRepository>,
        renderer: Arc<dyn InvoicePdfRenderer>,
        file_writer: Arc<dyn FileWriter>,
        data_directory: PathBuf,
    ) -> Self {
        Self {
            invoice_repo,
            customer_repo,
            business_repo,
            settings_repo,
            payment_repo,
            renderer,
            file_writer,
            data_directory,
        }
    }

    /// Renders any invoice at any status — the Preview action is reachable
    /// from a draft (user-flows.md §5 step 5), so this must not require an
    /// issued one.
    pub async fn render_invoice_pdf(
        &self,
        id: i64,
    ) -> Result<RenderedInvoicePdf, ApplicationError> {
        let invoice = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        let settings = self.settings_repo.get().await?;

        // Only a document that hasn't been issued needs the live records: an
        // issued one carries its own snapshot and must print that, even if
        // the customer or the business profile has changed since.
        let needs_live_records = invoice.invoice.status == InvoiceStatus::Draft;
        let live_business = if needs_live_records {
            self.business_repo.get().await?
        } else {
            None
        };
        let live_customer = match (needs_live_records, invoice.invoice.customer_id) {
            (true, Some(customer_id)) => self.customer_repo.find_by_id(customer_id).await?,
            _ => None,
        };

        let amount_paid_minor: i64 = self
            .payment_repo
            .list_for_invoice(id)
            .await?
            .iter()
            .map(|payment| payment.amount_minor)
            .sum();

        let mut document = build_invoice_pdf_document(InvoicePdfInput {
            invoice: &invoice.invoice,
            line_items: &invoice.line_items,
            settings: &settings,
            live_business: live_business.as_ref(),
            live_customer: live_customer.as_ref(),
            amount_paid_minor,
        });
        // `build_invoice_pdf_document` is pure domain code and knows nothing
        // of the filesystem, so a managed (relative) logo path comes back
        // exactly as stored. Resolve it here, at the one seam that does know
        // where the data directory is, before it reaches the renderer.
        document.logo_path = document.logo_path.map(|stored| {
            resolve_logo_path(&stored, &self.data_directory)
                .to_string_lossy()
                .into_owned()
        });

        Ok(RenderedInvoicePdf {
            file_name: suggested_file_name(&invoice.invoice),
            bytes: self.renderer.render(&document)?,
        })
    }

    /// Answers "will this logo actually print?" for Settings. The renderer
    /// skips an unloadable logo rather than failing the invoice, so without
    /// this the only way to find out is to notice its absence on a finished
    /// PDF.
    ///
    /// `path` arrives exactly as `business.logo_path` stores it — relative
    /// for anything chosen since this managed-path support landed, absolute
    /// for a legacy one — so it goes through the same resolution the render
    /// path uses before the renderer ever sees it.
    pub fn probe_logo(&self, path: &Path) -> LogoProbe {
        let resolved = resolve_logo_path(&path.to_string_lossy(), &self.data_directory);
        self.renderer.probe_logo(&resolved)
    }

    /// Renders and writes to a path the user already chose in the OS save
    /// dialog. Re-rendering rather than accepting bytes from the frontend
    /// keeps the webview out of the position of being able to write arbitrary
    /// content to disk under an invoice's name.
    pub async fn save_invoice_pdf(&self, id: i64, path: &Path) -> Result<(), ApplicationError> {
        let rendered = self.render_invoice_pdf(id).await?;
        self.file_writer.write(path, &rendered.bytes)?;
        Ok(())
    }
}

/// `Invoice-INV-2026-0007.pdf`, or `Invoice-draft-12.pdf` before the number
/// exists. Anything that could upset a filesystem — separators, the
/// separators Windows also disallows, control characters — is replaced with
/// `-`, since the invoice number format is user-configurable
/// (database-schema.md §7) and nothing stops someone putting a `/` in it.
fn suggested_file_name(invoice: &Invoice) -> String {
    let stem = match invoice.invoice_number.as_deref() {
        Some(number) if !number.trim().is_empty() => sanitize(number),
        _ => format!("draft-{}", invoice.id),
    };
    format!("Invoice-{stem}.pdf")
}

fn sanitize(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .collect()
}
