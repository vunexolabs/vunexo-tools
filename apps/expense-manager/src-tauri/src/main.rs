#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;
mod domain;
mod infrastructure;

use std::sync::Arc;

use tauri::Manager;

use application::backup::BackupUseCases;
use application::business::BusinessUseCases;
use application::categories::CategoryUseCases;
use application::dashboard::DashboardUseCases;
use application::expenses::ExpenseUseCases;
use application::export::ExportUseCases;
use application::ports::backup_archive::BackupArchive;
use application::ports::business_repository::BusinessRepository;
use application::ports::category_repository::CategoryRepository;
use application::ports::dashboard_repository::DashboardRepository;
use application::ports::database_file::DatabaseFile;
use application::ports::expense_repository::ExpenseRepository;
use application::ports::file_writer::FileWriter;
use application::ports::receipt_store::ReceiptStore;
use application::ports::report_repository::ReportRepository;
use application::ports::vendor_repository::VendorRepository;
use application::reports::ReportUseCases;
use application::vendors::VendorUseCases;
use infrastructure::database::database_file::SqliteDatabaseFile;
use infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
use infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
use infrastructure::database::sqlite_dashboard_repository::SqliteDashboardRepository;
use infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
use infrastructure::database::sqlite_report_repository::SqliteReportRepository;
use infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
use infrastructure::filesystem::backup::VexArchive;
use infrastructure::filesystem::file_writer::StdFileWriter;
use infrastructure::filesystem::receipts::FsReceiptStore;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("expense-manager.db");

            let pool = tauri::async_runtime::block_on(async {
                let pool = infrastructure::database::init_pool(&db_path).await?;
                infrastructure::database::run_migrations(&pool).await?;
                infrastructure::database::seed_defaults(&pool).await?;
                Ok::<_, anyhow::Error>(pool)
            })?;

            let business_repo: Arc<dyn BusinessRepository> =
                Arc::new(SqliteBusinessRepository::new(pool.clone()));
            let vendor_repo: Arc<dyn VendorRepository> =
                Arc::new(SqliteVendorRepository::new(pool.clone()));
            let category_repo: Arc<dyn CategoryRepository> =
                Arc::new(SqliteCategoryRepository::new(pool.clone()));
            let expense_repo: Arc<dyn ExpenseRepository> =
                Arc::new(SqliteExpenseRepository::new(pool.clone()));
            let dashboard_repo: Arc<dyn DashboardRepository> =
                Arc::new(SqliteDashboardRepository::new(pool.clone()));
            let report_repo: Arc<dyn ReportRepository> =
                Arc::new(SqliteReportRepository::new(pool.clone()));
            let database_file: Arc<dyn DatabaseFile> =
                Arc::new(SqliteDatabaseFile::new(pool, db_path));
            let backup_archive: Arc<dyn BackupArchive> = Arc::new(VexArchive::new());
            let file_writer: Arc<dyn FileWriter> = Arc::new(StdFileWriter::new());
            let receipt_store: Arc<dyn ReceiptStore> =
                Arc::new(FsReceiptStore::new(data_dir.clone()));

            app.manage(BusinessUseCases::new(business_repo));
            app.manage(VendorUseCases::new(vendor_repo.clone()));
            app.manage(CategoryUseCases::new(category_repo.clone()));
            app.manage(ExpenseUseCases::new(
                expense_repo,
                vendor_repo,
                category_repo,
                receipt_store.clone(),
            ));
            app.manage(DashboardUseCases::new(dashboard_repo));
            app.manage(ReportUseCases::new(report_repo));
            app.manage(ExportUseCases::new(file_writer));
            app.manage(BackupUseCases::new(
                database_file,
                backup_archive,
                receipt_store,
                data_dir,
                env!("CARGO_PKG_VERSION").to_string(),
                std::env::consts::OS.to_string(),
            ));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_business,
            commands::update_business,
            commands::get_business,
            commands::create_vendor,
            commands::update_vendor,
            commands::delete_vendor,
            commands::list_vendors,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::list_categories,
            commands::create_expense,
            commands::update_expense,
            commands::delete_expense,
            commands::list_expenses,
            commands::attach_receipt,
            commands::replace_receipt,
            commands::remove_receipt,
            commands::get_dashboard_metrics,
            commands::generate_category_summary,
            commands::generate_period_summary,
            commands::generate_deductible_summary,
            commands::generate_tax_itc_summary,
            commands::generate_top_vendors,
            commands::write_export_file,
            commands::suggested_backup_file_name,
            commands::backup_data,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running expense-manager");
}
