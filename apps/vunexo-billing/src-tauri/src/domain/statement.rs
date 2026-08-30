//! Customer statement types. database-schema-v2.md §7 — a read model, not a
//! domain entity: nothing here is persisted, it's the shape one
//! `StatementRepository::customer_statement` query returns.

use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatementEntryKind {
    Invoice,
    Payment,
}

/// One chronological line — either an invoice issued in range (increases the
/// balance) or a payment recorded in range (decreases it). `amount_minor` is
/// always positive; `kind` says which direction it moves the balance,
/// consistent with `database-schema-v2.md` §7's opening/closing formula
/// rather than encoding sign into the value itself.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatementEntry {
    pub date: NaiveDate,
    pub kind: StatementEntryKind,
    /// Invoice number for an `Invoice` entry; the source invoice's number
    /// for a `Payment` entry (a payment has no number of its own).
    pub reference: Option<String>,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StatementResult {
    pub opening_balance_minor: i64,
    pub entries: Vec<StatementEntry>,
    pub closing_balance_minor: i64,
}
