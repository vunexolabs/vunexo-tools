#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;
mod domain;
mod infrastructure;

use std::sync::Arc;

use tauri::Manager;

use application::backup::BackupUseCases;
use application::business::BusinessUseCases;
use application::customers::CustomerUseCases;
use application::dashboard::DashboardUseCases;
use application::export::ExportUseCases;
use application::invoices::InvoiceUseCases;
use application::payments::PaymentUseCases;
use application::pdf::PdfUseCases;
use application::ports::backup_archive::BackupArchive;
use application::ports::business_repository::BusinessRepository;
use application::ports::customer_repository::CustomerRepository;
use application::ports::dashboard_repository::DashboardRepository;
use application::ports::database_file::DatabaseFile;
use application::ports::file_writer::FileWriter;
use application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
use application::ports::invoice_pdf_renderer::InvoicePdfRenderer;
use application::ports::invoice_repository::InvoiceRepository;
use application::ports::payment_repository::PaymentRepository;
use application::ports::product_repository::ProductRepository;
use application::ports::settings_repository::SettingsRepository;
use application::ports::tax_rate_repository::TaxRateRepository;
use application::ports::transaction::TransactionManager;
use application::products::ProductUseCases;
use application::settings::SettingsUseCases;
use application::tax_rates::TaxRateUseCases;
use infrastructure::database::database_file::SqliteDatabaseFile;
use infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
use infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
use infrastructure::database::sqlite_dashboard_repository::SqliteDashboardRepository;
use infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
use infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
use infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
use infrastructure::database::sqlite_product_repository::SqliteProductRepository;
use infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
use infrastructure::database::sqlite_tax_rate_repository::SqliteTaxRateRepository;
use infrastructure::database::transaction::SqlxTransactionManager;
use infrastructure::filesystem::file_writer::StdFileWriter;
use infrastructure::filesystem::vbx_archive::VbxArchive;
use infrastructure::pdf::PrintpdfInvoiceRenderer;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("vunexo-billing.db");

            let pool = tauri::async_runtime::block_on(async {
                let pool = infrastructure::database::init_pool(&db_path).await?;
                infrastructure::database::run_migrations(&pool).await?;
                infrastructure::database::seed_defaults(&pool).await?;
                Ok::<_, anyhow::Error>(pool)
            })?;

            let tx_manager: Arc<dyn TransactionManager> =
                Arc::new(SqlxTransactionManager::new(pool.clone()));
            let business_repo: Arc<dyn BusinessRepository> =
                Arc::new(SqliteBusinessRepository::new(pool.clone()));
            let customer_repo: Arc<dyn CustomerRepository> =
                Arc::new(SqliteCustomerRepository::new(pool.clone()));
            let product_repo: Arc<dyn ProductRepository> =
                Arc::new(SqliteProductRepository::new(pool.clone()));
            let settings_repo: Arc<dyn SettingsRepository> =
                Arc::new(SqliteSettingsRepository::new(pool.clone()));
            let invoice_repo: Arc<dyn InvoiceRepository> =
                Arc::new(SqliteInvoiceRepository::new(pool.clone()));
            let payment_repo: Arc<dyn PaymentRepository> =
                Arc::new(SqlitePaymentRepository::new(pool.clone()));
            let tax_rate_repo: Arc<dyn TaxRateRepository> =
                Arc::new(SqliteTaxRateRepository::new(pool.clone()));
            let dashboard_repo: Arc<dyn DashboardRepository> =
                Arc::new(SqliteDashboardRepository::new(pool.clone()));
            let database_file: Arc<dyn DatabaseFile> =
                Arc::new(SqliteDatabaseFile::new(pool.clone(), db_path));
            let backup_archive: Arc<dyn BackupArchive> = Arc::new(VbxArchive::new());
            let sequencer: Arc<dyn InvoiceNumberSequencer> =
                Arc::new(SqliteInvoiceNumberSequencer::new(pool));
            let pdf_renderer: Arc<dyn InvoicePdfRenderer> =
                Arc::new(PrintpdfInvoiceRenderer::new());
            let file_writer: Arc<dyn FileWriter> = Arc::new(StdFileWriter::new());

            app.manage(BusinessUseCases::new(
                business_repo.clone(),
                tx_manager.clone(),
                file_writer.clone(),
                data_dir.clone(),
            ));
            app.manage(CustomerUseCases::new(
                customer_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(ProductUseCases::new(
                product_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(SettingsUseCases::new(
                settings_repo.clone(),
                invoice_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(PaymentUseCases::new(
                payment_repo.clone(),
                invoice_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(PdfUseCases::new(
                invoice_repo.clone(),
                customer_repo.clone(),
                business_repo.clone(),
                settings_repo.clone(),
                payment_repo.clone(),
                pdf_renderer,
                file_writer.clone(),
                data_dir.clone(),
            ));
            app.manage(TaxRateUseCases::new(
                tax_rate_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(BackupUseCases::new(
                database_file,
                backup_archive,
                business_repo.clone(),
                data_dir,
                env!("CARGO_PKG_VERSION").to_string(),
                std::env::consts::OS.to_string(),
            ));
            app.manage(ExportUseCases::new(
                business_repo.clone(),
                customer_repo.clone(),
                product_repo.clone(),
                invoice_repo.clone(),
                payment_repo.clone(),
                settings_repo.clone(),
                tax_rate_repo,
                file_writer.clone(),
            ));
            app.manage(DashboardUseCases::new(dashboard_repo));
            app.manage(InvoiceUseCases::new(
                invoice_repo,
                customer_repo,
                business_repo,
                settings_repo,
                sequencer,
                tx_manager,
            ));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::create_business,
            commands::get_business,
            commands::update_business,
            commands::create_customer,
            commands::update_customer,
            commands::archive_customer,
            commands::restore_customer,
            commands::delete_customer,
            commands::get_customer,
            commands::list_customers,
            commands::create_product,
            commands::update_product,
            commands::archive_product,
            commands::restore_product,
            commands::delete_product,
            commands::get_product,
            commands::list_products,
            commands::get_settings,
            commands::update_settings,
            commands::preview_next_invoice_number,
            commands::create_draft_invoice,
            commands::update_draft_invoice,
            commands::issue_invoice,
            commands::edit_issued_invoice,
            commands::cancel_invoice,
            commands::delete_draft_invoice,
            commands::duplicate_invoice,
            commands::get_invoice,
            commands::list_invoices,
            commands::record_payment,
            commands::update_payment,
            commands::delete_payment,
            commands::list_payments_for_invoice,
            commands::create_tax_rate,
            commands::update_tax_rate,
            commands::list_tax_rates,
            commands::get_dashboard_metrics,
            commands::render_invoice_pdf,
            commands::probe_business_logo,
            commands::suggested_backup_file_name,
            commands::backup_database,
            commands::inspect_backup,
            commands::restore_backup,
            commands::suggested_export_file_name,
            commands::export_data,
            commands::save_invoice_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vunexo-billing");
}
