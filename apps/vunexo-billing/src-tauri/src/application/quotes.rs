//! Quote use cases. application-architecture-v2.md §3/§4c. Mirrors
//! `application/invoices.rs`'s shape; see that module for the pattern this
//! follows.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::business::Business;
use crate::domain::calculation::{
    self, InvoiceCalculationInput, InvoiceCalculationResult, LineItemInput as CalcLineItemInput,
};
use crate::domain::customer::Customer;
use crate::domain::invoice::{BusinessSnapshotFields, CustomerSnapshotFields, DraftInvoiceInput};
use crate::domain::invoice_line_item::LineItemInput;
use crate::domain::quote::{
    DraftQuoteInput, DraftQuoteToSave, IssueQuoteData, Quote, QuoteFilter, QuoteStatus,
    QuoteSummary, QuoteWithLineItems,
};
use crate::domain::quote_line_item::{QuoteLineItem, QuoteLineItemInput, QuoteLineItemToSave};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::customer_repository::CustomerRepository;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::quote_number_sequencer::QuoteNumberSequencer;
use super::ports::quote_repository::QuoteRepository;
use super::ports::settings_repository::SettingsRepository;
use super::ports::transaction::TransactionManager;

pub struct QuoteUseCases {
    quote_repo: Arc<dyn QuoteRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
    business_repo: Arc<dyn BusinessRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
    sequencer: Arc<dyn QuoteNumberSequencer>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl QuoteUseCases {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        quote_repo: Arc<dyn QuoteRepository>,
        invoice_repo: Arc<dyn InvoiceRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
        business_repo: Arc<dyn BusinessRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
        sequencer: Arc<dyn QuoteNumberSequencer>,
        tx_manager: Arc<dyn TransactionManager>,
    ) -> Self {
        Self {
            quote_repo,
            invoice_repo,
            customer_repo,
            business_repo,
            settings_repo,
            sequencer,
            tx_manager,
        }
    }

