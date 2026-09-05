//! Category domain types. database-schema.md §11 (`categories`) —
//! user-flows.md §4.
//!
//! `default_deductible` is only ever a *default* applied when an expense is
//! first created (Round 1/2's historical-immutability principle) — nothing
//! in this module, or anywhere that reads an existing `Expense`, may re-read
//! it to decide what an already-recorded expense's deductibility is.

pub type CategoryId = i64;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub default_deductible: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CategoryFields {
    pub name: String,
    pub default_deductible: bool,
}

/// A category plus whether it can be deleted — same shape as `VendorListItem`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryListItem {
    #[serde(flatten)]
    pub category: Category,
    pub has_expenses: bool,
}

/// The starter category set seeded on first run (user-flows.md §4 — "exact
/// list finalized in implementation, not frozen here"). Chosen to cover the
/// common small-business expense shapes this product's user base (retail
/// shops, freelancers, service providers, contractors, traders, small
/// agencies, independent professionals — product-expense-manager.md's
/// Vision) actually incurs, with a reasonable-judgment default deductibility
/// each — the user can override any of them, per category and per expense.
pub const STARTER_CATEGORIES: &[(&str, bool)] = &[
    ("Rent", true),
    ("Utilities", true),
    ("Office Supplies", true),
    ("Travel", true),
    ("Professional Fees", true),
    ("Software/Subscriptions", true),
    ("Marketing", true),
    ("Miscellaneous", false),
];
