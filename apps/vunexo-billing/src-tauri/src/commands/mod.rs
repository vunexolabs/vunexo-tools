//! Thin Tauri command handlers: deserialize input, call into
//! `crate::application`, serialize output. No business logic here.
//! See docs/vunexo-billing/architecture.md and application-architecture.md §5.

use tauri::State;

use crate::application::business::BusinessUseCases;
use crate::application::customers::CustomerUseCases;
use crate::application::dashboard::DashboardUseCases;
use crate::application::invoices::InvoiceUseCases;
use crate::application::payments::PaymentUseCases;
use crate::application::products::ProductUseCases;
use crate::application::settings::SettingsUseCases;
use crate::application::tax_rates::TaxRateUseCases;
use crate::application::ApplicationError;
use crate::domain::business::Business;
use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};
use crate::domain::dashboard::DashboardMetrics;
use crate::domain::invoice::{
    DraftInvoiceInput, InvoiceFilter, InvoiceSummary, InvoiceWithLineItems,
};
use crate::domain::payment::{NewPayment, Payment, PaymentFields};
use crate::domain::product::{Product, ProductFields, ProductFilter, ProductListItem};
use crate::domain::settings::{Settings, SettingsFields};
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
