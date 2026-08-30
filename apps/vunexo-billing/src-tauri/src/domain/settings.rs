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
    /// database-schema-v2.md §6 — locks read-only after the first issued
    /// quote, same rule and same independence from `invoice_number_format`
    /// as the existing lock (`QuoteUseCases`/`InvoiceUseCases` each check
    /// their own document type's issuance history).
    pub quote_number_format: String,
    /// application-architecture-v2.md §3 — `None` means "use the built-in
    /// default template," never a row that has to exist before the payment
    /// reminder feature works.
    pub payment_reminder_template: Option<String>,
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
    /// `serde(default)` so the not-yet-updated V1 frontend (which doesn't
    /// send these fields) keeps working at the Tauri command boundary.
    #[serde(default = "default_quote_number_format")]
    pub quote_number_format: String,
    #[serde(default)]
    pub payment_reminder_template: Option<String>,
}

fn default_quote_number_format() -> String {
    "QUO-{year}-{seq:04d}".to_string()
}
