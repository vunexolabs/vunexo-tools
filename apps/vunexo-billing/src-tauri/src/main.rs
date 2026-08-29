#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;
mod domain;
mod infrastructure;

use std::sync::Arc;

use tauri::Manager;

use application::business::BusinessUseCases;
use application::customers::CustomerUseCases;
use application::invoices::InvoiceUseCases;
use application::ports::business_repository::BusinessRepository;
use application::ports::customer_repository::CustomerRepository;
use application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
use application::ports::invoice_repository::InvoiceRepository;
use application::ports::product_repository::ProductRepository;
use application::ports::settings_repository::SettingsRepository;
use application::ports::transaction::TransactionManager;
use application::products::ProductUseCases;
use application::settings::SettingsUseCases;
use infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
use infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
use infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
use infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
use infrastructure::database::sqlite_product_repository::SqliteProductRepository;
use infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
use infrastructure::database::transaction::SqlxTransactionManager;

fn main() {
    tauri::Builder::default()
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
            let sequencer: Arc<dyn InvoiceNumberSequencer> =
                Arc::new(SqliteInvoiceNumberSequencer::new(pool));

            app.manage(BusinessUseCases::new(
                business_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(CustomerUseCases::new(
                customer_repo.clone(),
                tx_manager.clone(),
            ));
            app.manage(ProductUseCases::new(product_repo, tx_manager.clone()));
            app.manage(SettingsUseCases::new(
                settings_repo.clone(),
                invoice_repo.clone(),
                tx_manager.clone(),
            ));
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
            commands::cancel_invoice,
            commands::delete_draft_invoice,
            commands::duplicate_invoice,
            commands::get_invoice,
            commands::list_invoices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vunexo-billing");
}
