//! Invoice use cases. application-architecture.md §4/§4c. Payment-driven
//! `set_status` is implemented in `application/payments.rs`.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::business::Business;
use crate::domain::calculation::{
    self, InvoiceCalculationInput, InvoiceCalculationResult, LineItemInput as CalcLineItemInput,
};
use crate::domain::customer::Customer;
use crate::domain::invoice::{
    BusinessSnapshotFields, CustomerSnapshotFields, DiscountType, DraftInvoiceInput,
    DraftInvoiceToSave, EditIssuedInvoiceData, InvoiceFilter, InvoiceStatus, InvoiceSummary,
    InvoiceWithLineItems, IssueInvoiceData,
};
use crate::domain::invoice_line_item::{InvoiceLineItem, LineItemInput, LineItemToSave};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::customer_repository::CustomerRepository;
use super::ports::invoice_number_sequencer::InvoiceNumberSequencer;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::settings_repository::SettingsRepository;
use super::ports::transaction::TransactionManager;

pub struct InvoiceUseCases {
    invoice_repo: Arc<dyn InvoiceRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
    business_repo: Arc<dyn BusinessRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
    sequencer: Arc<dyn InvoiceNumberSequencer>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl InvoiceUseCases {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        invoice_repo: Arc<dyn InvoiceRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
        business_repo: Arc<dyn BusinessRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
        sequencer: Arc<dyn InvoiceNumberSequencer>,
        tx_manager: Arc<dyn TransactionManager>,
    ) -> Self {
        Self {
            invoice_repo,
            customer_repo,
            business_repo,
            settings_repo,
            sequencer,
            tx_manager,
        }
    }

    pub async fn create_draft_invoice(
        &self,
        input: DraftInvoiceInput,
    ) -> Result<InvoiceWithLineItems, ApplicationError> {
        let calc = calculation::calculate_invoice(&to_calc_input(&input));
        let to_save = assemble_draft_to_save(input, &calc);

        let mut tx = self.tx_manager.begin().await?;
        match self.invoice_repo.create_draft(&mut *tx, to_save).await {
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

    pub async fn update_draft_invoice(
        &self,
        id: i64,
        input: DraftInvoiceInput,
    ) -> Result<InvoiceWithLineItems, ApplicationError> {
        let existing = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        if existing.invoice.status != InvoiceStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft invoice can be edited this way".into(),
            ));
        }

        let calc = calculation::calculate_invoice(&to_calc_input(&input));
        let to_save = assemble_draft_to_save(input, &calc);