    pub async fn create_draft_quote(
        &self,
        input: DraftQuoteInput,
    ) -> Result<QuoteWithLineItems, ApplicationError> {
        let calc = calculation::calculate_invoice(&to_calc_input(&input));
        let to_save = assemble_draft_to_save(input, &calc);

        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.create_draft(&mut *tx, to_save).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// user-flows-v2.md §2: a Quote is editable in `Draft` only — no
    /// `EditIssued`-equivalent exists, unlike invoices.
    pub async fn update_draft_quote(
        &self,
        id: i64,
        input: DraftQuoteInput,
    ) -> Result<QuoteWithLineItems, ApplicationError> {
        let existing = self
            .quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })?;
        if existing.quote.status != QuoteStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft quote can be edited".into(),
            ));
        }

        let calc = calculation::calculate_invoice(&to_calc_input(&input));
        let to_save = assemble_draft_to_save(input, &calc);

        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.update_draft(&mut *tx, id, to_save).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// Mirrors `InvoiceUseCases::issue_invoice` (application-architecture.md
    /// §4c) on the quote counter instead of the invoice one.
    pub async fn issue_quote(&self, id: i64) -> Result<QuoteWithLineItems, ApplicationError> {
        let existing = self
            .quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })?;
        if existing.quote.status != QuoteStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft quote can be issued".into(),
            ));
        }
        let customer_id = existing.quote.customer_id.ok_or_else(|| {
            ApplicationError::Validation("a customer is required to issue a quote".into())
        })?;
        if existing.line_items.is_empty() {
            return Err(ApplicationError::Validation(
                "at least one line item is required to issue a quote".into(),
            ));
        }

        let customer: Customer = self.customer_repo.find_by_id(customer_id).await?.ok_or(
            ApplicationError::NotFound {
                entity: "customer",
                id: customer_id,
            },
        )?;
        let business: Business = self.business_repo.get().await?.ok_or_else(|| {
            ApplicationError::Validation(
                "a business profile must exist before issuing quotes".into(),
            )
        })?;
        let settings = self.settings_repo.get().await?;

        let calc = calculation::calculate_invoice(&quote_line_items_to_calc_input(
            &existing.line_items,
            existing.quote.discount_type,
            existing.quote.discount_value,
        ));
        let line_items_to_save =
            assemble_line_items_to_save_from_existing(&existing.line_items, &calc);

        let mut tx = self.tx_manager.begin().await?;

        let today = Utc::now().date_naive();
        let quote_number = match self
            .sequencer
            .issue_next(&mut *tx, &settings.quote_number_format, today)
            .await
        {
            Ok(n) => n,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e.into());
            }
        };

        let data = IssueQuoteData {
            quote_number,
            customer_snapshot: CustomerSnapshotFields {
                name: Some(customer.name),
                phone: customer.phone,
                email: customer.email,
                address: customer.address,
                gstin: customer.gstin,
            },
            tax_regime_snapshot: business.tax_regime_code,
            business_snapshot: BusinessSnapshotFields {
                name: Some(business.name),
                address: business.address,
                gstin: business.gstin,
                phone: business.phone,
                email: business.email,
                bank_details: business.bank_details,
                upi_id: business.upi_id,
                logo_path: business.logo_path,
            },
            subtotal_minor: calc.subtotal_minor,
            discount_amount_minor: calc.discount_amount_minor,
            tax_amount_minor: calc.tax_amount_minor,
            total_minor: calc.total_minor,
            line_items: line_items_to_save,
        };

        match self.quote_repo.issue(&mut *tx, id, data).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn accept_quote(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self.get_quote(id).await?;
        if existing.status != QuoteStatus::Issued {
            return Err(ApplicationError::Validation(
                "only an issued quote can be accepted".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.accept(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn decline_quote(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self.get_quote(id).await?;
        if existing.status != QuoteStatus::Issued {
            return Err(ApplicationError::Validation(
                "only an issued quote can be declined".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.decline(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// `ACCEPTED` quotes may also be cancelled, not only `DRAFT`/`ISSUED` —
    /// user-flows-v2.md §2's explicit fix: acceptance means the customer
    /// agreed to the price, not that the job is guaranteed to happen.
    pub async fn cancel_quote(
        &self,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), ApplicationError> {
        let existing = self.get_quote(id).await?;
        if !matches!(
            existing.status,
            QuoteStatus::Draft | QuoteStatus::Issued | QuoteStatus::Accepted
        ) {
            return Err(ApplicationError::Validation(
                "only a draft, issued, or accepted quote can be cancelled".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.cancel(&mut *tx, id, reason).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn delete_draft_quote(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self.get_quote(id).await?;
        if existing.status != QuoteStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft quote can be deleted".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.quote_repo.delete_draft(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// application-architecture-v2.md §4c, transcribed exactly: the one V2
    /// use case that writes to two different aggregate tables in a single
    /// transaction. The two states this must never produce: `Converted` with
    /// no invoice row (orphaned), or `Accepted` with an invoice already
    /// pointing at it (half-applied). Does *not* call the invoice number
    /// sequencer — the resulting invoice stays `Draft`; it gets numbered
    /// later, at its own separate `IssueInvoice`.
    pub async fn convert_quote_to_invoice(
        &self,
        id: i64,
    ) -> Result<crate::domain::invoice::InvoiceWithLineItems, ApplicationError> {
        let existing = self
            .quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })?;
        if existing.quote.status != QuoteStatus::Accepted {
            return Err(ApplicationError::Conflict(
                "only an accepted quote can be converted to an invoice".into(),
            ));
        }

        let settings = self.settings_repo.get().await?;
        let invoice_date = Utc::now().date_naive();
        let due_date = Some(invoice_date + chrono::Duration::days(settings.default_due_days));

        let draft_input = DraftInvoiceInput {
            customer_id: existing.quote.customer_id,
            invoice_date,
            due_date,
            notes: existing.quote.notes.clone(),
            terms: existing.quote.terms.clone(),
            is_interstate: existing.quote.is_interstate,
            discount_type: existing.quote.discount_type,
            discount_value: existing.quote.discount_value,
            line_items: existing
                .line_items
                .iter()
                .map(|li| LineItemInput {
                    product_id: li.product_id,
                    description: li.description.clone(),
                    unit: li.unit.clone(),
                    quantity_thousandths: li.quantity_thousandths,
                    unit_price_minor: li.unit_price_minor,
                    line_discount_type: li.line_discount_type,
                    line_discount_value: li.line_discount_value,
                    tax_rate_id: li.tax_rate_id,
                    tax_rate_basis_points: li.tax_rate_basis_points,
                })
                .collect(),
        };
        let calc = calculation::calculate_invoice(&InvoiceCalculationInput {
            line_items: draft_input
                .line_items
                .iter()
                .map(|li| CalcLineItemInput {
                    quantity_thousandths: li.quantity_thousandths,
                    unit_price_minor: li.unit_price_minor,
                    tax_rate_basis_points: li.tax_rate_basis_points,
                    line_discount: li.line_discount_type.zip(li.line_discount_value),
                })
                .collect(),
            invoice_discount: draft_input.discount_type.zip(draft_input.discount_value),
        });
        let draft_to_save = super::invoices::assemble_draft_to_save(draft_input, &calc);

        let mut tx = self.tx_manager.begin().await?;
        let invoice = match self
            .invoice_repo
            .create_draft_from_quote(&mut *tx, id, draft_to_save)
            .await
        {
            Ok(invoice) => invoice,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e.into());
            }
        };
        match self.quote_repo.mark_converted(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(invoice)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// user-flows-v2.md §2 — copies customer, line items, discount, notes,
    /// terms into a new `DRAFT` Quote; never copies `accepted_at`/
    /// `converted_at` or the quote number.
    pub async fn duplicate_quote(&self, id: i64) -> Result<QuoteWithLineItems, ApplicationError> {
        let source = self
            .quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })?;
        let settings = self.settings_repo.get().await?;
        let quote_date = Utc::now().date_naive();
        let valid_until = Some(quote_date + chrono::Duration::days(settings.default_due_days));

        let draft_input = DraftQuoteInput {
            customer_id: source.quote.customer_id,
            quote_date,
            valid_until,
            notes: source.quote.notes.clone(),
            terms: source.quote.terms.clone(),
            is_interstate: source.quote.is_interstate,
            discount_type: source.quote.discount_type,
            discount_value: source.quote.discount_value,
            line_items: source
                .line_items
                .iter()
                .map(|li| QuoteLineItemInput {
                    product_id: li.product_id,
                    description: li.description.clone(),
                    unit: li.unit.clone(),
                    quantity_thousandths: li.quantity_thousandths,
                    unit_price_minor: li.unit_price_minor,
                    line_discount_type: li.line_discount_type,
                    line_discount_value: li.line_discount_value,
                    tax_rate_id: li.tax_rate_id,
                    tax_rate_basis_points: li.tax_rate_basis_points,
                })
                .collect(),
        };

        self.create_draft_quote(draft_input).await
    }

    pub async fn preview_next_quote_number(&self) -> Result<String, ApplicationError> {
        let settings = self.settings_repo.get().await?;
        let today = Utc::now().date_naive();
        Ok(self
            .sequencer
            .preview_next(&settings.quote_number_format, today)
            .await?)
    }

    pub async fn get_quote(&self, id: i64) -> Result<Quote, ApplicationError> {
        Ok(self
            .quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })?
            .quote)
    }

    pub async fn get_quote_with_line_items(
        &self,
        id: i64,
    ) -> Result<QuoteWithLineItems, ApplicationError> {
        self.quote_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "quote",
                id,
            })
    }

    pub async fn list_quotes(
        &self,
        filter: QuoteFilter,
    ) -> Result<Vec<QuoteSummary>, ApplicationError> {
        Ok(self.quote_repo.list(filter).await?)
    }
}

fn to_calc_input(draft: &DraftQuoteInput) -> InvoiceCalculationInput {
    InvoiceCalculationInput {
        line_items: draft
            .line_items
            .iter()
            .map(|li| CalcLineItemInput {
                quantity_thousandths: li.quantity_thousandths,
                unit_price_minor: li.unit_price_minor,
                tax_rate_basis_points: li.tax_rate_basis_points,
                line_discount: li.line_discount_type.zip(li.line_discount_value),
            })
            .collect(),
        invoice_discount: draft.discount_type.zip(draft.discount_value),
    }
}

fn quote_line_items_to_calc_input(
    line_items: &[QuoteLineItem],
    discount_type: Option<crate::domain::invoice::DiscountType>,
    discount_value: Option<i64>,
) -> InvoiceCalculationInput {
    InvoiceCalculationInput {
        line_items: line_items
            .iter()
            .map(|li| CalcLineItemInput {
                quantity_thousandths: li.quantity_thousandths,
                unit_price_minor: li.unit_price_minor,
                tax_rate_basis_points: li.tax_rate_basis_points,
                line_discount: li.line_discount_type.zip(li.line_discount_value),
            })
            .collect(),
        invoice_discount: discount_type.zip(discount_value),
    }
}

fn assemble_draft_to_save(
    draft: DraftQuoteInput,
    calc: &InvoiceCalculationResult,
) -> DraftQuoteToSave {
    let line_items = draft
        .line_items
        .into_iter()
        .zip(calc.lines.iter())
        .enumerate()
        .map(|(i, (input, result))| QuoteLineItemToSave {
            product_id: input.product_id,
            description: input.description,
            unit: input.unit,
            quantity_thousandths: input.quantity_thousandths,
            unit_price_minor: input.unit_price_minor,
            line_discount_type: input.line_discount_type,
            line_discount_value: input.line_discount_value,
            tax_rate_id: input.tax_rate_id,
            tax_rate_basis_points: input.tax_rate_basis_points,
            line_subtotal_minor: result.line_subtotal_minor,
            line_discount_amount_minor: result.line_discount_amount_minor,
            quote_discount_amount_minor: result.invoice_discount_amount_minor,
            taxable_amount_minor: result.taxable_amount_minor,
            line_tax_minor: result.line_tax_minor,
            line_total_minor: result.line_total_minor,
            sort_order: i as i64,
        })
        .collect();

    DraftQuoteToSave {
        customer_id: draft.customer_id,
        quote_date: draft.quote_date,
        valid_until: draft.valid_until,
        notes: draft.notes,
        terms: draft.terms,
        is_interstate: draft.is_interstate,
        discount_type: draft.discount_type,
        discount_value: draft.discount_value,
        subtotal_minor: calc.subtotal_minor,
        discount_amount_minor: calc.discount_amount_minor,
        tax_amount_minor: calc.tax_amount_minor,
        total_minor: calc.total_minor,
        line_items,
    }
}

fn assemble_line_items_to_save_from_existing(
    line_items: &[QuoteLineItem],
    calc: &InvoiceCalculationResult,
) -> Vec<QuoteLineItemToSave> {
    line_items
        .iter()
        .zip(calc.lines.iter())
        .enumerate()
        .map(|(i, (li, result))| QuoteLineItemToSave {
            product_id: li.product_id,
            description: li.description.clone(),
            unit: li.unit.clone(),
            quantity_thousandths: li.quantity_thousandths,
            unit_price_minor: li.unit_price_minor,
            line_discount_type: li.line_discount_type,
            line_discount_value: li.line_discount_value,
            tax_rate_id: li.tax_rate_id,
            tax_rate_basis_points: li.tax_rate_basis_points,
            line_subtotal_minor: result.line_subtotal_minor,
            line_discount_amount_minor: result.line_discount_amount_minor,
            quote_discount_amount_minor: result.invoice_discount_amount_minor,
            taxable_amount_minor: result.taxable_amount_minor,
            line_tax_minor: result.line_tax_minor,
            line_total_minor: result.line_total_minor,
            sort_order: i as i64,
        })
        .collect()
}

#[cfg(test)]
mod integration_tests {
    //! Exercises the whole stack against a real SQLite file, mirroring
    //! `application::invoices`'s integration test pattern — this is what
    //! actually proves `ConvertQuoteToInvoice`'s transaction (§4c) holds,
    //! not just that it compiles.
    use std::sync::Arc;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::invoices::InvoiceUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::invoice_repository::InvoiceRepository;
    use crate::application::ports::quote_number_sequencer::QuoteNumberSequencer;
    use crate::application::ports::quote_repository::QuoteRepository;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::DiscountType;
    use crate::domain::tax_regime::TaxRegimeCode;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::sqlite_invoice_number_sequencer::SqliteInvoiceNumberSequencer;
    use crate::infrastructure::database::sqlite_invoice_repository::SqliteInvoiceRepository;
    use crate::infrastructure::database::sqlite_quote_number_sequencer::SqliteQuoteNumberSequencer;
    use crate::infrastructure::database::sqlite_quote_repository::SqliteQuoteRepository;
    use crate::infrastructure::database::sqlite_settings_repository::SqliteSettingsRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        quotes: QuoteUseCases,
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
        let db_path =
            std::env::temp_dir().join(format!("vunexo_quote_test_{}_{}.db", std::process::id(), n));
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
        let quote_repo: Arc<dyn QuoteRepository> =
            Arc::new(SqliteQuoteRepository::new(pool.clone()));
        let invoice_sequencer: Arc<dyn InvoiceNumberSequencer> =
            Arc::new(SqliteInvoiceNumberSequencer::new(pool.clone()));
        let quote_sequencer: Arc<dyn QuoteNumberSequencer> =
            Arc::new(SqliteQuoteNumberSequencer::new(pool));

        TestApp {
            quotes: QuoteUseCases::new(
                quote_repo,
                invoice_repo.clone(),
                customer_repo.clone(),
                business_repo.clone(),
                settings_repo.clone(),
                quote_sequencer,
                tx_manager.clone(),
            ),
            invoices: InvoiceUseCases::new(
                invoice_repo,
                customer_repo.clone(),
                business_repo.clone(),
                settings_repo,
                invoice_sequencer,
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

    fn sample_line(qty_thousandths: i64, unit_price_minor: i64, tax_bp: i64) -> QuoteLineItemInput {
        QuoteLineItemInput {
            product_id: None,
            description: "Consulting".into(),
            unit: "hr".into(),
            quantity_thousandths: qty_thousandths,
            unit_price_minor,
            line_discount_type: None,
            line_discount_value: None,
            tax_rate_id: None,
            tax_rate_basis_points: tax_bp,
        }
    }

    async fn setup_business_and_customer(app: &TestApp) -> i64 {
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
                name: "Acme Traders".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();
        customer.id
    }

    #[tokio::test]
    async fn full_quote_lifecycle_issue_accept_convert() {
        let app = setup().await;
        let customer_id = setup_business_and_customer(&app).await;

        let draft = app
            .quotes
            .create_draft_quote(DraftQuoteInput {
                customer_id: Some(customer_id),
                quote_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                valid_until: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![sample_line(2000, 100_000, 1800)],
            })
            .await
            .expect("create_draft_quote");
        assert_eq!(draft.quote.status, QuoteStatus::Draft);
        assert_eq!(
            draft.quote.total_minor, 236_000,
            "matches Vector 1's totals"
        );

        let issued = app
            .quotes
            .issue_quote(draft.quote.id)
            .await
            .expect("issue_quote");
        assert_eq!(issued.quote.status, QuoteStatus::Issued);
        assert_eq!(issued.quote.quote_number.as_deref(), Some("QUO-2026-0001"));
        assert_eq!(issued.quote.tax_regime_snapshot, Some(TaxRegimeCode::InGst));

        app.quotes
            .accept_quote(issued.quote.id)
            .await
            .expect("accept_quote");
        let accepted = app.quotes.get_quote(issued.quote.id).await.unwrap();
        assert_eq!(accepted.status, QuoteStatus::Accepted);
        assert!(accepted.accepted_at.is_some());

        let invoice = app
            .quotes
            .convert_quote_to_invoice(issued.quote.id)
            .await
            .expect("convert_quote_to_invoice");
        assert_eq!(
            invoice.invoice.status,
            crate::domain::invoice::InvoiceStatus::Draft
        );
        assert_eq!(invoice.invoice.source_quote_id, Some(issued.quote.id));
        assert_eq!(invoice.invoice.total_minor, 236_000);
        assert!(
            invoice.invoice.invoice_number.is_none(),
            "converted invoice stays Draft — numbering happens at its own Issue"
        );

        let converted_quote = app.quotes.get_quote(issued.quote.id).await.unwrap();
        assert_eq!(converted_quote.status, QuoteStatus::Converted);
        assert!(converted_quote.converted_at.is_some());
        // The quote keeps its own number permanently — conversion doesn't touch it.
        assert_eq!(
            converted_quote.quote_number.as_deref(),
            Some("QUO-2026-0001")
        );
    }

    /// application-architecture-v2.md §4c / §8 row 1 — the two forbidden
    /// states this atomicity contract exists to prevent: `Converted` with no
    /// invoice, or `Accepted` with an invoice already pointing at it.
    #[tokio::test]
    async fn convert_quote_to_invoice_is_exactly_once() {
        let app = setup().await;
        let customer_id = setup_business_and_customer(&app).await;

        let draft = app
            .quotes
            .create_draft_quote(DraftQuoteInput {
                customer_id: Some(customer_id),
                quote_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                valid_until: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![sample_line(1000, 100_000, 0)],
            })
            .await
            .unwrap();
        let issued = app.quotes.issue_quote(draft.quote.id).await.unwrap();
        app.quotes.accept_quote(issued.quote.id).await.unwrap();

        let first = app
            .quotes
            .convert_quote_to_invoice(issued.quote.id)
            .await
            .expect("first conversion succeeds");
        assert!(first.invoice.source_quote_id.is_some());

        // Second attempt: the quote is now Converted, not Accepted — the
        // precondition check rejects it before any write is attempted.
        let second = app.quotes.convert_quote_to_invoice(issued.quote.id).await;
        assert!(
            matches!(second, Err(ApplicationError::Conflict(_))),
            "converting an already-converted quote must be rejected, not silently produce a second invoice"
        );

        // Exactly one invoice exists with this source_quote_id — proves the
        // DB-level partial unique index (database-schema-v2.md §9) holds,
        // not just the use-case-level precondition check.
        let all_invoices = app
            .invoices
            .list_invoices(crate::domain::invoice::InvoiceFilter::default())
            .await
            .unwrap();
        let matching = all_invoices
            .iter()
            .filter(|i| i.id == first.invoice.id)
            .count();
        assert_eq!(matching, 1);
    }

    #[tokio::test]
    async fn accepted_quote_can_be_cancelled() {
        let app = setup().await;
        let customer_id = setup_business_and_customer(&app).await;

        let draft = app
            .quotes
            .create_draft_quote(DraftQuoteInput {
                customer_id: Some(customer_id),
                quote_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                valid_until: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![sample_line(1000, 100_000, 0)],
            })
            .await
            .unwrap();
        let issued = app.quotes.issue_quote(draft.quote.id).await.unwrap();
        app.quotes.accept_quote(issued.quote.id).await.unwrap();

        app.quotes
            .cancel_quote(issued.quote.id, Some("customer backed out".into()))
            .await
            .expect("an accepted quote must be cancellable");
        let cancelled = app.quotes.get_quote(issued.quote.id).await.unwrap();
        assert_eq!(cancelled.status, QuoteStatus::Cancelled);
        assert_eq!(
            cancelled.cancel_reason.as_deref(),
            Some("customer backed out")
        );
    }

    #[tokio::test]
    async fn quote_and_invoice_numbering_sequences_are_independent() {
        let app = setup().await;
        let customer_id = setup_business_and_customer(&app).await;

        // Issue one invoice directly (no quote involved).
        let invoice_draft = app
            .invoices
            .create_draft_invoice(crate::domain::invoice::DraftInvoiceInput {
                customer_id: Some(customer_id),
                invoice_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                due_date: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![crate::domain::invoice_line_item::LineItemInput {
                    product_id: None,
                    description: "Consulting".into(),
                    unit: "hr".into(),
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
        let issued_invoice = app
            .invoices
            .issue_invoice(invoice_draft.invoice.id, None)
            .await
            .unwrap();
        assert_eq!(
            issued_invoice.invoice.invoice_number.as_deref(),
            Some("INV-2026-0001")
        );

        // Issue a quote — its number starts from 1, independent of the
        // invoice sequence above already being at 1.
        let quote_draft = app
            .quotes
            .create_draft_quote(DraftQuoteInput {
                customer_id: Some(customer_id),
                quote_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                valid_until: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: None,
                discount_value: None,
                line_items: vec![sample_line(1000, 100_000, 0)],
            })
            .await
            .unwrap();
        let issued_quote = app.quotes.issue_quote(quote_draft.quote.id).await.unwrap();
        assert_eq!(
            issued_quote.quote.quote_number.as_deref(),
            Some("QUO-2026-0001")
        );
    }

    #[tokio::test]
    async fn only_draft_quotes_can_be_edited() {
        let app = setup().await;
        let customer_id = setup_business_and_customer(&app).await;

        let draft = app
            .quotes
            .create_draft_quote(DraftQuoteInput {
                customer_id: Some(customer_id),
                quote_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                valid_until: None,
                notes: None,
                terms: None,
                is_interstate: false,
                discount_type: Some(DiscountType::Amount),
                discount_value: Some(1000),
                line_items: vec![
                    sample_line(1000, 100_000, 0),
                    sample_line(1000, 100_000, 0),
                    sample_line(1000, 100_000, 0),
                ],
            })
            .await
            .unwrap();
        // Matches calculation-engine.md Vector 3's largest-remainder allocation.
        assert_eq!(
            draft
                .line_items
                .iter()
                .map(|l| l.quote_discount_amount_minor)
                .collect::<Vec<_>>(),
            vec![334, 333, 333]
        );

        let issued = app.quotes.issue_quote(draft.quote.id).await.unwrap();
        let edit_attempt = app
            .quotes
            .update_draft_quote(
                issued.quote.id,
                DraftQuoteInput {
                    customer_id: Some(customer_id),
                    quote_date: issued.quote.quote_date,
                    valid_until: None,
                    notes: None,
                    terms: None,
                    is_interstate: false,
                    discount_type: None,
                    discount_value: None,
                    line_items: vec![sample_line(1000, 100_000, 0)],
                },
            )
            .await;
        assert!(
            matches!(edit_attempt, Err(ApplicationError::Validation(_))),
            "an issued quote must not be editable — unlike an invoice"
        );
    }
}
