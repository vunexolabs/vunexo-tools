#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;
mod domain;
mod infrastructure;

use std::sync::Arc;

use tauri::Manager;

use application::business::BusinessUseCases;
use application::customers::CustomerUseCases;
use application::ports::business_repository::BusinessRepository;
use application::ports::customer_repository::CustomerRepository;
use application::ports::transaction::TransactionManager;
use infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
use infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
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
                Ok::<_, anyhow::Error>(pool)
            })?;

            let tx_manager: Arc<dyn TransactionManager> =
                Arc::new(SqlxTransactionManager::new(pool.clone()));
            let business_repo: Arc<dyn BusinessRepository> =
                Arc::new(SqliteBusinessRepository::new(pool.clone()));
            let customer_repo: Arc<dyn CustomerRepository> =
                Arc::new(SqliteCustomerRepository::new(pool));

            app.manage(BusinessUseCases::new(business_repo, tx_manager.clone()));
            app.manage(CustomerUseCases::new(customer_repo, tx_manager));

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running vunexo-billing");
}
