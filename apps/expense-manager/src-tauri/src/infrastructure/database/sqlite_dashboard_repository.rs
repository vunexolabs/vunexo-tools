//! Implements `DashboardRepository` as `SUM`/`GROUP BY` queries, never as an
//! in-Rust reduction over a full expense list.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::application::ports::dashboard_repository::DashboardRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::dashboard::CategoryBreakdownRow;
use crate::domain::expense::Expense;
use crate::domain::money::MinorUnits;

pub struct SqliteDashboardRepository {
    pool: SqlitePool,
}

impl SqliteDashboardRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

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
impl DashboardRepository for SqliteDashboardRepository {
    async fn period_total(
        &self,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<i64, InfrastructureError> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(amount_minor) FROM expenses WHERE date >= ? AND date < ?",
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0))
    }

    /// Grouped by the *current* category (database-schema.md §4's "regroup
    /// by current category" reading) — a category rename mid-period simply
    /// shows under its current name, unlike Top Vendors' deliberately
    /// historical grouping.
    async fn category_breakdown(
        &self,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Vec<CategoryBreakdownRow>, InfrastructureError> {
        let rows = sqlx::query(
            "SELECT c.id AS category_id, c.name AS category_name, SUM(e.amount_minor) AS total_minor \
             FROM expenses e JOIN categories c ON c.id = e.category_id \
             WHERE e.date >= ? AND e.date < ? \
             GROUP BY c.id, c.name \
             ORDER BY total_minor DESC",
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| CategoryBreakdownRow {
                category_id: row.get("category_id"),
                category_name: row.get("category_name"),
                total_minor: row.get("total_minor"),
            })
            .collect())
    }

    async fn recent_expenses(&self, limit: i64) -> Result<Vec<Expense>, InfrastructureError> {
        let rows = sqlx::query(
            "SELECT id, date, amount_minor, tax_amount_minor, itc_eligible, deductible, \
             payment_method, notes, receipt_path, vendor_id, vendor_name_snapshot, category_id, \
             category_name_snapshot, created_at, updated_at \
             FROM expenses ORDER BY date DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(expense_from_row).collect())
    }
}
