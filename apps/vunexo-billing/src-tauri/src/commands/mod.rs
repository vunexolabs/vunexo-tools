//! Thin Tauri command handlers: deserialize input, call into
//! `crate::application`, serialize output. No business logic here.
//! See docs/vunexo-billing/architecture.md and application-architecture.md §5.

use base64::prelude::{Engine as _, BASE64_STANDARD};
use tauri::State;

use crate::application::backup::BackupUseCases;
use crate::application::business::BusinessUseCases;
use crate::application::customers::CustomerUseCases;
use crate::application::dashboard::DashboardUseCases;
use crate::application::export::ExportUseCases;
use crate::application::invoices::InvoiceUseCases;
use crate::application::payments::PaymentUseCases;
use crate::application::pdf::PdfUseCases;
use crate::application::products::ProductUseCases;
use crate::application::quotes::QuoteUseCases;
use crate::application::reminders::ReminderUseCases;
use crate::application::reports::ReportUseCases;
use crate::application::settings::SettingsUseCases;
use crate::application::statements::StatementUseCases;
use crate::application::tax_rates::TaxRateUseCases;
use crate::application::ApplicationError;
use crate::domain::backup::{backup_file_name, BackupMetadata};
use crate::domain::business::Business;
use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};
use crate::domain::dashboard::DashboardMetrics;
use crate::domain::export::ExportEntity;
use crate::domain::invoice::{
    DraftInvoiceInput, InvoiceFilter, InvoiceSummary, InvoiceWithLineItems,
};
use crate::domain::invoice_pdf::LogoProbe;
use crate::domain::payment::{NewPayment, Payment, PaymentFields};
use crate::domain::product::{Product, ProductFields, ProductFilter, ProductListItem};
use crate::domain::quote::{DraftQuoteInput, QuoteFilter, QuoteSummary, QuoteWithLineItems};
use crate::domain::report::{SalesGrouping, SalesSummaryResult, TaxSummaryResult};
use crate::domain::settings::{Settings, SettingsFields};
use crate::domain::statement::StatementResult;
use crate::domain::tax_rate::{TaxRate, TaxRateFields};

/// Round 1 technical spike: proves the React -> Tauri -> Rust round trip.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Vunexo Billing is wired up: React -> Tauri -> Rust.")
}

#[tauri::command]
pub async fn create_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.create_business(business).await
}

#[tauri::command]
pub async fn get_business(
    business_use_cases: State<'_, BusinessUseCases>,
) -> Result<Option<Business>, ApplicationError> {
    business_use_cases.get_business().await
}

#[tauri::command]
pub async fn update_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.update_business(business).await
}

#[tauri::command]
pub async fn create_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    fields: CustomerFields,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.create_customer(fields).await
}

#[tauri::command]
pub async fn update_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
    fields: CustomerFields,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.update_customer(id, fields).await
}

#[tauri::command]
pub async fn archive_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.archive_customer(id).await
}

#[tauri::command]
pub async fn restore_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.restore_customer(id).await
}

#[tauri::command]
pub async fn delete_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.delete_customer(id).await
}

#[tauri::command]
pub async fn get_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.get_customer(id).await
}

#[tauri::command]
pub async fn list_customers(
    customer_use_cases: State<'_, CustomerUseCases>,
    filter: CustomerFilter,
) -> Result<Vec<CustomerListItem>, ApplicationError> {
    customer_use_cases.list_customers(filter).await
}

#[tauri::command]
pub async fn create_product(
    product_use_cases: State<'_, ProductUseCases>,
    fields: ProductFields,
) -> Result<Product, ApplicationError> {
    product_use_cases.create_product(fields).await
}

#[tauri::command]
pub async fn update_product(
    product_use_cases: State<'_, ProductUseCases>,
    id: i64,
    fields: ProductFields,
) -> Result<Product, ApplicationError> {
    product_use_cases.update_product(id, fields).await
}

