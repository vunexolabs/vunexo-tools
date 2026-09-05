//! The single business profile. database-schema.md §11 (`business`, id
//! fixed at 1) — application-architecture.md's module layout.
//!
//! Independent of Vunexo Billing's own `Business` type — separate product,
//! separate data boundary (product-expense-manager.md's "Independent product
//! data boundary" principle) — this one carries no logo/GSTIN/tax-regime
//! fields, only what user-flows.md §1 actually asks for on the setup form.

pub const DEFAULT_CURRENCY_SYMBOL: &str = "₹";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Business {
    pub name: String,
    pub address: Option<String>,
    pub tax_info: Option<String>,
    pub currency_symbol: String,
}
