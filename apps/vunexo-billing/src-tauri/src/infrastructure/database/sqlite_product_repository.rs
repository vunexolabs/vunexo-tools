//! Mirrors sqlite_customer_repository.rs exactly.

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::product_repository::ProductRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::product::{
    Product, ProductFields, ProductFilter, ProductListItem, ProductStatus,
};

use super::transaction::sqlite_tx;

pub struct SqliteProductRepository {
    pool: SqlitePool,
}

impl SqliteProductRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn product_from_row(row: &sqlx::sqlite::SqliteRow) -> Product {
    Product {
        id: row.get("id"),
        name: row.get("name"),
        sku: row.get("sku"),
        description: row.get("description"),
        unit: row.get("unit"),
        price_minor: row.get("price_minor"),
        tax_rate_id: row.get("tax_rate_id"),
        hsn_sac_code: row.get("hsn_sac_code"),
        status: ProductStatus::from_db_str(row.get::<String, _>("status").as_str()),
    }
}

const SELECT_COLUMNS: &str =
    "id, name, sku, description, unit, price_minor, tax_rate_id, hsn_sac_code, status";

#[async_trait]
impl ProductRepository for SqliteProductRepository {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: ProductFields,
    ) -> Result<Product, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = sqlx::query(
            "INSERT INTO products (name, sku, description, unit, price_minor, tax_rate_id, hsn_sac_code) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&fields.name)
        .bind(&fields.sku)
        .bind(&fields.description)
        .bind(&fields.unit)
        .bind(fields.price_minor)
        .bind(fields.tax_rate_id)
        .bind(&fields.hsn_sac_code)
        .execute(&mut **conn)
        .await?
        .last_insert_rowid();

        Ok(Product {
            id,
            name: fields.name,
            sku: fields.sku,
            description: fields.description,
            unit: fields.unit,
            price_minor: fields.price_minor,
            tax_rate_id: fields.tax_rate_id,
            hsn_sac_code: fields.hsn_sac_code,
            status: ProductStatus::Active,
        })
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: ProductFields,
    ) -> Result<Product, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE products SET name = ?, sku = ?, description = ?, unit = ?, price_minor = ?, \
             tax_rate_id = ?, hsn_sac_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(&fields.name)
        .bind(&fields.sku)
        .bind(&fields.description)
        .bind(&fields.unit)
        .bind(fields.price_minor)
        .bind(fields.tax_rate_id)
        .bind(&fields.hsn_sac_code)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM products WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(product_from_row(&row))
    }

    async fn archive(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE products SET status = 'ARCHIVED', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn restore(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE products SET status = 'ACTIVE', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query("DELETE FROM products WHERE id = ?")
            .bind(id)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Product>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM products WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(product_from_row))
    }

    async fn list(
        &self,
        filter: ProductFilter,
    ) -> Result<Vec<ProductListItem>, InfrastructureError> {
        let where_clause = if filter.include_archived {
            ""
        } else {
            "WHERE p.status = 'ACTIVE'"
        };
        // A product referenced only from a Quote's line items (never
        // invoiced) has no row in `invoice_line_items`, but
        // `quote_line_items.product_id` is `ON DELETE RESTRICT` (migration
        // 0002) exactly like `invoice_line_items.product_id` — so this flag,
        // which the UI uses to decide whether Delete is even offered, must
        // check both tables or it offers a delete the database will then
        // refuse.
        let sql = format!(
            "SELECT p.{col}, \
             (EXISTS (SELECT 1 FROM invoice_line_items li WHERE li.product_id = p.id) \
              OR EXISTS (SELECT 1 FROM quote_line_items qli WHERE qli.product_id = p.id)) AS has_invoices \
             FROM products p {where_clause} ORDER BY p.name",
            col = SELECT_COLUMNS.replace(", ", ", p."),
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| ProductListItem {
                product: product_from_row(row),
                has_invoices: row.get::<bool, _>("has_invoices"),
            })
            .collect())
    }
}