        let mut tx = self.tx_manager.begin().await?;
        match self.invoice_repo.update_draft(&mut *tx, id, to_save).await {
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

    /// application-architecture.md §4c, transcribed: validate -> load
    /// customer + business -> issue_next (same tx) -> calculate -> issue
    /// (same tx). Any failure rolls back everything, including the counter
    /// increment — a failed issue never burns a number.
    pub async fn issue_invoice(
        &self,
        id: i64,
        custom_number: Option<String>,
    ) -> Result<InvoiceWithLineItems, ApplicationError> {
        let existing = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        if existing.invoice.status != InvoiceStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft invoice can be issued".into(),
            ));
        }
        let customer_id = existing.invoice.customer_id.ok_or_else(|| {
            ApplicationError::Validation("a customer is required to issue an invoice".into())
        })?;
        if existing.line_items.is_empty() {
            return Err(ApplicationError::Validation(
                "at least one line item is required to issue an invoice".into(),
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
                "a business profile must exist before issuing invoices".into(),
            )
        })?;
        let settings = self.settings_repo.get().await?;

        let calc = calculation::calculate_invoice(&invoice_line_items_to_calc_input(
            &existing.line_items,
            existing.invoice.discount_type,
            existing.invoice.discount_value,
        ));
        let line_items_to_save =
            assemble_line_items_to_save_from_existing(&existing.line_items, &calc);

        let mut tx = self.tx_manager.begin().await?;

        let (invoice_number, invoice_number_is_custom) = match custom_number {
            Some(n) if !n.trim().is_empty() => (n, true),
            _ => {
                let today = Utc::now().date_naive();
                match self
                    .sequencer
                    .issue_next(&mut *tx, &settings.invoice_number_format, today)
                    .await
                {
                    Ok(n) => (n, false),
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err(e.into());
                    }
                }
            }
        };

        let data = IssueInvoiceData {
            invoice_number,
            invoice_number_is_custom,
            customer_snapshot: CustomerSnapshotFields {
                name: Some(customer.name),
                phone: customer.phone,
                email: customer.email,
                address: customer.address,
                gstin: customer.gstin,
            },
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

        match self.invoice_repo.issue(&mut *tx, id, data).await {
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

    /// user-flows.md's "Editing an issued invoice" rule: allowed on
    /// `Issued`/`PartiallyPaid`/`Paid`, re-snapshots customer/business at
    /// *this* save (re-reading their current live data — an explicit,
    /// intentional edit of this invoice, unlike a customer record silently
    /// changing elsewhere, which must never touch old invoices), recomputes
    /// totals, never touches `payments` or `status`. If the new total drops
    /// below what's already been paid, the invoice simply stays whatever
    /// status it already was — the UI surfaces that as an overpayment, this
    /// use case never adjusts a payment record to compensate.
    pub async fn edit_issued_invoice(
        &self,
        id: i64,
        input: DraftInvoiceInput,
    ) -> Result<InvoiceWithLineItems, ApplicationError> {
        let existing = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        if !matches!(
            existing.invoice.status,
            InvoiceStatus::Issued | InvoiceStatus::PartiallyPaid | InvoiceStatus::Paid
        ) {
            return Err(ApplicationError::Validation(
                "only an issued, partially paid, or paid invoice can be edited this way".into(),
            ));
        }
        let customer_id = input.customer_id.ok_or_else(|| {
            ApplicationError::Validation("a customer is required on an issued invoice".into())
        })?;
        if input.line_items.is_empty() {
            return Err(ApplicationError::Validation(
                "at least one line item is required on an issued invoice".into(),
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
                "a business profile must exist before editing invoices".into(),
            )
        })?;

        let calc = calculation::calculate_invoice(&to_calc_input(&input));
        let draft_to_save = assemble_draft_to_save(input, &calc);
        let data = EditIssuedInvoiceData {
            customer_id: draft_to_save.customer_id,
            customer_snapshot: CustomerSnapshotFields {
                name: Some(customer.name),
                phone: customer.phone,
                email: customer.email,
                address: customer.address,
                gstin: customer.gstin,
            },
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
            is_interstate: draft_to_save.is_interstate,
            invoice_date: draft_to_save.invoice_date,
            due_date: draft_to_save.due_date,
            notes: draft_to_save.notes,
            terms: draft_to_save.terms,
            discount_type: draft_to_save.discount_type,
            discount_value: draft_to_save.discount_value,
            subtotal_minor: draft_to_save.subtotal_minor,
            discount_amount_minor: draft_to_save.discount_amount_minor,
            tax_amount_minor: draft_to_save.tax_amount_minor,
            total_minor: draft_to_save.total_minor,
            line_items: draft_to_save.line_items,
        };

        let mut tx = self.tx_manager.begin().await?;
        match self.invoice_repo.update_issued(&mut *tx, id, data).await {
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

    pub async fn cancel_invoice(
        &self,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), ApplicationError> {
        let existing = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        if !matches!(
            existing.invoice.status,
            InvoiceStatus::Issued | InvoiceStatus::PartiallyPaid | InvoiceStatus::Paid
        ) {
            return Err(ApplicationError::Validation(
                "only an issued, partially paid, or paid invoice can be cancelled".into(),
            ));
        }

        let mut tx = self.tx_manager.begin().await?;
        match self.invoice_repo.cancel(&mut *tx, id, reason).await {
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

    pub async fn delete_draft_invoice(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        if existing.invoice.status != InvoiceStatus::Draft {
            return Err(ApplicationError::Validation(
                "only a draft invoice can be deleted".into(),
            ));
        }

        let mut tx = self.tx_manager.begin().await?;
        match self.invoice_repo.delete_draft(&mut *tx, id).await {
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

    /// user-flows.md §5 — copies customer, line items, discount, notes,
    /// terms into a new `DRAFT`; never copies payments, status, or the
    /// invoice number.
    pub async fn duplicate_invoice(
        &self,
        id: i64,
    ) -> Result<InvoiceWithLineItems, ApplicationError> {
        let source = self
            .invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })?;
        let settings = self.settings_repo.get().await?;
        let invoice_date = Utc::now().date_naive();
        let due_date = Some(invoice_date + chrono::Duration::days(settings.default_due_days));

        let draft_input = DraftInvoiceInput {
            customer_id: source.invoice.customer_id,
            invoice_date,
            due_date,
            notes: source.invoice.notes.clone(),
            terms: source.invoice.terms.clone(),
            is_interstate: source.invoice.is_interstate,
            discount_type: source.invoice.discount_type,
            discount_value: source.invoice.discount_value,
            line_items: source
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

        self.create_draft_invoice(draft_input).await
    }

    /// ui-ux.md §4 — "Next invoice number • automatic": read-only, never
    /// reserves a number (database-schema.md §7).
    pub async fn preview_next_invoice_number(&self) -> Result<String, ApplicationError> {
        let settings = self.settings_repo.get().await?;
        let today = Utc::now().date_naive();
        Ok(self
            .sequencer
            .preview_next(&settings.invoice_number_format, today)
            .await?)
    }

    pub async fn get_invoice(&self, id: i64) -> Result<InvoiceWithLineItems, ApplicationError> {
        self.invoice_repo
            .get(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "invoice",
                id,
            })
    }

    pub async fn list_invoices(
        &self,
        filter: InvoiceFilter,
    ) -> Result<Vec<InvoiceSummary>, ApplicationError> {
        Ok(self.invoice_repo.list(filter).await?)
    }
}

fn to_calc_input(draft: &DraftInvoiceInput) -> InvoiceCalculationInput {
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

fn invoice_line_items_to_calc_input(
    line_items: &[InvoiceLineItem],
    discount_type: Option<DiscountType>,
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
    draft: DraftInvoiceInput,
    calc: &InvoiceCalculationResult,
) -> DraftInvoiceToSave {
    let line_items = draft
        .line_items
        .into_iter()
        .zip(calc.lines.iter())
        .enumerate()
        .map(|(i, (input, result))| LineItemToSave {
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
            invoice_discount_amount_minor: result.invoice_discount_amount_minor,
            taxable_amount_minor: result.taxable_amount_minor,
            line_tax_minor: result.line_tax_minor,
            line_total_minor: result.line_total_minor,
            sort_order: i as i64,
        })
        .collect();

    DraftInvoiceToSave {
        customer_id: draft.customer_id,
        invoice_date: draft.invoice_date,
        due_date: draft.due_date,
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
    line_items: &[InvoiceLineItem],
    calc: &InvoiceCalculationResult,
) -> Vec<LineItemToSave> {
    line_items
        .iter()
        .zip(calc.lines.iter())
        .enumerate()
        .map(|(i, (li, result))| LineItemToSave {
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
            invoice_discount_amount_minor: result.invoice_discount_amount_minor,
            taxable_amount_minor: result.taxable_amount_minor,
            line_tax_minor: result.line_tax_minor,
            line_total_minor: result.line_total_minor,
            sort_order: i as i64,
        })
        .collect()
}

#[cfg(test)]
mod integration_tests {
    //! Exercises the whole stack against a real SQLite file — repositories,
    //! transaction manager, and number sequencer included — not mocks. This
    //! is what actually proves `IssueInvoice`'s transaction (§4c) and the
    //! numbering guarantee work end to end, since none of that can be
    //! checked by compiling alone.
    use std::sync::Arc;

    use crate::application::business::BusinessUseCases;
    use crate::application::customers::CustomerUseCases;
    use crate::application::payments::PaymentUseCases;
    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::customer_repository::CustomerRepository;
    use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
    use crate::application::ports::invoice_repository::InvoiceRepository;
    use crate::application::ports::payment_repository::PaymentRepository;
    use crate::application::ports::settings_repository::SettingsRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::domain::customer::CustomerFields;
    use crate::domain::invoice::{DiscountType, DraftInvoiceInput, InvoiceStatus};
    use crate::domain::invoice_line_item::LineItemInput;
    use crate::domain::payment::{NewPayment, PaymentMethod};
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
        // An atomic counter, not just a timestamp: parallel #[tokio::test]s can call
        // setup() close enough together that clock resolution alone isn't a
        // reliable uniqueness guarantee, and two tests sharing one db file
        // corrupts both (this was observed in practice — see the commit this
        // comment shipped in).
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_invoice_test_{}_{}.db",
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

    fn sample_line(qty_thousandths: i64, unit_price_minor: i64, tax_bp: i64) -> LineItemInput {
        LineItemInput {
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

    #[tokio::test]
    async fn full_create_and_issue_flow_produces_correct_snapshot_and_totals() {
        let app = setup().await;

        let business = app
            .business
            .create_business(Business {
                name: "Vunexo Test Co".into(),
                logo_path: None,
                address: Some("221B Baker Street".into()),
                phone: None,
                email: None,
                gstin: Some("29ABCDE1234F1Z5".into()),
                bank_details: None,
                upi_id: None,
            })
            .await
            .expect("create_business");
        assert_eq!(business.name, "Vunexo Test Co");

        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Acme Traders".into(),
                phone: Some("9999999999".into()),
                email: None,
                address: Some("Old Address".into()),
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
                line_items: vec![sample_line(2000, 100_000, 1800)], // matches calculation-engine.md Vector 1
            })
            .await
            .expect("create_draft_invoice");
        assert_eq!(draft.invoice.status, InvoiceStatus::Draft);
        assert!(
            draft.invoice.invoice_number.is_none(),
            "draft must not have a number yet"
        );
        assert_eq!(
            draft.invoice.total_minor, 236_000,
            "matches Vector 1's ₹2,360 total"
        );

        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .expect("issue_invoice");
        assert_eq!(issued.invoice.status, InvoiceStatus::Issued);
        assert_eq!(
            issued.invoice.invoice_number.as_deref(),
            Some("INV-2026-0001")
        );
        assert!(!issued.invoice.invoice_number_is_custom);
        assert!(issued.invoice.issued_at.is_some());
        assert_eq!(issued.invoice.total_minor, 236_000);

        // The snapshot, not a live join — changing the customer afterward
        // must never affect this invoice (user-flows.md §3, database-schema.md §4).
        assert_eq!(
            issued.invoice.customer_snapshot_name.as_deref(),
            Some("Acme Traders")
        );
        assert_eq!(
            issued.invoice.customer_snapshot_address.as_deref(),
            Some("Old Address")
        );
        assert_eq!(
            issued.invoice.business_snapshot_name.as_deref(),
            Some("Vunexo Test Co")
        );
        assert_eq!(
            issued.invoice.business_snapshot_gstin.as_deref(),
            Some("29ABCDE1234F1Z5")
        );

        app.customers
            .update_customer(
                customer.id,
                CustomerFields {
                    name: "Acme Traders".into(),
                    phone: None,
                    email: None,
                    address: Some("New Address".into()),
                    gstin: None,
                },
            )
            .await
            .expect("update_customer");
        let reloaded = app
            .invoices
            .get_invoice(issued.invoice.id)
            .await
            .expect("get_invoice");
        assert_eq!(
            reloaded.invoice.customer_snapshot_address.as_deref(),
            Some("Old Address"),
            "snapshot must not change when the live customer record changes"
        );
    }

    #[tokio::test]
    async fn invoice_numbers_are_sequential_and_never_reused() {
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
                name: "Cust".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .expect("create_customer");

        let mut numbers = Vec::new();
        for _ in 0..3 {
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
                    line_items: vec![sample_line(1000, 10_000, 0)],
                })
                .await
                .unwrap();
            let issued = app
                .invoices
                .issue_invoice(draft.invoice.id, None)
                .await
                .unwrap();
            numbers.push(issued.invoice.invoice_number.unwrap());
        }
        assert_eq!(
            numbers,
            vec!["INV-2026-0001", "INV-2026-0002", "INV-2026-0003"]
        );
    }

    #[tokio::test]
    async fn cancel_invoice_sets_status_and_reason() {
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
            .unwrap();
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Cust".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();
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
                line_items: vec![sample_line(1000, 10_000, 0)],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();

        app.invoices
            .cancel_invoice(issued.invoice.id, Some("duplicate invoice".into()))
            .await
            .expect("cancel_invoice");
        let reloaded = app.invoices.get_invoice(issued.invoice.id).await.unwrap();
        assert_eq!(reloaded.invoice.status, InvoiceStatus::Cancelled);
        assert_eq!(
            reloaded.invoice.cancel_reason.as_deref(),
            Some("duplicate invoice")
        );
        assert!(reloaded.invoice.cancelled_at.is_some());
    }

    #[tokio::test]
    async fn draft_with_invoice_level_discount_matches_calculation_engine() {
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
            .unwrap();
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Cust".into(),
                phone: None,
                email: None,
                address: None,
                gstin: None,
            })
            .await
            .unwrap();

        // Three equal lines + a flat ₹10 discount == calculation-engine.md Vector 3.
        let draft = app
            .invoices
            .create_draft_invoice(DraftInvoiceInput {
                customer_id: Some(customer.id),
                invoice_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                due_date: None,
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

        assert_eq!(draft.invoice.subtotal_minor, 300_000);
        assert_eq!(draft.invoice.discount_amount_minor, 1000);
        assert_eq!(draft.invoice.total_minor, 299_000);
        assert_eq!(
            draft
                .line_items
                .iter()
                .map(|l| l.invoice_discount_amount_minor)
                .collect::<Vec<_>>(),
            vec![334, 333, 333]
        );
    }

    /// The inverse of `full_create_and_issue_flow_produces_correct_snapshot_and_totals`'s
    /// snapshot-immutability assertion: a bare `UpdateCustomer` must never
    /// leak into an old invoice, but `EditIssuedInvoice` — an explicit,
    /// intentional edit of *this* invoice — is specifically supposed to
    /// re-snapshot from whatever the customer/business look like right now
    /// (user-flows.md's "Editing an issued invoice" rule).
    #[tokio::test]
    async fn edit_issued_invoice_resnapshots_current_customer_and_business_data() {
        let app = setup().await;
        app.business
            .create_business(Business {
                name: "Vunexo Test Co".into(),
                logo_path: None,
                address: Some("Old Business Address".into()),
                phone: None,
                email: None,
                gstin: None,
                bank_details: None,
                upi_id: None,
            })
            .await
            .unwrap();
        let customer = app
            .customers
            .create_customer(CustomerFields {
                name: "Acme Traders".into(),
                phone: None,
                email: None,
                address: Some("Old Customer Address".into()),
                gstin: None,
            })
            .await
            .unwrap();
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
                line_items: vec![sample_line(1000, 100_000, 0)],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();
        assert_eq!(
            issued.invoice.customer_snapshot_address.as_deref(),
            Some("Old Customer Address")
        );

        // Both live records change *after* issue.
        app.customers
            .update_customer(
                customer.id,
                CustomerFields {
                    name: "Acme Traders".into(),
                    phone: None,
                    email: None,
                    address: Some("New Customer Address".into()),
                    gstin: None,
                },
            )
            .await
            .unwrap();
        app.business
            .update_business(Business {
                name: "Vunexo Test Co".into(),
                logo_path: None,
                address: Some("New Business Address".into()),
                phone: None,
                email: None,
                gstin: None,
                bank_details: None,
                upi_id: None,
            })
            .await
            .unwrap();

        let edited = app
            .invoices
            .edit_issued_invoice(
                issued.invoice.id,
                DraftInvoiceInput {
                    customer_id: Some(customer.id),
                    invoice_date: issued.invoice.invoice_date,
                    due_date: None,
                    notes: Some("fixed a typo".into()),
                    terms: None,
                    is_interstate: false,
                    discount_type: None,
                    discount_value: None,
                    line_items: vec![sample_line(2000, 100_000, 0)],
                },
            )
            .await
            .unwrap();

        assert_eq!(
            edited.invoice.customer_snapshot_address.as_deref(),
            Some("New Customer Address"),
            "an explicit edit must re-snapshot from the customer's current data"
        );
        assert_eq!(
            edited.invoice.business_snapshot_address.as_deref(),
            Some("New Business Address")
        );
        assert_eq!(
            edited.invoice.total_minor, 200_000,
            "recalculated for the new quantity"
        );
        assert_eq!(edited.invoice.notes.as_deref(), Some("fixed a typo"));
        // Never touched by an edit.
        assert_eq!(edited.invoice.invoice_number, issued.invoice.invoice_number);
        assert_eq!(edited.invoice.status, InvoiceStatus::Issued);
    }

    #[tokio::test]
    async fn edit_issued_invoice_leaves_status_and_payments_untouched_even_on_overpayment() {
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
                line_items: vec![sample_line(1000, 200_000, 0)],
            })
            .await
            .unwrap();
        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();
        assert_eq!(issued.invoice.total_minor, 200_000);

        app.payments
            .record_payment(NewPayment {
                invoice_id: issued.invoice.id,
                amount_minor: 200_000,
                method: PaymentMethod::Cash,
                paid_on: issued.invoice.invoice_date,
                reference: None,
            })
            .await
            .unwrap();
        let paid = app.invoices.get_invoice(issued.invoice.id).await.unwrap();
        assert_eq!(paid.invoice.status, InvoiceStatus::Paid);

        // Editing the invoice down to ₹1,000 must not touch the ₹2,000 payment
        // or flip status back — it stays PAID, now visibly overpaid.
        let edited = app
            .invoices
            .edit_issued_invoice(
                issued.invoice.id,
                DraftInvoiceInput {
                    customer_id: Some(customer.id),
                    invoice_date: issued.invoice.invoice_date,
                    due_date: None,
                    notes: None,
                    terms: None,
                    is_interstate: false,
                    discount_type: None,
                    discount_value: None,
                    line_items: vec![sample_line(1000, 100_000, 0)],
                },
            )
            .await
            .unwrap();

        assert_eq!(edited.invoice.total_minor, 100_000);
        assert_eq!(
            edited.invoice.status,
            InvoiceStatus::Paid,
            "editing an issued invoice must never touch status — only payments do"
        );
        let payments = app
            .payments
            .list_payments_for_invoice(issued.invoice.id)
            .await
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(
            payments[0].amount_minor, 200_000,
            "the payment record itself is untouched"
        );
    }

    #[tokio::test]
    async fn edit_issued_invoice_rejects_draft_and_cancelled_invoices() {
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
        let input = DraftInvoiceInput {
            customer_id: Some(customer.id),
            invoice_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
            due_date: None,
            notes: None,
            terms: None,
            is_interstate: false,
            discount_type: None,
            discount_value: None,
            line_items: vec![sample_line(1000, 100_000, 0)],
        };

        let draft = app
            .invoices
            .create_draft_invoice(input.clone())
            .await
            .unwrap();
        let draft_result = app
            .invoices
            .edit_issued_invoice(draft.invoice.id, input.clone())
            .await;
        assert!(matches!(draft_result, Err(ApplicationError::Validation(_))));

        let issued = app
            .invoices
            .issue_invoice(draft.invoice.id, None)
            .await
            .unwrap();
        app.invoices
            .cancel_invoice(issued.invoice.id, None)
            .await
            .unwrap();
        let cancelled_result = app
            .invoices
            .edit_issued_invoice(issued.invoice.id, input)
            .await;
        assert!(matches!(
            cancelled_result,
            Err(ApplicationError::Validation(_))
        ));
    }
}
