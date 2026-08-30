//! Payment domain types. database-schema.md §13 (`payments`), §8 (payment /
//! balance model) — payments are recorded independently of the invoice edit
//! lifecycle; the parent invoice's `status` is recalculated by
//! `application/payments.rs` whenever one is created, edited, or deleted,
//! never by a DB trigger (database-schema.md §8/§3).

use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethod {
    Cash,
    BankTransfer,
    Upi,
    Cheque,
    Other,
}

impl PaymentMethod {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "BANK_TRANSFER" => PaymentMethod::BankTransfer,
            "UPI" => PaymentMethod::Upi,
            "CHEQUE" => PaymentMethod::Cheque,
            "OTHER" => PaymentMethod::Other,
            _ => PaymentMethod::Cash,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            PaymentMethod::Cash => "CASH",
            PaymentMethod::BankTransfer => "BANK_TRANSFER",
            PaymentMethod::Upi => "UPI",
            PaymentMethod::Cheque => "CHEQUE",
            PaymentMethod::Other => "OTHER",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Payment {
    pub id: i64,
    pub invoice_id: i64,
    pub amount_minor: i64,
    pub method: PaymentMethod,
    pub paid_on: NaiveDate,
    pub reference: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// `RecordPayment`'s input (user-flows.md §6): amount, method, date,
/// optional reference, against a specific invoice.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NewPayment {
    pub invoice_id: i64,
    pub amount_minor: i64,
    pub method: PaymentMethod,
    pub paid_on: NaiveDate,
    pub reference: Option<String>,
}

/// `UpdatePayment`'s input — the same editable fields, minus `invoice_id`
/// (a payment is never moved to a different invoice).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaymentFields {
    pub amount_minor: i64,
    pub method: PaymentMethod,
    pub paid_on: NaiveDate,
    pub reference: Option<String>,
}
