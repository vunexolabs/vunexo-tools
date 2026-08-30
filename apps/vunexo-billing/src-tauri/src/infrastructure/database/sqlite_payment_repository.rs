use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::payment_repository::PaymentRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::payment::{NewPayment, Payment, PaymentFields, PaymentMethod};

use super::transaction::sqlite_tx;

pub struct SqlitePaymentRepository {
    pool: SqlitePool,
}

impl SqlitePaymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str =
    "id, invoice_id, amount_minor, method, paid_on, reference, created_at, updated_at";

fn payment_from_row(row: &sqlx::sqlite::SqliteRow) -> Payment {
    Payment {
        id: row.get("id"),
        invoice_id: row.get("invoice_id"),
        amount_minor: row.get("amount_minor"),
        method: PaymentMethod::from_db_str(row.get::<String, _>("method").as_str()),
        paid_on: row.get("paid_on"),
        reference: row.get("reference"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[async_trait]
impl PaymentRepository for SqlitePaymentRepository {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        payment: NewPayment,
    ) -> Result<Payment, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = sqlx::query(
            "INSERT INTO payments (invoice_id, amount_minor, method, paid_on, reference) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(payment.invoice_id)
        .bind(payment.amount_minor)
        .bind(payment.method.as_db_str())
        .bind(payment.paid_on)
        .bind(&payment.reference)
        .execute(&mut **conn)
        .await?
        .last_insert_rowid();

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM payments WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(payment_from_row(&row))
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: PaymentFields,
    ) -> Result<Payment, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE payments SET amount_minor = ?, method = ?, paid_on = ?, reference = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(fields.amount_minor)
        .bind(fields.method.as_db_str())
        .bind(fields.paid_on)
        .bind(&fields.reference)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM payments WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(payment_from_row(&row))
    }

    async fn delete(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query("DELETE FROM payments WHERE id = ?")
            .bind(id)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }

    async fn get(&self, id: i64) -> Result<Option<Payment>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM payments WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(payment_from_row))
    }

    async fn list_for_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, InfrastructureError> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM payments WHERE invoice_id = ? ORDER BY paid_on, id"
        ))
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(payment_from_row).collect())
    }

    async fn sum_for_invoice(
        &self,
        tx: &mut dyn Transaction,
        invoice_id: i64,
    ) -> Result<i64, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let row = sqlx::query(
            "SELECT COALESCE(SUM(amount_minor), 0) AS total FROM payments WHERE invoice_id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(row.get::<i64, _>("total"))
    }
}
