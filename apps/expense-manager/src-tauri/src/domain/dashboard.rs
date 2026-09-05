//! Dashboard aggregate types. user-flows.md §8 — "this period's total
//! spend, a category breakdown, a recent-expenses list." One small struct
//! per aggregate query the repository answers in SQL, never a `Vec<Expense>`
//! pulled into Rust and reduced there (same rule Billing's
//! `DashboardRepository` follows).

use super::expense::Expense;

/// A row grouped by the *current* category (joined on `category_id`, not
/// `category_name_snapshot`) — the dashboard shows where this period's money
/// is going right now, which is what "regroup by current category"
/// (database-schema.md §4) means for a live landing screen, unlike Top
/// Vendors' explicitly historical grouping (calculation-engine.md §5).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryBreakdownRow {
    pub category_id: i64,
    pub category_name: String,
    pub total_minor: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardMetrics {
    pub period_total_minor: i64,
    pub category_breakdown: Vec<CategoryBreakdownRow>,
    pub recent_expenses: Vec<Expense>,
}
