//! Dashboard use case. application-architecture.md §4 ("Dashboard").

use std::sync::Arc;

use chrono::{Datelike, Utc};

use crate::domain::dashboard::DashboardMetrics;

use super::error::ApplicationError;
use super::ports::dashboard_repository::DashboardRepository;

/// How many recent invoices the landing screen shows (user-flows.md §8).
const RECENT_INVOICES_LIMIT: i64 = 10;

pub struct DashboardUseCases {
    repo: Arc<dyn DashboardRepository>,
}

impl DashboardUseCases {
    pub fn new(repo: Arc<dyn DashboardRepository>) -> Self {
        Self { repo }
    }

    /// Assembles its response entirely from `DashboardRepository` — does no
    /// invoice iteration of its own (application-architecture.md §4).
    pub async fn get_dashboard_metrics(&self) -> Result<DashboardMetrics, ApplicationError> {
        let today = Utc::now().date_naive();
        let (year, month) = (today.year(), today.month());

        Ok(DashboardMetrics {
            today_sales_minor: self.repo.today_sales(today).await?,
            month_sales_minor: self.repo.month_sales(year, month).await?,
            outstanding_total_minor: self.repo.outstanding_total().await?,
            paid_total_minor: self.repo.paid_total(year, month).await?,
            overdue: self.repo.overdue_summary(today).await?,
            recent_invoices: self.repo.recent_invoices(RECENT_INVOICES_LIMIT).await?,
        })
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite, real repositories — proves the exact metric definitions
    //! in application-architecture.md §3c's table, not just that the SQL
    //! parses. Dates are computed relative to "now" (never hardcoded) so
    //! the test doesn't quietly rot as the calendar moves on.
    use std::sync::Arc;

    use chrono::Duration;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::invoices::InvoiceUseCases;
    use crate::application::payments::PaymentUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::dashboard_repository::DashboardRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::invoice_repository::InvoiceRepository;
    use crate::application::ports::payment_repository::PaymentRepository;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::DraftInvoiceInput;
    use crate::domain::invoice_line_item::LineItemInput;
    use crate::domain::payment::{NewPayment, PaymentMethod};
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_dashboard_repository::SqliteDashboardRepository;
    use crate::infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        dashboard: DashboardUseCases,
        invoices: InvoiceUseCases,
        payments: PaymentUseCases,
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
            "vunexo_dashboard_test_{}_{}.db",
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
        let payment_repo: Arc<dyn PaymentRepository> =
            Arc::new(SqlitePaymentRepository::new(pool.clone()));
        let dashboard_repo: Arc<dyn DashboardRepository> =
            Arc::new(SqliteDashboardRepository::new(pool.clone()));
        let sequencer: Arc<dyn InvoiceNumberSequencer> =
            Arc::new(SqliteInvoiceNumberSequencer::new(pool));

        TestApp {
            dashboard: DashboardUseCases::new(dashboard_repo),
            payments: PaymentUseCases::new(payment_repo, invoice_repo.clone(), tx_manager.clone()),
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

    async fn create_and_issue(
        app: &TestApp,
        customer_id: i64,
        total_rupees: i64,
        due_date: chrono::NaiveDate,
        invoice_date: chrono::NaiveDate,
    ) -> crate::domain::invoice::InvoiceWithLineItems {
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer_id),
                invoice_date,
                due_date: Some(due_date),
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
                    unit_price_minor: total_rupees * 100,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 0,
                }],
            })
            .await
            .expect("create_draft_invoice");
        app.invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .expect("issue_invoice")
    }

    #[tokio::test]
    async fn dashboard_metrics_match_hand_computed_expectations() {
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
            })
            .await
            .expect("create_business");
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Dashboard Test Customer".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .expect("create_customer");

        let today = chrono::Utc::now().date_naive();
        let yesterday = today - Duration::days(1);
        let next_month = today + Duration::days(60);

        // Invoice A: ₹1,000, overdue (due yesterday), unpaid — stays Issued.
        let _invoice_a = create_and_issue(&app, customer.id, 1000, yesterday, today).await;
        // Invoice B: ₹500, due next month, paid in full — becomes Paid.
        let invoice_b = create_and_issue(&app, customer.id, 500, next_month, today).await;
        app.payments
            .record_payment(NewPayment {
                invoice_id: invoice_b.invoice.id,
                amount_minor: invoice_b.invoice.total_minor,
                method: PaymentMethod::Cash,
                paid_on: today,
                reference: None,
            })
            .await
            .expect("record_payment (full)");
        // Invoice C: ₹300, due next month, partially paid ₹100.
        let invoice_c = create_and_issue(&app, customer.id, 300, next_month, today).await;
        app.payments
            .record_payment(NewPayment {
                invoice_id: invoice_c.invoice.id,
                amount_minor: 10_000,
                method: PaymentMethod::Cash,
                paid_on: today,
                reference: None,
            })
            .await
            .expect("record_payment (partial)");
        // Invoice D: cancelled after issue — must be excluded from every metric.
        let invoice_d = create_and_issue(&app, customer.id, 9999, next_month, today).await;
        app.invoices
            .cancel_invoice(invoice_d.invoice.id, None)
            .await
            .expect("cancel_invoice");
        // Invoice E: left as a draft — must be excluded from every sales/outstanding metric.
        app.invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: today,
                due_date: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![],
            })
            .await
            .expect("create_draft_invoice (E)");

        let metrics = app
            .dashboard
            .get_dashboard_metrics()
            .await
            .expect("get_dashboard_metrics");

        // today_sales / month_sales: A + B + C, excluding cancelled D and draft E.
        assert_eq!(metrics.today_sales_minor, 180_000, "1000+500+300 = 1800");
        assert_eq!(metrics.month_sales_minor, 180_000);

        // outstanding_total: A (1000 unpaid) + C (300-100=200 remaining), excluding paid B.
        assert_eq!(metrics.outstanding_total_minor, 120_000);

        // paid_total (this month): only B.
        assert_eq!(metrics.paid_total_minor, 50_000);

        // overdue: only A (due yesterday, still owes the full amount).
        assert_eq!(metrics.overdue.count, 1);
        assert_eq!(metrics.overdue.total_minor, 100_000);

        // recent_invoices: every invoice regardless of status (A, B, C, D, E) — recency, not a status filter.
        assert_eq!(metrics.recent_invoices.len(), 5);
    }
}
