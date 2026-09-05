use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::expense_repository::ExpenseRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::expense::{Expense, ExpenseFilter, ExpenseToSave};
use crate::domain::money::MinorUnits;

pub struct SqliteExpenseRepository {
    pool: SqlitePool,
}

impl SqliteExpenseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn fetch_by_id(&self, id: i64) -> Result<Option<Expense>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM expenses WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(expense_from_row))
    }
}

const SELECT_COLUMNS: &str = "id, date, amount_minor, tax_amount_minor, itc_eligible, deductible, \
     payment_method, notes, receipt_path, vendor_id, vendor_name_snapshot, category_id, \
     category_name_snapshot, created_at, updated_at";

fn expense_from_row(row: &sqlx::sqlite::SqliteRow) -> Expense {
    Expense {
        id: row.get("id"),
        date: row.get("date"),
        amount: MinorUnits(row.get::<i64, _>("amount_minor")),
        tax_amount: MinorUnits(row.get::<i64, _>("tax_amount_minor")),
        itc_eligible: row.get("itc_eligible"),
        deductible: row.get("deductible"),
        payment_method: row.get("payment_method"),
        notes: row.get("notes"),
        receipt_path: row.get("receipt_path"),
        vendor_id: row.get("vendor_id"),
        vendor_name_snapshot: row.get("vendor_name_snapshot"),
        category_id: row.get("category_id"),
        category_name_snapshot: row.get("category_name_snapshot"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[async_trait]
impl ExpenseRepository for SqliteExpenseRepository {
    async fn create(&self, fields: ExpenseToSave) -> Result<Expense, InfrastructureError> {
        let id = sqlx::query(
            "INSERT INTO expenses \
             (date, amount_minor, tax_amount_minor, itc_eligible, deductible, payment_method, \
              notes, vendor_id, vendor_name_snapshot, category_id, category_name_snapshot) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(fields.date)
        .bind(fields.amount.as_i64())
        .bind(fields.tax_amount.as_i64())
        .bind(fields.itc_eligible)
        .bind(fields.deductible)
        .bind(&fields.payment_method)
        .bind(&fields.notes)
        .bind(fields.vendor_id)
        .bind(&fields.vendor_name_snapshot)
        .bind(fields.category_id)
        .bind(&fields.category_name_snapshot)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        self.fetch_by_id(id).await?.ok_or_else(|| {
            InfrastructureError::Database("expense not found immediately after insert".into())
        })
    }

    async fn update(&self, id: i64, fields: ExpenseToSave) -> Result<Expense, InfrastructureError> {
        sqlx::query(
            "UPDATE expenses SET date = ?, amount_minor = ?, tax_amount_minor = ?, itc_eligible = ?, \
             deductible = ?, payment_method = ?, notes = ?, vendor_id = ?, vendor_name_snapshot = ?, \
             category_id = ?, category_name_snapshot = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(fields.date)
        .bind(fields.amount.as_i64())
        .bind(fields.tax_amount.as_i64())
        .bind(fields.itc_eligible)
        .bind(fields.deductible)
        .bind(&fields.payment_method)
        .bind(&fields.notes)
        .bind(fields.vendor_id)
        .bind(&fields.vendor_name_snapshot)
        .bind(fields.category_id)
        .bind(&fields.category_name_snapshot)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.fetch_by_id(id).await?.ok_or_else(|| {
            InfrastructureError::Database("expense not found immediately after update".into())
        })
    }

    async fn delete(&self, id: i64) -> Result<(), InfrastructureError> {
        sqlx::query("DELETE FROM expenses WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Expense>, InfrastructureError> {
        self.fetch_by_id(id).await
    }

    async fn list(&self, filter: ExpenseFilter) -> Result<Vec<Expense>, InfrastructureError> {
        let mut sql = format!("SELECT {SELECT_COLUMNS} FROM expenses WHERE 1 = 1");
        if filter.category_id.is_some() {
            sql.push_str(" AND category_id = ?");
        }
        if filter.vendor_id.is_some() {
            sql.push_str(" AND vendor_id = ?");
        }
        if filter.date_from.is_some() {
            sql.push_str(" AND date >= ?");
        }
        if filter.date_to.is_some() {
            sql.push_str(" AND date < ?");
        }
        sql.push_str(" ORDER BY date DESC, id DESC");

        let mut query = sqlx::query(&sql);
        if let Some(category_id) = filter.category_id {
            query = query.bind(category_id);
        }
        if let Some(vendor_id) = filter.vendor_id {
            query = query.bind(vendor_id);
        }
        if let Some(date_from) = filter.date_from {
            query = query.bind(date_from);
        }
        if let Some(date_to) = filter.date_to {
            query = query.bind(date_to);
        }
        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(expense_from_row).collect())
    }

    async fn set_receipt_path(
        &self,
        id: i64,
        receipt_path: Option<String>,
    ) -> Result<(), InfrastructureError> {
        sqlx::query(
            "UPDATE expenses SET receipt_path = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(receipt_path)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
