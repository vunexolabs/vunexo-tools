//! Product/service master data. database-schema.md §13 (`products`) —
//! application-architecture.md §2/§3b. Mirrors `domain::customer` exactly —
//! same ACTIVE/ARCHIVED lifecycle, same has_invoices-driven delete rule.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductStatus {
    Active,
    Archived,
}

impl ProductStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ARCHIVED" => ProductStatus::Archived,
            _ => ProductStatus::Active,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub unit: String,
    pub price_minor: i64,
    pub tax_rate_id: Option<i64>,
    pub hsn_sac_code: Option<String>,
    pub status: ProductStatus,
}

/// Fields a caller supplies when creating or fully replacing a product's
/// editable fields — see `domain::customer::CustomerFields`'s equivalent note.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProductFields {
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub unit: String,
    pub price_minor: i64,
    pub tax_rate_id: Option<i64>,
    pub hsn_sac_code: Option<String>,
}

/// `application-architecture.md` §3b's `ProductListItem` — `has_invoices`
/// computed in SQL (`EXISTS` against `invoice_line_items`), never by loading
/// invoices client-side.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProductListItem {
    #[serde(flatten)]
    pub product: Product,
    pub has_invoices: bool,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct ProductFilter {
    pub include_archived: bool,
}
