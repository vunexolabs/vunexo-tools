//! The single business profile. database-schema.md §13 (`business`, id
//! fixed at 1) — application-architecture.md §3b.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Business {
    pub name: String,
    pub logo_path: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub gstin: Option<String>,
    pub bank_details: Option<String>,
    pub upi_id: Option<String>,
}
