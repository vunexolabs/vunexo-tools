//! Report use cases. application-architecture-v2.md §3 ("Reports") — thin
//! pass-throughs to `ReportRepository`, same shape as `StatementUseCases`.

use std::sync::Arc;

use chrono::NaiveDate;

use crate::domain::report::{SalesGrouping, SalesSummaryResult, TaxSummaryResult};

use super::error::ApplicationError;
use super::ports::report_repository::ReportRepository;

pub struct ReportUseCases {
    repo: Arc<dyn ReportRepository>,
}

impl ReportUseCases {
    pub fn new(repo: Arc<dyn ReportRepository>) -> Self {
        Self { repo }
    }

    pub async fn generate_sales_report(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        group_by: SalesGrouping,
    ) -> Result<SalesSummaryResult, ApplicationError> {
        Ok(self
            .repo
            .sales_summary(range_start, range_end, group_by)
            .await?)
    }

    pub async fn generate_tax_summary_report(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxSummaryResult, ApplicationError> {
        Ok(self.repo.tax_summary(range_start, range_end).await?)
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::invoices::InvoiceUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::invoice_repository::InvoiceRepository;
    use crate::application::ports::report_repository::ReportRepository;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::DraftInvoiceInput;
    use crate::domain::invoice_line_item::LineItemInput;
    use crate::domain::tax_regime::TaxRegimeCode;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_report_repository::SqliteReportRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        reports: ReportUseCases,
        invoices: InvoiceUseCases,
        customers: CustomerUseCases,
        business: BusinessUseCases,
        db_path: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.db_path);
        }
    }

    async fn setup() -> TestApp {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_report_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let business_repo: Arc<dyn BusinessRepository> =
            Arc::new(SqliteBusinessRepository::new(pool.clone()));
        let customer_repo: Arc<dyn CustomerRepository> =
            Arc::new(SqliteCustomerRepository::new(pool.clone()));
        let settings_repo: Arc<dyn SettingsRepository> =
            Arc::new(SqliteSettingsRepository::new(pool.clone()));
        let invoice_repo: Arc<dyn InvoiceRepository> =
            Arc::new(SqliteInvoiceRepository::new(pool.clone()));
        let report_repo: Arc<dyn ReportRepository> =
            Arc::new(SqliteReportRepository::new(pool.clone()));
        let sequencer: Arc<dyn InvoiceNumberSequencer> =
            Arc::new(SqliteInvoiceNumberSequencer::new(pool));

        TestApp {
            reports: ReportUseCases::new(report_repo),
            invoices: InvoiceUseCases::new(
                invoice_repo,
                customer_repo.clone(),
                business_repo.clone(),
                settings_repo,
                sequencer,
                tx_manager.clone(),
            ),
            customers: CustomerUseCases::new(customer_repo, tx_manager.clone()),
            business: BusinessUseCases::new(
                business_repo,
                tx_manager,
                Arc::new(crate::infrastructure::filesystem::file_writer::StdFileWriter::new()),
                db_path.parent().unwrap().to_path_buf(),
            ),
            db_path,
        }
    }

    async fn issue(
        app: &TestApp,
        customer_id: i64,
        total_rupees: i64,
        tax_bp: i64,
        date: chrono::NaiveDate,
    ) {
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer_id),
                invoice_date: date,
                due_date: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![LineItemInput {
                    product_id: None,
                    description: "Consulting".into(),
                    unit: "hr".into(),
                    quantity_thousandths: 1000,
                    unit_price_minor: total_rupees * 100,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: tax_bp,
                }],
            })
            .await
            .unwrap();
        app.invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sales_report_grouped_by_customer_matches_hand_computed_totals() {
        let app = setup().await;
        app.business
            .create_business(Business {
                name: "Vunexo Test Co".into(),
                logo_path: None,
                address: None,
                phone: None,
                email: None,
                gstin: None,
                bank_details: None,
                upi_id: None,
                tax_regime_code: TaxRegimeCode::InGst,
            })
            .await
            .unwrap();
        let a = app
            .customers
            .create_customer(CustomerFields {
                name: "Customer A".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();
        let b = app
            .customers
            .create_customer(CustomerFields {
                name: "Customer B".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();

        let today = chrono::Utc::now().date_naive();
        issue(&app, a.id, 1000, 1800, today).await;
        issue(&app, a.id, 500, 1800, today).await;
        issue(&app, b.id, 2000, 0, today).await;

        let start = today;
        let end = today + chrono::Duration::days(1);
        let report = app
            .reports
            .generate_sales_report(start, end, crate::domain::report::SalesGrouping::Customer)
            .await
            .unwrap();

        // Sales totals are pre-tax subtotal + tax = total_minor, matching
        // what invoices.total_minor already is.
        assert_eq!(
            report.total_sales_minor,
            100_000 * 118 / 100 + 50_000 * 118 / 100 + 200_000
        );
        assert_eq!(report.rows.len(), 2);
        let a_row = report
            .rows
            .iter()
            .find(|r| r.label == "Customer A")
            .unwrap();
        assert_eq!(a_row.sales_minor, 100_000 * 118 / 100 + 50_000 * 118 / 100);
        let b_row = report
            .rows
            .iter()
            .find(|r| r.label == "Customer B")
            .unwrap();
        assert_eq!(b_row.sales_minor, 200_000);
    }

    #[tokio::test]
    async fn tax_summary_groups_by_regime_and_excludes_cancelled() {
        let app = setup().await;
        app.business
            .create_business(Business {
                name: "Vunexo Test Co".into(),
                logo_path: None,
                address: None,
                phone: None,
                email: None,
                gstin: None,
                bank_details: None,
                upi_id: None,
                tax_regime_code: TaxRegimeCode::InGst,
            })
            .await
            .unwrap();
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Tax Test Customer".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();
        let today = chrono::Utc::now().date_naive();

        issue(&app, customer.id, 1000, 1800, today).await; // tax = 180

        // A cancelled invoice must not contribute.
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: today,
                due_date: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![LineItemInput {
                    product_id: None,
                    description: "Item".into(),
                    unit: "pcs".into(),
                    quantity_thousandths: 1000,
                    unit_price_minor: 50_000,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 1800,
                }],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();
        app.invoices
            .cancel_invoice(issued.invoice.id, None)
            .await
            .unwrap();

        let start = today;
        let end = today + chrono::Duration::days(1);
        let summary = app
            .reports
            .generate_tax_summary_report(start, end)
            .await
            .unwrap();

        assert_eq!(
            summary.total_tax_minor, 18_000,
            "only the non-cancelled invoice's tax"
        );
        assert_eq!(summary.by_regime.len(), 1);
        assert_eq!(summary.by_regime[0].tax_regime, TaxRegimeCode::InGst);
        assert_eq!(summary.by_regime[0].tax_amount_minor, 18_000);
    }
}
