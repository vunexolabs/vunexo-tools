//! Invoice domain types. database-schema.md §13 (`invoices`) —
//! application-architecture.md §2/§3b. The payment-driven `set_status`
//! path is implemented in `application/payments.rs`.

use chrono::{DateTime, NaiveDate, Utc};

use super::tax_regime::TaxRegimeCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    Amount,
    Percentage,
}

impl DiscountType {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "PERCENTAGE" => DiscountType::Percentage,
            _ => DiscountType::Amount,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            DiscountType::Amount => "AMOUNT",
            DiscountType::Percentage => "PERCENTAGE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    PartiallyPaid,
    Paid,
    Cancelled,
}

impl InvoiceStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ISSUED" => InvoiceStatus::Issued,
            "PARTIALLY_PAID" => InvoiceStatus::PartiallyPaid,
            "PAID" => InvoiceStatus::Paid,
            "CANCELLED" => InvoiceStatus::Cancelled,
            _ => InvoiceStatus::Draft,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "DRAFT",
            InvoiceStatus::Issued => "ISSUED",
            InvoiceStatus::PartiallyPaid => "PARTIALLY_PAID",
            InvoiceStatus::Paid => "PAID",
            InvoiceStatus::Cancelled => "CANCELLED",
        }
    }
}

/// The full read model — one row of `invoices`, database-schema.md §13.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: Option<String>,
    pub invoice_number_is_custom: bool,
    pub status: InvoiceStatus,
    pub customer_id: Option<i64>,

    pub customer_snapshot_name: Option<String>,
    pub customer_snapshot_phone: Option<String>,
    pub customer_snapshot_email: Option<String>,
    pub customer_snapshot_address: Option<String>,
    pub customer_snapshot_gstin: Option<String>,

    pub business_snapshot_name: Option<String>,
    pub business_snapshot_address: Option<String>,
    pub business_snapshot_gstin: Option<String>,
    pub business_snapshot_phone: Option<String>,
    pub business_snapshot_email: Option<String>,
    pub business_snapshot_bank_details: Option<String>,
    pub business_snapshot_upi_id: Option<String>,
    pub business_snapshot_logo_path: Option<String>,

    pub is_interstate: bool,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,

    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,

    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,

    pub issued_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,

    /// `application-architecture-v2.md` §3 — display/traceability only, never
    /// a live data dependency (`user-flows-v2.md` §3). `None` for any invoice
    /// not created via `ConvertQuoteToInvoice`.
    pub source_quote_id: Option<i64>,
    /// Frozen at Issue exactly like every other snapshot column — `None`
    /// while `Draft`, and also `None` on a pre-V2 issued invoice (normalized
    /// to `IN_GST` at the repository boundary, `domain::tax_regime`'s
    /// `normalize_legacy_snapshot` — application-architecture-v2.md §4b).
    pub tax_regime_snapshot: Option<TaxRegimeCode>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InvoiceWithLineItems {
    #[serde(flatten)]
    pub invoice: Invoice,
    pub line_items: Vec<super::invoice_line_item::InvoiceLineItem>,
}

/// One row for the Invoices list (ui-ux.md §5) — a purpose-built projection,
/// not the full `Invoice` — `is_overdue` computed exactly per
/// database-schema.md §8, never redefined here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvoiceSummary {
    pub id: i64,
    pub invoice_number: Option<String>,
    pub status: InvoiceStatus,
    pub customer_name: Option<String>,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub total_minor: i64,
    pub amount_paid_minor: i64,
    pub is_overdue: bool,
}

/// What the caller supplies when creating or fully replacing a draft's
/// content — `application/invoices.rs`'s `CreateDraftInvoice`/
/// `UpdateDraftInvoice` run this through `domain::calculation::calculate_invoice`
/// before anything is persisted.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DraftInvoiceInput {
    pub customer_id: Option<i64>,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub is_interstate: bool,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,
    pub line_items: Vec<super::invoice_line_item::LineItemInput>,
}

/// What `InvoiceRepository::create_draft`/`update_draft` actually persist —
/// `DraftInvoiceInput` plus the `InvoiceCalculationResult` the use case
/// already computed, assembled together so the repository never calls
/// `calculate_invoice` itself (application-architecture.md §4a).
#[derive(Debug, Clone)]
pub struct DraftInvoiceToSave {
    pub customer_id: Option<i64>,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub is_interstate: bool,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub line_items: Vec<super::invoice_line_item::LineItemToSave>,
}

/// The customer/business snapshot fields frozen at Issue
/// (user-flows.md §5, database-schema.md §4).
#[derive(Debug, Clone, Default)]
pub struct CustomerSnapshotFields {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub gstin: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BusinessSnapshotFields {
    pub name: Option<String>,
    pub address: Option<String>,
    pub gstin: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bank_details: Option<String>,
    pub upi_id: Option<String>,
    pub logo_path: Option<String>,
}

/// Everything `InvoiceRepository::issue` needs to write in one transaction
/// (application-architecture.md §4c): the generated/custom number, both
/// snapshots, and the freshly computed totals.
#[derive(Debug, Clone)]
pub struct IssueInvoiceData {
    pub invoice_number: String,
    pub invoice_number_is_custom: bool,
    pub customer_snapshot: CustomerSnapshotFields,
    pub business_snapshot: BusinessSnapshotFields,
    /// Frozen from `business.tax_regime_code` as read at this Issue — never
    /// `None` for a V2-issued invoice (application-architecture-v2.md §4b).
    pub tax_regime_snapshot: TaxRegimeCode,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub line_items: Vec<super::invoice_line_item::LineItemToSave>,
}

/// Everything `InvoiceRepository::update_issued` needs to write in one
/// transaction (application-architecture.md §4 "EditIssuedInvoice";
/// user-flows.md's "Editing an issued invoice" rule): the freshly
/// re-snapshotted customer/business data (re-read at *this* save, per that
/// rule — an explicit edit, not a customer record silently drifting) plus
/// recomputed totals and line items. Deliberately excludes
/// `invoice_number`/`invoice_number_is_custom`/`status` — none of those are
/// ever touched by an edit; the number is immutable once issued and status
/// is exclusively payment-driven (`application/payments.rs`).
#[derive(Debug, Clone)]
pub struct EditIssuedInvoiceData {
    pub customer_id: Option<i64>,
    pub customer_snapshot: CustomerSnapshotFields,
    pub business_snapshot: BusinessSnapshotFields,
    /// Re-read fresh at this edit, same "explicit edit re-snapshots current
    /// data" principle §4d already applies to `business_snapshot`.
    pub tax_regime_snapshot: TaxRegimeCode,
    pub is_interstate: bool,
    pub invoice_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub line_items: Vec<super::invoice_line_item::LineItemToSave>,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct InvoiceFilter {
    pub status: Option<InvoiceStatus>,
}
