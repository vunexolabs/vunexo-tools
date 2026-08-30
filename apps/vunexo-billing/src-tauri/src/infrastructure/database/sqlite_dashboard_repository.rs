//! Implements `DashboardRepository` as `SUM`/`COUNT` queries, never as an
//! in-Rust reduction over a full invoice list (application-architecture.md
//! §3c). The `is_overdue` / `amount_paid` shapes mirror
//! `sqlite_invoice_repository.rs`'s `list()` exactly — same predicate,
//! same subquery — so "overdue" never means something subtly different
//! here than it does on the Invoices List.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::application::ports::dashboard_repository::DashboardRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::dashboard::OverdueSummary;
use crate::domain::invoice::{InvoiceStatus, InvoiceSummary};

pub struct SqliteDashboardRepository {
    pool: SqlitePool,
}

impl SqliteDashboardRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DashboardRepository for SqliteDashboardRepository {
    async fn today_sales(&self, today: NaiveDate) -> Result<i64, InfrastructureError> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(total_minor) FROM invoices \
             WHERE issued_at IS NOT NULL AND date(issued_at) = ? \
             AND status NOT IN ('DRAFT', 'CANCELLED')",
        )
        .bind(today)
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0))
    }

    async fn month_sales(&self, year: i32, month: u32) -> Result<i64, InfrastructureError> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(total_minor) FROM invoices \
             WHERE issued_at IS NOT NULL AND strftime('%Y-%m', issued_at) = ? \
             AND status NOT IN ('DRAFT', 'CANCELLED')",
        )
        .bind(format!("{year:04}-{month:02}"))
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0))
    }

    async fn outstanding_total(&self) -> Result<i64, InfrastructureError> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(i.total_minor - COALESCE(\
                (SELECT SUM(p.amount_minor) FROM payments p WHERE p.invoice_id = i.id), 0)) \
             FROM invoices i WHERE i.status IN ('ISSUED', 'PARTIALLY_PAID')",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0))
    }

    async fn paid_total(&self, year: i32, month: u32) -> Result<i64, InfrastructureError> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(total_minor) FROM invoices \
             WHERE status = 'PAID' AND issued_at IS NOT NULL AND strftime('%Y-%m', issued_at) = ?",
        )
        .bind(format!("{year:04}-{month:02}"))
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0))
    }

    async fn overdue_summary(
        &self,
        today: NaiveDate,
    ) -> Result<OverdueSummary, InfrastructureError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor - amount_paid), 0) AS total FROM ( \
                SELECT i.total_minor AS total_minor, \
                       COALESCE((SELECT SUM(p.amount_minor) FROM payments p WHERE p.invoice_id = i.id), 0) AS amount_paid \
                FROM invoices i \
                WHERE i.due_date IS NOT NULL AND i.due_date < ? AND i.status NOT IN ('DRAFT', 'CANCELLED') \
             ) sub WHERE (total_minor - amount_paid) > 0",
        )
        .bind(today)
        .fetch_one(&self.pool)
        .await?;
        Ok(OverdueSummary {
            count: row.get("cnt"),
            total_minor: row.get("total"),
        })
    }

    async fn recent_invoices(
        &self,
        limit: i64,
    ) -> Result<Vec<InvoiceSummary>, InfrastructureError> {
        let rows = sqlx::query(
            "SELECT \
                i.id, i.invoice_number, i.status, i.invoice_date, i.due_date, i.total_minor, \
                COALESCE(i.customer_snapshot_name, c.name) AS customer_name, \
                COALESCE((SELECT SUM(p.amount_minor) FROM payments p WHERE p.invoice_id = i.id), 0) AS amount_paid_minor, \
                (i.due_date IS NOT NULL AND i.due_date < date('now') AND i.status NOT IN ('DRAFT', 'CANCELLED') \
                 AND (i.total_minor - COALESCE((SELECT SUM(p2.amount_minor) FROM payments p2 WHERE p2.invoice_id = i.id), 0)) > 0 \
                ) AS is_overdue \
             FROM invoices i \
             LEFT JOIN customers c ON c.id = i.customer_id \
             ORDER BY i.invoice_date DESC, i.id DESC \
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| InvoiceSummary {
                id: row.get("id"),
                invoice_number: row.get("invoice_number"),
                status: InvoiceStatus::from_db_str(row.get::<String, _>("status").as_str()),
                customer_name: row.get("customer_name"),
                invoice_date: row.get("invoice_date"),
                due_date: row.get("due_date"),
                total_minor: row.get("total_minor"),
                amount_paid_minor: row.get("amount_paid_minor"),
                is_overdue: row.get::<bool, _>("is_overdue"),
            })
            .collect())
    }
}
