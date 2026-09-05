//! Vendor domain types. database-schema.md §11 (`vendors`) —
//! user-flows.md §3.

pub type VendorId = i64;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Vendor {
    pub id: VendorId,
    pub name: String,
    pub contact: Option<String>,
    pub notes: Option<String>,
}

/// What a create/update form submits — no `id`, no timestamps.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VendorFields {
    pub name: String,
    pub contact: Option<String>,
    pub notes: Option<String>,
}

/// A vendor plus whether it can be deleted — user-flows.md §3's blocked-delete
/// UX needs to know this without a second round trip, same shape as Billing's
/// `CustomerListItem`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VendorListItem {
    #[serde(flatten)]
    pub vendor: Vendor,
    pub has_expenses: bool,
}
