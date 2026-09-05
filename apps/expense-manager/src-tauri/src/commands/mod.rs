//! Thin Tauri command handlers: deserialize input, call into
//! `crate::application`, serialize output. No business logic here.
//! Command names and set match application-architecture.md's "Tauri command
//! surface" section exactly (27 commands).

use tauri::State;

use crate::application::backup::BackupUseCases;
use crate::application::business::BusinessUseCases;
use crate::application::categories::CategoryUseCases;
use crate::application::dashboard::DashboardUseCases;
use crate::application::expenses::ExpenseUseCases;
use crate::application::export::ExportUseCases;
use crate::application::reports::ReportUseCases;
use crate::application::vendors::VendorUseCases;
use crate::application::ApplicationError;
use crate::domain::backup::backup_file_name;
use crate::domain::business::Business;
use crate::domain::category::{Category, CategoryFields, CategoryId, CategoryListItem};
use crate::domain::dashboard::DashboardMetrics;
use crate::domain::expense::{Expense, ExpenseFilter, ExpenseInput};
use crate::domain::report::{
    CategorySummaryResult, DeductibleSummaryResult, PeriodSummaryResult, TaxItcSummaryResult,
    TopVendorsResult,
};
use crate::domain::vendor::{Vendor, VendorFields, VendorId, VendorListItem};

#[tauri::command]
pub async fn create_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.create_business(business).await
}

#[tauri::command]
pub async fn update_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.update_business(business).await
}

#[tauri::command]
pub async fn get_business(
    business_use_cases: State<'_, BusinessUseCases>,
) -> Result<Option<Business>, ApplicationError> {
    business_use_cases.get_business().await
}

#[tauri::command]
pub async fn create_vendor(
    vendor_use_cases: State<'_, VendorUseCases>,
    fields: VendorFields,
) -> Result<Vendor, ApplicationError> {
    vendor_use_cases.create_vendor(fields).await
}

#[tauri::command]
pub async fn update_vendor(
    vendor_use_cases: State<'_, VendorUseCases>,
    id: VendorId,
    fields: VendorFields,
) -> Result<Vendor, ApplicationError> {
    vendor_use_cases.update_vendor(id, fields).await
}

#[tauri::command]
pub async fn delete_vendor(
    vendor_use_cases: State<'_, VendorUseCases>,
    id: VendorId,
) -> Result<(), ApplicationError> {
    vendor_use_cases.delete_vendor(id).await
}

#[tauri::command]
pub async fn list_vendors(
    vendor_use_cases: State<'_, VendorUseCases>,
) -> Result<Vec<VendorListItem>, ApplicationError> {
    vendor_use_cases.list_vendors().await
}

#[tauri::command]
pub async fn create_category(
    category_use_cases: State<'_, CategoryUseCases>,
    fields: CategoryFields,
) -> Result<Category, ApplicationError> {
    category_use_cases.create_category(fields).await
}

#[tauri::command]
pub async fn update_category(
    category_use_cases: State<'_, CategoryUseCases>,
    id: CategoryId,
    fields: CategoryFields,
) -> Result<Category, ApplicationError> {
    category_use_cases.update_category(id, fields).await
}

#[tauri::command]
pub async fn delete_category(
    category_use_cases: State<'_, CategoryUseCases>,
    id: CategoryId,
) -> Result<(), ApplicationError> {
    category_use_cases.delete_category(id).await
}

#[tauri::command]
pub async fn list_categories(
    category_use_cases: State<'_, CategoryUseCases>,
) -> Result<Vec<CategoryListItem>, ApplicationError> {
    category_use_cases.list_categories().await
}

#[tauri::command]
pub async fn create_expense(
    expense_use_cases: State<'_, ExpenseUseCases>,
    input: ExpenseInput,
) -> Result<Expense, ApplicationError> {
    expense_use_cases.create_expense(input).await
}

