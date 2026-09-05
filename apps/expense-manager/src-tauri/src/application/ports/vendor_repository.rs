//! application-architecture.md's module layout.

use async_trait::async_trait;

use crate::domain::vendor::{Vendor, VendorFields, VendorId, VendorListItem};

use super::infrastructure_error::InfrastructureError;

#[async_trait]
pub trait VendorRepository: Send + Sync {
    async fn create(&self, fields: VendorFields) -> Result<Vendor, InfrastructureError>;
    async fn update(
        &self,
        id: VendorId,
        fields: VendorFields,
    ) -> Result<Vendor, InfrastructureError>;
    async fn delete(&self, id: VendorId) -> Result<(), InfrastructureError>;
    async fn find_by_id(&self, id: VendorId) -> Result<Option<Vendor>, InfrastructureError>;
    async fn list(&self) -> Result<Vec<VendorListItem>, InfrastructureError>;
    /// user-flows.md §3 — the check `DeleteVendor` calls before allowing a
    /// delete, same pattern as Billing's `has_invoices`.
    async fn has_expenses(&self, id: VendorId) -> Result<bool, InfrastructureError>;
}
