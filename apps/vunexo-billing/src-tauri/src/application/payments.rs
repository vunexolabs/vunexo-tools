//! Payment use cases. application-architecture.md §4 ("Payments"),
//! user-flows.md §6, database-schema.md §3/§8.
//!
//! `RecordPayment` / `UpdatePayment` / `DeletePayment` each run, in one
//! transaction: confirm the precondition, write the payment change, `SUM`
//! payments for the invoice, and call `InvoiceRepository::set_status` with
//! the recalculated status — never touching the invoice's own total,
//! discount, tax, or line items.

use std::sync::Arc;

use crate::domain::invoice::InvoiceStatus;
use crate::domain::payment::{NewPayment, Payment, PaymentFields};

use super::error::ApplicationError;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::payment_repository::PaymentRepository;
use super::ports::transaction::{Transaction, TransactionManager};

pub struct PaymentUseCases {
    payment_repo: Arc<dyn PaymentRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl PaymentUseCases {
    pub fn new(
        payment_repo: Arc<dyn PaymentRepository>,
        invoice_repo: Arc<dyn InvoiceRepository>,
        tx_manager: Arc<dyn TransactionManager>,
    ) -> Self {
        Self {
            payment_repo,
            invoice_repo,
            tx_manager,
        }
    }

    /// user-flows.md §6: overpayment is allowed (flagged by the UI, never
    /// clamped here); a `CANCELLED` invoice accepts no further payments
    /// (database-schema.md §3's state-invariant table); a `DRAFT` invoice
    /// has nothing due yet, so it isn't a valid target either.
    pub async fn record_payment(&self, payment: NewPayment) -> Result<Payment, ApplicationError> {
        if payment.amount_minor <= 0 {
            return Err(ApplicationError::Validation(
                "payment amount must be greater than zero".into(),
            ));
        }
        let invoice_id = payment.invoice_id;

        let existing =
            self.invoice_repo
                .get(invoice_id)
                .await?
                .ok_or(ApplicationError::NotFound {
                    entity: "invoice",
                    id: invoice_id,
                })?;
        match existing.invoice.status {
            InvoiceStatus::Draft => {
                return Err(ApplicationError::Validation(
                    "a draft invoice has no amount due yet — issue it before recording a payment"
                        .into(),
                ));
            }
            InvoiceStatus::Cancelled => {
                return Err(ApplicationError::Conflict(
                    "this invoice is cancelled and can no longer accept payments".into(),
                ));
            }
            InvoiceStatus::Issued | InvoiceStatus::PartiallyPaid | InvoiceStatus::Paid => {}
        }

        let mut tx = self.tx_manager.begin().await?;
        let created = match self.payment_repo.create(&mut *tx, payment).await {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e.into());
            }
        };
        if let Err(e) = self
            .recalculate_status(
                &mut *tx,
                invoice_id,
                existing.invoice.status,
                existing.invoice.total_minor,
            )
            .await
        {
            let _ = tx.rollback().await;
            return Err(e);
        }
        tx.commit().await?;
        Ok(created)
    }

    /// database-schema.md §11: a payment is editable regardless of the
    /// parent invoice's status (people mis-key amounts) — unlike
    /// `record_payment`, this never blocks on `CANCELLED`; recalculation
    /// itself keeps `CANCELLED` terminal (see `recalculate_status`).
    pub async fn update_payment(
        &self,
        id: i64,
        fields: PaymentFields,
    ) -> Result<Payment, ApplicationError> {
        if fields.amount_minor <= 0 {
            return Err(ApplicationError::Validation(
                "payment amount must be greater than zero".into(),
            ));
        }
        let existing = self
            .payment_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "payment",
                id,
            })?;
        let invoice = self.invoice_repo.get(existing.invoice_id).await?.ok_or(
            ApplicationError::NotFound {
                entity: "invoice",
                id: existing.invoice_id,
            },
        )?;

        let mut tx = self.tx_manager.begin().await?;
        let updated = match self.payment_repo.update(&mut *tx, id, fields).await {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e.into());
            }
        };
        if let Err(e) = self
            .recalculate_status(
                &mut *tx,
                existing.invoice_id,
                invoice.invoice.status,
                invoice.invoice.total_minor,
            )
            .await
        {
            let _ = tx.rollback().await;
            return Err(e);
        }
        tx.commit().await?;
        Ok(updated)
    }

    pub async fn delete_payment(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self
            .payment_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "payment",
                id,
            })?;
        let invoice = self.invoice_repo.get(existing.invoice_id).await?.ok_or(
            ApplicationError::NotFound {
                entity: "invoice",
                id: existing.invoice_id,
            },
        )?;

        let mut tx = self.tx_manager.begin().await?;
        if let Err(e) = self.payment_repo.delete(&mut *tx, id).await {
            let _ = tx.rollback().await;
            return Err(e.into());
        }
        if let Err(e) = self
            .recalculate_status(
                &mut *tx,
                existing.invoice_id,
                invoice.invoice.status,
                invoice.invoice.total_minor,
            )
            .await
        {
            let _ = tx.rollback().await;
            return Err(e);
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_payments_for_invoice(
        &self,
        invoice_id: i64,
    ) -> Result<Vec<Payment>, ApplicationError> {
        Ok(self.payment_repo.list_for_invoice(invoice_id).await?)
    }

    async fn recalculate_status(
        &self,
        tx: &mut dyn Transaction,
        invoice_id: i64,
        current_status: InvoiceStatus,
        total_minor: i64,
    ) -> Result<(), ApplicationError> {
        let amount_paid = self.payment_repo.sum_for_invoice(tx, invoice_id).await?;
        let new_status = next_status(current_status, amount_paid, total_minor);
        self.invoice_repo
            .set_status(tx, invoice_id, new_status)
            .await?;
        Ok(())
    }
}

