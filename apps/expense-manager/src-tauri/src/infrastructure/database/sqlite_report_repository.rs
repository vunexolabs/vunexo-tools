//! Implements `ReportRepository` — SQL aggregates only, calculation-engine.md
//! §2's rule ("computed in SQL, not summed in a Rust loop"). Every range is
//! `[range_start, range_end)`, matching Billing's own report/statement
//! convention.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::report_repository::ReportRepository;
use crate::domain::report::{
    CategorySummaryResult, CategorySummaryRow, DeductibleSummaryResult, PeriodSummaryResult,
    PeriodSummaryRow, TaxItcSummaryResult, TopVendorRow, TopVendorsResult,
};

pub struct SqliteReportRepository {
    pool: SqlitePool,
}

impl SqliteReportRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReportRepository for SqliteReportRepository {
    /// calculation-engine.md §7 Vector 1. Grouped by the *current* category,
    /// same "regroup by current category" rule the dashboard uses
    /// (database-schema.md §4) — deliberately the mirror image of Top
    /// Vendors' snapshot-based grouping below.
    async fn category_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<CategorySummaryResult, InfrastructureError> {
        let total_minor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM expenses WHERE date >= ? AND date < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            "SELECT c.id AS category_id, c.name AS category_name, SUM(e.amount_minor) AS total_minor \
             FROM expenses e JOIN categories c ON c.id = e.category_id \
             WHERE e.date >= ? AND e.date < ? \
             GROUP BY c.id, c.name \
             ORDER BY total_minor DESC",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await?;
        let rows = rows
            .iter()
            .map(|row| CategorySummaryRow {
                category_id: row.get("category_id"),
                category_name: row.get("category_name"),
                total_minor: row.get("total_minor"),
            })
            .collect();

        Ok(CategorySummaryResult { total_minor, rows })
    }

    /// Grouped by calendar month (`strftime('%Y-%m', date)`).
    async fn period_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<PeriodSummaryResult, InfrastructureError> {
        let total_minor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM expenses WHERE date >= ? AND date < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            "SELECT strftime('%Y-%m', date) AS period, SUM(amount_minor) AS total_minor \
             FROM expenses WHERE date >= ? AND date < ? \
             GROUP BY period ORDER BY period ASC",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await?;
        let rows = rows
            .iter()
            .map(|row| PeriodSummaryRow {
                period: row.get("period"),
                total_minor: row.get("total_minor"),
            })
            .collect();

        Ok(PeriodSummaryResult { total_minor, rows })
    }

    /// calculation-engine.md §3/§7 Vector 2 — reads `expenses.deductible`
    /// only, never `categories.default_deductible`.
    async fn deductible_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<DeductibleSummaryResult, InfrastructureError> {
        let row = sqlx::query(
            "SELECT \
                COALESCE(SUM(CASE WHEN deductible = 1 THEN amount_minor ELSE 0 END), 0) AS deductible_minor, \
                COALESCE(SUM(CASE WHEN deductible = 0 THEN amount_minor ELSE 0 END), 0) AS non_deductible_minor \
             FROM expenses WHERE date >= ? AND date < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;
        Ok(DeductibleSummaryResult {
            deductible_minor: row.get("deductible_minor"),
            non_deductible_minor: row.get("non_deductible_minor"),
        })
    }

    /// calculation-engine.md §4/§7 Vector 3 — two independent sums.
    async fn tax_itc_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxItcSummaryResult, InfrastructureError> {
        let row = sqlx::query(
            "SELECT \
                COALESCE(SUM(tax_amount_minor), 0) AS tax_paid_minor, \
                COALESCE(SUM(CASE WHEN itc_eligible = 1 THEN tax_amount_minor ELSE 0 END), 0) AS itc_eligible_minor \
             FROM expenses WHERE date >= ? AND date < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;
        Ok(TaxItcSummaryResult {
            tax_paid_minor: row.get("tax_paid_minor"),
            itc_eligible_minor: row.get("itc_eligible_minor"),
        })
    }

    /// calculation-engine.md §5/§7 Vector 4 — grouped by
    /// `vendor_name_snapshot`, deliberately not `vendor_id`. Expenses with no
    /// vendor picked are excluded — there is nothing to rank them under.
    async fn top_vendors(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        limit: i64,
    ) -> Result<TopVendorsResult, InfrastructureError> {
        let rows = sqlx::query(
            "SELECT vendor_name_snapshot, SUM(amount_minor) AS total_minor \
             FROM expenses \
             WHERE date >= ? AND date < ? AND vendor_name_snapshot IS NOT NULL \
             GROUP BY vendor_name_snapshot \
             ORDER BY total_minor DESC \
             LIMIT ?",
        )
        .bind(range_start)
        .bind(range_end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let rows = rows
            .iter()
            .map(|row| TopVendorRow {
                vendor_name_snapshot: row.get("vendor_name_snapshot"),
                total_minor: row.get("total_minor"),
            })
            .collect();
        Ok(TopVendorsResult { rows })
    }
}
