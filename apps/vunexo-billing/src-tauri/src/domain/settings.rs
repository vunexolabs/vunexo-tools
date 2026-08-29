//! App-level settings (single row, seeded with schema defaults at DB init —
//! never absent, unlike `Business`). database-schema.md §13 (`settings`) —
//! application-architecture.md §3b.

#[derive(Debug, Clone, serde::Serialize)]
pub struct Settings {
    pub country_code: String,
    pub currency_code: String,
    pub date_format: String,
    pub invoice_number_format: String,
    pub default_due_days: i64,
    pub default_tax_rate_id: Option<i64>,
}

/// Full-replace update input — see `domain::customer::CustomerFields`'s
/// equivalent note on why this isn't a sparse patch.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SettingsFields {
    pub country_code: String,
    pub currency_code: String,
    pub date_format: String,
    pub invoice_number_format: String,
    pub default_due_days: i64,
    pub default_tax_rate_id: Option<i64>,
}
