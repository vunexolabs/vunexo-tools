//! Customer master data. database-schema.md §13 (`customers`) —
//! application-architecture.md §2/§3b.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CustomerStatus {
    Active,
    Archived,
}

impl CustomerStatus {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "ARCHIVED" => CustomerStatus::Archived,
            _ => CustomerStatus::Active,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub gstin: Option<String>,
    pub status: CustomerStatus,
}

/// Fields a caller supplies when creating or fully replacing a customer's
/// editable fields (a form-based UI submits the whole form, not a sparse
/// patch — see `application-architecture.md` §4's `CustomerChanges` note).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CustomerFields {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub gstin: Option<String>,
}

/// `application-architecture.md` §3b's `CustomerListItem` — `has_invoices`
/// drives the archive-vs-delete UI decision (`ui-ux.md` §3) and must be
/// computed in SQL (`EXISTS`), never by loading invoices client-side. Also
/// true when a Quote (not just an Invoice) references this customer —
/// `quotes.customer_id` is `ON DELETE RESTRICT` too (migration 0002), so the
/// name is kept for API stability but the check spans both tables.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CustomerListItem {
    #[serde(flatten)]
    pub customer: Customer,
    pub has_invoices: bool,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct CustomerFilter {
    pub include_archived: bool,
}
