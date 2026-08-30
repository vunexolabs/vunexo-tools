//! Quote domain types. database-schema-v2.md §3/§4/§9 (`quotes`) —
//! application-architecture-v2.md §1/§2. Mirrors `domain::invoice` closely;
//! see that module for the shared snapshot-field shapes
//! (`CustomerSnapshotFields`/`BusinessSnapshotFields`, reused as-is).

use chrono::{DateTime, NaiveDate, Utc};

use super::invoice::{BusinessSnapshotFields, CustomerSnapshotFields, DiscountType};
use super::quote_line_item::{QuoteLineItem, QuoteLineItemInput, QuoteLineItemToSave};
use super::tax_regime::TaxRegimeCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuoteStatus {
    Draft,
    Issued,
    Accepted,
    Declined,
    Converted,
    Cancelled,
}

impl QuoteStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ISSUED" => QuoteStatus::Issued,
            "ACCEPTED" => QuoteStatus::Accepted,
            "DECLINED" => QuoteStatus::Declined,
            "CONVERTED" => QuoteStatus::Converted,
            "CANCELLED" => QuoteStatus::Cancelled,
            _ => QuoteStatus::Draft,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            QuoteStatus::Draft => "DRAFT",
            QuoteStatus::Issued => "ISSUED",
            QuoteStatus::Accepted => "ACCEPTED",
            QuoteStatus::Declined => "DECLINED",
            QuoteStatus::Converted => "CONVERTED",
            QuoteStatus::Cancelled => "CANCELLED",
        }
    }
}

/// The full read model — one row of `quotes`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Quote {
    pub id: i64,
    pub quote_number: Option<String>,
    pub status: QuoteStatus,
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

    pub tax_regime_snapshot: Option<TaxRegimeCode>,
    pub is_interstate: bool,

    pub quote_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,

    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,

    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,

    pub issued_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub declined_at: Option<DateTime<Utc>>,
    pub converted_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QuoteWithLineItems {
    #[serde(flatten)]
    pub quote: Quote,
    pub line_items: Vec<QuoteLineItem>,
}

/// One row for the Quotes list (ui-ux-v2.md §2) — mirrors `InvoiceSummary`.
/// No `is_overdue`-equivalent here; `is_expired` is derived the same way but
/// is cheap enough to compute in the use case from `valid_until` + `status`
/// rather than needing its own SQL predicate baked into every list row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuoteSummary {
    pub id: i64,
    pub quote_number: Option<String>,
    pub status: QuoteStatus,
    pub customer_name: Option<String>,
    pub quote_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub total_minor: i64,
    pub is_expired: bool,
}

/// What the caller supplies when creating or fully replacing a draft's
/// content — mirrors `DraftInvoiceInput`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DraftQuoteInput {
    pub customer_id: Option<i64>,
    pub quote_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub is_interstate: bool,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,
    pub line_items: Vec<QuoteLineItemInput>,
}

/// What `QuoteRepository::create_draft`/`update_draft` actually persist.
#[derive(Debug, Clone)]
pub struct DraftQuoteToSave {
    pub customer_id: Option<i64>,
    pub quote_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub is_interstate: bool,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<i64>,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub line_items: Vec<QuoteLineItemToSave>,
}

/// Everything `QuoteRepository::issue` needs to write in one transaction —
/// mirrors `IssueInvoiceData`.
#[derive(Debug, Clone)]
pub struct IssueQuoteData {
    pub quote_number: String,
    pub customer_snapshot: CustomerSnapshotFields,
    pub business_snapshot: BusinessSnapshotFields,
    pub tax_regime_snapshot: TaxRegimeCode,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub line_items: Vec<QuoteLineItemToSave>,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct QuoteFilter {
    pub status: Option<QuoteStatus>,
}
