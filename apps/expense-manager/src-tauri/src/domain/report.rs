//! Report types. calculation-engine.md — exactly five named reports, not a
//! configurable report builder. Every field here is a plain integer sum;
//! nothing in this module performs arithmetic itself (that's SQL's job, in
//! `infrastructure::database::sqlite_report_repository`) — these are read
//! models only.

/// Category Summary (calculation-engine.md §7, Vector 1) — grouped by the
/// *current* category, same "regroup by current category" rule the
/// dashboard's breakdown uses (database-schema.md §4), since a category
/// rename should not fragment this report the way a vendor rename
/// deliberately does fragment Top Vendors.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategorySummaryRow {
    pub category_id: i64,
    pub category_name: String,
    pub total_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CategorySummaryResult {
    pub total_minor: i64,
    pub rows: Vec<CategorySummaryRow>,
}

/// Period Summary — grouped by calendar month (`YYYY-MM`), the coarsest
/// grain that stays useful across an arbitrarily long date range without a
/// separate "granularity" control the locked docs never asked for.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PeriodSummaryRow {
    pub period: String,
    pub total_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PeriodSummaryResult {
    pub total_minor: i64,
    pub rows: Vec<PeriodSummaryRow>,
}

/// Deductible / Non-Deductible Summary (calculation-engine.md §3/§7 Vector 2)
/// — reads `expenses.deductible` only, never recomputed from a category's
/// current `default_deductible`.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct DeductibleSummaryResult {
    pub deductible_minor: i64,
    pub non_deductible_minor: i64,
}

/// Tax / ITC Summary (calculation-engine.md §4/§7 Vector 3) — two
/// independent sums, never one field standing in for both facts.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct TaxItcSummaryResult {
    pub tax_paid_minor: i64,
    pub itc_eligible_minor: i64,
}

/// Top Vendors (calculation-engine.md §5/§7 Vector 4) — grouped by
/// `vendor_name_snapshot`, deliberately **not** `vendor_id`: a vendor renamed
/// partway through the range must show as two rows, one per name snapshot
/// that was current when each expense was recorded. Not a bug to "fix" by
/// grouping on the live name instead.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopVendorRow {
    pub vendor_name_snapshot: String,
    pub total_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TopVendorsResult {
    pub rows: Vec<TopVendorRow>,
}
