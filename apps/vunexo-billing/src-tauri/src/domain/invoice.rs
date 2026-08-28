//! Invoice domain types. Only `DiscountType` lands in this slice of Round 7 —
//! it's what `domain::calculation` needs; the full `Invoice`/`InvoiceStatus`
//! types (application-architecture.md §2) are implemented alongside the
//! repositories and use cases that operate on them.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscountType {
    Amount,
    Percentage,
}
