//! Implements `ReportRepository` — SQL aggregates only, same rule as
//! `DashboardRepository`/`StatementRepository`. `Draft`/`Cancelled`
//! invoices never contribute, same discipline as the dashboard metrics.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::report_repository::ReportRepository;
use crate::domain::report::{
    SalesGrouping, SalesSummaryResult, SalesSummaryRow, TaxSummaryResult, TaxSummaryRow,
};
use crate::domain::tax_regime::normalize_legacy_snapshot;

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
    async fn sales_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        group_by: SalesGrouping,
    ) -> Result<SalesSummaryResult, InfrastructureError> {
        let total_sales_minor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_minor), 0) FROM invoices \
             WHERE status != 'CANCELLED' AND issued_at IS NOT NULL \
             AND date(issued_at) >= ? AND date(issued_at) < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;

        let rows = match group_by {
            SalesGrouping::None => Vec::new(),
            SalesGrouping::Product => {
                let rows = sqlx::query(
                    "SELECT COALESCE(pr.name, li.description) AS label, \
                            SUM(li.line_total_minor) AS sales_minor \
                     FROM invoice_line_items li \
                     JOIN invoices i ON i.id = li.invoice_id \
                     LEFT JOIN products pr ON pr.id = li.product_id \
                     WHERE i.status != 'CANCELLED' AND i.issued_at IS NOT NULL \
                     AND date(i.issued_at) >= ? AND date(i.issued_at) < ? \
                     GROUP BY label \
                     ORDER BY sales_minor DESC",
                )
                .bind(range_start)
                .bind(range_end)
                .fetch_all(&self.pool)
                .await?;
                rows.iter()
                    .map(|row| SalesSummaryRow {
                        label: row.get("label"),
                        sales_minor: row.get("sales_minor"),
                    })
                    .collect()
            }
            SalesGrouping::Customer => {
                let rows = sqlx::query(
                    "SELECT COALESCE(i.customer_snapshot_name, c.name, 'Unknown') AS label, \
                            SUM(i.total_minor) AS sales_minor \
                     FROM invoices i \
                     LEFT JOIN customers c ON c.id = i.customer_id \
                     WHERE i.status != 'CANCELLED' AND i.issued_at IS NOT NULL \
                     AND date(i.issued_at) >= ? AND date(i.issued_at) < ? \
                     GROUP BY label \
                     ORDER BY sales_minor DESC",
                )
                .bind(range_start)
                .bind(range_end)
                .fetch_all(&self.pool)
                .await?;
                rows.iter()
                    .map(|row| SalesSummaryRow {
                        label: row.get("label"),
                        sales_minor: row.get("sales_minor"),
                    })
                    .collect()
            }
        };

        Ok(SalesSummaryResult {
            total_sales_minor,
            rows,
        })
    }

    async fn tax_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxSummaryResult, InfrastructureError> {
        let total_tax_minor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(tax_amount_minor), 0) FROM invoices \
             WHERE status != 'CANCELLED' AND issued_at IS NOT NULL \
             AND date(issued_at) >= ? AND date(issued_at) < ?",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            "SELECT tax_regime_snapshot, SUM(tax_amount_minor) AS tax_amount_minor FROM invoices \
             WHERE status != 'CANCELLED' AND issued_at IS NOT NULL \
             AND date(issued_at) >= ? AND date(issued_at) < ? \
             GROUP BY tax_regime_snapshot",
        )
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await?;

        let by_regime = rows
            .iter()
            .map(|row| {
                let raw: Option<String> = row.get("tax_regime_snapshot");
                TaxSummaryRow {
                    tax_regime: normalize_legacy_snapshot(raw.as_deref()),
                    tax_amount_minor: row.get("tax_amount_minor"),
                }
            })
            .collect();

        Ok(TaxSummaryResult {
            total_tax_minor,
            by_regime,
        })
    }
}
