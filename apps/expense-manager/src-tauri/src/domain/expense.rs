//! Expense domain types. database-schema.md §11 (`expenses`) —
//! application-architecture.md's domain types section.
//!
//! No status enum, no draft/issued lifecycle (user-flows.md §5 — "No
//! draft/issued state machine ... an expense is either recorded or it
//! isn't").

use chrono::{DateTime, NaiveDate, Utc};

use super::category::CategoryId;
use super::money::MinorUnits;
use super::vendor::VendorId;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Expense {
    pub id: i64,
    pub date: NaiveDate,
    pub amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub itc_eligible: bool,
    pub deductible: bool,
    pub payment_method: String,
    pub notes: Option<String>,
    pub receipt_path: Option<String>,
    pub vendor_id: Option<VendorId>,
    pub vendor_name_snapshot: Option<String>,
    pub category_id: CategoryId,
    pub category_name_snapshot: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What the Expense Editor submits, for both create and update
/// (application-architecture.md's `CreateExpense`/`UpdateExpense` note).
/// `deductible` arrives already resolved by the frontend — pre-filled from
/// the picked category's `default_deductible` and possibly overridden by the
/// user (ui-ux.md §4) — this backend never re-derives it from the category.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExpenseInput {
    pub date: NaiveDate,
    pub amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub itc_eligible: bool,
    pub deductible: bool,
    pub payment_method: String,
    pub notes: Option<String>,
    pub vendor_id: Option<VendorId>,
    pub category_id: CategoryId,
}

/// What a repository actually writes — `ExpenseInput` plus the snapshot
/// fields the use case layer has already resolved (application-architecture.md:
/// "resolves `vendor_name_snapshot`/`category_name_snapshot` by reading the
/// live vendor/category row once ... never re-read afterward"). Kept as its
/// own type so no repository implementation can accidentally derive a
/// snapshot itself from a live join.
#[derive(Debug, Clone)]
pub struct ExpenseToSave {
    pub date: NaiveDate,
    pub amount: MinorUnits,
    pub tax_amount: MinorUnits,
    pub itc_eligible: bool,
    pub deductible: bool,
    pub payment_method: String,
    pub notes: Option<String>,
    pub vendor_id: Option<VendorId>,
    pub vendor_name_snapshot: Option<String>,
    pub category_id: CategoryId,
    pub category_name_snapshot: String,
}

/// ui-ux.md §5 — Expenses List filters: category, vendor, and an (inclusive
/// start, exclusive end) date range, all optional and combinable.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct ExpenseFilter {
    pub category_id: Option<CategoryId>,
    pub vendor_id: Option<VendorId>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
}
