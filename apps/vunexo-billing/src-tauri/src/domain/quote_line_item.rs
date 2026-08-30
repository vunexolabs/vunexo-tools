//! Quote line item domain types. database-schema-v2.md §4/§9
//! (`quote_line_items`) — mirrors `domain::invoice_line_item` field-for-field,
//! the one rename being `quote_discount_amount_minor` (this line's allocated
//! share of a *quote*-level discount, not an invoice-level one).

use super::invoice::DiscountType;

#[derive(Debug, Clone, serde::Serialize)]
pub struct QuoteLineItem {
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
    pub quote_discount_amount_minor: i64,
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i64,
}

/// What the caller supplies for one line — same shape as
/// `invoice_line_item::LineItemInput`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct QuoteLineItemInput {
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

/// One line as `QuoteRepository` actually persists it.
#[derive(Debug, Clone)]
pub struct QuoteLineItemToSave {
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
    pub quote_discount_amount_minor: i64,
    pub taxable_amount_minor: i64,
    pub line_tax_minor: i64,
    pub line_total_minor: i64,
    pub sort_order: i64,
}
