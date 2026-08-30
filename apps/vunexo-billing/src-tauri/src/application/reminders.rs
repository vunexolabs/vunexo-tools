//! Payment reminder use case. application-architecture-v2.md §3/§4c
//! ("Reminders"). Deliberately the smallest use case in the codebase: two
//! reads and a string format, no repository write, no transaction — Round 2
//! locked "no send-tracking," so there is nothing to persist.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::currency::{currency_meta, format_minor};
use crate::domain::invoice::InvoiceStatus;

use super::error::ApplicationError;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::payment_repository::PaymentRepository;
use super::ports::settings_repository::SettingsRepository;

/// user-flows-v2.md §6's template, with the five placeholders ui-ux-v2.md
/// §7 names. Used whenever `settings.payment_reminder_template` is `None` —
/// an application-code constant, not a row that has to exist first
/// (database-schema-v2.md §8).
pub const DEFAULT_REMINDER_TEMPLATE: &str = "Payment Reminder\n\
\n\
Invoice: {invoice_number}\n\
Amount Due: {amount_due}\n\
Due Date: {due_date}\n\
\n\
Hi {customer_name},\n\
\n\
This is a friendly reminder that invoice {invoice_number} for {amount_due} is currently overdue (due {due_date}). Please arrange payment at your earliest convenience.\n\
\n\
Thank you,\n\
{business_name}";

pub struct ReminderUseCases {
    invoice_repo: Arc<dyn InvoiceRepository>,
    payment_repo: Arc<dyn PaymentRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
}

impl ReminderUseCases {
    pub fn new(
        invoice_repo: Arc<dyn InvoiceRepository>,
        payment_repo: Arc<dyn PaymentRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
    ) -> Self {
        Self {
            invoice_repo,
            payment_repo,
            settings_repo,
        }
    }

    /// Precondition: the invoice is overdue, using the exact same predicate
    /// `database-schema.md` §8 already defines for the dashboard/list
    /// `is_overdue` badge — not a second, subtly different definition.
    pub async fn generate_reminder_message(
        &self,
        invoice_id: i64,
    ) -> Result<String, ApplicationError> {
        let invoice = self
            .invoice_repo
            .get(invoice_id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id: invoice_id,
            })?
            .invoice;

        let payments = self.payment_repo.list_for_invoice(invoice_id).await?;
        let amount_paid_minor: i64 = payments.iter().map(|p| p.amount_minor).sum();
        let amount_due_minor = invoice.total_minor - amount_paid_minor;

        let today = Utc::now().date_naive();
        let is_overdue = invoice.due_date.is_some_and(|d| d < today)
            && !matches!(
                invoice.status,
                InvoiceStatus::Draft | InvoiceStatus::Cancelled
            )
            && amount_due_minor > 0;
        if !is_overdue {
            return Err(ApplicationError::Validation(
                "a payment reminder can only be generated for an overdue invoice".into(),
            ));
        }

        let settings = self.settings_repo.get().await?;
        let meta = currency_meta(&settings.currency_code);
        let amount_due = match meta.symbol {
            Some(symbol) => format!(
                "{symbol}{}",
                format_minor(amount_due_minor, meta.decimals, meta.indian_grouping)
            ),
            None => format!(
                "{} {}",
                settings.currency_code,
                format_minor(amount_due_minor, meta.decimals, meta.indian_grouping)
            ),
        };

        let template = settings
            .payment_reminder_template
            .as_deref()
            .unwrap_or(DEFAULT_REMINDER_TEMPLATE);

        Ok(template
            .replace(
                "{invoice_number}",
                invoice.invoice_number.as_deref().unwrap_or(""),
            )
            .replace("{amount_due}", &amount_due)
            .replace(
                "{due_date}",
                &invoice
                    .due_date
                    .map(|d| d.format("%d/%m/%Y").to_string())
                    .unwrap_or_default(),
            )
            .replace(
                "{customer_name}",
                invoice.customer_snapshot_name.as_deref().unwrap_or(""),
            )
            .replace(
                "{business_name}",
                invoice.business_snapshot_name.as_deref().unwrap_or(""),
            ))
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
    use crate::application::ports::payment_repository::PaymentRepository;
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
    use crate::infrastructure::database::sqlite_payment_repository::SqlitePaymentRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        reminders: ReminderUseCases,
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
            "vunexo_reminder_test_{}_{}.db",
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
            reminders: ReminderUseCases::new(
                invoice_repo.clone(),
                payment_repo,
                settings_repo.clone(),
            ),
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

    #[tokio::test]
    async fn generates_a_reminder_for_an_overdue_invoice() {
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
                name: "Overdue Customer".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();

        let today = chrono::Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: yesterday,
                due_date: Some(yesterday),
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
                    unit_price_minor: 150_000,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 0,
                }],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();

        let message = app
            .reminders
            .generate_reminder_message(issued.invoice.id)
            .await
            .expect("generate_reminder_message");

        assert!(message.contains(issued.invoice.invoice_number.as_deref().unwrap()));
        assert!(message.contains("Overdue Customer"));
        assert!(message.contains("Vunexo Test Co"));
        assert!(message.contains("₹1,500.00"));
    }

    #[tokio::test]
    async fn rejects_a_not_yet_due_invoice() {
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
                name: "Not Overdue Customer".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();

        let today = chrono::Utc::now().date_naive();
        let next_month = today + chrono::Duration::days(30);
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: today,
                due_date: Some(next_month),
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
                    unit_price_minor: 100_000,
                    line_discount_type: None,
                    line_discount_value: None,
                    tax_rate_id: None,
                    tax_rate_basis_points: 0,
                }],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();

        let result = app
            .reminders
            .generate_reminder_message(issued.invoice.id)
            .await;
        assert!(matches!(result, Err(ApplicationError::Validation(_))));
    }
}
