//! Tax rate domain types. database-schema.md §13 (`tax_rates`), §6 (tax
//! representation) — small, user-maintained master data (a handful of GST
//! slabs); no delete in V1 (application-architecture.md §3b/§4).

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxRate {
    pub id: i64,
    pub name: String,
    pub rate_basis_points: i64,
}

/// Full-replace input, same shape for create and update — see
/// `domain::customer::CustomerFields`'s equivalent note on why this isn't a
/// sparse patch.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TaxRateFields {
    pub name: String,
    pub rate_basis_points: i64,
}
