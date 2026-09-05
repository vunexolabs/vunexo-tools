use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::vendor_repository::VendorRepository;
use crate::domain::vendor::{Vendor, VendorFields, VendorId, VendorListItem};

pub struct SqliteVendorRepository {
    pool: SqlitePool,
}

impl SqliteVendorRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn vendor_from_row(row: &sqlx::sqlite::SqliteRow) -> Vendor {
    Vendor {
        id: row.get("id"),
        name: row.get("name"),
        contact: row.get("contact"),
        notes: row.get("notes"),
    }
}

const SELECT_COLUMNS: &str = "id, name, contact, notes";

#[async_trait]
impl VendorRepository for SqliteVendorRepository {
    async fn create(&self, fields: VendorFields) -> Result<Vendor, InfrastructureError> {
        let id = sqlx::query("INSERT INTO vendors (name, contact, notes) VALUES (?, ?, ?)")
            .bind(&fields.name)
            .bind(&fields.contact)
            .bind(&fields.notes)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
        Ok(Vendor {
            id,
            name: fields.name,
            contact: fields.contact,
            notes: fields.notes,
        })
    }

    async fn update(
        &self,
        id: VendorId,
        fields: VendorFields,
    ) -> Result<Vendor, InfrastructureError> {
        sqlx::query(
            "UPDATE vendors SET name = ?, contact = ?, notes = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&fields.name)
        .bind(&fields.contact)
        .bind(&fields.notes)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(Vendor {
            id,
            name: fields.name,
            contact: fields.contact,
            notes: fields.notes,
        })
    }

    async fn delete(&self, id: VendorId) -> Result<(), InfrastructureError> {
        sqlx::query("DELETE FROM vendors WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: VendorId) -> Result<Option<Vendor>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM vendors WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(vendor_from_row))
    }

    async fn list(&self) -> Result<Vec<VendorListItem>, InfrastructureError> {
        let sql = format!(
            "SELECT v.{col}, EXISTS (SELECT 1 FROM expenses e WHERE e.vendor_id = v.id) AS has_expenses \
             FROM vendors v ORDER BY v.name",
            col = SELECT_COLUMNS.replace(", ", ", v."),
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| VendorListItem {
                vendor: vendor_from_row(row),
                has_expenses: row.get::<bool, _>("has_expenses"),
            })
            .collect())
    }

    async fn has_expenses(&self, id: VendorId) -> Result<bool, InfrastructureError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM expenses WHERE vendor_id = ?)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }
}