/// database-schema.md §3's worked example / user-flows.md §6:
/// `amount_paid == 0` -> `Issued`, `0 < amount_paid < total` -> `PartiallyPaid`,
/// `amount_paid >= total` -> `Paid`. `CANCELLED` is terminal and is never
/// resurrected by a payment edit — blocking *new* payments on a cancelled
/// invoice is `record_payment`'s job, not this function's.
fn next_status(current: InvoiceStatus, amount_paid_minor: i64, total_minor: i64) -> InvoiceStatus {
    if current == InvoiceStatus::Cancelled {
        return InvoiceStatus::Cancelled;
    }
    if total_minor > 0 && amount_paid_minor >= total_minor {
        InvoiceStatus::Paid
    } else if amount_paid_minor > 0 {
        InvoiceStatus::PartiallyPaid
    } else {
        InvoiceStatus::Issued
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite, real repositories, real transaction manager — not
    //! mocks. This is what actually proves the recalculation rule in
    //! database-schema.md §3/§8 and user-flows.md §6 (including the
    //! `CANCELLED`-stays-terminal edge case) works end to end.
    use std::sync::Arc;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::DraftInvoiceInput;
    use crate::domain::invoice_line_item::LineItemInput;
    use crate::domain::payment::PaymentMethod;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::super::invoices::InvoiceUseCases;
    use super::*;

    struct TestApp {
        payments: PaymentUseCases,
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
        // See invoices.rs's own setup() for why this is an atomic counter
        // rather than just a timestamp — parallel #[tokio::test]s calling
        // setup() close together need a stronger uniqueness guarantee.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_payment_test_{}_{}.db",
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
        let sequencer: Arc<dyn InvoiceNumberSequencer> =
            Arc::new(SqliteInvoiceNumberSequencer::new(pool));

        TestApp {
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

    /// Issues a ₹2,360-total invoice (calculation-engine.md Vector 1) and
    /// returns its id — every test here starts from the same known total.
    async fn setup_issued_invoice(app: &TestApp) -> i64 {
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
                tax_regime_code: crate::domain::tax_regime::TaxRegimeCode::InGst,
            })
            .await
            .expect("create_business");
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Acme Traders".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .expect("create_customer");
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
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
                    quantity_thousandths: 2000,
                    unit_price_minor: 100_000,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 1800,
                }],
            })
            .await
            .expect("create_draft_invoice");
        assert_eq!(draft.invoice.total_minor, 236_000);

        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .expect("issue_invoice");
        issued.invoice.id
    }

    fn sample_payment(invoice_id: i64, amount_minor: i64) -> NewPayment {
        NewPayment {
            invoice_id,
            amount_minor,
            method: PaymentMethod::Cash,
            paid_on: chrono::NaiveDate::from_ymd_opt(2026, 8, 29).unwrap(),
            reference: None,
        }
    }

    #[tokio::test]
    async fn recording_a_partial_then_full_payment_transitions_status_correctly() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;

        app.payments
            .record_payment(sample_payment(invoice_id, 100_000))
            .await
            .expect("record_payment (partial)");
        let after_partial = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(after_partial.invoice.status, InvoiceStatus::PartiallyPaid);

        app.payments
            .record_payment(sample_payment(invoice_id, 136_000))
            .await
            .expect("record_payment (remainder)");
        let after_full = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(after_full.invoice.status, InvoiceStatus::Paid);
    }

    #[tokio::test]
    async fn overpayment_is_recorded_in_full_not_clamped_to_the_total() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;

        let payment = app
            .payments
            .record_payment(sample_payment(invoice_id, 300_000))
            .await
            .expect("record_payment (overpayment)");
        assert_eq!(
            payment.amount_minor, 300_000,
            "the recorded amount must be the exact overpayment, never clamped to the invoice total"
        );
        let invoice = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(invoice.invoice.status, InvoiceStatus::Paid);
    }

    #[tokio::test]
    async fn editing_a_payment_down_recalculates_status_back_to_partially_paid() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;

        let payment = app
            .payments
            .record_payment(sample_payment(invoice_id, 236_000))
            .await
            .expect("record_payment (full)");
        assert_eq!(
            app.invoices
                .get_invoice(invoice_id)
                .await
                .unwrap()
                .invoice
                .status,
            InvoiceStatus::Paid
        );

        app.payments
            .update_payment(
                payment.id,
                PaymentFields {
                    amount_minor: 50_000,
                    method: PaymentMethod::Cash,
                    paid_on: payment.paid_on,
                    reference: Some("corrected — mis-keyed the first amount".into()),
                },
            )
            .await
            .expect("update_payment");

        let invoice = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(invoice.invoice.status, InvoiceStatus::PartiallyPaid);
    }

    #[tokio::test]
    async fn deleting_the_only_payment_recalculates_status_back_to_issued() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;

        let payment = app
            .payments
            .record_payment(sample_payment(invoice_id, 236_000))
            .await
            .expect("record_payment (full)");
        assert_eq!(
            app.invoices
                .get_invoice(invoice_id)
                .await
                .unwrap()
                .invoice
                .status,
            InvoiceStatus::Paid
        );

        app.payments
            .delete_payment(payment.id)
            .await
            .expect("delete_payment");

        let invoice = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(invoice.invoice.status, InvoiceStatus::Issued);
        let remaining = app
            .payments
            .list_payments_for_invoice(invoice_id)
            .await
            .expect("list_payments_for_invoice");
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn recording_a_payment_against_a_cancelled_invoice_is_rejected() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;
        app.invoices
            .cancel_invoice(invoice_id, Some("customer withdrew order".into()))
            .await
            .expect("cancel_invoice");

        let result = app
            .payments
            .record_payment(sample_payment(invoice_id, 50_000))
            .await;
        assert!(matches!(result, Err(ApplicationError::Conflict(_))));
    }

    #[tokio::test]
    async fn editing_a_payment_on_a_now_cancelled_invoice_keeps_status_cancelled() {
        let app = setup().await;
        let invoice_id = setup_issued_invoice(&app).await;
        let payment = app
            .payments
            .record_payment(sample_payment(invoice_id, 100_000))
            .await
            .expect("record_payment");
        app.invoices
            .cancel_invoice(invoice_id, None)
            .await
            .expect("cancel_invoice");

        // database-schema.md §11: payments stay editable regardless of the
        // parent invoice's status (record-keeping correction) — but §3
        // still makes CANCELLED terminal, so the recalculation must not
        // resurrect it into ISSUED/PARTIALLY_PAID/PAID.
        app.payments
            .update_payment(
                payment.id,
                PaymentFields {
                    amount_minor: 50_000,
                    method: PaymentMethod::Cash,
                    paid_on: payment.paid_on,
                    reference: None,
                },
            )
            .await
            .expect("update_payment");

        let invoice = app
            .invoices
            .get_invoice(invoice_id)
            .await
            .expect("get_invoice");
        assert_eq!(invoice.invoice.status, InvoiceStatus::Cancelled);
    }
}
