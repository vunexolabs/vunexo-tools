//! Implements `StatementRepository` per the exact query shape
//! database-schema-v2.md §7 specifies: opening balance = non-cancelled
//! invoices issued before `range_start` minus payments made before
//! `range_start`; closing balance = opening + in-range invoices - in-range
//! payments. This is the property that makes `closing(period N) ==
//! opening(period N+1)` hold by construction.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::statement_repository::StatementRepository;
use crate::domain::statement::{StatementEntry, StatementEntryKind, StatementResult};

pub struct SqliteStatementRepository {
    pool: SqlitePool,
}

impl SqliteStatementRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StatementRepository for SqliteStatementRepository {
    async fn customer_statement(
        &self,
        customer_id: i64,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<StatementResult, InfrastructureError> {
        let invoices_before: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_minor), 0) FROM invoices \
             WHERE customer_id = ? AND status != 'CANCELLED' AND issued_at IS NOT NULL \
             AND date(issued_at) < ?",
        )
        .bind(customer_id)
        .bind(range_start)
        .fetch_one(&self.pool)
        .await?;

        let payments_before: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(p.amount_minor), 0) FROM payments p \
             JOIN invoices i ON p.invoice_id = i.id \
             WHERE i.customer_id = ? AND p.paid_on < ?",
        )
        .bind(customer_id)
        .bind(range_start)
        .fetch_one(&self.pool)
        .await?;

        let opening_balance_minor = invoices_before - payments_before;

        let invoice_rows = sqlx::query(
            "SELECT invoice_number, date(issued_at) AS issued_date, total_minor FROM invoices \
             WHERE customer_id = ? AND status != 'CANCELLED' AND issued_at IS NOT NULL \
             AND date(issued_at) >= ? AND date(issued_at) < ? \
             ORDER BY issued_at",
        )
        .bind(customer_id)
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await?;

        let payment_rows = sqlx::query(
            "SELECT i.invoice_number, p.paid_on, p.amount_minor FROM payments p \
             JOIN invoices i ON p.invoice_id = i.id \
             WHERE i.customer_id = ? AND p.paid_on >= ? AND p.paid_on < ? \
             ORDER BY p.paid_on",
        )
        .bind(customer_id)
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await?;

        let mut invoices_in_range_total: i64 = 0;
        let mut entries: Vec<StatementEntry> = Vec::new();
        for row in &invoice_rows {
            let amount: i64 = row.get("total_minor");
            invoices_in_range_total += amount;
            entries.push(StatementEntry {
                date: row.get("issued_date"),
                kind: StatementEntryKind::Invoice,
                reference: row.get("invoice_number"),
                amount_minor: amount,
            });
        }

        let mut payments_in_range_total: i64 = 0;
        for row in &payment_rows {
            let amount: i64 = row.get("amount_minor");
            payments_in_range_total += amount;
            entries.push(StatementEntry {
                date: row.get("paid_on"),
                kind: StatementEntryKind::Payment,
                reference: row.get("invoice_number"),
                amount_minor: amount,
            });
        }

        // Stable sort: invoices were pushed before payments above, so a tie
        // on the same date keeps the invoice entry first — a deterministic,
        // sensible order (billed, then paid) rather than an arbitrary one.
        entries.sort_by_key(|e| e.date);

        let closing_balance_minor =
            opening_balance_minor + invoices_in_range_total - payments_in_range_total;

        Ok(StatementResult {
            opening_balance_minor,
            entries,
            closing_balance_minor,
        })
    }
}
