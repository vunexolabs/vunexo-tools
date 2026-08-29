//! Invoice line item domain types. database-schema.md §13
//! (`invoice_line_items`) — application-architecture.md §2.

use super::invoice::DiscountType;

/// The full read model — one row of `invoice_line_items`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvoiceLineItem {
    pub id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub unit: String,
    pub quantity_thousandths: i64,
    pub unit_price_minor: i64,
    pub line_discount_type: Option<DiscountType>,
    pub line_discount_value: Option<i64>,
    pub tax_rate_id: Option<i64>,
    pub tax_rate_basis_points: i64,
    pub line_subtotal_minor: i64,
    pub line_discount_amount_minor: i64,
    pub invoice_discount_amount_minor: i64,
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i64,
}

/// What the caller supplies for one line — the raw, pre-calculation shape.
/// Frozen (`description`/`unit`/`unit_price_minor`/`tax_rate_basis_points`)
/// the moment it's added to the invoice, per user-flows.md §5's "draft vs.
/// issue" rule — not read live through `product_id` at render time.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineItemInput {
    pub product_id: Option<i64>,
    pub description: String,
    pub unit: String,
    pub quantity_thousandths: i64,
    pub unit_price_minor: i64,
    pub line_discount_type: Option<DiscountType>,
    pub line_discount_value: Option<i64>,
    pub tax_rate_id: Option<i64>,
    pub tax_rate_basis_points: i64,
}

/// One line as `InvoiceRepository` actually persists it — `LineItemInput`'s
/// raw fields plus every `*_minor` value `domain::calculation::calculate_invoice`
/// computed for it, assembled by the use case before the repository ever
/// sees it (calculation-engine.md §4, application-architecture.md §4a).
#[derive(Debug, Clone)]
pub struct LineItemToSave {
    pub product_id: Option<i64>,
    pub description: String,
    pub unit: String,
    pub quantity_thousandths: i64,
    pub unit_price_minor: i64,
    pub line_discount_type: Option<DiscountType>,
    pub line_discount_value: Option<i64>,
    pub tax_rate_id: Option<i64>,
    pub tax_rate_basis_points: i64,
    pub line_subtotal_minor: i64,
    pub line_discount_amount_minor: i64,
    pub invoice_discount_amount_minor: i64,
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i64,
}