#[tauri::command]
pub async fn archive_product(
    product_use_cases: State<'_, ProductUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    product_use_cases.archive_product(id).await
}

#[tauri::command]
pub async fn restore_product(
    product_use_cases: State<'_, ProductUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    product_use_cases.restore_product(id).await
}

#[tauri::command]
pub async fn delete_product(
    product_use_cases: State<'_, ProductUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    product_use_cases.delete_product(id).await
}

#[tauri::command]
pub async fn get_product(
    product_use_cases: State<'_, ProductUseCases>,
    id: i64,
) -> Result<Product, ApplicationError> {
    product_use_cases.get_product(id).await
}

#[tauri::command]
pub async fn list_products(
    product_use_cases: State<'_, ProductUseCases>,
    filter: ProductFilter,
) -> Result<Vec<ProductListItem>, ApplicationError> {
    product_use_cases.list_products(filter).await
}

#[tauri::command]
pub async fn get_settings(
    settings_use_cases: State<'_, SettingsUseCases>,
) -> Result<Settings, ApplicationError> {
    settings_use_cases.get_settings().await
}

#[tauri::command]
pub async fn update_settings(
    settings_use_cases: State<'_, SettingsUseCases>,
    fields: SettingsFields,
) -> Result<Settings, ApplicationError> {
    settings_use_cases.update_settings(fields).await
}

#[tauri::command]
pub async fn preview_next_invoice_number(
    invoice_use_cases: State<'_, InvoiceUseCases>,
) -> Result<String, ApplicationError> {
    invoice_use_cases.preview_next_invoice_number().await
}

#[tauri::command]
pub async fn create_draft_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    input: DraftInvoiceInput,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.create_draft_invoice(input).await
}

#[tauri::command]
pub async fn update_draft_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
    input: DraftInvoiceInput,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.update_draft_invoice(id, input).await
}

#[tauri::command]
pub async fn issue_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
    custom_number: Option<String>,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.issue_invoice(id, custom_number).await
}

#[tauri::command]
pub async fn edit_issued_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
    input: DraftInvoiceInput,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.edit_issued_invoice(id, input).await
}

#[tauri::command]
pub async fn cancel_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
    reason: Option<String>,
) -> Result<(), ApplicationError> {
    invoice_use_cases.cancel_invoice(id, reason).await
}

#[tauri::command]
pub async fn delete_draft_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    invoice_use_cases.delete_draft_invoice(id).await
}

#[tauri::command]
pub async fn duplicate_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.duplicate_invoice(id).await
}

#[tauri::command]
pub async fn get_invoice(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    id: i64,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    invoice_use_cases.get_invoice(id).await
}

#[tauri::command]
pub async fn list_invoices(
    invoice_use_cases: State<'_, InvoiceUseCases>,
    filter: InvoiceFilter,
) -> Result<Vec<InvoiceSummary>, ApplicationError> {
    invoice_use_cases.list_invoices(filter).await
}

#[tauri::command]
pub async fn preview_next_quote_number(
    quote_use_cases: State<'_, QuoteUseCases>,
) -> Result<String, ApplicationError> {
    quote_use_cases.preview_next_quote_number().await
}

#[tauri::command]
pub async fn create_draft_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    input: DraftQuoteInput,
) -> Result<QuoteWithLineItems, ApplicationError> {
    quote_use_cases.create_draft_quote(input).await
}

#[tauri::command]
pub async fn update_draft_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
    input: DraftQuoteInput,
) -> Result<QuoteWithLineItems, ApplicationError> {
    quote_use_cases.update_draft_quote(id, input).await
}

#[tauri::command]
pub async fn issue_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<QuoteWithLineItems, ApplicationError> {
    quote_use_cases.issue_quote(id).await
}

#[tauri::command]
pub async fn accept_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    quote_use_cases.accept_quote(id).await
}

#[tauri::command]
pub async fn decline_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    quote_use_cases.decline_quote(id).await
}

