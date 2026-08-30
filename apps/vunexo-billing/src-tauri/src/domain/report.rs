//! Report types. user-flows-v2.md §5 — exactly two named reports, not a
//! configurable report builder. Both are read models: nothing here is
//! persisted.

use super::tax_regime::TaxRegimeCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SalesGrouping {
    None,
    Product,
    Customer,
}

/// One grouped row (product or customer name) — empty when `SalesGrouping::None`,
/// where `total_sales_minor` alone is the answer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesSummaryRow {
    pub label: String,
    pub sales_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesSummaryResult {
    pub total_sales_minor: i64,
    pub rows: Vec<SalesSummaryRow>,
}

/// Tax collected, grouped by the regime it was actually snapshotted under —
/// database-schema-v2.md §7's mixed-regime edge case: never silently summed
/// across regimes when a range spans a switch.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxSummaryRow {
    pub tax_regime: TaxRegimeCode,
    pub tax_amount_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxSummaryResult {
    pub total_tax_minor: i64,
    pub by_regime: Vec<TaxSummaryRow>,
}