#[tauri::command]
pub async fn update_expense(
    expense_use_cases: State<'_, ExpenseUseCases>,
    id: i64,
    input: ExpenseInput,
) -> Result<Expense, ApplicationError> {
    expense_use_cases.update_expense(id, input).await
}

#[tauri::command]
pub async fn delete_expense(
    expense_use_cases: State<'_, ExpenseUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    expense_use_cases.delete_expense(id).await
}

#[tauri::command]
pub async fn list_expenses(
    expense_use_cases: State<'_, ExpenseUseCases>,
    filter: ExpenseFilter,
) -> Result<Vec<Expense>, ApplicationError> {
    expense_use_cases.list_expenses(filter).await
}

#[tauri::command]
pub async fn attach_receipt(
    expense_use_cases: State<'_, ExpenseUseCases>,
    id: i64,
    path: String,
) -> Result<Expense, ApplicationError> {
    expense_use_cases
        .attach_receipt(id, std::path::Path::new(&path))
        .await
}

#[tauri::command]
pub async fn replace_receipt(
    expense_use_cases: State<'_, ExpenseUseCases>,
    id: i64,
    path: String,
) -> Result<Expense, ApplicationError> {
    expense_use_cases
        .replace_receipt(id, std::path::Path::new(&path))
        .await
}

#[tauri::command]
pub async fn remove_receipt(
    expense_use_cases: State<'_, ExpenseUseCases>,
    id: i64,
) -> Result<Expense, ApplicationError> {
    expense_use_cases.remove_receipt(id).await
}

#[tauri::command]
pub async fn get_dashboard_metrics(
    dashboard_use_cases: State<'_, DashboardUseCases>,
) -> Result<DashboardMetrics, ApplicationError> {
    dashboard_use_cases.get_dashboard_metrics().await
}

#[tauri::command]
pub async fn generate_category_summary(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<CategorySummaryResult, ApplicationError> {
    report_use_cases
        .generate_category_summary(range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_period_summary(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<PeriodSummaryResult, ApplicationError> {
    report_use_cases
        .generate_period_summary(range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_deductible_summary(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<DeductibleSummaryResult, ApplicationError> {
    report_use_cases
        .generate_deductible_summary(range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_tax_itc_summary(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<TaxItcSummaryResult, ApplicationError> {
    report_use_cases
        .generate_tax_itc_summary(range_start, range_end)
        .await
}

#[tauri::command]
pub async fn generate_top_vendors(
    report_use_cases: State<'_, ReportUseCases>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
) -> Result<TopVendorsResult, ApplicationError> {
    report_use_cases
        .generate_top_vendors(range_start, range_end)
        .await
}

/// Writes CSV/JSON built client-side (Reports screen) to a path already
/// chosen in the OS save dialog — same generic "frontend renders the export
/// text, backend just writes it" pattern as Billing's `write_export_file`.
#[tauri::command]
pub async fn write_export_file(
    export_use_cases: State<'_, ExportUseCases>,
    path: String,
    contents: String,
) -> Result<(), ApplicationError> {
    export_use_cases
        .write_text_file(std::path::Path::new(&path), &contents)
        .await
}

/// The default name the backup save dialog offers (user-flows.md §9). Not
/// named in application-architecture.md's command-surface list, which
/// enumerates exactly `backup_data`/`restore_backup` — added anyway, as a
/// thin, stateless convenience (same shape as Billing's own
/// `suggested_backup_file_name`) rather than duplicating the file-name
/// format in TypeScript.
#[tauri::command]
pub fn suggested_backup_file_name() -> String {
    backup_file_name(chrono::Utc::now().date_naive())
}

#[tauri::command]
pub async fn backup_data(
    backup_use_cases: State<'_, BackupUseCases>,
    path: String,
) -> Result<(), ApplicationError> {
    backup_use_cases
        .backup_to(std::path::Path::new(&path))
        .await
}

/// Replaces all local data, then **restarts the app** and therefore never
/// returns on success — mirrors Billing's `restore_backup` exactly.
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