#[tauri::command]
pub async fn cancel_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
    reason: Option<String>,
) -> Result<(), ApplicationError> {
    quote_use_cases.cancel_quote(id, reason).await
}

/// application-architecture-v2.md §4c — the one command that produces a new
/// invoice as a side effect. Returns the resulting Draft invoice, matching
/// user-flows-v2.md §3's "user lands on the new Draft Invoice."
#[tauri::command]
pub async fn convert_quote_to_invoice(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<InvoiceWithLineItems, ApplicationError> {
    quote_use_cases.convert_quote_to_invoice(id).await
}

#[tauri::command]
pub async fn duplicate_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<QuoteWithLineItems, ApplicationError> {
    quote_use_cases.duplicate_quote(id).await
}

#[tauri::command]
pub async fn delete_draft_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    quote_use_cases.delete_draft_quote(id).await
}

#[tauri::command]
pub async fn get_quote(
    quote_use_cases: State<'_, QuoteUseCases>,
    id: i64,
) -> Result<QuoteWithLineItems, ApplicationError> {
    quote_use_cases.get_quote_with_line_items(id).await
}

#[tauri::command]
pub async fn list_quotes(
    quote_use_cases: State<'_, QuoteUseCases>,
    filter: QuoteFilter,
) -> Result<Vec<QuoteSummary>, ApplicationError> {
    quote_use_cases.list_quotes(filter).await
}

#[tauri::command]
pub async fn record_payment(
    payment_use_cases: State<'_, PaymentUseCases>,
    payment: NewPayment,
) -> Result<Payment, ApplicationError> {
    payment_use_cases.record_payment(payment).await
}

#[tauri::command]
pub async fn update_payment(
    payment_use_cases: State<'_, PaymentUseCases>,
    id: i64,
    fields: PaymentFields,
) -> Result<Payment, ApplicationError> {
    payment_use_cases.update_payment(id, fields).await
}

#[tauri::command]
pub async fn delete_payment(
    payment_use_cases: State<'_, PaymentUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    payment_use_cases.delete_payment(id).await
}

#[tauri::command]
pub async fn list_payments_for_invoice(
    payment_use_cases: State<'_, PaymentUseCases>,
    invoice_id: i64,
) -> Result<Vec<Payment>, ApplicationError> {
    payment_use_cases
        .list_payments_for_invoice(invoice_id)
        .await
}

#[tauri::command]
pub async fn create_tax_rate(
    tax_rate_use_cases: State<'_, TaxRateUseCases>,
    fields: TaxRateFields,
) -> Result<TaxRate, ApplicationError> {
    tax_rate_use_cases.create_tax_rate(fields).await
}

#[tauri::command]
pub async fn update_tax_rate(
    tax_rate_use_cases: State<'_, TaxRateUseCases>,
    id: i64,
    fields: TaxRateFields,
) -> Result<TaxRate, ApplicationError> {
    tax_rate_use_cases.update_tax_rate(id, fields).await
}

#[tauri::command]
pub async fn list_tax_rates(
    tax_rate_use_cases: State<'_, TaxRateUseCases>,
) -> Result<Vec<TaxRate>, ApplicationError> {
    tax_rate_use_cases.list_tax_rates().await
}

#[tauri::command]
pub async fn get_dashboard_metrics(
    dashboard_use_cases: State<'_, DashboardUseCases>,
) -> Result<DashboardMetrics, ApplicationError> {
    dashboard_use_cases.get_dashboard_metrics().await
}

/// A rendered invoice on its way to the webview. The bytes are base64 rather
/// than a `Vec<u8>`, because Tauri serializes a byte vector as a JSON array
/// of numbers — several times the size, for a payload that is already tens of
/// kilobytes. The frontend turns this straight back into a Blob for the
/// preview pane.
#[derive(serde::Serialize)]
pub struct RenderedInvoicePdfPayload {
    pub file_name: String,
    pub bytes_base64: String,
}

