use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::customer_repository::CustomerRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::transaction::Transaction;
use crate::domain::customer::{
    Customer, CustomerFields, CustomerFilter, CustomerListItem, CustomerStatus,
};

use super::transaction::sqlite_tx;

pub struct SqliteCustomerRepository {
    pool: SqlitePool,
}

impl SqliteCustomerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn customer_from_row(row: &sqlx::sqlite::SqliteRow) -> Customer {
    Customer {
        id: row.get("id"),
        name: row.get("name"),
        phone: row.get("phone"),
        email: row.get("email"),
        address: row.get("address"),
        gstin: row.get("gstin"),
        status: CustomerStatus::from_db_str(row.get::<String, _>("status").as_str()),
    }
}

const SELECT_COLUMNS: &str = "id, name, phone, email, address, gstin, status";

#[async_trait]
impl CustomerRepository for SqliteCustomerRepository {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: CustomerFields,
    ) -> Result<Customer, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = sqlx::query(
            "INSERT INTO customers (name, phone, email, address, gstin) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&fields.name)
        .bind(&fields.phone)
        .bind(&fields.email)
        .bind(&fields.address)
        .bind(&fields.gstin)
        .execute(&mut **conn)
        .await?
        .last_insert_rowid();

        Ok(Customer {
            id,
            name: fields.name,
            phone: fields.phone,
            email: fields.email,
            address: fields.address,
            gstin: fields.gstin,
            status: CustomerStatus::Active,
        })
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: CustomerFields,
    ) -> Result<Customer, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, gstin = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(&fields.name)
        .bind(&fields.phone)
        .bind(&fields.email)
        .bind(&fields.address)
        .bind(&fields.gstin)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM customers WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(customer_from_row(&row))
    }

    async fn archive(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE customers SET status = 'ARCHIVED', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn restore(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE customers SET status = 'ACTIVE', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query("DELETE FROM customers WHERE id = ?")
            .bind(id)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Customer>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM customers WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(customer_from_row))
    }

    async fn list(
        &self,
        filter: CustomerFilter,
    ) -> Result<Vec<CustomerListItem>, InfrastructureError> {
        let where_clause = if filter.include_archived {
            ""
        } else {
            "WHERE c.status = 'ACTIVE'"
        };
        let sql = format!(
            "SELECT c.{col}, \
             EXISTS (SELECT 1 FROM invoices i WHERE i.customer_id = c.id) AS has_invoices \
             FROM customers c {where_clause} ORDER BY c.name",
            col = SELECT_COLUMNS.replace(", ", ", c."),
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| CustomerListItem {
                customer: customer_from_row(row),
                has_invoices: row.get::<bool, _>("has_invoices"),
            })
            .collect())
    }
}
