//! CSV and JSON export use cases. ui-ux.md §6.
//!
//! Every export shares three properties with backup, stated there and worth
//! restating because they constrain the code: **read-only** (nothing in the
//! database is touched), saved through the OS dialog, and all-or-nothing —
//! a failure produces no file rather than a truncated one.

use std::path::Path;
use std::sync::Arc;

use crate::domain::customer::{Customer, CustomerFilter};
use crate::domain::export::{
    customers_csv, invoices_csv, products_csv, ExportEntity, InvoiceExport, InvoiceExportRow,
};
use crate::domain::invoice::InvoiceFilter;
use crate::domain::product::{Product, ProductFilter};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::customer_repository::CustomerRepository;
use super::ports::file_writer::FileWriter;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::payment_repository::PaymentRepository;
use super::ports::product_repository::ProductRepository;
use super::ports::settings_repository::SettingsRepository;
use super::ports::tax_rate_repository::TaxRateRepository;

pub struct ExportUseCases {
    business_repo: Arc<dyn BusinessRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
    product_repo: Arc<dyn ProductRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
    payment_repo: Arc<dyn PaymentRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
    tax_rate_repo: Arc<dyn TaxRateRepository>,
    file_writer: Arc<dyn FileWriter>,
}

impl ExportUseCases {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        business_repo: Arc<dyn BusinessRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
        product_repo: Arc<dyn ProductRepository>,
        invoice_repo: Arc<dyn InvoiceRepository>,
        payment_repo: Arc<dyn PaymentRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
        tax_rate_repo: Arc<dyn TaxRateRepository>,
        file_writer: Arc<dyn FileWriter>,
    ) -> Self {
        Self {
            business_repo,
            customer_repo,
            product_repo,
            invoice_repo,
            payment_repo,
            settings_repo,
            tax_rate_repo,
            file_writer,
        }
    }

    pub async fn export_to(
        &self,
        entity: ExportEntity,
        destination: &Path,
    ) -> Result<(), ApplicationError> {
        let contents = self.render(entity).await?;
        self.file_writer.write(destination, contents.as_bytes())?;
        Ok(())
    }

    async fn render(&self, entity: ExportEntity) -> Result<String, ApplicationError> {
        let settings = self.settings_repo.get().await?;
        match entity {
            ExportEntity::Customers => Ok(customers_csv(&self.all_customers().await?)),
            ExportEntity::Products => Ok(products_csv(
                &self.all_products().await?,
                &self.tax_rate_repo.list().await?,
                &settings.currency_code,
            )),
            ExportEntity::Invoices => Ok(invoices_csv(
                &self.invoice_rows().await?,
                &settings.currency_code,
            )),
            ExportEntity::All => self.all_data_json().await,
        }
    }

    /// Exports include archived records deliberately: an export is a copy of
    /// what the business has, not of what the pickers currently offer.
    async fn all_customers(&self) -> Result<Vec<Customer>, ApplicationError> {
        Ok(self
            .customer_repo
            .list(CustomerFilter {
                include_archived: true,
            })
            .await?
            .into_iter()
            .map(|item| item.customer)
            .collect())
    }

    async fn all_products(&self) -> Result<Vec<Product>, ApplicationError> {
        Ok(self
            .product_repo
            .list(ProductFilter {
                include_archived: true,
            })
            .await?
            .into_iter()
            .map(|item| item.product)
            .collect())
    }

    async fn invoice_rows(&self) -> Result<Vec<InvoiceExportRow>, ApplicationError> {
        let summaries = self.invoice_repo.list(InvoiceFilter::default()).await?;
        let mut rows = Vec::with_capacity(summaries.len());
        for summary in summaries {
            // The summary projection carries the paid total already, but not
            // the subtotal/discount/tax breakdown an accountant needs.
            let Some(invoice) = self.invoice_repo.get(summary.id).await? else {
                continue;
            };
            rows.push(InvoiceExportRow {
                invoice_number: summary.invoice_number,
                status: summary.status,
                customer_name: summary.customer_name,
                invoice_date: summary.invoice_date.to_string(),
                due_date: summary.due_date.map(|d| d.to_string()),
                subtotal_minor: invoice.invoice.subtotal_minor,
                discount_amount_minor: invoice.invoice.discount_amount_minor,
                tax_amount_minor: invoice.invoice.tax_amount_minor,
                total_minor: invoice.invoice.total_minor,
                amount_paid_minor: summary.amount_paid_minor,
            });
        }
        Ok(rows)
    }

    /// Every table in database-schema.md §13, as the domain shapes rather
    /// than a raw table dump (ui-ux.md §6).
    async fn all_data_json(&self) -> Result<String, ApplicationError> {
        let summaries = self.invoice_repo.list(InvoiceFilter::default()).await?;
        let mut invoices = Vec::with_capacity(summaries.len());
        for summary in &summaries {
            let Some(full) = self.invoice_repo.get(summary.id).await? else {
                continue;
            };
            let payments = self.payment_repo.list_for_invoice(summary.id).await?;
            invoices.push((full, payments));
        }
        let invoice_exports: Vec<InvoiceExport<'_>> = invoices
            .iter()
            .map(|(full, payments)| InvoiceExport {
                invoice: &full.invoice,
                line_items: &full.line_items,
                payments,
            })
            .collect();

        let document = serde_json::json!({
            "exported_at": chrono::Utc::now(),
            "business": self.business_repo.get().await?,
            "settings": self.settings_repo.get().await?,
            "tax_rates": self.tax_rate_repo.list().await?,
            "customers": self.all_customers().await?,
            "products": self.all_products().await?,
            "invoices": invoice_exports,
        });
        serde_json::to_string_pretty(&document).map_err(|err| {
            crate::application::ports::infrastructure_error::InfrastructureError::Io(format!(
                "could not serialize the export: {err}"
            ))
            .into()
        })
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::application::customers::CustomerUseCases;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::customer::CustomerFields;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
    use crate::infrastructure::database::sqlite_product_repository::SqliteProductRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::sqlite_tax_rate_repository::SqliteTaxRateRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};
    use crate::infrastructure::filesystem::file_writer::StdFileWriter;

    use super::*;

    struct TestApp {
        exports: ExportUseCases,
        customers: CustomerUseCases,
        dir: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    async fn setup(tag: &str) -> TestApp {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("vunexo_export_{tag}_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pool = init_pool(&dir.join("db.sqlite")).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let customer_repo: Arc<dyn CustomerRepository> =
            Arc::new(SqliteCustomerRepository::new(pool.clone()));

        TestApp {
            exports: ExportUseCases::new(
                Arc::new(SqliteBusinessRepository::new(pool.clone())),
                customer_repo.clone(),
                Arc::new(SqliteProductRepository::new(pool.clone())),
                Arc::new(SqliteInvoiceRepository::new(pool.clone())),
                Arc::new(SqlitePaymentRepository::new(pool.clone())),
                Arc::new(SqliteSettingsRepository::new(pool.clone())),
                Arc::new(SqliteTaxRateRepository::new(pool)),
                Arc::new(StdFileWriter::new()),
            ),
            customers: CustomerUseCases::new(customer_repo, tx_manager),
            dir,
        }
    }

    async fn add_customer(app: &TestApp, name: &str) -> i64 {
        app.customers
            .create_customer(CustomerFields {
                name: name.to_string(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .expect("create_customer")
            .id
    }

    #[tokio::test]
    async fn exporting_customers_writes_a_csv_the_user_picked_the_path_for() {
        let app = setup("customers").await;
        add_customer(&app, "Acme, Traders").await;

        let path = app.dir.join("customers.csv");
        app.exports
            .export_to(ExportEntity::Customers, &path)
            .await
            .expect("export_to");

        let csv = std::fs::read_to_string(&path).expect("read export");
        assert!(csv.starts_with("name,phone,email,address,gstin,status\r\n"));
        // The comma in the name must be quoted, not treated as a separator.
        assert!(csv.contains("\"Acme, Traders\""));
    }

    #[tokio::test]
    async fn exports_include_archived_records() {
        // An export is a copy of what the business has, not of what the
        // pickers currently offer.
        let app = setup("archived").await;
        let id = add_customer(&app, "Retired Customer").await;
        app.customers.archive_customer(id).await.expect("archive");

        let path = app.dir.join("customers.csv");
        app.exports
            .export_to(ExportEntity::Customers, &path)
            .await
            .expect("export_to");

        let csv = std::fs::read_to_string(&path).expect("read export");
        assert!(csv.contains("Retired Customer"));
        assert!(csv.contains("ARCHIVED"));
    }

    #[tokio::test]
    async fn the_json_export_carries_every_table_even_when_empty() {
        let app = setup("json").await;
        add_customer(&app, "Acme").await;

        let path = app.dir.join("all.json");
        app.exports
            .export_to(ExportEntity::All, &path)
            .await
            .expect("export_to");

        let raw = std::fs::read_to_string(&path).expect("read export");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        for key in [
            "exported_at",
            "business",
            "settings",
            "tax_rates",
            "customers",
            "products",
            "invoices",
        ] {
            assert!(parsed.get(key).is_some(), "JSON export is missing {key:?}");
        }
        assert_eq!(parsed["customers"][0]["name"], "Acme");
        // Domain shapes, not raw rows: an invoice carries its line items.
        assert!(parsed["invoices"].is_array());
    }

    #[tokio::test]
    async fn an_export_never_changes_the_data_it_reads() {
        let app = setup("readonly").await;
        add_customer(&app, "Acme").await;

        for entity in [
            ExportEntity::Customers,
            ExportEntity::Products,
            ExportEntity::Invoices,
            ExportEntity::All,
        ] {
            app.exports
                .export_to(entity, &app.dir.join("out"))
                .await
                .expect("export_to");
        }

        let customers = app
            .customers
            .list_customers(CustomerFilter {
                include_archived: true,
            })
            .await
            .expect("list");
        assert_eq!(customers.len(), 1);
        assert_eq!(customers[0].customer.name, "Acme");
    }
}