#[tauri::command]
pub async fn render_invoice_pdf(
    pdf_use_cases: State<'_, PdfUseCases>,
    id: i64,
) -> Result<RenderedInvoicePdfPayload, ApplicationError> {
    let rendered = pdf_use_cases.render_invoice_pdf(id).await?;
    Ok(RenderedInvoicePdfPayload {
        file_name: rendered.file_name,
        bytes_base64: BASE64_STANDARD.encode(&rendered.bytes),
    })
}

/// Reports whether the business logo at `path` can actually be printed, so
/// Settings can say so at the moment it is chosen rather than leaving the
/// user to infer it from a logo-less invoice.
#[tauri::command]
pub fn probe_business_logo(pdf_use_cases: State<'_, PdfUseCases>, path: String) -> LogoProbe {
    pdf_use_cases.probe_logo(std::path::Path::new(&path))
}

/// `path` is whatever the OS save dialog returned to the frontend; the PDF
/// itself is re-rendered here rather than sent down and back up again.
#[tauri::command]
pub async fn save_invoice_pdf(
    pdf_use_cases: State<'_, PdfUseCases>,
    id: i64,
    path: String,
) -> Result<(), ApplicationError> {
    pdf_use_cases
        .save_invoice_pdf(id, std::path::Path::new(&path))
        .await
}

/// The default name the backup save dialog offers (user-flows.md §9).
#[tauri::command]
pub fn suggested_backup_file_name() -> String {
    backup_file_name(chrono::Utc::now().date_naive())
}

#[tauri::command]
pub async fn backup_database(
    backup_use_cases: State<'_, BackupUseCases>,
    path: String,
) -> Result<(), ApplicationError> {
    backup_use_cases
        .backup_to(std::path::Path::new(&path))
        .await
}

/// Reads a `.vbx`'s metadata without unpacking it, so the confirmation dialog
/// can say what is about to replace the user's data — and so an archive this
/// build can't read is refused before anything is touched.
#[tauri::command]
pub async fn inspect_backup(
    backup_use_cases: State<'_, BackupUseCases>,
    path: String,
) -> Result<BackupMetadata, ApplicationError> {
    backup_use_cases.inspect_backup(std::path::Path::new(&path))
}

/// Replaces all local data, then **restarts the app** and therefore never
/// returns on success. Every repository holds a pool that `restore_from` has
/// closed, so continuing to run would mean serving a database nothing can
/// read; a restart is the honest end of this operation, not a convenience.
#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    backup_use_cases: State<'_, BackupUseCases>,
    path: String,
) -> Result<(), ApplicationError> {
    backup_use_cases
        .restore_from(std::path::Path::new(&path))
        .await?;
    app.restart();
}

#[tauri::command]
pub fn suggested_export_file_name(entity: ExportEntity) -> String {
    entity.suggested_file_name().to_string()
}

#[tauri::command]
pub async fn export_data(
    export_use_cases: State<'_, ExportUseCases>,
    entity: ExportEntity,
    path: String,
) -> Result<(), ApplicationError> {
    export_use_cases
        .export_to(entity, std::path::Path::new(&path))
        .await
}

#[tauri::command]
pub async fn generate_customer_statement(
    statement_use_cases: State<'_, StatementUseCases>,
    customer_id: i64,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<StatementResult, ApplicationError> {
    statement_use_cases
        .generate_customer_statement(customer_id, range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_sales_report(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
    group_by: SalesGrouping,
) -> Result<SalesSummaryResult, ApplicationError> {
    report_use_cases
        .generate_sales_report(range_start, range_end, group_by)
        .await
}

#[tauri::command]
pub async fn generate_tax_summary_report(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<TaxSummaryResult, ApplicationError> {
    report_use_cases
        .generate_tax_summary_report(range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_reminder_message(
    reminder_use_cases: State<'_, ReminderUseCases>,
    invoice_id: i64,
) -> Result<String, ApplicationError> {
    reminder_use_cases
        .generate_reminder_message(invoice_id)
        .await
}
