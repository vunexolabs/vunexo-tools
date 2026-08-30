//! Statement use case. application-architecture-v2.md §3 ("Statements") —
//! a thin pass-through to `StatementRepository`, no validation beyond
//! "customer exists," no transaction (read-only).

use std::sync::Arc;

use chrono::NaiveDate;

use crate::domain::statement::StatementResult;

use super::error::ApplicationError;
use super::ports::customer_repository::CustomerRepository;
use super::ports::statement_repository::StatementRepository;

pub struct StatementUseCases {
    repo: Arc<dyn StatementRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
}

impl StatementUseCases {
    pub fn new(
        repo: Arc<dyn StatementRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
    ) -> Self {
        Self {
            repo,
            customer_repo,
        }
    }

    pub async fn generate_customer_statement(
        &self,
        customer_id: i64,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<StatementResult, ApplicationError> {
        // find_by_id also confirms the customer exists before hitting the
        // statement query — a NotFound here is a clearer error than an
        // empty statement for a customer id that was never real.
        self.customer_repo
            .find_by_id(customer_id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "customer",
                id: customer_id,
            })?;

        Ok(self
            .repo
            .customer_statement(customer_id, range_start, range_end)
            .await?)
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite. `issued_at` is always `CURRENT_TIMESTAMP` at Issue (no
    //! use case lets a test backdate it directly), so invoices are backdated
    //! via a raw `UPDATE` after issuing — the one place this test module
    //! reaches past the application layer, deliberately, to construct a
    //! fixture spanning three real periods. `paid_on` needs no such trick;
    //! `NewPayment` already takes it as a plain date.
    use std::sync::Arc;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::invoices::InvoiceUseCases;
    use crate::application::payments::PaymentUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::invoice_repository::InvoiceRepository;
    use crate::application::ports::payment_repository::PaymentRepository;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::application::ports::statement_repository::StatementRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::DraftInvoiceInput;
    use crate::domain::invoice_line_item::LineItemInput;
    use crate::domain::payment::{NewPayment, PaymentMethod};
    use crate::domain::tax_regime::TaxRegimeCode;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::sqlite_statement_repository::SqliteStatementRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        statements: StatementUseCases,
        invoices: InvoiceUseCases,
        payments: PaymentUseCases,
        customers: CustomerUseCases,
        business: BusinessUseCases,
        pool: sqlx::SqlitePool,
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
            "vunexo_statement_test_{}_{}.db",
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
        let statement_repo: Arc<dyn StatementRepository> =
            Arc::new(SqliteStatementRepository::new(pool.clone()));
        let sequencer: Arc<dyn InvoiceNumberSequencer> =
            Arc::new(SqliteInvoiceNumberSequencer::new(pool.clone()));

        TestApp {
            statements: StatementUseCases::new(statement_repo, customer_repo.clone()),
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
            pool,
            db_path,
        }
    }

    async fn issue_backdated(
        app: &TestApp,
        customer_id: i64,
        total_rupees: i64,
        issued_on: chrono::NaiveDate,
    ) -> i64 {
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer_id),
                invoice_date: issued_on,
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
                    unit_price_minor: total_rupees * 100,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 0,
                }],
            })
            .await
            .expect("create_draft_invoice");
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .expect("issue_invoice");

        sqlx::query("UPDATE invoices SET issued_at = ? WHERE id = ?")
            .bind(format!("{issued_on} 12:00:00"))
            .bind(issued.invoice.id)
            .execute(&app.pool)
            .await
            .expect("backdate issued_at");

        issued.invoice.id
    }

    /// database-schema-v2.md §7's flagged invariant: closing balance of one
    /// period equals opening balance of the next, by construction — not
    /// approximately, exactly, across three real consecutive periods.
    #[tokio::test]
    async fn closing_balance_equals_next_periods_opening_balance() {
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
                name: "Statement Test Customer".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();

        let jan = chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let feb = chrono::NaiveDate::from_ymd_opt(2026, 2, 10).unwrap();
        let mar = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        let inv_jan = issue_backdated(&app, customer.id, 1000, jan).await;
        let inv_feb = issue_backdated(&app, customer.id, 2000, feb).await;
        let _inv_mar = issue_backdated(&app, customer.id, 3000, mar).await;

        app.payments
            .record_payment(NewPayment {
                invoice_id: inv_jan,
                amount_minor: 60_000, // Rs 600, partial
                method: PaymentMethod::Cash,
                paid_on: chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                reference: None,
            })
            .await
            .unwrap();
        app.payments
            .record_payment(NewPayment {
                invoice_id: inv_feb,
                amount_minor: 200_000, // Rs 2,000, full
                method: PaymentMethod::Cash,
                paid_on: chrono::NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
                reference: None,
            })
            .await
            .unwrap();

        let p1_start = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let p2_start = chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let p3_start = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let p3_end = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();

        let period1 = app
            .statements
            .generate_customer_statement(customer.id, p1_start, p2_start)
            .await
            .unwrap();
        let period2 = app
            .statements
            .generate_customer_statement(customer.id, p2_start, p3_start)
            .await
            .unwrap();
        let period3 = app
            .statements
            .generate_customer_statement(customer.id, p3_start, p3_end)
            .await
            .unwrap();

        assert_eq!(period1.opening_balance_minor, 0, "nothing before January");
        // Jan: +1000 invoice, -600 payment => closing 400.
        assert_eq!(period1.closing_balance_minor, 40_000);
        assert_eq!(
            period2.opening_balance_minor, period1.closing_balance_minor,
            "February's opening must equal January's closing exactly"
        );
        // Feb: opening 400 + 2000 invoice - 2000 payment => closing 400.
        assert_eq!(period2.closing_balance_minor, 40_000);
        assert_eq!(
            period3.opening_balance_minor, period2.closing_balance_minor,
            "March's opening must equal February's closing exactly"
        );
        // Mar: opening 400 + 3000 invoice, no payment => closing 3400.
        assert_eq!(period3.closing_balance_minor, 340_000);

        assert_eq!(
            period1.entries.len(),
            2,
            "one invoice + one payment in January"
        );
        assert_eq!(
            period3.entries.len(),
            1,
            "one invoice, no payment, in March"
        );
    }

    #[tokio::test]
    async fn unknown_customer_is_rejected() {
        let app = setup().await;
        let result = app
            .statements
            .generate_customer_statement(
                999,
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            )
            .await;
        assert!(matches!(result, Err(ApplicationError::NotFound { .. })));
    }
}
